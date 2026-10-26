//! The app's view of the sky. One `SkyHandle` per app, polled every ~15 s
//! while the view is visible. Everything here converts core types into plain
//! values for Dart; the logic lives in passingover-core.

use std::time::{Instant, SystemTime};

use flutter_rust_bridge::frb;
use passingover_core::data::StandingData;
use passingover_core::flight::{local_time, Flight};
use passingover_core::geo::{Coordinate, Radius};
use passingover_core::http::{user_agent, UreqClient};
use passingover_core::position::{
    choose_primary, AdsbFi, AdsbLol, DailyUsage, FetchError, PositionSource, SourceId,
};
use passingover_core::regions::Regions;
use passingover_core::story::Units;
use passingover_core::{credits, story, Overhead, Sky};

/// An aircraft position source (D17).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    AdsbLol,
    AdsbFi,
}

impl From<Source> for SourceId {
    fn from(s: Source) -> Self {
        match s {
            Source::AdsbLol => SourceId::AdsbLol,
            Source::AdsbFi => SourceId::AdsbFi,
        }
    }
}

impl From<SourceId> for Source {
    fn from(s: SourceId) -> Self {
        match s {
            SourceId::AdsbLol => Source::AdsbLol,
            SourceId::AdsbFi => Source::AdsbFi,
        }
    }
}

/// Position requests sent today (D15), saved by the app between runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Usage {
    /// Days since the Unix epoch, UTC.
    pub day: u32,
    pub adsb_lol: u32,
    pub adsb_fi: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlightView {
    /// Stable per aircraft (ICAO address), for list animations.
    pub key: String,
    /// `SK 945`, a callsign, or a description.
    pub title: String,
    /// Destination city, e.g. `Chicago`.
    pub destination: Option<String>,
    pub origin: Option<String>,
    /// Wall-clock time at the destination, `HH:MM`; not set when it lands
    /// nearby.
    pub local_time: Option<String>,
    /// `9°C and overcast`; not set when it lands nearby.
    pub weather: Option<String>,
    /// `about 7 h 40 min`.
    pub landing_in: Option<String>,
    /// The destination is near the user.
    pub lands_nearby: bool,
    /// "Left Stockholm about 1 h 10 min ago and will cross …".
    pub journey: Option<String>,
    /// The whole story in one sentence.
    pub line: String,
    pub distance_km: f64,
    pub altitude_m: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credit {
    pub text: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NowView {
    /// Nearest first.
    pub flights: Vec<FlightView>,
    /// The sources used for this view, to credit next to it.
    pub credits: Vec<Credit>,
    pub source: Source,
    /// The source's timestamp for the positions.
    pub updated_unix_ms: i64,
}

/// A view, or why there isn't one.
#[derive(Debug, Clone, PartialEq)]
pub struct PollResult {
    pub view: Option<NowView>,
    pub error: Option<String>,
    /// When every source is resting: seconds until one is available.
    pub retry_after_secs: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataInfo {
    pub routes: u32,
    pub airports: u32,
}

/// How often to poll (D15): every `interval_ms` plus or minus `jitter_ms`
/// (`ambient_interval_ms` in ambient mode), never more often than
/// `min_interval_ms`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PollTiming {
    pub interval_ms: u32,
    pub ambient_interval_ms: u32,
    pub jitter_ms: u32,
    pub min_interval_ms: u32,
}

#[frb(sync)]
pub fn poll_timing() -> PollTiming {
    use passingover_core::polite::{
        AMBIENT_POLL_INTERVAL, MIN_POLL_INTERVAL, POLL_INTERVAL, POLL_JITTER,
    };
    PollTiming {
        interval_ms: POLL_INTERVAL.as_millis() as u32,
        ambient_interval_ms: AMBIENT_POLL_INTERVAL.as_millis() as u32,
        jitter_ms: POLL_JITTER.as_millis() as u32,
        min_interval_ms: MIN_POLL_INTERVAL.as_millis() as u32,
    }
}

/// Pick this install's primary source from a uniform `roll` in 0..1 (D17).
#[frb(sync)]
pub fn choose_primary_source(roll: f64) -> Source {
    choose_primary(roll).into()
}

/// Every data source, for "About the data".
#[frb(sync)]
pub fn data_credits() -> Vec<Credit> {
    credits::all().into_iter().map(credit).collect()
}

#[frb(opaque)]
pub struct SkyHandle {
    sky: Sky,
    client: UreqClient,
}

impl SkyHandle {
    #[frb(sync)]
    pub fn new(primary: Source, seed: u32) -> SkyHandle {
        SkyHandle {
            sky: Sky::new(primary.into(), seed.into()),
            client: UreqClient::new(&user_agent("PassingOver")),
        }
    }

    /// Load the bundled standing data (gzipped tables, D29).
    pub fn load_data(
        &mut self,
        routes_gz: Vec<u8>,
        airports_gz: Vec<u8>,
        airlines_gz: Vec<u8>,
    ) -> Result<DataInfo, String> {
        let data = StandingData::from_gzip(&routes_gz, &airports_gz, &airlines_gz)
            .map_err(|e| e.to_string())?;
        let info = DataInfo {
            routes: data.route_count() as u32,
            airports: data.airport_count() as u32,
        };
        self.sky.set_data(data);
        Ok(info)
    }

    /// Load the bundled countries and seas (D33).
    pub fn load_regions(&mut self, regions_gz: Vec<u8>) -> Result<(), String> {
        let regions = Regions::from_gzip(&regions_gz).map_err(|e| e.to_string())?;
        self.sky.set_regions(regions);
        Ok(())
    }

    #[frb(sync)]
    pub fn primary(&self) -> Source {
        self.sky.primary().into()
    }

    #[frb(sync)]
    pub fn usage(&self) -> Usage {
        let u = self.sky.usage();
        Usage {
            day: u.day,
            adsb_lol: u.adsb_lol,
            adsb_fi: u.adsb_fi,
        }
    }

    #[frb(sync)]
    pub fn set_usage(&mut self, usage: Usage) {
        self.sky.set_usage(DailyUsage {
            day: usage.day,
            adsb_lol: usage.adsb_lol,
            adsb_fi: usage.adsb_fi,
        });
    }

    /// Fetch what is overhead. `lat`/`lon` stay on the device; only a
    /// rounded point (~1 km) is sent.
    pub fn poll(&mut self, lat: f64, lon: f64, radius_km: f64, imperial: bool) -> PollResult {
        let (here, radius) = match place(lat, lon, radius_km) {
            Ok(p) => p,
            Err(error) => return failure(error, None),
        };
        let wall = SystemTime::now();
        match self
            .sky
            .poll(&self.client, here, radius, wall, Instant::now())
        {
            Ok(overhead) => PollResult {
                view: Some(view(&overhead, wall, units(imperial))),
                error: None,
                retry_after_secs: None,
            },
            Err(FetchError::Unavailable { retry_after }) => failure(
                "No aircraft data source is available right now.".into(),
                retry_after.map(|d| d.as_secs().min(u32::MAX.into()) as u32),
            ),
            Err(e) => failure(e.to_string(), None),
        }
    }

    /// The view for a recorded source response, without any network: for
    /// tests and offline demos.
    pub fn view_from_snapshot(
        &mut self,
        source: Source,
        body: Vec<u8>,
        lat: f64,
        lon: f64,
        radius_km: f64,
        imperial: bool,
    ) -> Result<NowView, String> {
        let (here, radius) = place(lat, lon, radius_km)?;
        let snapshot = match SourceId::from(source) {
            SourceId::AdsbLol => AdsbLol.parse(&body),
            SourceId::AdsbFi => AdsbFi.parse(&body),
        }
        .map_err(|e| e.to_string())?;
        let overhead = self
            .sky
            .describe(&snapshot, here, radius, None, Instant::now());
        Ok(view(&overhead, snapshot.time, units(imperial)))
    }
}

fn units(imperial: bool) -> Units {
    if imperial {
        Units::Imperial
    } else {
        Units::Metric
    }
}

fn place(lat: f64, lon: f64, radius_km: f64) -> Result<(Coordinate, Radius), String> {
    Ok((
        Coordinate::new(lat, lon).map_err(|e| e.to_string())?,
        Radius::from_kilometres(radius_km).map_err(|e| e.to_string())?,
    ))
}

fn failure(error: String, retry_after_secs: Option<u32>) -> PollResult {
    PollResult {
        view: None,
        error: Some(error),
        retry_after_secs,
    }
}

fn credit(c: credits::Credit) -> Credit {
    Credit {
        text: c.text,
        url: c.url,
    }
}

/// `now` is the wall clock used for local times at destinations.
fn view(overhead: &Overhead, now: SystemTime, units: Units) -> NowView {
    let mut used = vec![credit(credits::for_source(overhead.source))];
    used.extend(
        overhead
            .route_sources()
            .into_iter()
            .map(|s| credit(credits::for_route_source(s))),
    );
    used.extend(
        overhead
            .weather_sources()
            .into_iter()
            .map(|s| credit(credits::for_weather_source(s))),
    );
    let updated_unix_ms = overhead
        .time
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64);
    NowView {
        flights: overhead
            .flights
            .iter()
            .map(|f| flight_view(f, now, units))
            .collect(),
        credits: used,
        source: overhead.source.into(),
        updated_unix_ms,
    }
}

fn flight_view(f: &Flight, now: SystemTime, units: Units) -> FlightView {
    let destination = f.leg.as_ref().map(|l| &l.destination);
    let far = !f.lands_nearby;
    FlightView {
        key: f.aircraft.icao24.clone(),
        title: story::title(f).to_owned(),
        destination: destination.map(|d| d.place().to_owned()),
        origin: f.leg.as_ref().map(|l| l.origin.place().to_owned()),
        local_time: destination
            .filter(|_| far)
            .and_then(|d| d.time_zone.as_deref())
            .and_then(|tz| local_time(tz, now)),
        weather: f
            .weather
            .as_ref()
            .filter(|_| far)
            .map(|w| story::weather_phrase(w, units)),
        landing_in: f.time_to_landing.map(story::landing_phrase),
        lands_nearby: f.lands_nearby,
        journey: story::journey(f),
        line: story::dreamy_line(f, now, units),
        distance_km: f.distance_m / 1000.0,
        altitude_m: f.aircraft.altitude_m(),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    const FIXTURES: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/passingover-core/tests/fixtures"
    );

    fn gz(path: &str) -> Vec<u8> {
        let text = std::fs::read(format!("{FIXTURES}/{path}")).unwrap();
        let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        e.write_all(&text).unwrap();
        e.finish().unwrap()
    }

    fn handle_with_data() -> SkyHandle {
        let mut handle = SkyHandle::new(Source::AdsbLol, 1);
        let info = handle
            .load_data(
                gz("data/routes.tsv"),
                gz("data/airports.tsv"),
                gz("data/airlines.tsv"),
            )
            .unwrap();
        assert_eq!(info.routes, 6);
        handle
    }

    #[test]
    fn a_recorded_snapshot_becomes_views() {
        let body = std::fs::read(format!(
            "{FIXTURES}/positions/adsb_lol_point_ekch_10nm.json"
        ))
        .unwrap();
        let view = handle_with_data()
            .view_from_snapshot(Source::AdsbLol, body, 55.62, 12.66, 18.52, false)
            .unwrap();
        assert_eq!(view.source, Source::AdsbLol);
        assert_eq!(view.updated_unix_ms, 1_792_966_614_001);
        let titles: Vec<&str> = view.flights.iter().map(|f| f.title.as_str()).collect();
        assert_eq!(titles, ["SK 7724", "SK 2802"]);
        let sk2802 = &view.flights[1];
        assert_eq!(sk2802.key, "4ac9e2");
        assert_eq!(sk2802.destination.as_deref(), Some("Copenhagen"));
        assert_eq!(sk2802.origin.as_deref(), Some("Lisbon"));
        // Landing at Copenhagen, where the user is: no time or weather there.
        assert!(sk2802.lands_nearby);
        assert_eq!(sk2802.local_time, None);
        assert!(sk2802.landing_in.as_deref().unwrap().starts_with("about "));
        assert!(sk2802
            .journey
            .as_deref()
            .unwrap()
            .starts_with("Left Lisbon about "));
        let credits: Vec<&str> = view.credits.iter().map(|c| c.url.as_str()).collect();
        assert_eq!(
            credits,
            [
                "https://adsb.lol",
                "https://github.com/vradarserver/standing-data"
            ]
        );
    }

    #[test]
    fn bad_input_is_an_error_not_a_panic() {
        let mut handle = SkyHandle::new(Source::AdsbFi, 1);
        let result = handle.poll(91.0, 0.0, 30.0, false);
        assert!(result.view.is_none());
        assert!(result.error.unwrap().contains("latitude"));
        assert!(handle.load_data(vec![1, 2, 3], vec![], vec![]).is_err());
        assert!(handle.load_regions(vec![1, 2, 3]).is_err());
    }

    #[test]
    fn usage_round_trips() {
        let mut handle = SkyHandle::new(Source::AdsbLol, 1);
        let usage = Usage {
            day: 20_386,
            adsb_lol: 12,
            adsb_fi: 3,
        };
        handle.set_usage(usage);
        assert_eq!(handle.usage(), usage);
        assert_eq!(handle.primary(), Source::AdsbLol);
        assert_eq!(choose_primary_source(0.9), Source::AdsbFi);
        assert!(data_credits().len() >= 5);
    }
}
