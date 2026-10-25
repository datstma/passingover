//! Airports and airlines from the bundled standing data (D11, D14, D24).

use crate::geo::Coordinate;

#[derive(Debug, Clone, PartialEq)]
pub struct Airport {
    /// The code routes use: ICAO, or IATA when there is no ICAO code.
    pub code: String,
    pub icao: Option<String>,
    pub iata: Option<String>,
    pub name: String,
    /// Nearest city or town, e.g. `Chicago`.
    pub city: Option<String>,
    /// ISO 3166-1 alpha-2 country code.
    pub country: String,
    pub position: Coordinate,
    /// IANA time zone, e.g. `America/Chicago`.
    pub time_zone: Option<String>,
}

impl Airport {
    /// The name to show for a destination: the city, else the airport name.
    pub fn place(&self) -> &str {
        self.city.as_deref().unwrap_or(&self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Airline {
    /// The code routes and callsigns use: ICAO, or IATA when there is none.
    pub code: String,
    pub icao: Option<String>,
    pub iata: Option<String>,
    pub name: String,
}
