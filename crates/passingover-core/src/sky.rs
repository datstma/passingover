//! The sky above one place: one poll fetches positions, keeps the flying
//! aircraft nearby and resolves where they're going. Shared by the CLI and
//! the app, so front ends only lay things out.

use std::time::{Instant, SystemTime};

use crate::data::StandingData;
use crate::flight::{Flight, flights_overhead};
use crate::geo::{Coordinate, QueryPoint, Radius};
use crate::http::HttpClient;
use crate::position::{Area, DailyUsage, FetchError, PositionFeed, Snapshot, SourceId};
use crate::route::{RouteLookup, RouteSourceId};

/// What one poll saw.
#[derive(Debug, Clone, PartialEq)]
pub struct Overhead {
    pub source: SourceId,
    /// The source's timestamp for the positions.
    pub time: SystemTime,
    /// Nearest first.
    pub flights: Vec<Flight>,
}

impl Overhead {
    /// Route sources used, in order of first use, for attribution.
    pub fn route_sources(&self) -> Vec<RouteSourceId> {
        let mut sources = Vec::new();
        for s in self.flights.iter().filter_map(|f| f.route_source) {
            if !sources.contains(&s) {
                sources.push(s);
            }
        }
        sources
    }
}

/// Position feed, route lookup and standing data for one front end.
#[derive(Debug)]
pub struct Sky {
    feed: PositionFeed,
    routes: RouteLookup,
    data: Option<StandingData>,
}

impl Sky {
    /// `seed` drives backoff jitter; use a random one outside tests.
    pub fn new(primary: SourceId, seed: u64) -> Self {
        Self {
            feed: PositionFeed::new(primary, seed),
            routes: RouteLookup::new(),
            data: None,
        }
    }

    pub fn primary(&self) -> SourceId {
        self.feed.primary()
    }

    /// Use the bundled standing data for routes and airports. Without it,
    /// routes are looked up online only.
    pub fn set_data(&mut self, data: StandingData) {
        self.data = Some(data);
    }

    pub fn data(&self) -> Option<&StandingData> {
        self.data.as_ref()
    }

    pub fn usage(&self) -> DailyUsage {
        self.feed.usage()
    }

    pub fn set_usage(&mut self, usage: DailyUsage) {
        self.feed.set_usage(usage);
    }

    /// Fetch what is overhead. `here` stays on the device; only its rounded
    /// [`QueryPoint`] is sent.
    pub fn poll(
        &mut self,
        client: &dyn HttpClient,
        here: Coordinate,
        radius: Radius,
        wall: SystemTime,
        now: Instant,
    ) -> Result<Overhead, FetchError> {
        let area = Area {
            center: QueryPoint::from(here),
            radius,
        };
        let snapshot = self.feed.fetch(client, &area, wall, now)?;
        Ok(self.describe(&snapshot, here, radius, Some(client), now))
    }

    /// Turn a snapshot into flights; `client` enables online route lookups.
    pub fn describe(
        &mut self,
        snapshot: &Snapshot,
        here: Coordinate,
        radius: Radius,
        client: Option<&dyn HttpClient>,
        now: Instant,
    ) -> Overhead {
        let (routes, data) = (&mut self.routes, self.data.as_ref());
        let flights = flights_overhead(snapshot, here, radius, |aircraft| {
            routes.lookup(aircraft.callsign.as_deref()?, data, client, now)
        });
        Overhead {
            source: snapshot.source,
            time: snapshot.time,
            flights,
        }
    }
}
