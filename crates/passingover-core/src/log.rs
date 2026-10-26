//! The daydream log (M5, D38): where the planes that passed over were going,
//! kept on the device for gentle statistics such as "this month, planes
//! over you went to 61 cities in 24 countries".
//!
//! Only flight facts are stored (when, which flight, from and to where),
//! never the user's location. The file is tab-separated, one passage per
//! line after the header `#passingover-log\t1`: seen (Unix ms), flight,
//! ICAO 24-bit address, origin code, origin place, destination code,
//! destination place, destination country, destination latitude,
//! destination longitude.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::geo::Coordinate;
use crate::sky::Overhead;

pub const LOG_HEADER: &str = "#passingover-log\t1";

/// The same aircraft and callsign within this time is the same passage.
const SAME_PASSAGE: Duration = Duration::from_secs(2 * 60 * 60);

/// One flight that passed over.
#[derive(Debug, Clone, PartialEq)]
pub struct Passage {
    pub seen_unix_ms: i64,
    pub flight: String,
    pub icao24: String,
    pub origin_code: Option<String>,
    pub origin: Option<String>,
    pub destination_code: String,
    pub destination: String,
    /// ISO 3166-1 alpha-2.
    pub destination_country: String,
    pub destination_position: Coordinate,
}

/// Counts for a period, for [`crate::story::log_summary`].
#[derive(Debug, Clone, PartialEq)]
pub struct LogStats {
    pub flights: u32,
    /// Distinct destination places.
    pub places: u32,
    pub countries: u32,
    /// Most frequent destinations with their counts, most first (up to 3).
    pub top: Vec<(String, u32)>,
    /// The destination farthest from `here`, with the distance in km.
    pub farthest: Option<(String, f64)>,
    /// When the log's first passage (in any period) was seen.
    pub first_seen_unix_ms: Option<i64>,
}

#[derive(Debug, Default)]
pub struct DaydreamLog {
    path: Option<PathBuf>,
    passages: Vec<Passage>,
    /// Aircraft + flight → last seen, to log each passage once.
    recent: HashMap<(String, String), i64>,
}

impl DaydreamLog {
    /// A log that is only kept in memory.
    pub fn in_memory() -> Self {
        Self::default()
    }

    /// Open (or start) the log file at `path`.
    pub fn open(path: &Path) -> io::Result<Self> {
        let mut log = Self {
            path: Some(path.to_owned()),
            ..Self::default()
        };
        match File::open(path) {
            Ok(file) => {
                let mut lines = BufReader::new(file).lines();
                if let Some(header) = lines.next().transpose()?
                    && header != LOG_HEADER
                {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("{}: not a daydream log", path.display()),
                    ));
                }
                for line in lines {
                    // A torn last line (e.g. power cut mid-write) is skipped.
                    if let Some(p) = parse_line(&line?) {
                        log.remember(&p);
                        log.passages.push(p);
                    }
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        Ok(log)
    }

    pub fn passages(&self) -> &[Passage] {
        &self.passages
    }

    /// Record the flights in `overhead` that have a known destination and
    /// haven't been logged in the last two hours. Returns how many were new.
    pub fn record(&mut self, overhead: &Overhead, wall: SystemTime) -> io::Result<usize> {
        let now = unix_ms(wall);
        let mut new = Vec::new();
        for f in &overhead.flights {
            let Some(leg) = &f.leg else { continue };
            let flight = f
                .flight_number
                .clone()
                .unwrap_or_else(|| f.aircraft.icao24.clone());
            let key = (f.aircraft.icao24.clone(), flight.clone());
            if self
                .recent
                .get(&key)
                .is_some_and(|&t| now - t < SAME_PASSAGE.as_millis() as i64)
            {
                self.recent.insert(key, now);
                continue;
            }
            self.recent.insert(key, now);
            new.push(Passage {
                seen_unix_ms: now,
                flight,
                icao24: f.aircraft.icao24.clone(),
                origin_code: Some(leg.origin.code.clone()),
                origin: Some(leg.origin.place().to_owned()),
                destination_code: leg.destination.code.clone(),
                destination: leg.destination.place().to_owned(),
                destination_country: leg.destination.country.clone(),
                destination_position: leg.destination.position,
            });
        }
        if let Some(path) = &self.path
            && !new.is_empty()
        {
            let fresh = !path.exists();
            let torn = !fresh && !ends_with_newline(path)?;
            let mut file = OpenOptions::new().create(true).append(true).open(path)?;
            let mut text = String::new();
            if fresh {
                text.push_str(LOG_HEADER);
                text.push('\n');
            } else if torn {
                // Finish a line torn by e.g. a power cut, so the next
                // passage starts on its own line.
                text.push('\n');
            }
            for p in &new {
                text.push_str(&line(p));
                text.push('\n');
            }
            file.write_all(text.as_bytes())?;
        }
        let count = new.len();
        self.passages.extend(new);
        Ok(count)
    }

    /// Counts for passages seen at or after `since_unix_ms`; `here` is used
    /// for "the farthest", and is not stored.
    pub fn stats(&self, since_unix_ms: i64, here: Option<Coordinate>) -> LogStats {
        let period: Vec<&Passage> = self
            .passages
            .iter()
            .filter(|p| p.seen_unix_ms >= since_unix_ms)
            .collect();
        let mut by_place: HashMap<&str, u32> = HashMap::new();
        let mut countries: Vec<&str> = Vec::new();
        for p in &period {
            *by_place.entry(p.destination.as_str()).or_default() += 1;
            if !countries.contains(&p.destination_country.as_str()) {
                countries.push(&p.destination_country);
            }
        }
        let mut top: Vec<(String, u32)> = by_place
            .iter()
            .map(|(place, n)| ((*place).to_owned(), *n))
            .collect();
        top.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let places = top.len() as u32;
        top.truncate(3);
        let farthest = here.and_then(|here| {
            period
                .iter()
                .map(|p| (p, here.distance_to(&p.destination_position)))
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(p, d)| (p.destination.clone(), d / 1000.0))
        });
        LogStats {
            flights: period.len() as u32,
            places,
            countries: countries.len() as u32,
            top,
            farthest,
            first_seen_unix_ms: self.passages.first().map(|p| p.seen_unix_ms),
        }
    }

    /// Forget everything, including the file.
    pub fn clear(&mut self) -> io::Result<()> {
        self.passages.clear();
        self.recent.clear();
        if let Some(path) = &self.path {
            match std::fs::remove_file(path) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
                _ => {}
            }
        }
        Ok(())
    }

    fn remember(&mut self, p: &Passage) {
        self.recent
            .insert((p.icao24.clone(), p.flight.clone()), p.seen_unix_ms);
    }
}

fn ends_with_newline(path: &Path) -> io::Result<bool> {
    let mut file = File::open(path)?;
    if file.metadata()?.len() == 0 {
        return Ok(true);
    }
    file.seek(SeekFrom::End(-1))?;
    let mut last = [0u8];
    file.read_exact(&mut last)?;
    Ok(last[0] == b'\n')
}

fn unix_ms(t: SystemTime) -> i64 {
    t.duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

fn clean(s: &str) -> String {
    s.replace(['\t', '\n', '\r'], " ")
}

fn line(p: &Passage) -> String {
    [
        p.seen_unix_ms.to_string(),
        clean(&p.flight),
        clean(&p.icao24),
        clean(p.origin_code.as_deref().unwrap_or("")),
        clean(p.origin.as_deref().unwrap_or("")),
        clean(&p.destination_code),
        clean(&p.destination),
        clean(&p.destination_country),
        format!("{:.4}", p.destination_position.lat()),
        format!("{:.4}", p.destination_position.lon()),
    ]
    .join("\t")
}

fn parse_line(line: &str) -> Option<Passage> {
    let f: Vec<&str> = line.split('\t').collect();
    let [
        seen,
        flight,
        icao,
        ocode,
        origin,
        dcode,
        dest,
        country,
        lat,
        lon,
    ] = f[..]
    else {
        return None;
    };
    let optional = |s: &str| (!s.is_empty()).then(|| s.to_owned());
    Some(Passage {
        seen_unix_ms: seen.parse().ok()?,
        flight: flight.to_owned(),
        icao24: icao.to_owned(),
        origin_code: optional(ocode),
        origin: optional(origin),
        destination_code: dcode.to_owned(),
        destination: dest.to_owned(),
        destination_country: country.to_owned(),
        destination_position: Coordinate::new(lat.parse().ok()?, lon.parse().ok()?).ok()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aircraft::Aircraft;
    use crate::airport::Airport;
    use crate::flight::Flight;
    use crate::position::SourceId;
    use crate::route::Leg;

    fn airport(code: &str, city: &str, country: &str, lat: f64, lon: f64) -> Airport {
        Airport {
            code: code.into(),
            icao: Some(code.into()),
            iata: None,
            name: city.into(),
            city: Some(city.into()),
            country: country.into(),
            position: Coordinate::new(lat, lon).unwrap(),
            time_zone: None,
        }
    }

    fn flight(icao: &str, number: &str, to: Airport) -> Flight {
        Flight {
            aircraft: Aircraft {
                icao24: icao.into(),
                callsign: None,
                registration: None,
                type_designator: None,
                category: None,
                position: Coordinate::new(55.6, 12.6).unwrap(),
                position_time: SystemTime::UNIX_EPOCH,
                on_ground: false,
                baro_altitude_m: None,
                geo_altitude_m: None,
                ground_speed_mps: None,
                track_deg: None,
                vertical_rate_mps: None,
            },
            distance_m: 0.0,
            flight_number: Some(number.into()),
            leg: Some(Leg {
                origin: airport("EKCH", "Copenhagen", "DK", 55.6, 12.6),
                destination: to,
            }),
            route_source: None,
            time_to_landing: None,
            lands_nearby: false,
            departed_ago: None,
            ahead: Vec::new(),
            weather: None,
            weather_source: None,
        }
    }

    fn london() -> Airport {
        airport("EGLL", "London", "GB", 51.47, -0.45)
    }
    fn singapore() -> Airport {
        airport("WSSS", "Singapore", "SG", 1.36, 103.99)
    }
    fn paris() -> Airport {
        airport("LFPG", "Paris", "FR", 49.01, 2.55)
    }

    fn overhead(flights: Vec<Flight>) -> Overhead {
        Overhead {
            source: SourceId::AdsbLol,
            time: SystemTime::UNIX_EPOCH,
            flights,
        }
    }

    fn at(hours: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_792_000_000 + hours * 3600)
    }

    #[test]
    fn each_passage_is_logged_once() {
        let mut log = DaydreamLog::in_memory();
        let sky = overhead(vec![flight("a1", "BA 811", london())]);
        assert_eq!(log.record(&sky, at(0)).unwrap(), 1);
        // Still overhead a minute later, and an hour later: same passage.
        assert_eq!(
            log.record(&sky, at(0) + Duration::from_secs(60)).unwrap(),
            0
        );
        assert_eq!(log.record(&sky, at(1)).unwrap(), 0);
        // Tomorrow's BA 811 is a new passage.
        assert_eq!(log.record(&sky, at(24)).unwrap(), 1);
        assert_eq!(log.passages().len(), 2);
    }

    #[test]
    fn flights_without_a_destination_are_not_logged() {
        let mut log = DaydreamLog::in_memory();
        let mut unknown = flight("b2", "XX 1", london());
        unknown.leg = None;
        assert_eq!(log.record(&overhead(vec![unknown]), at(0)).unwrap(), 0);
    }

    #[test]
    fn stats_count_places_countries_and_the_farthest() {
        let mut log = DaydreamLog::in_memory();
        log.record(&overhead(vec![flight("a1", "BA 811", london())]), at(0))
            .unwrap();
        log.record(
            &overhead(vec![
                flight("a2", "SK 501", london()),
                flight("a3", "SQ 351", singapore()),
            ]),
            at(30),
        )
        .unwrap();
        log.record(&overhead(vec![flight("a4", "AF 1351", paris())]), at(60))
            .unwrap();
        let here = Coordinate::new(55.68, 12.57).unwrap();

        let all = log.stats(0, Some(here));
        assert_eq!((all.flights, all.places, all.countries), (4, 3, 3));
        assert_eq!(all.top[0], ("London".to_owned(), 2));
        let (far, km) = all.farthest.clone().unwrap();
        assert_eq!(far, "Singapore");
        assert!((9_900.0..10_100.0).contains(&km), "{km}");
        assert_eq!(all.first_seen_unix_ms, Some(1_792_000_000_000));

        let later = log.stats(1_792_000_000_000 + 25 * 3_600_000, None);
        assert_eq!((later.flights, later.places), (3, 3));
        assert_eq!(later.farthest, None);
    }

    #[test]
    fn the_file_survives_a_restart_and_can_be_cleared() {
        let dir = std::env::temp_dir().join(format!("passingover-log-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("log.tsv");
        let _ = std::fs::remove_file(&path);

        let mut log = DaydreamLog::open(&path).unwrap();
        log.record(&overhead(vec![flight("a1", "BA 811", london())]), at(0))
            .unwrap();
        log.record(&overhead(vec![flight("a3", "SQ 351", singapore())]), at(1))
            .unwrap();
        // A torn last line is skipped, not an error.
        OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"17920")
            .unwrap();

        let mut reopened = DaydreamLog::open(&path).unwrap();
        assert_eq!(reopened.passages(), log.passages());
        // The same passage right after a restart isn't logged twice.
        let again = reopened
            .record(&overhead(vec![flight("a3", "SQ 351", singapore())]), at(1))
            .unwrap();
        assert_eq!(again, 0);

        // And a new one after the torn line lands on a line of its own.
        reopened
            .record(&overhead(vec![flight("a4", "AF 1351", paris())]), at(2))
            .unwrap();
        assert_eq!(DaydreamLog::open(&path).unwrap().passages().len(), 3);

        reopened.clear().unwrap();
        assert!(!path.exists());
        assert!(reopened.passages().is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn other_files_are_refused() {
        let dir =
            std::env::temp_dir().join(format!("passingover-log-refuse-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("other.tsv");
        std::fs::write(&path, "something else\n").unwrap();
        assert!(DaydreamLog::open(&path).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
