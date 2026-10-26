//! The sky above one place: one poll fetches positions, keeps the flying
//! aircraft nearby and resolves where they're going. Shared by the CLI and
//! the app, so front ends only lay things out.

use std::time::{Instant, SystemTime};

use crate::data::StandingData;
use crate::flight::{Flight, flights_overhead};
use crate::geo::{Coordinate, QueryPoint, Radius};
use crate::http::HttpClient;
use crate::position::{Area, DailyUsage, FetchError, PositionFeed, Snapshot, SourceId};
use crate::regions::Regions;
use crate::route::{RouteLookup, RouteSourceId};
use crate::weather::{WeatherLookup, WeatherSourceId};

/// What one poll saw.
#[derive(Debug, Clone, PartialEq)]
pub struct Overhead {
    pub source: SourceId,
    /// The source's timestamp for the positions.
    pub time: SystemTime,
    /// Nearest first.
    pub flights: Vec<Flight>,
}

impl Overhead {
    /// Route sources used, in order of first use, for attribution.
    pub fn route_sources(&self) -> Vec<RouteSourceId> {
        let mut sources = Vec::new();
        for s in self.flights.iter().filter_map(|f| f.route_source) {
            if !sources.contains(&s) {
                sources.push(s);
            }
        }
        sources
    }

    /// Weather sources used, in order of first use, for attribution.
    pub fn weather_sources(&self) -> Vec<WeatherSourceId> {
        let mut sources = Vec::new();
        for s in self.flights.iter().filter_map(|f| f.weather_source) {
            if !sources.contains(&s) {
                sources.push(s);
            }
        }
        sources
    }
}

/// Position feed, route lookup and standing data for one front end.
#[derive(Debug)]
pub struct Sky {
    feed: PositionFeed,
    routes: RouteLookup,
    data: Option<StandingData>,
    regions: Option<Regions>,
    weather: WeatherLookup,
}

impl Sky {
    /// `seed` drives backoff jitter; use a random one outside tests.
    pub fn new(primary: SourceId, seed: u64) -> Self {
        Self {
            feed: PositionFeed::new(primary, seed),
            routes: RouteLookup::new(),
            data: None,
            regions: None,
            weather: WeatherLookup::new(),
        }
    }

    pub fn primary(&self) -> SourceId {
        self.feed.primary()
    }

    /// Use the bundled standing data for routes and airports. Without it,
    /// routes are looked up online only.
    pub fn set_data(&mut self, data: StandingData) {
        self.data = Some(data);
    }

    pub fn data(&self) -> Option<&StandingData> {
        self.data.as_ref()
    }

    /// Use country and sea shapes for "will cross …" (D33).
    pub fn set_regions(&mut self, regions: Regions) {
        self.regions = Some(regions);
    }

    pub fn usage(&self) -> DailyUsage {
        self.feed.usage()
    }

    pub fn set_usage(&mut self, usage: DailyUsage) {
        self.feed.set_usage(usage);
    }

    /// Fetch what is overhead. `here` stays on the device; only its rounded
    /// [`QueryPoint`] is sent.
    pub fn poll(
        &mut self,
        client: &dyn HttpClient,
        here: Coordinate,
        radius: Radius,
        wall: SystemTime,
        now: Instant,
    ) -> Result<Overhead, FetchError> {
        let area = Area {
            center: QueryPoint::from(here),
            radius,
        };
        let snapshot = self.feed.fetch(client, &area, wall, now)?;
        Ok(self.describe(&snapshot, here, radius, Some(client), now))
    }

    /// Turn a snapshot into flights; `client` enables online route and
    /// weather lookups, otherwise only cached weather is used.
    pub fn describe(
        &mut self,
        snapshot: &Snapshot,
        here: Coordinate,
        radius: Radius,
        client: Option<&dyn HttpClient>,
        now: Instant,
    ) -> Overhead {
        let (routes, data) = (&mut self.routes, self.data.as_ref());
        let mut flights = flights_overhead(snapshot, here, radius, |aircraft| {
            routes.lookup(aircraft.callsign.as_deref()?, data, client, now)
        });
        if let Some(regions) = &self.regions {
            for f in &mut flights {
                if let Some(leg) = &f.leg {
                    f.ahead = regions.ahead(f.aircraft.position, leg.destination.position);
                }
            }
        }
        let destinations: Vec<(String, Coordinate)> = flights
            .iter()
            .filter_map(|f| {
                let d = &f.leg.as_ref()?.destination;
                Some((d.code.clone(), d.position))
            })
            .collect();
        if !destinations.is_empty() {
            let weather = self.weather.get(&destinations, client, now);
            for f in &mut flights {
                let found = f
                    .leg
                    .as_ref()
                    .and_then(|l| weather.get(&l.destination.code));
                if let Some((w, source)) = found {
                    f.weather = Some(*w);
                    f.weather_source = Some(*source);
                }
            }
        }
        Overhead {
            source: snapshot.source,
            time: snapshot.time,
            flights,
        }
    }
}
