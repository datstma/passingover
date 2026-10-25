//! One aircraft as reported by a position source.

use std::time::SystemTime;

use crate::geo::Coordinate;

/// An aircraft (or other ADS-B emitter) at a moment in time.
///
/// Units are SI (D22): metres, metres per second, degrees.
#[derive(Debug, Clone, PartialEq)]
pub struct Aircraft {
    /// 24-bit ICAO address as lowercase hex. Non-ICAO addresses (e.g. TIS-B)
    /// start with `~`.
    pub icao24: String,
    /// Flight callsign such as `SAS945`, trimmed; `None` if not broadcast.
    pub callsign: Option<String>,
    /// Registration such as `SE-ROB`, from the source's aircraft database.
    pub registration: Option<String>,
    /// ICAO aircraft type designator such as `A20N`.
    pub type_designator: Option<String>,
    /// ADS-B emitter category such as `A3` (`A*` aircraft, `B*` gliders,
    /// balloons and similar, `C*` surface vehicles and obstacles).
    pub category: Option<String>,
    pub position: Coordinate,
    /// When `position` was last updated.
    pub position_time: SystemTime,
    /// The source reports the aircraft on the ground.
    pub on_ground: bool,
    /// Barometric (pressure) altitude in metres.
    pub baro_altitude_m: Option<f64>,
    /// Geometric (GNSS) altitude in metres.
    pub geo_altitude_m: Option<f64>,
    pub ground_speed_mps: Option<f64>,
    /// Track over ground, degrees clockwise from true north.
    pub track_deg: Option<f64>,
    /// Climb (positive) or descent (negative) rate in metres per second.
    pub vertical_rate_mps: Option<f64>,
}

impl Aircraft {
    /// Best available altitude in metres: barometric, else geometric.
    pub fn altitude_m(&self) -> Option<f64> {
        self.baro_altitude_m.or(self.geo_altitude_m)
    }

    /// Whether this is an aircraft in the air, as opposed to one on the
    /// ground, a ground vehicle, or a fixed beacon such as a tower.
    pub fn is_flying(&self) -> bool {
        if self.on_ground {
            return false;
        }
        let surface_category = self.category.as_deref().is_some_and(|c| c.starts_with('C'));
        let tower = self.type_designator.as_deref() == Some("TWR");
        !surface_category && !tower
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flying() -> Aircraft {
        Aircraft {
            icao24: "4ac9e2".into(),
            callsign: Some("SAS2802".into()),
            registration: Some("SE-ROB".into()),
            type_designator: Some("A20N".into()),
            category: Some("A3".into()),
            position: Coordinate::new(55.6, 12.6).unwrap(),
            position_time: SystemTime::UNIX_EPOCH,
            on_ground: false,
            baro_altitude_m: Some(2103.12),
            geo_altitude_m: Some(2209.8),
            ground_speed_mps: Some(129.3),
            track_deg: Some(64.6),
            vertical_rate_mps: Some(-3.9),
        }
    }

    #[test]
    fn altitude_prefers_barometric() {
        let mut a = flying();
        assert_eq!(a.altitude_m(), Some(2103.12));
        a.baro_altitude_m = None;
        assert_eq!(a.altitude_m(), Some(2209.8));
    }

    #[test]
    fn is_flying_excludes_ground_traffic_vehicles_and_towers() {
        assert!(flying().is_flying());

        let on_ground = Aircraft {
            on_ground: true,
            ..flying()
        };
        assert!(!on_ground.is_flying());

        let vehicle = Aircraft {
            category: Some("C0".into()),
            ..flying()
        };
        assert!(!vehicle.is_flying());

        let tower = Aircraft {
            category: None,
            type_designator: Some("TWR".into()),
            ..flying()
        };
        assert!(!tower.is_flying());

        let glider = Aircraft {
            category: Some("B1".into()),
            ..flying()
        };
        assert!(glider.is_flying());
    }
}
