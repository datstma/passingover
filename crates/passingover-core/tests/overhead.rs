//! From a recorded snapshot to dreamy lines, using a small subset of the
//! standing data (tests/fixtures/README.md). No network.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use passingover_core::data::StandingData;
use passingover_core::flight::flights_overhead;
use passingover_core::geo::{Coordinate, Radius};
use passingover_core::position::{AdsbLol, PositionSource};
use passingover_core::route::{RouteLookup, RouteSourceId};
use passingover_core::story::dreamy_line;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[test]
fn flights_over_copenhagen_airport() {
    let data = StandingData::load(&fixtures().join("data")).unwrap();
    let body = std::fs::read(fixtures().join("positions/adsb_lol_point_ekch_10nm.json")).unwrap();
    let snapshot = AdsbLol.parse(&body).unwrap();
    // The recorded query point: Copenhagen Airport, a public location.
    let here = Coordinate::new(55.62, 12.66).unwrap();
    let radius = Radius::from_nautical_miles(10.0).unwrap();

    let mut lookup = RouteLookup::new();
    let now = Instant::now();
    let flights = flights_overhead(&snapshot, here, radius, |aircraft| {
        lookup.lookup(aircraft.callsign.as_deref()?, Some(&data), None, now)
    });

    // Two aircraft are airborne; both are arriving at Copenhagen.
    let numbers: Vec<_> = flights.iter().map(|f| f.flight_number.as_deref()).collect();
    assert_eq!(numbers, [Some("SK 7724"), Some("SK 2802")]);
    assert!(
        flights
            .windows(2)
            .all(|w| w[0].distance_m <= w[1].distance_m)
    );
    for flight in &flights {
        let leg = flight.leg.as_ref().unwrap();
        assert_eq!(leg.destination.code, "EKCH");
        assert_eq!(flight.route_source, Some(RouteSourceId::StandingData));
        assert!(flight.time_to_landing.unwrap() <= Duration::from_secs(30 * 60));
    }
    let sk2802 = &flights[1];
    assert_eq!(sk2802.leg.as_ref().unwrap().origin.place(), "Lisbon");

    // 2026-10-25 22:16 UTC is 23:16 in Copenhagen (CET since that morning).
    let line = dreamy_line(sk2802, snapshot.time);
    assert!(
        line.starts_with("SK 2802 → Copenhagen. It's 23:16 there. Landing in about "),
        "{line}"
    );
}
