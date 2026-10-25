//! Route lookup: bundled standing data first, then adsbdb (D11, D15).

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use super::{AdsbDb, RouteInfo, RouteSource, RouteSourceId, flight_number, normalise_callsign};
use crate::data::StandingData;
use crate::http::HttpClient;
use crate::polite::{
    ROUTE_CACHE_TTL, ROUTE_LOOKUPS_PER_MINUTE, ROUTE_MISS_CACHE_TTL, ROUTE_SOURCE_PAUSE,
};

/// Finds routes by callsign. adsbdb answers are kept in memory only, never
/// written anywhere (its data may not be stored, D11).
#[derive(Debug, Default)]
pub struct RouteLookup {
    online: AdsbDb,
    /// Normalised callsign → (expiry, answer).
    cache: HashMap<String, (Instant, Option<RouteInfo>)>,
    /// When recent online lookups were made, for the per-minute limit.
    recent: VecDeque<Instant>,
    paused_until: Option<Instant>,
}

impl RouteLookup {
    pub fn new() -> Self {
        Self::default()
    }

    /// The route for a broadcast callsign. `data` is the bundled standing
    /// data, if loaded; `client` enables the online fallback.
    pub fn lookup(
        &mut self,
        raw_callsign: &str,
        data: Option<&StandingData>,
        client: Option<&dyn HttpClient>,
        now: Instant,
    ) -> Option<RouteInfo> {
        let iata_to_code = |iata: &str| Some(data?.airline_code_for_iata(iata)?.to_owned());
        let (code, number) = normalise_callsign(raw_callsign, iata_to_code)?;
        if let Some(info) = data.and_then(|d| from_standing_data(d, &code, &number)) {
            return Some(info);
        }
        let callsign = format!("{code}{number}");
        let mut info = self.online(&callsign, client?, now)?;
        // adsbdb has no time zones; take them from the bundled airports.
        if let Some(data) = data {
            for airport in &mut info.airports {
                let known = airport
                    .icao
                    .as_deref()
                    .and_then(|icao| data.airport_by_icao(icao));
                airport.time_zone = known.and_then(|a| a.time_zone.clone());
            }
        }
        Some(info)
    }

    fn online(
        &mut self,
        callsign: &str,
        client: &dyn HttpClient,
        now: Instant,
    ) -> Option<RouteInfo> {
        if let Some((expires, answer)) = self.cache.get(callsign)
            && now < *expires
        {
            return answer.clone();
        }
        if self.paused_until.is_some_and(|t| now < t) {
            return None;
        }
        while self
            .recent
            .front()
            .is_some_and(|&t| now.duration_since(t) >= Duration::from_secs(60))
        {
            self.recent.pop_front();
        }
        if self.recent.len() >= ROUTE_LOOKUPS_PER_MINUTE {
            return None;
        }
        self.recent.push_back(now);

        let response = client.get(&self.online.request(callsign));
        let answer = match response {
            Ok(r) if r.status == 200 => self.online.parse(&r.body).ok(),
            Ok(r) if r.status == 404 => Some(None),
            Ok(r) if r.status == 429 || r.status >= 500 => None,
            Ok(_) => Some(None),
            Err(_) => None,
        };
        let Some(answer) = answer else {
            self.paused_until = Some(now + ROUTE_SOURCE_PAUSE);
            return None;
        };
        let ttl = if answer.is_some() {
            ROUTE_CACHE_TTL
        } else {
            ROUTE_MISS_CACHE_TTL
        };
        self.cache
            .insert(callsign.to_owned(), (now + ttl, answer.clone()));
        answer
    }
}

fn from_standing_data(data: &StandingData, code: &str, number: &str) -> Option<RouteInfo> {
    let callsign = format!("{code}{number}");
    let stored = data.route(&callsign)?;
    let airports = stored
        .airport_codes
        .iter()
        .map(|c| data.airport(c).cloned())
        .collect::<Option<Vec<_>>>()?;
    (airports.len() >= 2).then(|| RouteInfo {
        flight_number: flight_number(data.airline(code), code, number),
        callsign,
        airports,
        source: RouteSourceId::StandingData,
    })
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::data::{AIRLINES_HEADER, AIRPORTS_HEADER, ROUTES_HEADER};
    use crate::http::{Request, Response, TransportError};

    const ADSBDB_SAS945: &[u8] =
        include_bytes!("../../tests/fixtures/routes/adsbdb_callsign_SAS945.json");
    const ADSBDB_UNKNOWN: &[u8] =
        include_bytes!("../../tests/fixtures/routes/adsbdb_callsign_unknown.json");

    /// Serves one response for every request and counts requests.
    struct Fixed {
        status: u16,
        body: &'static [u8],
        calls: RefCell<usize>,
    }

    impl Fixed {
        fn new(status: u16, body: &'static [u8]) -> Self {
            Self {
                status,
                body,
                calls: RefCell::new(0),
            }
        }
    }

    impl HttpClient for Fixed {
        fn get(&self, _: &Request) -> Result<Response, TransportError> {
            *self.calls.borrow_mut() += 1;
            Ok(Response::new(self.status, Vec::new(), self.body.to_vec()))
        }
    }

    fn data() -> StandingData {
        let routes = format!("{ROUTES_HEADER}\nSAS\t1225\tEKCH-ESSA\n");
        let airports = format!(
            "{AIRPORTS_HEADER}\n\
             EKCH\tEKCH\tCPH\tCopenhagen Kastrup\tCopenhagen\tDK\t55.618\t12.656\tEurope/Copenhagen\n\
             ESSA\tESSA\tARN\tStockholm-Arlanda Airport\tStockholm\tSE\t59.652\t17.919\tEurope/Stockholm\n\
             KORD\tKORD\tORD\tChicago O'Hare\tChicago\tUS\t41.979\t-87.905\tAmerica/Chicago\n"
        );
        let airlines = format!("{AIRLINES_HEADER}\nSAS\tSAS\tSK\tSAS\n");
        StandingData::parse(routes, &airports, &airlines).unwrap()
    }

    #[test]
    fn bundled_data_answers_without_the_network() {
        let client = Fixed::new(500, b"");
        let info = RouteLookup::new()
            .lookup("SK1225", Some(&data()), Some(&client), Instant::now())
            .unwrap();
        assert_eq!(info.callsign, "SAS1225");
        assert_eq!(info.flight_number, "SK 1225");
        assert_eq!(info.source, RouteSourceId::StandingData);
        assert_eq!(info.airports[1].place(), "Stockholm");
        assert_eq!(*client.calls.borrow(), 0);
    }

    #[test]
    fn falls_back_to_adsbdb_and_fills_in_time_zones() {
        let client = Fixed::new(200, ADSBDB_SAS945);
        let info = RouteLookup::new()
            .lookup("SAS945", Some(&data()), Some(&client), Instant::now())
            .unwrap();
        assert_eq!(info.source, RouteSourceId::AdsbDb);
        assert_eq!(info.flight_number, "SK 945");
        let destination = &info.airports[1];
        assert_eq!(destination.code, "KORD");
        assert_eq!(destination.place(), "Chicago");
        assert_eq!(destination.time_zone.as_deref(), Some("America/Chicago"));
    }

    #[test]
    fn online_answers_and_misses_are_cached() {
        let t0 = Instant::now();
        let mut lookup = RouteLookup::new();
        let hit = Fixed::new(200, ADSBDB_SAS945);
        lookup.lookup("SAS945", None, Some(&hit), t0).unwrap();
        lookup
            .lookup("SAS945", None, Some(&hit), t0 + Duration::from_secs(3600))
            .unwrap();
        assert_eq!(*hit.calls.borrow(), 1);

        let miss = Fixed::new(404, ADSBDB_UNKNOWN);
        assert_eq!(lookup.lookup("ZZZ9999", None, Some(&miss), t0), None);
        assert_eq!(
            lookup.lookup("ZZZ9999", None, Some(&miss), t0 + Duration::from_secs(60)),
            None
        );
        assert_eq!(*miss.calls.borrow(), 1);
        // Misses expire after six hours.
        lookup.lookup(
            "ZZZ9999",
            None,
            Some(&miss),
            t0 + Duration::from_secs(6 * 3600),
        );
        assert_eq!(*miss.calls.borrow(), 2);
    }

    #[test]
    fn online_lookups_are_limited_per_minute_and_paused_after_errors() {
        let t0 = Instant::now();
        let mut lookup = RouteLookup::new();
        let miss = Fixed::new(404, ADSBDB_UNKNOWN);
        for n in 0..15 {
            lookup.lookup(&format!("ZZZ{n}"), None, Some(&miss), t0);
        }
        assert_eq!(*miss.calls.borrow(), ROUTE_LOOKUPS_PER_MINUTE);
        lookup.lookup("ZZZ99", None, Some(&miss), t0 + Duration::from_secs(61));
        assert_eq!(*miss.calls.borrow(), ROUTE_LOOKUPS_PER_MINUTE + 1);

        let mut lookup = RouteLookup::new();
        let down = Fixed::new(503, b"");
        lookup.lookup("ZZZ1", None, Some(&down), t0);
        lookup.lookup("ZZZ2", None, Some(&down), t0 + Duration::from_secs(60));
        assert_eq!(*down.calls.borrow(), 1);
        lookup.lookup("ZZZ2", None, Some(&down), t0 + ROUTE_SOURCE_PAUSE);
        assert_eq!(*down.calls.borrow(), 2);
    }

    #[test]
    fn non_flight_callsigns_are_never_looked_up() {
        let client = Fixed::new(200, ADSBDB_SAS945);
        assert_eq!(
            RouteLookup::new().lookup("SEROB", None, Some(&client), Instant::now()),
            None
        );
        assert_eq!(*client.calls.borrow(), 0);
    }
}
