//! Parse recorded adsb.lol and adsb.fi responses (tests/fixtures/README.md).

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use passingover_core::position::{AdsbFi, AdsbLol, PositionSource, Snapshot, SourceId};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/positions")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn flying_callsigns(snapshot: &Snapshot) -> Vec<&str> {
    let mut callsigns: Vec<&str> = snapshot
        .aircraft
        .iter()
        .filter(|a| a.is_flying())
        .filter_map(|a| a.callsign.as_deref())
        .collect();
    callsigns.sort_unstable();
    callsigns
}

#[test]
fn adsb_lol_point_response() {
    let snap = AdsbLol
        .parse(&fixture("adsb_lol_point_ekch_10nm.json"))
        .unwrap();
    assert_eq!(snap.source, SourceId::AdsbLol);
    assert_eq!(
        snap.time,
        SystemTime::UNIX_EPOCH + Duration::from_millis(1_792_966_614_001)
    );
    assert_eq!(snap.aircraft.len(), 10);
    assert_eq!(flying_callsigns(&snap), ["SAS2802", "SAS7724"]);

    let sas = snap.aircraft.iter().find(|a| a.icao24 == "4ac9e2").unwrap();
    assert_eq!(sas.callsign.as_deref(), Some("SAS2802"));
    assert_eq!(sas.registration.as_deref(), Some("SE-ROB"));
    assert_eq!(sas.type_designator.as_deref(), Some("A20N"));
    assert_eq!(sas.category.as_deref(), Some("A3"));
    // 6900 ft and 251.3 kt in the response.
    assert!((sas.baro_altitude_m.unwrap() - 2103.12).abs() < 0.01);
    assert!((sas.ground_speed_mps.unwrap() - 129.28).abs() < 0.01);
    assert!(sas.position_time < snap.time);
}

#[test]
fn adsb_fi_v3_response() {
    let snap = AdsbFi.parse(&fixture("adsb_fi_v3_ekch_10nm.json")).unwrap();
    assert_eq!(snap.source, SourceId::AdsbFi);
    assert_eq!(snap.aircraft.len(), 13);
    assert_eq!(flying_callsigns(&snap), ["SAS2802", "SAS7724"]);
}

#[test]
fn ground_traffic_vehicles_and_towers_are_not_flying() {
    let snap = AdsbFi.parse(&fixture("adsb_fi_v3_ekch_10nm.json")).unwrap();
    let not_flying: Vec<_> = snap.aircraft.iter().filter(|a| !a.is_flying()).collect();
    assert_eq!(not_flying.len(), 11);
    assert!(
        not_flying
            .iter()
            .any(|a| a.on_ground && a.category.as_deref() == Some("A3"))
    );
    assert!(
        not_flying
            .iter()
            .any(|a| a.type_designator.as_deref() == Some("TWR"))
    );
}
