//! Live aircraft positions from community ADS-B feeds.
//!
//! Sources are sans-IO (D21): a [`PositionSource`] turns an [`Area`] into a
//! [`Request`] and parses a response body into a [`Snapshot`]. A
//! [`PositionFeed`] sends the requests through an
//! [`HttpClient`](crate::http::HttpClient) and applies the backoff, rate
//! limits and failover from D15 and D17.

mod adsb_fi;
mod adsb_lol;
mod feed;
mod readsb;

pub use adsb_fi::AdsbFi;
pub use adsb_lol::AdsbLol;
pub use feed::{FetchError, PositionFeed, choose_primary, source_for};

use std::time::SystemTime;

use crate::aircraft::Aircraft;
use crate::geo::{QueryPoint, Radius};
pub use crate::http::ParseError;
use crate::http::Request;

/// The area to search: a privacy-rounded point and a radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Area {
    pub center: QueryPoint,
    pub radius: Radius,
}

/// The aircraft a source reported at one moment.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub source: SourceId,
    /// The source's own timestamp for this snapshot.
    pub time: SystemTime,
    pub aircraft: Vec<Aircraft>,
}

/// The position sources Passing Over knows about (D17).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceId {
    AdsbLol,
    AdsbFi,
}

impl SourceId {
    /// Display name, e.g. for the "About data" screen.
    pub fn name(self) -> &'static str {
        match self {
            SourceId::AdsbLol => "adsb.lol",
            SourceId::AdsbFi => "adsb.fi",
        }
    }

    /// Attribution the source requires wherever its data is shown
    /// (docs/data-sources.md).
    pub fn attribution(self) -> Attribution {
        match self {
            SourceId::AdsbLol => Attribution {
                text: "Aircraft data: adsb.lol (ODbL)",
                url: "https://adsb.lol",
                license_url: Some("https://opendatacommons.org/licenses/odbl/1-0/"),
            },
            SourceId::AdsbFi => Attribution {
                text: "Aircraft data: adsb.fi",
                url: "https://adsb.fi",
                license_url: None,
            },
        }
    }
}

/// How to credit a data source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attribution {
    pub text: &'static str,
    pub url: &'static str,
    pub license_url: Option<&'static str>,
}

/// A source of live aircraft positions.
pub trait PositionSource {
    fn id(&self) -> SourceId;

    /// The request that fetches every aircraft within `area`. Sources with a
    /// maximum radius clamp to it.
    fn request(&self, area: &Area) -> Request;

    /// Parse the body of a successful (HTTP 200) response.
    fn parse(&self, body: &[u8]) -> Result<Snapshot, ParseError>;
}
