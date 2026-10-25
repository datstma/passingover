//! Flights overhead: aircraft plus where they're going (D11, D12).

use std::time::{Duration, SystemTime};

use crate::aircraft::Aircraft;
use crate::airport::Airport;
use crate::geo::{Coordinate, Radius};
use crate::position::Snapshot;
use crate::route::{Leg, RouteInfo, RouteSourceId, current_leg};

const KNOT_MPS: f64 = 1852.0 / 3600.0;
/// Typical airliner cruise speed, used when the aircraft isn't cruising yet.
const TYPICAL_CRUISE_MPS: f64 = 450.0 * KNOT_MPS;
/// At or above FL250 and this speed, the aircraft's own ground speed is used.
const CRUISE_ALTITUDE_M: f64 = 7_620.0;
const MIN_CRUISE_SPEED_MPS: f64 = 250.0 * KNOT_MPS;
/// Within this distance of the destination the aircraft is arriving.
const ARRIVING_WITHIN_M: f64 = 100_000.0;
const MIN_ARRIVAL_SPEED_MPS: f64 = 120.0 * KNOT_MPS;
/// Added for descent and approach from cruise, and for the final landing.
const APPROACH_ALLOWANCE: Duration = Duration::from_secs(15 * 60);
const LANDING_ALLOWANCE: Duration = Duration::from_secs(5 * 60);

/// An aircraft overhead and what we know about its journey.
#[derive(Debug, Clone, PartialEq)]
pub struct Flight {
    pub aircraft: Aircraft,
    /// Distance from the user in metres. Computed on the device from the
    /// precise location, which never leaves it.
    pub distance_m: f64,
    /// For display, e.g. `SK 945`; the raw callsign if the route is unknown.
    pub flight_number: Option<String>,
    /// The leg being flown, if the route is known and plausible.
    pub leg: Option<Leg>,
    pub route_source: Option<RouteSourceId>,
    /// Estimated time to landing (D12), rounded to five minutes.
    pub time_to_landing: Option<Duration>,
}

/// The flying aircraft in `snapshot` within `radius` of `user`, nearest
/// first, with routes from `route_for`.
pub fn flights_overhead(
    snapshot: &Snapshot,
    user: Coordinate,
    radius: Radius,
    mut route_for: impl FnMut(&Aircraft) -> Option<RouteInfo>,
) -> Vec<Flight> {
    let mut nearby: Vec<(f64, &Aircraft)> = snapshot
        .aircraft
        .iter()
        .filter(|a| a.is_flying())
        .map(|a| (user.distance_to(&a.position), a))
        .filter(|(d, _)| *d <= radius.metres())
        .collect();
    nearby.sort_by(|a, b| a.0.total_cmp(&b.0));
    nearby
        .into_iter()
        .map(|(distance_m, aircraft)| {
            let route = route_for(aircraft);
            let leg = route
                .as_ref()
                .and_then(|r| current_leg(aircraft, &r.airports));
            Flight {
                distance_m,
                flight_number: route
                    .as_ref()
                    .map(|r| r.flight_number.clone())
                    .or_else(|| aircraft.callsign.clone()),
                route_source: route.as_ref().map(|r| r.source),
                time_to_landing: leg
                    .as_ref()
                    .map(|l| time_to_landing(aircraft, &l.destination)),
                leg,
                aircraft: aircraft.clone(),
            }
        })
        .collect()
}

/// Estimated time to landing at `destination` (D12): distance left at the
/// aircraft's speed (a typical cruise speed until it is cruising), plus
/// time for descent and approach, rounded to five minutes.
pub fn time_to_landing(aircraft: &Aircraft, destination: &Airport) -> Duration {
    let remaining = aircraft.position.distance_to(&destination.position);
    let speed = aircraft.ground_speed_mps;
    let (seconds, allowance) = if remaining < ARRIVING_WITHIN_M {
        let speed = speed.unwrap_or(0.0).max(MIN_ARRIVAL_SPEED_MPS);
        (remaining / speed, LANDING_ALLOWANCE)
    } else {
        let cruising = aircraft
            .altitude_m()
            .is_some_and(|a| a >= CRUISE_ALTITUDE_M)
            && speed.is_some_and(|s| s >= MIN_CRUISE_SPEED_MPS);
        let speed = if cruising {
            speed.unwrap_or(TYPICAL_CRUISE_MPS)
        } else {
            TYPICAL_CRUISE_MPS
        };
        (remaining / speed, APPROACH_ALLOWANCE)
    };
    round_to_five_minutes(Duration::from_secs_f64(seconds) + allowance)
}

/// Round to the nearest five minutes, at least five.
pub fn round_to_five_minutes(d: Duration) -> Duration {
    let fives = (d.as_secs_f64() / 300.0).round().max(1.0);
    Duration::from_secs(fives as u64 * 300)
}

/// Local wall-clock time in an IANA time zone, as `HH:MM`.
pub fn local_time(time_zone: &str, at: SystemTime) -> Option<String> {
    let zoned = jiff::Timestamp::try_from(at).ok()?.in_tz(time_zone).ok()?;
    Some(zoned.strftime("%H:%M").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn airport(lat: f64, lon: f64) -> Airport {
        Airport {
            code: "KORD".into(),
            icao: Some("KORD".into()),
            iata: Some("ORD".into()),
            name: "Chicago O'Hare".into(),
            city: Some("Chicago".into()),
            country: "US".into(),
            position: Coordinate::new(lat, lon).unwrap(),
            time_zone: Some("America/Chicago".into()),
        }
    }

    fn aircraft(lat: f64, lon: f64, altitude_m: f64, speed_kt: f64) -> Aircraft {
        Aircraft {
            icao24: "abc123".into(),
            callsign: Some("SAS945".into()),
            registration: None,
            type_designator: None,
            category: Some("A5".into()),
            position: Coordinate::new(lat, lon).unwrap(),
            position_time: SystemTime::UNIX_EPOCH,
            on_ground: false,
            baro_altitude_m: Some(altitude_m),
            geo_altitude_m: None,
            ground_speed_mps: Some(speed_kt * KNOT_MPS),
            track_deg: Some(290.0),
            vertical_rate_mps: None,
        }
    }

    fn minutes(d: Duration) -> u64 {
        d.as_secs() / 60
    }

    #[test]
    fn cruising_aircraft_use_their_ground_speed() {
        // 1000 km from the destination at FL350 and 500 kt (926 km/h):
        // 65 min + 15 min approach = 80 min.
        let dest = airport(0.0, 0.0);
        let ac = aircraft(8.993, 0.0, 10_668.0, 500.0);
        assert_eq!(minutes(time_to_landing(&ac, &dest)), 80);
    }

    #[test]
    fn climbing_aircraft_use_a_typical_cruise_speed() {
        // Same distance, climbing slowly through 3000 m: 450 kt (833 km/h)
        // is 72 min, plus 15 min = 87 min, rounded to 85.
        let dest = airport(0.0, 0.0);
        let ac = aircraft(8.993, 0.0, 3_000.0, 250.0);
        assert_eq!(minutes(time_to_landing(&ac, &dest)), 85);
    }

    #[test]
    fn arriving_aircraft_use_their_own_speed_and_a_short_allowance() {
        // 30 km out at 180 kt (333 km/h): 5.4 min + 5 min, rounded to 10.
        let dest = airport(0.0, 0.0);
        let ac = aircraft(0.2698, 0.0, 900.0, 180.0);
        assert_eq!(minutes(time_to_landing(&ac, &dest)), 10);
    }

    #[test]
    fn rounding_is_to_five_minutes_and_never_zero() {
        let r = |s| minutes(round_to_five_minutes(Duration::from_secs(s)));
        assert_eq!(r(0), 5);
        assert_eq!(r(149), 5);
        assert_eq!(r(451), 10);
        assert_eq!(r(7 * 3600 + 38 * 60), 460);
    }

    #[test]
    fn local_time_uses_the_destination_zone() {
        // 2026-10-25 20:12 UTC is 15:12 in Chicago (CDT, UTC-5).
        let at = SystemTime::UNIX_EPOCH + Duration::from_secs(1_792_959_120);
        assert_eq!(local_time("America/Chicago", at).as_deref(), Some("15:12"));
        assert_eq!(local_time("Europe/Stockholm", at).as_deref(), Some("21:12"));
        assert_eq!(local_time("Not/AZone", at), None);
    }
}
