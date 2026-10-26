//! Weather at the destination (M3, D13, D34): Open-Meteo first, MET Norway as
//! fallback. Requests carry destination airport coordinates only, never the
//! user's location.

use std::collections::HashMap;
use std::time::Instant;

use serde::Deserialize;

use crate::geo::Coordinate;
use crate::http::{HttpClient, ParseError, Request};
use crate::polite::{
    MET_NORWAY_PER_POLL, MET_NORWAY_TTL, OPEN_METEO_BATCH, OPEN_METEO_TTL, WEATHER_SOURCE_PAUSE,
};

/// What the sky is doing, in a few words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    Clear,
    MostlyClear,
    PartlyCloudy,
    Overcast,
    Fog,
    Drizzle,
    FreezingDrizzle,
    Rain,
    FreezingRain,
    Sleet,
    Snow,
    RainShowers,
    SnowShowers,
    Thunderstorms,
}

impl Condition {
    /// For "9°C and overcast".
    pub fn phrase(self) -> &'static str {
        match self {
            Condition::Clear => "clear",
            Condition::MostlyClear => "mostly clear",
            Condition::PartlyCloudy => "partly cloudy",
            Condition::Overcast => "overcast",
            Condition::Fog => "foggy",
            Condition::Drizzle => "drizzling",
            Condition::FreezingDrizzle => "freezing drizzle",
            Condition::Rain => "raining",
            Condition::FreezingRain => "freezing rain",
            Condition::Sleet => "sleet",
            Condition::Snow => "snowing",
            Condition::RainShowers => "rain showers",
            Condition::SnowShowers => "snow showers",
            Condition::Thunderstorms => "thunderstorms",
        }
    }

    /// From a WMO weather code, as Open-Meteo reports it.
    pub fn from_wmo(code: u32) -> Option<Self> {
        Some(match code {
            0 => Condition::Clear,
            1 => Condition::MostlyClear,
            2 => Condition::PartlyCloudy,
            3 => Condition::Overcast,
            45 | 48 => Condition::Fog,
            51 | 53 | 55 => Condition::Drizzle,
            56 | 57 => Condition::FreezingDrizzle,
            61 | 63 | 65 => Condition::Rain,
            66 | 67 => Condition::FreezingRain,
            71 | 73 | 75 | 77 => Condition::Snow,
            80..=82 => Condition::RainShowers,
            85 | 86 => Condition::SnowShowers,
            95..=99 => Condition::Thunderstorms,
            _ => return None,
        })
    }

    /// From a MET Norway symbol code such as `partlycloudy_night`.
    pub fn from_met_symbol(symbol: &str) -> Option<Self> {
        let base = symbol.split('_').next()?;
        Some(match base {
            "clearsky" => Condition::Clear,
            "fair" => Condition::MostlyClear,
            "partlycloudy" => Condition::PartlyCloudy,
            "cloudy" => Condition::Overcast,
            "fog" => Condition::Fog,
            s if s.contains("thunder") => Condition::Thunderstorms,
            s if s.contains("sleet") => Condition::Sleet,
            s if s.contains("snowshowers") => Condition::SnowShowers,
            s if s.contains("snow") => Condition::Snow,
            s if s.contains("rainshowers") => Condition::RainShowers,
            s if s.contains("rain") => Condition::Rain,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Weather {
    pub temperature_c: f64,
    pub condition: Option<Condition>,
}

/// Where weather came from, for attribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeatherSourceId {
    OpenMeteo,
    MetNorway,
}

/// A source of current weather (sans-IO, D21).
pub trait WeatherSource {
    fn id(&self) -> WeatherSourceId;
    /// Places per request.
    fn max_batch(&self) -> usize;
    fn request(&self, places: &[Coordinate]) -> Request;
    /// One answer per requested place, in order.
    fn parse(&self, body: &[u8], places: usize) -> Result<Vec<Option<Weather>>, ParseError>;
}

/// Open-Meteo's forecast API, `current` values. Free for non-commercial use;
/// requires the link "Weather data by Open-Meteo.com" next to the data.
#[derive(Debug, Clone, Copy, Default)]
pub struct OpenMeteo;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum OpenMeteoBody {
    Many(Vec<OpenMeteoPlace>),
    One(OpenMeteoPlace),
}

#[derive(Debug, Deserialize)]
struct OpenMeteoPlace {
    current: Option<OpenMeteoCurrent>,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoCurrent {
    temperature_2m: Option<f64>,
    weather_code: Option<u32>,
}

impl WeatherSource for OpenMeteo {
    fn id(&self) -> WeatherSourceId {
        WeatherSourceId::OpenMeteo
    }

    fn max_batch(&self) -> usize {
        OPEN_METEO_BATCH
    }

    fn request(&self, places: &[Coordinate]) -> Request {
        let join = |f: fn(&Coordinate) -> f64| {
            places
                .iter()
                .map(|p| format!("{:.2}", f(p)))
                .collect::<Vec<_>>()
                .join(",")
        };
        Request {
            url: format!(
                "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}\
                 &current=temperature_2m,weather_code&timezone=GMT",
                join(Coordinate::lat),
                join(Coordinate::lon),
            ),
        }
    }

    fn parse(&self, body: &[u8], places: usize) -> Result<Vec<Option<Weather>>, ParseError> {
        let list = match serde_json::from_slice(body)? {
            OpenMeteoBody::Many(list) => list,
            OpenMeteoBody::One(one) => vec![one],
        };
        if list.len() != places {
            return Err(ParseError::Source(format!(
                "asked for {places} places, got {}",
                list.len()
            )));
        }
        Ok(list
            .into_iter()
            .map(|p| {
                let c = p.current?;
                Some(Weather {
                    temperature_c: c.temperature_2m?,
                    condition: c.weather_code.and_then(Condition::from_wmo),
                })
            })
            .collect())
    }
}

/// MET Norway's Locationforecast (compact). Needs an identifying
/// User-Agent; one place per request.
#[derive(Debug, Clone, Copy, Default)]
pub struct MetNorway;

#[derive(Debug, Deserialize)]
struct MetBody {
    properties: MetProperties,
}

#[derive(Debug, Deserialize)]
struct MetProperties {
    timeseries: Vec<MetStep>,
}

#[derive(Debug, Deserialize)]
struct MetStep {
    data: MetData,
}

#[derive(Debug, Deserialize)]
struct MetData {
    instant: MetInstant,
    next_1_hours: Option<MetNext>,
    next_6_hours: Option<MetNext>,
}

#[derive(Debug, Deserialize)]
struct MetInstant {
    details: MetDetails,
}

#[derive(Debug, Deserialize)]
struct MetDetails {
    air_temperature: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct MetNext {
    summary: MetSummary,
}

#[derive(Debug, Deserialize)]
struct MetSummary {
    symbol_code: String,
}

impl WeatherSource for MetNorway {
    fn id(&self) -> WeatherSourceId {
        WeatherSourceId::MetNorway
    }

    fn max_batch(&self) -> usize {
        1
    }

    fn request(&self, places: &[Coordinate]) -> Request {
        let p = places[0];
        Request {
            url: format!(
                "https://api.met.no/weatherapi/locationforecast/2.0/compact?lat={:.2}&lon={:.2}",
                p.lat(),
                p.lon()
            ),
        }
    }

    fn parse(&self, body: &[u8], _places: usize) -> Result<Vec<Option<Weather>>, ParseError> {
        let body: MetBody = serde_json::from_slice(body)?;
        let weather = body
            .properties
            .timeseries
            .into_iter()
            .next()
            .and_then(|step| {
                let next = step.data.next_1_hours.or(step.data.next_6_hours);
                Some(Weather {
                    temperature_c: step.data.instant.details.air_temperature?,
                    condition: next
                        .and_then(|n| Condition::from_met_symbol(&n.summary.symbol_code)),
                })
            });
        Ok(vec![weather])
    }
}

/// Weather for destination airports, cached per airport (D15, D34).
#[derive(Debug, Default)]
pub struct WeatherLookup {
    /// Airport code → (expiry, answer, source).
    cache: HashMap<String, (Instant, Option<Weather>, WeatherSourceId)>,
    open_meteo_paused_until: Option<Instant>,
    met_paused_until: Option<Instant>,
}

impl WeatherLookup {
    pub fn new() -> Self {
        Self::default()
    }

    /// Weather for each `(airport code, position)`. Fetches what isn't cached
    /// when `client` is given: one batched Open-Meteo request, or a few MET
    /// Norway requests if Open-Meteo fails.
    pub fn get(
        &mut self,
        places: &[(String, Coordinate)],
        client: Option<&dyn HttpClient>,
        now: Instant,
    ) -> HashMap<String, (Weather, WeatherSourceId)> {
        let mut missing: Vec<&(String, Coordinate)> = Vec::new();
        for place in places {
            let fresh = self.cache.get(&place.0).is_some_and(|(t, ..)| now < *t);
            if !fresh && !missing.iter().any(|m| m.0 == place.0) {
                missing.push(place);
            }
        }
        if let Some(client) = client
            && !missing.is_empty()
        {
            self.fetch(&missing, client, now);
        }
        places
            .iter()
            .filter_map(|(code, _)| {
                let (_, weather, source) = self.cache.get(code)?;
                Some((code.clone(), ((*weather)?, *source)))
            })
            .collect()
    }

    fn fetch(&mut self, missing: &[&(String, Coordinate)], client: &dyn HttpClient, now: Instant) {
        let open_meteo_ok = self.open_meteo_paused_until.is_none_or(|t| now >= t);
        if open_meteo_ok {
            let batch = &missing[..missing.len().min(OPEN_METEO_BATCH)];
            if self.ask(&OpenMeteo, batch, client, now, OPEN_METEO_TTL) {
                return;
            }
            self.open_meteo_paused_until = Some(now + WEATHER_SOURCE_PAUSE);
        }
        if self.met_paused_until.is_some_and(|t| now < t) {
            return;
        }
        for place in missing.iter().take(MET_NORWAY_PER_POLL) {
            if !self.ask(&MetNorway, &[place], client, now, MET_NORWAY_TTL) {
                self.met_paused_until = Some(now + WEATHER_SOURCE_PAUSE);
                return;
            }
        }
    }

    /// One request; caches the answers. False if the request failed.
    fn ask(
        &mut self,
        source: &dyn WeatherSource,
        places: &[&(String, Coordinate)],
        client: &dyn HttpClient,
        now: Instant,
        ttl: std::time::Duration,
    ) -> bool {
        let coordinates: Vec<Coordinate> = places.iter().map(|p| p.1).collect();
        let answers = match client.get(&source.request(&coordinates)) {
            Ok(r) if r.status == 200 => source.parse(&r.body, places.len()),
            _ => return false,
        };
        let Ok(answers) = answers else {
            return false;
        };
        for (place, weather) in places.iter().zip(answers) {
            self.cache
                .insert(place.0.clone(), (now + ttl, weather, source.id()));
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::time::Duration;

    use super::*;
    use crate::http::{Response, TransportError};

    const OPEN_METEO: &[u8] =
        include_bytes!("../tests/fixtures/weather/open_meteo_current_kord_essa.json");
    const MET: &[u8] = include_bytes!("../tests/fixtures/weather/met_norway_compact_kord.json");

    fn kord() -> Coordinate {
        Coordinate::new(41.98, -87.9).unwrap()
    }
    fn essa() -> Coordinate {
        Coordinate::new(59.65, 17.92).unwrap()
    }

    #[test]
    fn open_meteo_batches_places_and_parses_in_order() {
        let url = OpenMeteo.request(&[kord(), essa()]).url;
        assert!(
            url.contains("latitude=41.98,59.65&longitude=-87.90,17.92"),
            "{url}"
        );
        let w = OpenMeteo.parse(OPEN_METEO, 2).unwrap();
        assert_eq!(
            w,
            [
                Some(Weather {
                    temperature_c: 16.0,
                    condition: Some(Condition::Clear)
                }),
                Some(Weather {
                    temperature_c: 9.9,
                    condition: Some(Condition::Overcast)
                }),
            ]
        );
        assert!(OpenMeteo.parse(OPEN_METEO, 3).is_err());
    }

    #[test]
    fn met_norway_reads_the_first_step() {
        let url = MetNorway.request(&[kord()]).url;
        assert!(url.ends_with("compact?lat=41.98&lon=-87.90"), "{url}");
        let w = MetNorway.parse(MET, 1).unwrap();
        assert_eq!(
            w,
            [Some(Weather {
                temperature_c: 16.4,
                condition: Some(Condition::PartlyCloudy)
            })]
        );
    }

    #[test]
    fn codes_map_to_phrases() {
        assert_eq!(Condition::from_wmo(63).unwrap().phrase(), "raining");
        assert_eq!(Condition::from_wmo(86).unwrap().phrase(), "snow showers");
        assert_eq!(Condition::from_wmo(4), None);
        assert_eq!(
            Condition::from_met_symbol("lightrainshowersandthunder_day"),
            Some(Condition::Thunderstorms)
        );
        assert_eq!(
            Condition::from_met_symbol("heavysnowshowers_night"),
            Some(Condition::SnowShowers)
        );
        assert_eq!(
            Condition::from_met_symbol("cloudy"),
            Some(Condition::Overcast)
        );
    }

    /// Serves Open-Meteo or MET answers by host and counts requests.
    struct Hosts {
        open_meteo: Option<&'static [u8]>,
        calls: RefCell<Vec<&'static str>>,
    }

    impl HttpClient for Hosts {
        fn get(&self, request: &Request) -> Result<Response, TransportError> {
            if request.url.contains("open-meteo") {
                self.calls.borrow_mut().push("open-meteo");
                match self.open_meteo {
                    Some(body) => Ok(Response::new(200, Vec::new(), body.to_vec())),
                    None => Ok(Response::new(503, Vec::new(), Vec::new())),
                }
            } else {
                self.calls.borrow_mut().push("met");
                Ok(Response::new(200, Vec::new(), MET.to_vec()))
            }
        }
    }

    fn places() -> Vec<(String, Coordinate)> {
        vec![("KORD".into(), kord()), ("ESSA".into(), essa())]
    }

    #[test]
    fn one_batched_request_then_the_cache() {
        let t0 = Instant::now();
        let client = Hosts {
            open_meteo: Some(OPEN_METEO),
            calls: RefCell::new(Vec::new()),
        };
        let mut lookup = WeatherLookup::new();
        let w = lookup.get(&places(), Some(&client), t0);
        assert_eq!(w["ESSA"].0.temperature_c, 9.9);
        assert_eq!(w["KORD"].1, WeatherSourceId::OpenMeteo);
        lookup.get(&places(), Some(&client), t0 + Duration::from_secs(14 * 60));
        assert_eq!(*client.calls.borrow(), ["open-meteo"]);
        // Without a client, only the cache answers.
        let cached = lookup.get(&places(), None, t0 + Duration::from_secs(60));
        assert_eq!(cached.len(), 2);
    }

    #[test]
    fn falls_back_to_met_norway_and_pauses_open_meteo() {
        let t0 = Instant::now();
        let client = Hosts {
            open_meteo: None,
            calls: RefCell::new(Vec::new()),
        };
        let mut lookup = WeatherLookup::new();
        let w = lookup.get(&places()[..1], Some(&client), t0);
        assert_eq!(
            w["KORD"],
            (
                Weather {
                    temperature_c: 16.4,
                    condition: Some(Condition::PartlyCloudy)
                },
                WeatherSourceId::MetNorway
            )
        );
        // Open-Meteo is paused for five minutes: the next miss goes to MET.
        lookup.get(&places(), Some(&client), t0 + Duration::from_secs(60));
        assert_eq!(*client.calls.borrow(), ["open-meteo", "met", "met"]);
    }
}
