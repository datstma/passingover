//! adsb.fi open data: <https://github.com/adsbfi/opendata>. Personal,
//! non-commercial use only; attribution with a link is required.

use super::{Area, ParseError, PositionSource, Snapshot, SourceId, readsb};
use crate::http::Request;

/// The adsb.fi v3 `lat/lon/dist` API.
#[derive(Debug, Clone, Copy, Default)]
pub struct AdsbFi;

impl AdsbFi {
    /// Largest radius the API accepts, in nautical miles.
    pub const MAX_RADIUS_NM: u32 = 250;
}

impl PositionSource for AdsbFi {
    fn id(&self) -> SourceId {
        SourceId::AdsbFi
    }

    fn request(&self, area: &Area) -> Request {
        Request {
            url: format!(
                "https://opendata.adsb.fi/api/v3/lat/{}/lon/{}/dist/{}",
                area.center.lat_param(),
                area.center.lon_param(),
                area.radius.whole_nautical_miles(Self::MAX_RADIUS_NM),
            ),
        }
    }

    fn parse(&self, body: &[u8]) -> Result<Snapshot, ParseError> {
        readsb::parse(self.id(), body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::{Coordinate, QueryPoint, Radius};

    #[test]
    fn request_uses_rounded_point_and_clamped_radius() {
        let area = Area {
            center: QueryPoint::from(Coordinate::new(-33.8568, 151.2153).unwrap()),
            radius: Radius::from_nautical_miles(400.0).unwrap(),
        };
        assert_eq!(
            AdsbFi.request(&area).url,
            "https://opendata.adsb.fi/api/v3/lat/-33.86/lon/151.22/dist/250"
        );
    }
}
