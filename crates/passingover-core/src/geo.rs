//! Coordinates, privacy-rounded query points, radii and distances.
//!
//! All values are SI: degrees for angles, metres for distances (D22).

use thiserror::Error;

/// Metres in one international nautical mile.
pub const METRES_PER_NAUTICAL_MILE: f64 = 1852.0;

/// Mean Earth radius in metres (IUGG), used for great-circle distances.
pub(crate) const EARTH_RADIUS_M: f64 = 6_371_008.8;

/// An invalid coordinate or radius.
#[derive(Debug, Clone, Copy, PartialEq, Error)]
pub enum GeoError {
    #[error("latitude {0} is not between -90 and 90")]
    InvalidLatitude(f64),
    #[error("longitude {0} is not between -180 and 180")]
    InvalidLongitude(f64),
    #[error("radius {0} m is not a positive, finite distance")]
    InvalidRadius(f64),
}

/// A WGS 84 position in decimal degrees.
///
/// This may be the user's precise location, so it must never be sent to a
/// third party directly; convert it to a [`QueryPoint`] first.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coordinate {
    lat: f64,
    lon: f64,
}

impl Coordinate {
    pub fn new(lat: f64, lon: f64) -> Result<Self, GeoError> {
        if !lat.is_finite() || !(-90.0..=90.0).contains(&lat) {
            return Err(GeoError::InvalidLatitude(lat));
        }
        if !lon.is_finite() || !(-180.0..=180.0).contains(&lon) {
            return Err(GeoError::InvalidLongitude(lon));
        }
        Ok(Self { lat, lon })
    }

    pub fn lat(&self) -> f64 {
        self.lat
    }

    pub fn lon(&self) -> f64 {
        self.lon
    }

    /// Great-circle (haversine) distance to `other`, in metres.
    pub fn distance_to(&self, other: &Coordinate) -> f64 {
        let (lat1, lat2) = (self.lat.to_radians(), other.lat.to_radians());
        let dlat = lat2 - lat1;
        let dlon = (other.lon - self.lon).to_radians();
        let a = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
        2.0 * EARTH_RADIUS_M * a.sqrt().min(1.0).asin()
    }

    /// Initial great-circle bearing to `other`, in degrees clockwise from
    /// true north (0–360).
    pub fn bearing_to(&self, other: &Coordinate) -> f64 {
        let (lat1, lat2) = (self.lat.to_radians(), other.lat.to_radians());
        let dlon = (other.lon - self.lon).to_radians();
        let y = dlon.sin() * lat2.cos();
        let x = lat1.cos() * lat2.sin() - lat1.sin() * lat2.cos() * dlon.cos();
        y.atan2(x).to_degrees().rem_euclid(360.0)
    }
}

impl Coordinate {
    /// The point `fraction` (0–1) of the way along the great circle to
    /// `other`.
    pub fn toward(&self, other: &Coordinate, fraction: f64) -> Coordinate {
        let d = self.distance_to(other) / EARTH_RADIUS_M;
        if d < 1e-12 {
            return *self;
        }
        let (lat1, lon1) = (self.lat.to_radians(), self.lon.to_radians());
        let (lat2, lon2) = (other.lat.to_radians(), other.lon.to_radians());
        let a = ((1.0 - fraction) * d).sin() / d.sin();
        let b = (fraction * d).sin() / d.sin();
        let x = a * lat1.cos() * lon1.cos() + b * lat2.cos() * lon2.cos();
        let y = a * lat1.cos() * lon1.sin() + b * lat2.cos() * lon2.sin();
        let z = a * lat1.sin() + b * lat2.sin();
        Coordinate {
            lat: z.atan2(x.hypot(y)).to_degrees(),
            lon: y.atan2(x).to_degrees(),
        }
    }
}

/// Smallest difference between two bearings in degrees (0–180).
pub fn angle_between(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

/// A location that is safe to send to a third party.
///
/// It is the user's coordinate rounded to [`QueryPoint::DECIMALS`] decimal
/// places, about 1.1 km north–south and less east–west (D15). Sources only
/// accept a `QueryPoint`, so a precise location can't leave the device by
/// accident.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QueryPoint(Coordinate);

impl QueryPoint {
    /// Decimal places kept when rounding.
    pub const DECIMALS: usize = 2;

    pub fn lat(&self) -> f64 {
        self.0.lat
    }

    pub fn lon(&self) -> f64 {
        self.0.lon
    }

    /// The rounded point as a coordinate, e.g. for distance calculations.
    pub fn coordinate(&self) -> Coordinate {
        self.0
    }

    /// Latitude formatted for a URL, with exactly [`QueryPoint::DECIMALS`] decimals.
    pub fn lat_param(&self) -> String {
        format!("{:.*}", Self::DECIMALS, self.0.lat)
    }

    /// Longitude formatted for a URL, with exactly [`QueryPoint::DECIMALS`] decimals.
    pub fn lon_param(&self) -> String {
        format!("{:.*}", Self::DECIMALS, self.0.lon)
    }
}

impl From<Coordinate> for QueryPoint {
    fn from(c: Coordinate) -> Self {
        let scale = 10f64.powi(Self::DECIMALS as i32);
        // `+ 0.0` turns -0.0 into 0.0 so it never prints as "-0.00".
        let round = |v: f64| (v * scale).round() / scale + 0.0;
        // Rounding keeps values within range: ±90 and ±180 are already round.
        Self(Coordinate {
            lat: round(c.lat),
            lon: round(c.lon),
        })
    }
}

/// A search radius around a point.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Radius {
    metres: f64,
}

impl Radius {
    /// Default search radius: 20 nautical miles, about 37 km (D15).
    pub const DEFAULT: Radius = Radius {
        metres: 20.0 * METRES_PER_NAUTICAL_MILE,
    };

    pub fn from_metres(metres: f64) -> Result<Self, GeoError> {
        if metres.is_finite() && metres > 0.0 {
            Ok(Self { metres })
        } else {
            Err(GeoError::InvalidRadius(metres))
        }
    }

    pub fn from_kilometres(km: f64) -> Result<Self, GeoError> {
        Self::from_metres(km * 1000.0)
    }

    pub fn from_nautical_miles(nm: f64) -> Result<Self, GeoError> {
        Self::from_metres(nm * METRES_PER_NAUTICAL_MILE)
    }

    pub fn metres(&self) -> f64 {
        self.metres
    }

    pub fn nautical_miles(&self) -> f64 {
        self.metres / METRES_PER_NAUTICAL_MILE
    }

    /// Whole nautical miles for APIs that only take integers: rounded up so
    /// the area is never smaller than asked, and limited to `1..=max`.
    pub fn whole_nautical_miles(&self, max: u32) -> u32 {
        // The float-to-int `as` cast saturates; the clamp then bounds it.
        (self.nautical_miles().ceil() as u32).clamp(1, max.max(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coord(lat: f64, lon: f64) -> Coordinate {
        Coordinate::new(lat, lon).unwrap()
    }

    #[test]
    fn coordinate_rejects_out_of_range_and_non_finite_values() {
        assert_eq!(
            Coordinate::new(90.5, 0.0),
            Err(GeoError::InvalidLatitude(90.5))
        );
        assert_eq!(
            Coordinate::new(0.0, -180.1),
            Err(GeoError::InvalidLongitude(-180.1))
        );
        assert!(Coordinate::new(f64::NAN, 0.0).is_err());
        assert!(Coordinate::new(0.0, f64::INFINITY).is_err());
        assert!(Coordinate::new(-90.0, 180.0).is_ok());
    }

    #[test]
    fn query_point_rounds_to_two_decimals() {
        let p = QueryPoint::from(coord(59.651_94, 17.918_61));
        assert_eq!((p.lat(), p.lon()), (59.65, 17.92));
        assert_eq!(
            (p.lat_param(), p.lon_param()),
            ("59.65".into(), "17.92".into())
        );
    }

    #[test]
    fn query_point_never_prints_negative_zero() {
        let p = QueryPoint::from(coord(-0.001, -0.004));
        assert_eq!(
            (p.lat_param(), p.lon_param()),
            ("0.00".into(), "0.00".into())
        );
    }

    #[test]
    fn query_point_moves_the_location_by_at_most_about_a_kilometre() {
        // Worst case is half a step on both axes; at the equator that is
        // sqrt(2) * 0.005° ≈ 786 m.
        for &(lat, lon) in &[
            (0.004_99, 0.004_99),
            (59.654_9, 17.925_1),
            (-33.855, 151.215),
        ] {
            let c = coord(lat, lon);
            let moved = c.distance_to(&QueryPoint::from(c).coordinate());
            assert!(moved < 800.0, "{lat},{lon} moved {moved} m");
        }
    }

    #[test]
    fn haversine_distance_matches_known_values() {
        // One degree of latitude is about 111.2 km.
        let d = coord(0.0, 0.0).distance_to(&coord(1.0, 0.0));
        assert!((d - 111_195.0).abs() < 10.0, "{d}");
        // Copenhagen (EKCH) to Stockholm Arlanda (ESSA) is about 547 km.
        let d = coord(55.6181, 12.6561).distance_to(&coord(59.6519, 17.9186));
        assert!((d - 547_000.0).abs() < 3_000.0, "{d}");
        assert_eq!(coord(10.0, 20.0).distance_to(&coord(10.0, 20.0)), 0.0);
    }

    #[test]
    fn bearings_point_the_right_way() {
        let origin = coord(0.0, 0.0);
        assert!((origin.bearing_to(&coord(1.0, 0.0)) - 0.0).abs() < 1e-9);
        assert!((origin.bearing_to(&coord(0.0, 1.0)) - 90.0).abs() < 1e-9);
        assert!((origin.bearing_to(&coord(-1.0, 0.0)) - 180.0).abs() < 1e-9);
        assert!((origin.bearing_to(&coord(0.0, -1.0)) - 270.0).abs() < 1e-9);
        // Stockholm to Chicago starts out west-north-west (great circle).
        let b = coord(59.6519, 17.9186).bearing_to(&coord(41.9786, -87.9048));
        assert!((300.0..320.0).contains(&b), "{b}");
    }

    #[test]
    fn toward_follows_the_great_circle() {
        let (a, b) = (coord(0.0, 0.0), coord(0.0, 90.0));
        let mid = a.toward(&b, 0.5);
        assert!((mid.lat() - 0.0).abs() < 1e-9 && (mid.lon() - 45.0).abs() < 1e-9);
        assert_eq!(a.toward(&b, 0.0), a);
        // Stockholm to Chicago passes far north, over Greenland's latitude.
        let mid = coord(59.6519, 17.9186).toward(&coord(41.9786, -87.9048), 0.5);
        assert!(mid.lat() > 60.0, "{mid:?}");
    }

    #[test]
    fn angle_between_wraps_around_north() {
        assert_eq!(angle_between(350.0, 10.0), 20.0);
        assert_eq!(angle_between(10.0, 350.0), 20.0);
        assert_eq!(angle_between(90.0, 270.0), 180.0);
    }

    #[test]
    fn radius_converts_units_and_rejects_bad_values() {
        let r = Radius::from_nautical_miles(10.0).unwrap();
        assert_eq!(r.metres(), 18_520.0);
        assert_eq!(
            Radius::from_kilometres(37.04).unwrap().nautical_miles(),
            20.0
        );
        assert_eq!(Radius::DEFAULT.nautical_miles(), 20.0);
        assert!(Radius::from_metres(0.0).is_err());
        assert!(Radius::from_metres(-5.0).is_err());
        assert!(Radius::from_metres(f64::NAN).is_err());
    }

    #[test]
    fn whole_nautical_miles_rounds_up_and_clamps() {
        let nm = |v: f64| Radius::from_nautical_miles(v).unwrap();
        assert_eq!(nm(20.0).whole_nautical_miles(250), 20);
        assert_eq!(nm(20.1).whole_nautical_miles(250), 21);
        assert_eq!(nm(0.2).whole_nautical_miles(250), 1);
        assert_eq!(nm(400.0).whole_nautical_miles(250), 250);
    }
}
