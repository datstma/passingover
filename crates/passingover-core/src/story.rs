//! Turning a flight into calm sentences, e.g.
//! "SK 945 → Chicago. It's 15:12 there, 9°C and overcast. Landing in about
//! 7 h 40 min." and "Left Stockholm about 1 h 10 min ago and will cross the
//! Norwegian Sea, Greenland and Canada."

use std::time::{Duration, SystemTime};

use crate::flight::{Flight, local_time};
use crate::log::LogStats;
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

/// Gentle sentences about the daydream log for a period such as "This
/// month" (D38).
pub fn log_summary(stats: &LogStats, period: &str, units: Units) -> Vec<String> {
    if stats.flights == 0 {
        return vec![if stats.first_seen_unix_ms.is_none() {
            "Nothing here yet. Planes that pass over will be remembered here.".to_owned()
        } else {
            format!("{period}, no flights with a known destination passed over.")
        }];
    }
    let planes = plural(stats.flights, "plane", "planes");
    let mut lines = vec![if stats.places == 1 {
        let only = stats.top.first().map_or("", |t| t.0.as_str());
        format!("{period}, {planes} passed over, all on their way to {only}.")
    } else {
        format!(
            "{period}, {planes} passed over on their way to {} in {}.",
            plural(stats.places, "city", "cities"),
            plural(stats.countries, "country", "countries"),
        )
    }];
    if let Some((place, km)) = &stats.farthest
        && stats.places > 1
    {
        lines.push(format!(
            "The farthest was going to {place}, about {} away.",
            distance(*km, units)
        ));
    }
    let often: Vec<String> = stats
        .top
        .iter()
        .filter(|(_, n)| *n > 1)
        .map(|(place, n)| format!("{place} ({})", grouped(*n)))
        .collect();
    if stats.places > 1 && !often.is_empty() {
        lines.push(format!("Most often: {}.", list(&often)));
    }
    lines
}

/// `1 plane`, `1,284 planes`.
fn plural(n: u32, one: &str, many: &str) -> String {
    format!("{} {}", grouped(n), if n == 1 { one } else { many })
}

/// `1,284`.
fn grouped(n: u32) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// `10,000 km` or `6,200 mi`, rounded to a calm figure.
fn distance(km: f64, units: Units) -> String {
    let (value, unit) = match units {
        Units::Metric => (km, "km"),
        Units::Imperial => (km / 1.609_344, "mi"),
    };
    let step = if value >= 1000.0 { 100.0 } else { 10.0 };
    let rounded = ((value / step).round() * step).max(step) as u32;
    format!("{} {unit}", grouped(rounded))
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

    fn stats(flights: u32, places: u32, countries: u32) -> LogStats {
        LogStats {
            flights,
            places,
            countries,
            top: vec![
                ("London".into(), 34),
                ("Paris".into(), 21),
                ("Oslo".into(), 1),
            ],
            farthest: Some(("Singapore".into(), 9_987.0)),
            first_seen_unix_ms: Some(0),
        }
    }

    #[test]
    fn the_log_reads_gently() {
        assert_eq!(
            log_summary(&stats(1284, 61, 24), "This month", Units::Metric),
            [
                "This month, 1,284 planes passed over on their way to 61 cities in 24 countries.",
                "The farthest was going to Singapore, about 10,000 km away.",
                "Most often: London (34) and Paris (21).",
            ]
        );
        assert_eq!(
            log_summary(&stats(5, 2, 1), "Today", Units::Imperial)[1],
            "The farthest was going to Singapore, about 6,200 mi away."
        );
        let mut one = stats(3, 1, 1);
        one.top.truncate(1);
        assert_eq!(
            log_summary(&one, "This week", Units::Metric),
            ["This week, 3 planes passed over, all on their way to London."]
        );
    }

    #[test]
    fn an_empty_log_says_so() {
        let mut empty = stats(0, 0, 0);
        assert_eq!(
            log_summary(&empty, "This week", Units::Metric),
            ["This week, no flights with a known destination passed over."]
        );
        empty.first_seen_unix_ms = None;
        assert!(log_summary(&empty, "This week", Units::Metric)[0].starts_with("Nothing here yet"));
    }

    #[test]
    fn durations_read_naturally() {
        let d = |m: u64| human_duration(Duration::from_secs(m * 60));
        assert_eq!(d(35), "35 min");
        assert_eq!(d(120), "2 h");
        assert_eq!(d(460), "7 h 40 min");
    }
}
