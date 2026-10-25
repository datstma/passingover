//! Where a flight is going (D11): callsign normalisation, the route lookup
//! (bundled standing data first, then adsbdb), and a plausibility check that
//! picks the leg the aircraft is actually flying.

mod adsbdb;
mod lookup;

pub use adsbdb::AdsbDb;
pub use lookup::RouteLookup;

use crate::aircraft::Aircraft;
use crate::airport::{Airline, Airport};
use crate::geo::{Coordinate, EARTH_RADIUS_M, angle_between};
use crate::http::{ParseError, Request};

/// Within this distance of either airport, any heading is plausible:
/// departures and arrivals turn in every direction.
const NEAR_AIRPORT_M: f64 = 50_000.0;
/// Away from the airports, the aircraft must be this close to the great
/// circle, or within [`MAX_CROSS_TRACK_SHARE`] of the leg length if that is
/// more (long flights follow winds and oceanic tracks, not great circles).
const MAX_CROSS_TRACK_M: f64 = 75_000.0;
const MAX_CROSS_TRACK_SHARE: f64 = 0.12;
/// Away from the airports, the track must point within this many degrees of
/// the destination.
const MAX_HEADING_OFF_DEG: f64 = 90.0;

/// Where route information came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RouteSourceId {
    /// The bundled VRS standing data (CC0).
    StandingData,
    AdsbDb,
}

impl RouteSourceId {
    pub fn name(self) -> &'static str {
        match self {
            RouteSourceId::StandingData => "VRS standing data",
            RouteSourceId::AdsbDb => "adsbdb",
        }
    }

    pub fn url(self) -> &'static str {
        match self {
            RouteSourceId::StandingData => "https://github.com/vradarserver/standing-data",
            RouteSourceId::AdsbDb => "https://www.adsbdb.com",
        }
    }
}

/// A callsign's route with its airports resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct RouteInfo {
    /// Normalised callsign, e.g. `SAS945`.
    pub callsign: String,
    /// For display, e.g. `SK 945`.
    pub flight_number: String,
    /// In flight order; at least two.
    pub airports: Vec<Airport>,
    pub source: RouteSourceId,
}

/// The leg of a route the aircraft is flying now.
#[derive(Debug, Clone, PartialEq)]
pub struct Leg {
    pub origin: Airport,
    pub destination: Airport,
}

/// A network source of routes by callsign (sans-IO, D21).
pub trait RouteSource {
    fn id(&self) -> RouteSourceId;
    /// The request for a normalised callsign.
    fn request(&self, callsign: &str) -> Request;
    /// Parse a response body. `Ok(None)` means the source doesn't know the
    /// callsign.
    fn parse(&self, body: &[u8]) -> Result<Option<RouteInfo>, ParseError>;
}

/// Normalise a broadcast callsign the way the VRS standing data does, into
/// (airline code, flight number): `SAS0945` → (`SAS`, `945`). A two-letter
/// IATA code is swapped for the airline's code via `iata_to_code` (`SK` →
/// `SAS`). Returns `None` for callsigns that aren't airline flight numbers,
/// such as registrations or ground vehicles.
pub fn normalise_callsign(
    raw: &str,
    iata_to_code: impl Fn(&str) -> Option<String>,
) -> Option<(String, String)> {
    let callsign = raw.trim().to_ascii_uppercase();
    let b = callsign.as_bytes();
    if b.is_empty() || !b.iter().all(u8::is_ascii_alphanumeric) {
        return None;
    }
    let letter = |i: usize| b.get(i).is_some_and(u8::is_ascii_uppercase);
    let digit = |i: usize| b.get(i).is_some_and(u8::is_ascii_digit);
    // The code is `[A-Z]{2,3}`, `[A-Z][0-9]` or `[0-9][A-Z]`, and the number
    // must start with a digit; try the alternatives in that order.
    let code_len = if letter(0) && letter(1) && letter(2) && digit(3) {
        3
    } else if (letter(0) && letter(1) || letter(0) && digit(1) || digit(0) && letter(1)) && digit(2)
    {
        2
    } else {
        return None;
    };
    let (code, number) = callsign.split_at(code_len);
    let code = if code_len == 2 {
        iata_to_code(code).unwrap_or_else(|| code.to_owned())
    } else {
        code.to_owned()
    };
    // Strip leading zeros, keeping one if nothing (or only letters) is left.
    let trimmed = number.trim_start_matches('0');
    let number = if trimmed.starts_with(|c: char| c.is_ascii_digit()) {
        trimmed.to_owned()
    } else {
        format!("0{trimmed}")
    };
    // Valid numbers: up to four characters, digits first, then at most two
    // letters (n, nn, nA, nnn, nnA, nAA, nnnn, nnnA, nnAA).
    let digits = number.bytes().take_while(u8::is_ascii_digit).count();
    let letters_only_after = number.bytes().skip(digits).all(|c| c.is_ascii_uppercase());
    (number.len() <= 4 && number.len() - digits <= 2 && letters_only_after)
        .then_some((code, number))
}

/// A flight number for display: the airline's IATA code if it has one,
/// e.g. `SK 945`, else its code, e.g. `XYZ 12`.
pub fn flight_number(airline: Option<&Airline>, code: &str, number: &str) -> String {
    let prefix = airline.and_then(|a| a.iata.as_deref()).unwrap_or(code);
    format!("{prefix} {number}")
}

/// The leg of `airports` (in flight order) that `aircraft` is plausibly
/// flying, or `None` if no leg fits where it is and where it's heading.
pub fn current_leg(aircraft: &Aircraft, airports: &[Airport]) -> Option<Leg> {
    airports
        .windows(2)
        .filter_map(|pair| Some((leg_score(aircraft, &pair[0], &pair[1])?, pair)))
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, pair)| Leg {
            origin: pair[0].clone(),
            destination: pair[1].clone(),
        })
}

/// Lower is a better fit; `None` if the leg is implausible.
fn leg_score(aircraft: &Aircraft, origin: &Airport, destination: &Airport) -> Option<f64> {
    let (start, end, here) = (origin.position, destination.position, aircraft.position);
    let total = start.distance_to(&end);
    if total < 1_000.0 {
        return None;
    }
    let near_airport =
        start.distance_to(&here) < NEAR_AIRPORT_M || here.distance_to(&end) < NEAR_AIRPORT_M;
    let (cross, along) = cross_and_along_track(start, end, here);
    let heading_off = aircraft
        .track_deg
        .map(|track| angle_between(track, here.bearing_to(&end)));
    if !near_airport {
        let off_track = cross.abs() > MAX_CROSS_TRACK_M.max(MAX_CROSS_TRACK_SHARE * total);
        let beyond_ends = along < -NEAR_AIRPORT_M || along > total + NEAR_AIRPORT_M;
        let wrong_way = heading_off.is_some_and(|h| h > MAX_HEADING_OFF_DEG);
        if off_track || beyond_ends || wrong_way {
            return None;
        }
    }
    Some(cross.abs() / total + heading_off.unwrap_or(MAX_HEADING_OFF_DEG) / 180.0)
}

/// Signed cross-track and along-track distances in metres of `point`
/// relative to the great circle from `start` to `end`.
fn cross_and_along_track(start: Coordinate, end: Coordinate, point: Coordinate) -> (f64, f64) {
    let d13 = start.distance_to(&point) / EARTH_RADIUS_M;
    let dtheta = (start.bearing_to(&point) - start.bearing_to(&end)).to_radians();
    let cross = (d13.sin() * dtheta.sin()).asin();
    let along = (d13.cos() / cross.cos()).clamp(-1.0, 1.0).acos();
    let sign = if dtheta.cos() < 0.0 { -1.0 } else { 1.0 };
    (cross * EARTH_RADIUS_M, sign * along * EARTH_RADIUS_M)
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use super::*;

    fn no_iata(_: &str) -> Option<String> {
        None
    }

    fn easyjet(iata: &str) -> Option<String> {
        (iata == "U2").then(|| "EZY".to_owned())
    }

    fn norm(raw: &str) -> Option<String> {
        normalise_callsign(raw, easyjet).map(|(c, n)| format!("{c}{n}"))
    }

    #[test]
    fn normalises_like_vrs() {
        // The examples from the VRS routes README.
        assert_eq!(norm("EZY1200").as_deref(), Some("EZY1200"));
        assert_eq!(norm("EZY0001").as_deref(), Some("EZY1"));
        assert_eq!(norm("EZY0000").as_deref(), Some("EZY0"));
        assert_eq!(norm("EZY00AB").as_deref(), Some("EZY0AB"));
        assert_eq!(norm("U21234").as_deref(), Some("EZY1234"));
        // Padding, case and the two-character code forms.
        assert_eq!(norm(" sas945  ").as_deref(), Some("SAS945"));
        assert_eq!(norm("DLH3CF").as_deref(), Some("DLH3CF"));
        assert_eq!(
            normalise_callsign("SK945", no_iata),
            Some(("SK".into(), "945".into()))
        );
        assert_eq!(
            normalise_callsign("9W123", no_iata),
            Some(("9W".into(), "123".into()))
        );
    }

    #[test]
    fn rejects_non_flight_callsigns() {
        for raw in [
            "", "SEROB", "TXLU03", "SQBREF1", "SAS12345", "SAS1ABC", "SAS1A2", "OY-KBP",
        ] {
            assert_eq!(norm(raw), None, "{raw}");
        }
    }

    #[test]
    fn flight_number_prefers_iata() {
        let sas = Airline {
            code: "SAS".into(),
            icao: Some("SAS".into()),
            iata: Some("SK".into()),
            name: "SAS".into(),
        };
        assert_eq!(flight_number(Some(&sas), "SAS", "945"), "SK 945");
        assert_eq!(flight_number(None, "XYZ", "12"), "XYZ 12");
    }

    fn airport(code: &str, lat: f64, lon: f64) -> Airport {
        Airport {
            code: code.into(),
            icao: Some(code.into()),
            iata: None,
            name: code.into(),
            city: None,
            country: "XX".into(),
            position: Coordinate::new(lat, lon).unwrap(),
            time_zone: None,
        }
    }

    fn aircraft_at(lat: f64, lon: f64, track: f64) -> Aircraft {
        Aircraft {
            icao24: "abc123".into(),
            callsign: None,
            registration: None,
            type_designator: None,
            category: Some("A3".into()),
            position: Coordinate::new(lat, lon).unwrap(),
            position_time: SystemTime::UNIX_EPOCH,
            on_ground: false,
            baro_altitude_m: Some(11_000.0),
            geo_altitude_m: None,
            ground_speed_mps: Some(240.0),
            track_deg: Some(track),
            vertical_rate_mps: None,
        }
    }

    // Stockholm Arlanda, Copenhagen, Chicago O'Hare.
    fn essa() -> Airport {
        airport("ESSA", 59.6519, 17.9186)
    }
    fn ekch() -> Airport {
        airport("EKCH", 55.6181, 12.6561)
    }
    fn kord() -> Airport {
        airport("KORD", 41.9786, -87.9048)
    }

    #[test]
    fn accepts_an_aircraft_on_the_way() {
        // Over southern Norway, heading west-north-west: on the way to Chicago.
        let leg = current_leg(&aircraft_at(60.3, 10.0, 290.0), &[essa(), kord()]).unwrap();
        assert_eq!(leg.destination.code, "KORD");
    }

    #[test]
    fn rejects_off_track_or_wrong_way_aircraft() {
        // Over Rome: nowhere near Stockholm–Chicago.
        assert_eq!(
            current_leg(&aircraft_at(41.9, 12.5, 290.0), &[essa(), kord()]),
            None
        );
        // On the track but flying east, away from Chicago.
        assert_eq!(
            current_leg(&aircraft_at(60.3, 10.0, 100.0), &[essa(), kord()]),
            None
        );
    }

    #[test]
    fn any_heading_is_fine_near_an_airport() {
        // 20 km from Arlanda, turning south after take-off.
        assert!(current_leg(&aircraft_at(59.5, 17.7, 180.0), &[essa(), kord()]).is_some());
    }

    #[test]
    fn picks_the_leg_being_flown_on_multi_stop_routes() {
        let route = [ekch(), essa(), kord()];
        // Between Copenhagen and Stockholm, heading north-east.
        let leg = current_leg(&aircraft_at(57.6, 15.3, 40.0), &route).unwrap();
        assert_eq!(
            (leg.origin.code.as_str(), leg.destination.code.as_str()),
            ("EKCH", "ESSA")
        );
        // Just past Stockholm, heading for Chicago.
        let leg = current_leg(&aircraft_at(59.8, 17.2, 300.0), &route).unwrap();
        assert_eq!(
            (leg.origin.code.as_str(), leg.destination.code.as_str()),
            ("ESSA", "KORD")
        );
    }
}
