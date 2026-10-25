//! adsbdb: <https://www.adsbdb.com>. Online route lookups only: its route
//! data may not be copied into other databases, so results are cached in
//! memory and never stored (D11).

use serde::Deserialize;

use super::{RouteInfo, RouteSource, RouteSourceId};
use crate::airport::Airport;
use crate::geo::Coordinate;
use crate::http::{ParseError, Request};

/// The adsbdb `/v0/callsign` API.
#[derive(Debug, Clone, Copy, Default)]
pub struct AdsbDb;

#[derive(Debug, Deserialize)]
struct Envelope {
    response: Body,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Body {
    Route { flightroute: Box<FlightRoute> },
    Message(String),
}

#[derive(Debug, Deserialize)]
struct FlightRoute {
    callsign: String,
    callsign_iata: Option<String>,
    origin: RawAirport,
    destination: RawAirport,
}

#[derive(Debug, Deserialize)]
struct RawAirport {
    country_iso_name: String,
    iata_code: Option<String>,
    icao_code: Option<String>,
    latitude: f64,
    longitude: f64,
    municipality: Option<String>,
    name: String,
}

impl RawAirport {
    fn into_airport(self) -> Option<Airport> {
        let code = self.icao_code.clone().or_else(|| self.iata_code.clone())?;
        Some(Airport {
            code,
            icao: self.icao_code,
            iata: self.iata_code,
            name: self.name,
            city: self.municipality.filter(|m| !m.is_empty()),
            country: self.country_iso_name,
            position: Coordinate::new(self.latitude, self.longitude).ok()?,
            time_zone: None,
        })
    }
}

impl RouteSource for AdsbDb {
    fn id(&self) -> RouteSourceId {
        RouteSourceId::AdsbDb
    }

    fn request(&self, callsign: &str) -> Request {
        Request {
            url: format!("https://api.adsbdb.com/v0/callsign/{callsign}"),
        }
    }

    fn parse(&self, body: &[u8]) -> Result<Option<RouteInfo>, ParseError> {
        let route = match serde_json::from_slice::<Envelope>(body)?.response {
            Body::Route { flightroute } => *flightroute,
            Body::Message(m) if m == "unknown callsign" => return Ok(None),
            Body::Message(m) => return Err(ParseError::Source(m)),
        };
        let (Some(origin), Some(destination)) = (
            route.origin.into_airport(),
            route.destination.into_airport(),
        ) else {
            return Ok(None);
        };
        // `SK945` → `SK 945`; IATA airline codes are two characters.
        let flight_number = match route.callsign_iata {
            Some(iata) if iata.len() > 2 && iata.is_char_boundary(2) => {
                format!("{} {}", &iata[..2], &iata[2..])
            }
            _ => route.callsign.clone(),
        };
        Ok(Some(RouteInfo {
            callsign: route.callsign,
            flight_number,
            airports: vec![origin, destination],
            source: RouteSourceId::AdsbDb,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_url() {
        assert_eq!(
            AdsbDb.request("SAS945").url,
            "https://api.adsbdb.com/v0/callsign/SAS945"
        );
    }

    #[test]
    fn other_messages_are_errors() {
        let err = AdsbDb
            .parse(br#"{"response":"invalid callsign"}"#)
            .unwrap_err();
        assert!(matches!(err, ParseError::Source(m) if m == "invalid callsign"));
    }
}
