//! Who to thank: every data source Passing Over uses, with the attribution
//! its terms ask for (docs/data-sources.md). Shown on "About the data".

use crate::position::SourceId;
use crate::route::RouteSourceId;
use crate::weather::WeatherSourceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credit {
    pub text: String,
    pub url: String,
}

impl Credit {
    fn new(text: &str, url: &str) -> Self {
        Self {
            text: text.to_owned(),
            url: url.to_owned(),
        }
    }
}

/// The credit for an aircraft position source.
pub fn for_source(id: SourceId) -> Credit {
    let a = id.attribution();
    Credit::new(a.text, a.url)
}

/// The credit for a route source.
pub fn for_route_source(id: RouteSourceId) -> Credit {
    Credit::new(&format!("Routes: {}", id.name()), id.url())
}

/// The credit for a weather source, with the wording its licence asks for.
pub fn for_weather_source(id: WeatherSourceId) -> Credit {
    match id {
        WeatherSourceId::OpenMeteo => {
            Credit::new("Weather data by Open-Meteo.com", "https://open-meteo.com/")
        }
        WeatherSourceId::MetNorway => {
            Credit::new("Weather: data from MET Norway", "https://api.met.no/")
        }
    }
}

/// Every source, in the order "About the data" lists them.
pub fn all() -> Vec<Credit> {
    vec![
        for_source(SourceId::AdsbLol),
        Credit::new(
            "adsb.lol data is under the Open Database License 1.0",
            "https://opendatacommons.org/licenses/odbl/1-0/",
        ),
        for_source(SourceId::AdsbFi),
        Credit::new(
            "Routes, airports and airlines: VRS standing data (CC0)",
            RouteSourceId::StandingData.url(),
        ),
        Credit::new(
            "Routes not in the bundled data: adsbdb",
            RouteSourceId::AdsbDb.url(),
        ),
        Credit::new(
            "Airport time zones: timezone-boundary-builder, © OpenStreetMap contributors (ODbL)",
            "https://github.com/evansiroky/timezone-boundary-builder",
        ),
        for_weather_source(WeatherSourceId::OpenMeteo),
        for_weather_source(WeatherSourceId::MetNorway),
        Credit::new(
            "Countries and seas: made with Natural Earth",
            "https://www.naturalearthdata.com/",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_credit_links_somewhere() {
        let credits = all();
        assert!(credits.len() >= 5);
        assert!(credits.iter().all(|c| c.url.starts_with("https://")));
        assert_eq!(for_source(SourceId::AdsbFi).url, "https://adsb.fi");
    }
}
