//! Turning a flight into a calm sentence, e.g.
//! "SK 945 → Chicago. It's 15:12 there. Landing in about 7 h 40 min."

use std::time::{Duration, SystemTime};

use crate::flight::{Flight, local_time};

/// One short line about a flight, as the CLI prints it.
pub fn dreamy_line(flight: &Flight, now: SystemTime) -> String {
    let name = flight
        .flight_number
        .as_deref()
        .unwrap_or("A plane with no callsign");
    let Some(leg) = &flight.leg else {
        return format!("{name}, destination unknown.");
    };
    let mut line = format!("{name} → {}.", leg.destination.place());
    if let Some(time) = leg
        .destination
        .time_zone
        .as_deref()
        .and_then(|tz| local_time(tz, now))
    {
        line.push_str(&format!(" It's {time} there."));
    }
    if let Some(left) = flight.time_to_landing {
        line.push_str(&format!(" Landing in about {}.", human_duration(left)));
    }
    line
}

/// `7 h 40 min`, `2 h`, `35 min`.
pub fn human_duration(d: Duration) -> String {
    let minutes = d.as_secs() / 60;
    match (minutes / 60, minutes % 60) {
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m} min"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aircraft::Aircraft;
    use crate::airport::Airport;
    use crate::geo::Coordinate;
    use crate::route::Leg;

    fn flight(leg: Option<Leg>, number: Option<&str>, left: Option<u64>) -> Flight {
        let position = Coordinate::new(59.0, 17.0).unwrap();
        Flight {
            aircraft: Aircraft {
                icao24: "4ac9e2".into(),
                callsign: None,
                registration: None,
                type_designator: None,
                category: None,
                position,
                position_time: SystemTime::UNIX_EPOCH,
                on_ground: false,
                baro_altitude_m: None,
                geo_altitude_m: None,
                ground_speed_mps: None,
                track_deg: None,
                vertical_rate_mps: None,
            },
            distance_m: 0.0,
            flight_number: number.map(str::to_owned),
            leg,
            route_source: None,
            time_to_landing: left.map(|m| Duration::from_secs(m * 60)),
        }
    }

    fn chicago(tz: Option<&str>) -> Leg {
        let airport = |code: &str, city: &str| Airport {
            code: code.into(),
            icao: Some(code.into()),
            iata: None,
            name: format!("{city} Airport"),
            city: Some(city.into()),
            country: "XX".into(),
            position: Coordinate::new(0.0, 0.0).unwrap(),
            time_zone: tz.map(str::to_owned),
        };
        Leg {
            origin: airport("ESSA", "Stockholm"),
            destination: airport("KORD", "Chicago"),
        }
    }

    #[test]
    fn full_line() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_792_959_120);
        let f = flight(
            Some(chicago(Some("America/Chicago"))),
            Some("SK 945"),
            Some(460),
        );
        assert_eq!(
            dreamy_line(&f, now),
            "SK 945 → Chicago. It's 15:12 there. Landing in about 7 h 40 min."
        );
    }

    #[test]
    fn leaves_out_what_is_unknown() {
        let now = SystemTime::UNIX_EPOCH;
        let f = flight(Some(chicago(None)), Some("SK 945"), None);
        assert_eq!(dreamy_line(&f, now), "SK 945 → Chicago.");
        assert_eq!(
            dreamy_line(&flight(None, Some("SAS1225"), None), now),
            "SAS1225, destination unknown."
        );
        assert_eq!(
            dreamy_line(&flight(None, None, None), now),
            "A plane with no callsign, destination unknown."
        );
    }

    #[test]
    fn durations_read_naturally() {
        let d = |m: u64| human_duration(Duration::from_secs(m * 60));
        assert_eq!(d(35), "35 min");
        assert_eq!(d(120), "2 h");
        assert_eq!(d(460), "7 h 40 min");
    }
}
