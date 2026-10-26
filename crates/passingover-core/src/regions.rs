//! The countries and seas a flight passes over (M3, D33): point-in-polygon on
//! simplified Natural Earth shapes (public domain), built by `cargo xtask data`.
//!
//! `regions.tsv` starts with the header `#passingover-regions\t1`, then one
//! polygon per line: `kind\tname\trings`. `kind` is `c` (country) or `w`
//! (water); `name` is ready for a sentence ("Norway", "the North Sea");
//! `rings` are separated by `;`, the first is the outline and any others are
//! holes, each `lon,lat lon,lat …` in degrees.

use std::io::Read;
use std::path::Path;

use crate::data::DataError;
use crate::geo::Coordinate;

pub const REGIONS_FILE: &str = "regions.tsv";
pub const REGIONS_HEADER: &str = "#passingover-regions\t1";

/// Distance between samples when walking a route.
const STEP_M: f64 = 25_000.0;
/// At most this many samples per route (20,000 km at 25 km).
const MAX_SAMPLES: usize = 800;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionKind {
    Country,
    Water,
}

/// One stretch of a route over a single country or sea.
#[derive(Debug, Clone, PartialEq)]
pub struct Stretch {
    /// Ready for a sentence: "Norway", "the North Sea".
    pub name: String,
    pub kind: RegionKind,
    pub length_m: f64,
}

#[derive(Debug)]
struct Polygon {
    region: usize,
    /// min lon, min lat, max lon, max lat.
    bbox: [f64; 4],
    rings: Vec<Vec<(f64, f64)>>,
}

impl Polygon {
    fn contains(&self, lon: f64, lat: f64) -> bool {
        let [x0, y0, x1, y1] = self.bbox;
        if lon < x0 || lon > x1 || lat < y0 || lat > y1 {
            return false;
        }
        // Even-odd rule over all rings, so holes are excluded.
        let mut inside = false;
        for ring in &self.rings {
            let mut j = ring.len() - 1;
            for i in 0..ring.len() {
                let ((xi, yi), (xj, yj)) = (ring[i], ring[j]);
                if (yi > lat) != (yj > lat) && lon < (xj - xi) * (lat - yi) / (yj - yi) + xi {
                    inside = !inside;
                }
                j = i;
            }
        }
        inside
    }
}

/// Countries and seas, for "what will it cross".
#[derive(Debug)]
pub struct Regions {
    regions: Vec<(String, RegionKind)>,
    /// Countries first, so land wins where outlines overlap the sea.
    polygons: Vec<Polygon>,
}

impl Regions {
    pub fn load(dir: &Path) -> Result<Self, DataError> {
        let path = dir.join(REGIONS_FILE);
        let text =
            std::fs::read_to_string(&path).map_err(|source| DataError::Io { path, source })?;
        Self::parse(&text)
    }

    /// Load from gzipped bytes, as the app bundles them.
    pub fn from_gzip(bytes: &[u8]) -> Result<Self, DataError> {
        let mut text = String::new();
        flate2::read::GzDecoder::new(bytes)
            .read_to_string(&mut text)
            .map_err(|source| DataError::Gzip {
                file: REGIONS_FILE,
                source,
            })?;
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> Result<Self, DataError> {
        let error = |line: usize, message: &str| DataError::Format {
            file: REGIONS_FILE,
            line,
            message: message.to_owned(),
        };
        let mut lines = text.lines();
        if lines.next() != Some(REGIONS_HEADER) {
            return Err(error(
                1,
                "unexpected header; rebuild with `cargo xtask data`",
            ));
        }
        let mut regions: Vec<(String, RegionKind)> = Vec::new();
        let mut polygons = Vec::new();
        for (n, line) in lines.enumerate().filter(|(_, l)| !l.is_empty()) {
            let n = n + 2;
            let mut fields = line.split('\t');
            let (Some(kind), Some(name), Some(rings), None) =
                (fields.next(), fields.next(), fields.next(), fields.next())
            else {
                return Err(error(n, "expected 3 columns"));
            };
            let kind = match kind {
                "c" => RegionKind::Country,
                "w" => RegionKind::Water,
                _ => return Err(error(n, "kind must be c or w")),
            };
            let rings = rings
                .split(';')
                .map(|ring| {
                    ring.split(' ')
                        .map(|pair| {
                            let (lon, lat) = pair.split_once(',')?;
                            Some((lon.parse().ok()?, lat.parse().ok()?))
                        })
                        .collect::<Option<Vec<(f64, f64)>>>()
                        .filter(|r| r.len() >= 3)
                })
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| error(n, "invalid ring"))?;
            let region = match regions.iter().position(|(r, k)| r == name && *k == kind) {
                Some(i) => i,
                None => {
                    regions.push((name.to_owned(), kind));
                    regions.len() - 1
                }
            };
            let mut bbox = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
            for &(lon, lat) in &rings[0] {
                bbox = [
                    bbox[0].min(lon),
                    bbox[1].min(lat),
                    bbox[2].max(lon),
                    bbox[3].max(lat),
                ];
            }
            polygons.push(Polygon {
                region,
                bbox,
                rings,
            });
        }
        polygons.sort_by_key(|p| regions[p.region].1 == RegionKind::Water);
        Ok(Self { regions, polygons })
    }

    /// The country under `point`, else the named sea, else `None`.
    pub fn at(&self, point: Coordinate) -> Option<(&str, RegionKind)> {
        self.polygons
            .iter()
            .find(|p| p.contains(point.lon(), point.lat()))
            .map(|p| {
                let (name, kind) = &self.regions[p.region];
                (name.as_str(), *kind)
            })
    }

    /// The stretches of the great circle from `from` to `to`, in order. Parts
    /// over unnamed water are left out.
    pub fn along(&self, from: Coordinate, to: Coordinate) -> Vec<Stretch> {
        let total = from.distance_to(&to);
        let samples = ((total / STEP_M).ceil() as usize).clamp(1, MAX_SAMPLES);
        let step = total / samples as f64;
        let mut stretches: Vec<Stretch> = Vec::new();
        for i in 0..=samples {
            let point = from.toward(&to, i as f64 / samples as f64);
            let Some((name, kind)) = self.at(point) else {
                continue;
            };
            match stretches.last_mut() {
                Some(last) if last.name == name => last.length_m += step,
                _ => stretches.push(Stretch {
                    name: name.to_owned(),
                    kind,
                    length_m: step,
                }),
            }
        }
        stretches
    }
}

/// Stretches shorter than this aren't worth mentioning.
const MIN_STRETCH_M: f64 = 50_000.0;
/// At most this many places in "will cross …".
const MAX_AHEAD: usize = 3;

impl Regions {
    /// What a flight at `here` will cross on the way to `destination`, for
    /// "will cross the Norwegian Sea, Greenland and Canada": leaves out the
    /// region it is over now and the destination's region (wherever they
    /// appear on the way), merges repeats, drops short stretches and keeps
    /// the longest few, in route order.
    pub fn ahead(&self, here: Coordinate, destination: Coordinate) -> Vec<String> {
        let skip: Vec<&str> = [self.at(here), self.at(destination)]
            .into_iter()
            .flatten()
            .map(|(name, _)| name)
            .collect();
        let stretches: Vec<Stretch> = self
            .along(here, destination)
            .into_iter()
            .filter(|s| !skip.contains(&s.name.as_str()))
            .collect();
        // Merge repeats, keeping the first position.
        let mut merged: Vec<(usize, String, f64)> = Vec::new();
        for (i, s) in stretches.into_iter().enumerate() {
            match merged.iter_mut().find(|m| m.1 == s.name) {
                Some(m) => m.2 += s.length_m,
                None => merged.push((i, s.name, s.length_m)),
            }
        }
        merged.retain(|m| m.2 >= MIN_STRETCH_M);
        merged.sort_by(|a, b| b.2.total_cmp(&a.2));
        merged.truncate(MAX_AHEAD);
        merged.sort_by_key(|m| m.0);
        merged.into_iter().map(|m| m.1).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two square "countries" side by side, one with a lake (a hole), and a
    /// sea around them.
    fn sample() -> Regions {
        Regions::parse(&format!(
            "{REGIONS_HEADER}\n\
             c\tWestland\t0,0 10,0 10,10 0,10;4,4 6,4 6,6 4,6\n\
             c\tEastland\t10,0 20,0 20,10 10,10\n\
             w\tthe Middle Sea\t-10,-10 30,-10 30,20 -10,20\n"
        ))
        .unwrap()
    }

    fn at(lat: f64, lon: f64) -> Coordinate {
        Coordinate::new(lat, lon).unwrap()
    }

    #[test]
    fn countries_win_over_the_sea_and_holes_are_excluded() {
        let r = sample();
        assert_eq!(r.at(at(2.0, 2.0)), Some(("Westland", RegionKind::Country)));
        assert_eq!(r.at(at(5.0, 15.0)), Some(("Eastland", RegionKind::Country)));
        assert_eq!(
            r.at(at(5.0, 5.0)),
            Some(("the Middle Sea", RegionKind::Water))
        );
        assert_eq!(
            r.at(at(15.0, 25.0)),
            Some(("the Middle Sea", RegionKind::Water))
        );
        assert_eq!(r.at(at(50.0, 50.0)), None);
    }

    #[test]
    fn walking_a_route_lists_stretches_in_order() {
        let stretches = sample().along(at(2.0, -5.0), at(2.0, 25.0));
        let names: Vec<&str> = stretches.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            ["the Middle Sea", "Westland", "Eastland", "the Middle Sea"]
        );
        // Each country is about 10° of longitude wide: ~1,110 km.
        assert!((stretches[1].length_m - 1_110_000.0).abs() < 60_000.0);
    }

    #[test]
    fn ahead_skips_here_and_the_destination_country() {
        // From the sea west of Westland to the middle of Eastland.
        let r = sample();
        assert_eq!(r.ahead(at(2.0, -5.0), at(2.0, 15.0)), ["Westland"]);
        // Within one country, nothing is worth naming.
        assert!(r.ahead(at(1.0, 1.0), at(2.0, 9.0)).is_empty());
        // From inside Westland to the sea beyond Eastland: whatever the
        // destination lies in is left out, here the sea.
        assert_eq!(r.ahead(at(2.0, 2.0), at(2.0, 25.0)), ["Eastland"]);
    }

    #[test]
    fn rejects_bad_files() {
        assert!(Regions::parse("#passingover-regions\t0\n").is_err());
        let bad = format!("{REGIONS_HEADER}\nx\tX\t0,0 1,0 1,1\n");
        assert!(Regions::parse(&bad).is_err());
        let short_ring = format!("{REGIONS_HEADER}\nc\tX\t0,0 1,0\n");
        assert!(Regions::parse(&short_ring).is_err());
    }
}
