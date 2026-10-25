//! Live aircraft positions from community ADS-B feeds.
//!
//! Sources are sans-IO (D21): a [`PositionSource`] turns an [`Area`] into a
//! [`Request`] and parses a response body into a [`Snapshot`]. Sending the
//! request, with the User-Agent, timeouts, backoff and failover from D15,
//! is the job of a separate transport layer.

mod adsb_fi;
mod adsb_lol;
mod readsb;

pub use adsb_fi::AdsbFi;
pub use adsb_lol::AdsbLol;

use std::time::SystemTime;

use thiserror::Error;

use crate::aircraft::Aircraft;
use crate::geo::{QueryPoint, Radius};

/// The area to search: a privacy-rounded point and a radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Area {
    pub center: QueryPoint,
    pub radius: Radius,
}

/// An HTTP GET request for the transport layer to send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub url: String,
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

/// A response body that could not be turned into a [`Snapshot`].
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("unexpected response format: {0}")]
    Json(#[from] serde_json::Error),
    #[error("the source reported an error: {0}")]
    Source(String),
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
