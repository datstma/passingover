//! Opt-in tests against the real services. They make a handful of requests,
//! so they are `#[ignore]`d: run them with `cargo test -- --ignored`.

#![cfg(feature = "http")]

use std::time::Instant;

use passingover_core::geo::{Coordinate, QueryPoint, Radius};
use passingover_core::http::{UreqClient, user_agent};
use passingover_core::position::{Area, PositionFeed, SourceId};

fn client() -> UreqClient {
    UreqClient::new(&user_agent("PassingOver-tests"))
}

/// A small circle around Copenhagen Airport, a public location.
fn area() -> Area {
    Area {
        center: QueryPoint::from(Coordinate::new(55.618, 12.656).unwrap()),
        radius: Radius::from_nautical_miles(10.0).unwrap(),
    }
}

#[test]
#[ignore = "live network"]
fn both_position_sources_answer() {
    for primary in [SourceId::AdsbLol, SourceId::AdsbFi] {
        let mut feed = PositionFeed::new(primary, 1);
        let snapshot = feed.fetch(&client(), &area(), Instant::now()).unwrap();
        assert_eq!(snapshot.source, primary, "{primary:?} did not answer");
    }
}

#[test]
#[ignore = "live network"]
fn adsbdb_knows_a_long_standing_route() {
    use passingover_core::route::{RouteLookup, RouteSourceId};

    let info = RouteLookup::new()
        .lookup("SAS945", None, Some(&client()), Instant::now())
        .expect("adsbdb answered");
    assert_eq!(info.source, RouteSourceId::AdsbDb);
    assert_eq!(info.airports.len(), 2);
}
