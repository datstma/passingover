//! adsb.lol: <https://api.adsb.lol/docs>. Data under the ODbL 1.0.

use super::{Area, ParseError, PositionSource, Request, Snapshot, SourceId, readsb};

/// The adsb.lol `/v2/point` API.
#[derive(Debug, Clone, Copy, Default)]
pub struct AdsbLol;

impl AdsbLol {
    /// Largest radius `/v2/point` accepts, in nautical miles.
    pub const MAX_RADIUS_NM: u32 = 250;
}

impl PositionSource for AdsbLol {
    fn id(&self) -> SourceId {
        SourceId::AdsbLol
    }

    fn request(&self, area: &Area) -> Request {
        Request {
            url: format!(
                "https://api.adsb.lol/v2/point/{}/{}/{}",
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
    fn request_uses_rounded_point_and_whole_nautical_miles() {
        let area = Area {
            center: QueryPoint::from(Coordinate::new(55.6181, 12.6561).unwrap()),
            radius: Radius::from_nautical_miles(19.2).unwrap(),
        };
        assert_eq!(
            AdsbLol.request(&area).url,
            "https://api.adsb.lol/v2/point/55.62/12.66/20"
        );
    }
}
