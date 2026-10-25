//! The bundled standing data: routes, airports and airlines (D11, D14, D24).
//!
//! `cargo xtask data` builds the files from the VRS standing data and adds
//! IANA time zones. Each is UTF-8, tab-separated, with a header line naming
//! the table and format version; empty fields mean "unknown".
//!
//! | File | Header | Columns |
//! |---|---|---|
//! | `routes.tsv` | `#passingover-routes\t1` | airline code, flight number, airport codes joined by `-` |
//! | `airports.tsv` | `#passingover-airports\t1` | code, ICAO, IATA, name, city, country, latitude, longitude, time zone |
//! | `airlines.tsv` | `#passingover-airlines\t1` | code, ICAO, IATA, name |
//!
//! Routes are sorted by callsign (airline code followed by number) so they
//! can be binary-searched in place: 620k routes cost about their file size
//! in memory, not a hash map's worth.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::airport::{Airline, Airport};
use crate::geo::Coordinate;

pub const ROUTES_FILE: &str = "routes.tsv";
pub const AIRPORTS_FILE: &str = "airports.tsv";
pub const AIRLINES_FILE: &str = "airlines.tsv";
pub const ROUTES_HEADER: &str = "#passingover-routes\t1";
pub const AIRPORTS_HEADER: &str = "#passingover-airports\t1";
pub const AIRLINES_HEADER: &str = "#passingover-airlines\t1";

#[derive(Debug, Error)]
pub enum DataError {
    #[error("{}: {source}", path.display())]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{file} line {line}: {message}")]
    Format {
        file: &'static str,
        line: usize,
        message: String,
    },
}

/// A route as stored: callsign parts and airport codes in flight order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredRoute {
    pub airline_code: String,
    pub number: String,
    pub airport_codes: Vec<String>,
}

/// The loaded standing data.
#[derive(Debug)]
pub struct StandingData {
    routes: String,
    /// Byte offset of each route line in `routes`.
    route_starts: Vec<u32>,
    airports: HashMap<String, Airport>,
    /// ICAO code → airport code, for airports whose code is not their ICAO.
    icao_to_code: HashMap<String, String>,
    airlines: HashMap<String, Airline>,
    /// IATA airline code → airline code.
    iata_to_airline: HashMap<String, String>,
}

impl StandingData {
    /// Load the three tables from a directory built by `cargo xtask data`.
    pub fn load(dir: &Path) -> Result<Self, DataError> {
        let read = |name: &str| {
            let path = dir.join(name);
            std::fs::read_to_string(&path).map_err(|source| DataError::Io { path, source })
        };
        Self::parse(
            read(ROUTES_FILE)?,
            &read(AIRPORTS_FILE)?,
            &read(AIRLINES_FILE)?,
        )
    }

    /// Parse the three tables from memory, e.g. from bundled assets.
    pub fn parse(routes: String, airports: &str, airlines: &str) -> Result<Self, DataError> {
        let route_starts = index_routes(&routes)?;
        let airports = parse_airports(airports)?;
        let icao_to_code = airports
            .values()
            .filter_map(|a| Some((a.icao.clone()?, a.code.clone())))
            .filter(|(icao, code)| icao != code)
            .collect();
        let airlines = parse_airlines(airlines)?;
        let iata_to_airline = airlines
            .values()
            .filter_map(|a| Some((a.iata.clone()?, a.code.clone())))
            .collect();
        Ok(Self {
            routes,
            route_starts,
            airports,
            icao_to_code,
            airlines,
            iata_to_airline,
        })
    }

    pub fn route_count(&self) -> usize {
        self.route_starts.len()
    }

    pub fn airport_count(&self) -> usize {
        self.airports.len()
    }

    /// The route for an already normalised callsign such as `SAS945`.
    pub fn route(&self, callsign: &str) -> Option<StoredRoute> {
        let i = self
            .route_starts
            .binary_search_by(|&start| route_key_cmp(self.route_line(start), callsign))
            .ok()?;
        let mut fields = self.route_line(self.route_starts[i]).split('\t');
        Some(StoredRoute {
            airline_code: fields.next()?.to_owned(),
            number: fields.next()?.to_owned(),
            airport_codes: fields.next()?.split('-').map(str::to_owned).collect(),
        })
    }

    pub fn airport(&self, code: &str) -> Option<&Airport> {
        self.airports.get(code)
    }

    pub fn airport_by_icao(&self, icao: &str) -> Option<&Airport> {
        let code = self.icao_to_code.get(icao).map_or(icao, String::as_str);
        self.airports.get(code)
    }

    pub fn airline(&self, code: &str) -> Option<&Airline> {
        self.airlines.get(code)
    }

    /// The airline code (ICAO where known) for an IATA airline code.
    pub fn airline_code_for_iata(&self, iata: &str) -> Option<&str> {
        self.iata_to_airline.get(iata).map(String::as_str)
    }

    fn route_line(&self, start: u32) -> &str {
        let rest = &self.routes[start as usize..];
        rest.split_once('\n').map_or(rest, |(line, _)| line)
    }
}

/// The callsign (airline code followed by number) of a route line, as bytes.
fn callsign_bytes(line: &str) -> impl Iterator<Item = u8> + '_ {
    let mut fields = line.split('\t');
    let code = fields.next().unwrap_or("");
    let number = fields.next().unwrap_or("");
    code.bytes().chain(number.bytes())
}

/// Compare a route line's callsign with `key`.
fn route_key_cmp(line: &str, key: &str) -> Ordering {
    callsign_bytes(line).cmp(key.bytes())
}

fn index_routes(text: &str) -> Result<Vec<u32>, DataError> {
    let file = ROUTES_FILE;
    if u32::try_from(text.len()).is_err() {
        return Err(format_error(file, 0, "file is larger than 4 GiB"));
    }
    let mut lines = text.split_inclusive('\n');
    check_header(file, lines.next(), ROUTES_HEADER)?;
    let mut starts = Vec::new();
    let mut offset = ROUTES_HEADER.len() + 1;
    let mut previous: Option<&str> = None;
    for (n, raw) in lines.enumerate() {
        let line = raw.trim_end_matches(['\n', '\r']);
        if !line.is_empty() {
            if line.split('\t').count() != 3 {
                return Err(format_error(file, n + 2, "expected 3 columns"));
            }
            if previous
                .is_some_and(|p| callsign_bytes(p).cmp(callsign_bytes(line)) != Ordering::Less)
            {
                return Err(format_error(
                    file,
                    n + 2,
                    "routes are not sorted by callsign",
                ));
            }
            previous = Some(line);
            starts.push(offset as u32);
        }
        offset += raw.len();
    }
    Ok(starts)
}

fn parse_airports(text: &str) -> Result<HashMap<String, Airport>, DataError> {
    let file = AIRPORTS_FILE;
    let mut lines = text.lines();
    check_header(file, lines.next(), AIRPORTS_HEADER)?;
    let mut airports = HashMap::new();
    for (n, line) in lines.enumerate().filter(|(_, l)| !l.is_empty()) {
        let f: Vec<&str> = line.split('\t').collect();
        let [code, icao, iata, name, city, country, lat, lon, tz] = f[..] else {
            return Err(format_error(file, n + 2, "expected 9 columns"));
        };
        let number = |s: &str| s.parse::<f64>().ok();
        let position = number(lat)
            .zip(number(lon))
            .and_then(|(lat, lon)| Coordinate::new(lat, lon).ok())
            .ok_or_else(|| format_error(file, n + 2, "invalid latitude or longitude"))?;
        airports.insert(
            code.to_owned(),
            Airport {
                code: code.to_owned(),
                icao: optional(icao),
                iata: optional(iata),
                name: name.to_owned(),
                city: optional(city),
                country: country.to_owned(),
                position,
                time_zone: optional(tz),
            },
        );
    }
    Ok(airports)
}

fn parse_airlines(text: &str) -> Result<HashMap<String, Airline>, DataError> {
    let file = AIRLINES_FILE;
    let mut lines = text.lines();
    check_header(file, lines.next(), AIRLINES_HEADER)?;
    let mut airlines = HashMap::new();
    for (n, line) in lines.enumerate().filter(|(_, l)| !l.is_empty()) {
        let f: Vec<&str> = line.split('\t').collect();
        let [code, icao, iata, name] = f[..] else {
            return Err(format_error(file, n + 2, "expected 4 columns"));
        };
        airlines.insert(
            code.to_owned(),
            Airline {
                code: code.to_owned(),
                icao: optional(icao),
                iata: optional(iata),
                name: name.to_owned(),
            },
        );
    }
    Ok(airlines)
}

fn check_header(file: &'static str, line: Option<&str>, expected: &str) -> Result<(), DataError> {
    match line.map(|l| l.trim_end_matches(['\n', '\r'])) {
        Some(l) if l == expected => Ok(()),
        _ => Err(format_error(
            file,
            1,
            &format!("expected header {expected:?}; rebuild with `cargo xtask data`"),
        )),
    }
}

fn format_error(file: &'static str, line: usize, message: &str) -> DataError {
    DataError::Format {
        file,
        line,
        message: message.to_owned(),
    }
}

fn optional(s: &str) -> Option<String> {
    (!s.is_empty()).then(|| s.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> StandingData {
        let routes = format!(
            "{ROUTES_HEADER}\nSAS\t1225\tEKCH-ESSA\nSAS\t945\tESSA-KORD\nU2\t1234\tEGKK-LFPG\n"
        );
        let airports = format!(
            "{AIRPORTS_HEADER}\n\
             ESSA\tESSA\tARN\tStockholm-Arlanda Airport\tStockholm\tSE\t59.651901\t17.918600\tEurope/Stockholm\n\
             KORD\tKORD\tORD\tChicago O'Hare International Airport\tChicago\tUS\t41.978600\t-87.904800\tAmerica/Chicago\n\
             XYZ\t\tXYZ\tNo ICAO Field\t\tUS\t40\t-100\t\n"
        );
        let airlines = format!(
            "{AIRLINES_HEADER}\nSAS\tSAS\tSK\tScandinavian Airlines\nU2\t\tU2\tNo ICAO Air\n"
        );
        StandingData::parse(routes, &airports, &airlines).unwrap()
    }

    #[test]
    fn finds_routes_by_callsign() {
        let data = sample();
        assert_eq!(data.route_count(), 3);
        let route = data.route("SAS945").unwrap();
        assert_eq!(route.airline_code, "SAS");
        assert_eq!(route.number, "945");
        assert_eq!(route.airport_codes, ["ESSA", "KORD"]);
        assert_eq!(
            data.route("U21234").unwrap().airport_codes,
            ["EGKK", "LFPG"]
        );
        assert_eq!(data.route("SAS94"), None);
        assert_eq!(data.route("SAS9450"), None);
        assert_eq!(data.route("AAA1"), None);
        assert_eq!(data.route("ZZZ1"), None);
    }

    #[test]
    fn airports_and_airlines_resolve() {
        let data = sample();
        let ord = data.airport("KORD").unwrap();
        assert_eq!(ord.place(), "Chicago");
        assert_eq!(ord.time_zone.as_deref(), Some("America/Chicago"));
        assert_eq!(
            data.airport_by_icao("ESSA").unwrap().iata.as_deref(),
            Some("ARN")
        );
        let xyz = data.airport("XYZ").unwrap();
        assert_eq!((xyz.icao.as_ref(), xyz.place()), (None, "No ICAO Field"));
        assert_eq!(data.airline("SAS").unwrap().iata.as_deref(), Some("SK"));
        assert_eq!(data.airline_code_for_iata("SK"), Some("SAS"));
        assert_eq!(data.airline_code_for_iata("U2"), Some("U2"));
    }

    #[test]
    fn rejects_unsorted_routes_and_wrong_headers() {
        let unsorted = format!("{ROUTES_HEADER}\nSAS\t945\tESSA-KORD\nSAS\t1225\tEKCH-ESSA\n");
        let airports = format!("{AIRPORTS_HEADER}\n");
        let airlines = format!("{AIRLINES_HEADER}\n");
        let err = StandingData::parse(unsorted, &airports, &airlines).unwrap_err();
        assert!(err.to_string().contains("not sorted"), "{err}");

        let old = "#passingover-routes\t0\n".to_owned();
        let err = StandingData::parse(old, &airports, &airlines).unwrap_err();
        assert!(err.to_string().contains("cargo xtask data"), "{err}");
    }
}
