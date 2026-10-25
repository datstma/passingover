//! The readsb / ADS-B Exchange v2 JSON shape served by adsb.lol and adsb.fi.
//!
//! Field reference: <https://github.com/wiedehopf/readsb/blob/dev/README-json.md>.
//! Units on the wire are feet, knots and feet per minute; they are converted
//! to SI here (D22).

use std::time::{Duration, SystemTime};

use serde::Deserialize;

use super::{ParseError, Snapshot, SourceId};
use crate::aircraft::Aircraft;
use crate::geo::Coordinate;

const METRES_PER_FOOT: f64 = 0.3048;
const MPS_PER_KNOT: f64 = 1852.0 / 3600.0;
const MPS_PER_FOOT_PER_MINUTE: f64 = METRES_PER_FOOT / 60.0;

#[derive(Debug, Deserialize)]
struct Response {
    ac: Option<Vec<RawAircraft>>,
    /// Snapshot time since the Unix epoch: milliseconds in the adsb.lol and
    /// adsb.fi APIs, seconds in readsb's own `aircraft.json`.
    now: Option<f64>,
    msg: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawAircraft {
    hex: String,
    flight: Option<String>,
    r: Option<String>,
    t: Option<String>,
    category: Option<String>,
    lat: Option<f64>,
    lon: Option<f64>,
    alt_baro: Option<AltBaro>,
    alt_geom: Option<f64>,
    gs: Option<f64>,
    track: Option<f64>,
    baro_rate: Option<f64>,
    geom_rate: Option<f64>,
    /// Seconds since any message was received.
    seen: Option<f64>,
    /// Seconds since the position was last updated.
    seen_pos: Option<f64>,
}

/// `alt_baro` is feet, or the string `"ground"`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum AltBaro {
    Feet(f64),
    Text(String),
}

pub(super) fn parse(source: SourceId, body: &[u8]) -> Result<Snapshot, ParseError> {
    let response: Response = serde_json::from_slice(body)?;
    let Some(raw) = response.ac else {
        let msg = response.msg.unwrap_or_else(|| "no aircraft list".into());
        return Err(ParseError::Source(msg));
    };
    let time = response.now.map_or(SystemTime::UNIX_EPOCH, epoch_to_time);
    let aircraft = raw.into_iter().filter_map(|a| convert(a, time)).collect();
    Ok(Snapshot {
        source,
        time,
        aircraft,
    })
}

fn epoch_to_time(now: f64) -> SystemTime {
    // Anything this large is milliseconds: 1e11 seconds is the year 5138.
    // Whole milliseconds are converted exactly rather than through float
    // seconds, which would lose precision.
    let since_epoch = if now > 1e11 {
        Duration::from_millis(now as u64)
    } else {
        Duration::try_from_secs_f64(now).unwrap_or_default()
    };
    SystemTime::UNIX_EPOCH + since_epoch
}

/// Convert one aircraft; `None` if it has no valid position.
fn convert(a: RawAircraft, snapshot_time: SystemTime) -> Option<Aircraft> {
    let position = Coordinate::new(a.lat?, a.lon?).ok()?;
    let age = a
        .seen_pos
        .or(a.seen)
        .and_then(|s| Duration::try_from_secs_f64(s).ok())
        .unwrap_or_default();
    let (on_ground, baro_altitude_m) = match a.alt_baro {
        Some(AltBaro::Feet(ft)) => (false, Some(ft * METRES_PER_FOOT)),
        Some(AltBaro::Text(t)) => (t == "ground", None),
        None => (false, None),
    };
    Some(Aircraft {
        icao24: a.hex.trim().to_ascii_lowercase(),
        callsign: non_empty(a.flight),
        registration: non_empty(a.r),
        type_designator: non_empty(a.t),
        category: non_empty(a.category),
        position,
        position_time: snapshot_time.checked_sub(age).unwrap_or(snapshot_time),
        on_ground,
        baro_altitude_m,
        geo_altitude_m: a.alt_geom.map(|ft| ft * METRES_PER_FOOT),
        ground_speed_mps: a.gs.map(|kt| kt * MPS_PER_KNOT),
        track_deg: a.track,
        vertical_rate_mps: a
            .baro_rate
            .or(a.geom_rate)
            .map(|fpm| fpm * MPS_PER_FOOT_PER_MINUTE),
    })
}

/// Trim padding (callsigns are space-padded to 8 characters) and drop empties.
fn non_empty(s: Option<String>) -> Option<String> {
    s.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_one(aircraft_json: &str) -> Option<Aircraft> {
        let body = format!(r#"{{"ac":[{aircraft_json}],"now":1792966614001,"msg":"No error"}}"#);
        parse(SourceId::AdsbLol, body.as_bytes())
            .unwrap()
            .aircraft
            .into_iter()
            .next()
    }

    #[test]
    fn converts_units_to_si() {
        let a = parse_one(
            r#"{"hex":"4AC9E2","flight":"SAS2802 ","lat":55.6,"lon":12.6,
                "alt_baro":10000,"alt_geom":10250,"gs":100,"track":64.6,
                "baro_rate":-1000,"seen_pos":1.5}"#,
        )
        .unwrap();
        assert_eq!(a.icao24, "4ac9e2");
        assert_eq!(a.callsign.as_deref(), Some("SAS2802"));
        assert!((a.baro_altitude_m.unwrap() - 3048.0).abs() < 1e-9);
        assert!((a.geo_altitude_m.unwrap() - 3124.2).abs() < 1e-9);
        assert!((a.ground_speed_mps.unwrap() - 51.444_444).abs() < 1e-6);
        assert!((a.vertical_rate_mps.unwrap() + 5.08).abs() < 1e-9);
        assert_eq!(a.track_deg, Some(64.6));
        assert!(!a.on_ground);
    }

    #[test]
    fn ground_altitude_means_on_ground() {
        let a = parse_one(r#"{"hex":"45ac50","lat":55.6,"lon":12.6,"alt_baro":"ground"}"#).unwrap();
        assert!(a.on_ground);
        assert_eq!(a.baro_altitude_m, None);
    }

    #[test]
    fn position_time_is_snapshot_time_minus_seen_pos() {
        let a = parse_one(r#"{"hex":"aa","lat":1,"lon":2,"seen_pos":1.5,"seen":0.2}"#).unwrap();
        let expected = SystemTime::UNIX_EPOCH + Duration::from_millis(1_792_966_614_001 - 1_500);
        assert_eq!(a.position_time, expected);
    }

    #[test]
    fn aircraft_without_valid_position_are_skipped() {
        assert!(parse_one(r#"{"hex":"aa","lat":55.6}"#).is_none());
        assert!(parse_one(r#"{"hex":"aa","lat":95.0,"lon":12.6}"#).is_none());
    }

    #[test]
    fn blank_strings_become_none() {
        let a = parse_one(r#"{"hex":"aa","lat":1,"lon":2,"flight":"        ","r":""}"#).unwrap();
        assert_eq!(a.callsign, None);
        assert_eq!(a.registration, None);
    }

    #[test]
    fn now_in_seconds_is_accepted() {
        let snap = parse(SourceId::AdsbFi, br#"{"ac":[],"now":1792966614.5}"#).unwrap();
        assert_eq!(
            snap.time,
            SystemTime::UNIX_EPOCH + Duration::from_millis(1_792_966_614_500)
        );
    }

    #[test]
    fn missing_aircraft_list_is_a_source_error() {
        let err = parse(SourceId::AdsbLol, br#"{"msg":"invalid radius"}"#).unwrap_err();
        assert!(matches!(err, ParseError::Source(m) if m == "invalid radius"));
    }

    #[test]
    fn malformed_json_is_a_json_error() {
        let err = parse(SourceId::AdsbLol, b"<html>Bad gateway</html>").unwrap_err();
        assert!(matches!(err, ParseError::Json(_)));
    }
}
