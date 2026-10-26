//! Turning a flight into calm sentences, e.g.
//! "SK 945 → Chicago. It's 15:12 there, 9°C and overcast. Landing in about
//! 7 h 40 min." and "Left Stockholm about 1 h 10 min ago and will cross the
//! Norwegian Sea, Greenland and Canada."

use std::time::{Duration, SystemTime};

use crate::flight::{Flight, local_time};
use crate::weather::Weather;

/// How to show values that have units. Metric by default.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Units {
    #[default]
    Metric,
    Imperial,
}

/// A flight's name for display: `SK 945`, its callsign, or a description.
pub fn title(flight: &Flight) -> &str {
    flight
        .flight_number
        .as_deref()
        .unwrap_or("A plane with no callsign")
}

/// `about 7 h 40 min`, for an estimated time to landing (D12).
pub fn landing_phrase(left: Duration) -> String {
    format!("about {}", human_duration(left))
}

/// `9°C` or `48°F`, rounded to whole degrees.
pub fn temperature(celsius: f64, units: Units) -> String {
    let value = match units {
        Units::Metric => celsius,
        Units::Imperial => celsius * 9.0 / 5.0 + 32.0,
    };
    // `+ 0.0` keeps -0.4 from printing as "-0".
    let rounded = value.round() + 0.0;
    match units {
        Units::Metric => format!("{rounded:.0}°C"),
        Units::Imperial => format!("{rounded:.0}°F"),
    }
}

/// `9°C and overcast`, or just `9°C` if the sky is unknown.
pub fn weather_phrase(weather: &Weather, units: Units) -> String {
    let t = temperature(weather.temperature_c, units);
    match weather.condition {
        Some(c) => format!("{t} and {}", c.phrase()),
        None => t,
    }
}

/// `A`, `A and B`, `A, B and C`.
pub fn list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// One short line about a flight, as the CLI prints it.
pub fn dreamy_line(flight: &Flight, now: SystemTime, units: Units) -> String {
    let name = title(flight);
    let Some(leg) = &flight.leg else {
        return format!("{name}, destination unknown.");
    };
    let place = leg.destination.place();
    if flight.lands_nearby {
        return match flight.time_to_landing {
            Some(left) => format!(
                "{name} → {place}, landing nearby in {}.",
                landing_phrase(left)
            ),
            None => format!("{name} → {place}, landing nearby."),
        };
    }
    let mut line = format!("{name} → {place}.");
    let time = leg
        .destination
        .time_zone
        .as_deref()
        .and_then(|tz| local_time(tz, now));
    let weather = flight.weather.as_ref().map(|w| weather_phrase(w, units));
    match (time, weather) {
        (Some(t), Some(w)) => line.push_str(&format!(" It's {t} there, {w}.")),
        (Some(t), None) => line.push_str(&format!(" It's {t} there.")),
        (None, Some(w)) => line.push_str(&format!(" It's {w} there.")),
        (None, None) => {}
    }
    if let Some(left) = flight.time_to_landing {
        line.push_str(&format!(" Landing in {}.", landing_phrase(left)));
    }
    line
}

/// Where the flight came from, how long ago, and what it will cross:
/// "Left Stockholm about 1 h 10 min ago and will cross the Norwegian Sea,
/// Greenland and Canada."
pub fn journey(flight: &Flight) -> Option<String> {
    let leg = flight.leg.as_ref()?;
    let origin = leg.origin.place();
    let left = match flight.departed_ago? {
        Duration::ZERO => format!("Just left {origin}"),
        ago => format!("Left {origin} about {} ago", human_duration(ago)),
    };
    Some(if flight.ahead.is_empty() {
        format!("{left}.")
    } else {
        format!("{left} and will cross {}.", list(&flight.ahead))
    })
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
    use crate::weather::Condition;

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
            lands_nearby: false,
            departed_ago: None,
            ahead: Vec::new(),
            weather: None,
            weather_source: None,
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

    fn overcast(c: f64) -> Weather {
        Weather {
            temperature_c: c,
            condition: Some(Condition::Overcast),
        }
    }

    // 2026-10-25 20:12 UTC, 15:12 in Chicago.
    fn evening() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_792_959_120)
    }

    #[test]
    fn full_line_with_weather() {
        let mut f = flight(
            Some(chicago(Some("America/Chicago"))),
            Some("SK 945"),
            Some(460),
        );
        f.weather = Some(overcast(9.2));
        assert_eq!(
            dreamy_line(&f, evening(), Units::Metric),
            "SK 945 → Chicago. It's 15:12 there, 9°C and overcast. Landing in about 7 h 40 min."
        );
        assert_eq!(
            dreamy_line(&f, evening(), Units::Imperial),
            "SK 945 → Chicago. It's 15:12 there, 49°F and overcast. Landing in about 7 h 40 min."
        );
    }

    #[test]
    fn leaves_out_what_is_unknown() {
        let now = SystemTime::UNIX_EPOCH;
        let mut f = flight(Some(chicago(None)), Some("SK 945"), None);
        assert_eq!(dreamy_line(&f, now, Units::Metric), "SK 945 → Chicago.");
        f.weather = Some(overcast(-0.4));
        assert_eq!(
            dreamy_line(&f, now, Units::Metric),
            "SK 945 → Chicago. It's 0°C and overcast there."
        );
        assert_eq!(
            dreamy_line(&flight(None, Some("SAS1225"), None), now, Units::Metric),
            "SAS1225, destination unknown."
        );
        assert_eq!(
            dreamy_line(&flight(None, None, None), now, Units::Metric),
            "A plane with no callsign, destination unknown."
        );
    }

    #[test]
    fn landing_nearby_skips_the_time_and_weather_there() {
        let mut f = flight(
            Some(chicago(Some("America/Chicago"))),
            Some("AA 43"),
            Some(5),
        );
        f.weather = Some(overcast(11.0));
        f.lands_nearby = true;
        assert_eq!(
            dreamy_line(&f, evening(), Units::Metric),
            "AA 43 → Chicago, landing nearby in about 5 min."
        );
    }

    #[test]
    fn journey_tells_where_it_left_and_what_lies_ahead() {
        let mut f = flight(Some(chicago(None)), Some("SK 945"), Some(460));
        assert_eq!(journey(&f), None, "no departure estimate yet");
        f.departed_ago = Some(Duration::from_secs(70 * 60));
        assert_eq!(journey(&f).unwrap(), "Left Stockholm about 1 h 10 min ago.");
        f.ahead = vec![
            "the Norwegian Sea".into(),
            "Greenland".into(),
            "Canada".into(),
        ];
        assert_eq!(
            journey(&f).unwrap(),
            "Left Stockholm about 1 h 10 min ago and will cross the Norwegian Sea, Greenland and Canada."
        );
        f.departed_ago = Some(Duration::ZERO);
        f.ahead.truncate(1);
        assert_eq!(
            journey(&f).unwrap(),
            "Just left Stockholm and will cross the Norwegian Sea."
        );
    }

    #[test]
    fn temperatures_and_lists_read_naturally() {
        assert_eq!(temperature(-3.4, Units::Metric), "-3°C");
        assert_eq!(temperature(-0.2, Units::Metric), "0°C");
        assert_eq!(temperature(20.0, Units::Imperial), "68°F");
        assert_eq!(list(&["A".into()]), "A");
        assert_eq!(list(&["A".into(), "B".into()]), "A and B");
        assert_eq!(list(&["A".into(), "B".into(), "C".into()]), "A, B and C");
    }

    #[test]
    fn durations_read_naturally() {
        let d = |m: u64| human_duration(Duration::from_secs(m * 60));
        assert_eq!(d(35), "35 min");
        assert_eq!(d(120), "2 h");
        assert_eq!(d(460), "7 h 40 min");
    }
}
