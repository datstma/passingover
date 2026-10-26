//! Developer tasks for Passing Over.
//!
//! ```text
//! cargo xtask data [--out DIR]   build the route/airport/airline tables
//! cargo xtask icons              render the app icons from design/icon/
//! ```
//!
//! `data` shallow-clones (or updates) the VRS standing data into
//! `target/xtask/`, adds an IANA time zone to every airport with tzf-rs, and
//! writes the tables described in `passingover_core::data` to `data/` (or
//! `--out`), with a manifest and a NOTICE (D24). It also writes gzipped
//! copies for the app to bundle to `app/assets/data/` (D29).
//!
//! `icons` renders the icon masters into each platform's icon files (D43).

use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};

mod icons;

const VRS_REPO: &str = "https://github.com/vradarserver/standing-data.git";
const VRS_BRANCH: &str = "main";

/// Natural Earth (public domain), pinned to a release (D33).
const NATURAL_EARTH: &str =
    "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/v5.1.2/geojson";
const NE_COUNTRIES: &str = "ne_110m_admin_0_countries.geojson";
const NE_SEAS: &str = "ne_50m_geography_marine_polys.geojson";
/// Sea outlines are simplified to this tolerance in degrees (about 2 km).
const SEA_TOLERANCE_DEG: f64 = 0.02;

// Must match passingover_core::data.
const ROUTES_HEADER: &str = "#passingover-routes\t1";
const AIRPORTS_HEADER: &str = "#passingover-airports\t1";
const AIRLINES_HEADER: &str = "#passingover-airlines\t1";
const REGIONS_HEADER: &str = "#passingover-regions\t2";

type Result<T> = std::result::Result<T, String>;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("data") => data(&args[1..]),
        Some("icons") => icons::icons(&args[1..]),
        _ => Err("usage: cargo xtask data [--out DIR] | cargo xtask icons".into()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives in the workspace")
        .to_path_buf()
}

fn data(args: &[String]) -> Result<()> {
    let root = workspace_root();
    let out = match args {
        [] => root.join("data"),
        [flag, dir] if flag == "--out" => PathBuf::from(dir),
        _ => return Err("usage: cargo xtask data [--out DIR]".into()),
    };
    let vrs = root.join("target/xtask/vrs-standing-data");
    update_checkout(&vrs)?;
    let commit = git(&vrs, &["rev-parse", "HEAD"])?;
    let commit_date = git(&vrs, &["log", "-1", "--format=%cs"])?;
    println!("VRS standing data at {commit} ({commit_date})");

    let airlines = read_airlines(&vrs)?;
    let airports = read_airports(&vrs)?;
    let codes: HashSet<&str> = airports.iter().map(|a| a[0].as_str()).collect();
    let (routes, skipped) = read_routes(&vrs, &codes)?;
    println!(
        "{} routes ({skipped} skipped: unknown airport), {} airports, {} airlines",
        routes.len(),
        airports.len(),
        airlines.len()
    );

    let ne = root.join("target/xtask/natural-earth-v5.1.2");
    let regions = read_regions(&download(&ne, NE_COUNTRIES)?, &download(&ne, NE_SEAS)?)?;
    println!("{} region polygons", regions.len());

    fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    write_table(&out.join("routes.tsv"), ROUTES_HEADER, &routes)?;
    write_table(&out.join("airports.tsv"), AIRPORTS_HEADER, &airports)?;
    write_table(&out.join("airlines.tsv"), AIRLINES_HEADER, &airlines)?;
    write_table(&out.join("regions.tsv"), REGIONS_HEADER, &regions)?;
    let manifest = manifest(&commit, &commit_date, &routes, &airports, &airlines);
    write(&out.join("manifest.json"), &manifest)?;
    write(&out.join("NOTICE.md"), NOTICE)?;
    println!("wrote {}", out.display());

    let assets = root.join("app/assets/data");
    fs::create_dir_all(&assets).map_err(|e| format!("{}: {e}", assets.display()))?;
    for name in ["routes.tsv", "airports.tsv", "airlines.tsv", "regions.tsv"] {
        let text = fs::read(out.join(name)).map_err(|e| format!("{name}: {e}"))?;
        write_gzip(&assets.join(format!("{name}.gz")), &text)?;
    }
    write(&assets.join("manifest.json"), &manifest)?;
    write(&assets.join("NOTICE.md"), NOTICE)?;
    println!("wrote {}", assets.display());
    Ok(())
}

fn write_gzip(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    encoder.write_all(bytes).map_err(|e| e.to_string())?;
    let gz = encoder.finish().map_err(|e| e.to_string())?;
    fs::write(path, gz).map_err(|e| format!("{}: {e}", path.display()))
}

/// Clone the VRS repository, or bring an existing shallow clone up to date.
fn update_checkout(dir: &Path) -> Result<()> {
    if dir.join(".git").exists() {
        git(
            dir,
            &["fetch", "--quiet", "--depth", "1", "origin", VRS_BRANCH],
        )?;
        git(dir, &["reset", "--quiet", "--hard", "FETCH_HEAD"])?;
    } else {
        let parent = dir.parent().ok_or("checkout has no parent directory")?;
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        let dir = dir.to_str().ok_or("non-UTF-8 path")?;
        git(
            parent,
            &[
                "clone", "--quiet", "--depth", "1", "--branch", VRS_BRANCH, VRS_REPO, dir,
            ],
        )?;
    }
    Ok(())
}

fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| format!("running git: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// Every CSV file under `dir`, sorted, with its records (header skipped).
/// Columns are read by position, which the VRS schema keeps stable.
fn csv_records(dir: &Path) -> Result<Vec<csv::StringRecord>> {
    let mut files = Vec::new();
    collect_csv(dir, &mut files)?;
    files.sort();
    let mut records = Vec::new();
    for file in files {
        let mut reader = csv::ReaderBuilder::new()
            .flexible(true)
            .from_path(&file)
            .map_err(|e| format!("{}: {e}", file.display()))?;
        for record in reader.records() {
            records.push(record.map_err(|e| format!("{}: {e}", file.display()))?);
        }
    }
    Ok(records)
}

fn collect_csv(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            collect_csv(&path, files)?;
        } else if path.extension().is_some_and(|e| e == "csv") {
            files.push(path);
        }
    }
    Ok(())
}

/// A field cleaned for TSV: no tabs or line breaks, trimmed.
fn field(record: &csv::StringRecord, i: usize) -> String {
    record
        .get(i)
        .unwrap_or("")
        .replace(['\t', '\n', '\r'], " ")
        .trim()
        .to_owned()
}

/// Code, ICAO, IATA, name.
fn read_airlines(vrs: &Path) -> Result<Vec<Vec<String>>> {
    let mut rows: Vec<Vec<String>> = csv_records(&vrs.join("airlines/schema-01"))?
        .iter()
        .map(|r| vec![field(r, 0), field(r, 2), field(r, 3), field(r, 1)])
        .filter(|row| !row[0].is_empty())
        .collect();
    rows.sort();
    rows.dedup_by(|a, b| a[0] == b[0]);
    Ok(rows)
}

/// Code, ICAO, IATA, name, city, country, latitude, longitude, time zone.
fn read_airports(vrs: &Path) -> Result<Vec<Vec<String>>> {
    let finder = tzf_rs::DefaultFinder::new();
    let mut rows = Vec::new();
    for r in csv_records(&vrs.join("airports/schema-01"))? {
        let (Ok(lat), Ok(lon)) = (field(&r, 6).parse::<f64>(), field(&r, 7).parse::<f64>()) else {
            continue;
        };
        if field(&r, 0).is_empty()
            || !(-90.0..=90.0).contains(&lat)
            || !(-180.0..=180.0).contains(&lon)
        {
            continue;
        }
        let tz = finder.get_tz_name(lon, lat).to_owned();
        rows.push(vec![
            field(&r, 0),
            field(&r, 2),
            field(&r, 3),
            field(&r, 1),
            field(&r, 4),
            field(&r, 5),
            format!("{lat:.6}"),
            format!("{lon:.6}"),
            tz,
        ]);
    }
    rows.sort();
    rows.dedup_by(|a, b| a[0] == b[0]);
    Ok(rows)
}

/// Airline code, number, airport codes; sorted by callsign (code + number).
fn read_routes(vrs: &Path, airports: &HashSet<&str>) -> Result<(Vec<Vec<String>>, usize)> {
    let mut by_callsign = BTreeMap::new();
    let mut skipped = 0;
    for r in csv_records(&vrs.join("routes/schema-01"))? {
        let (code, number, stops) = (field(&r, 1), field(&r, 2), field(&r, 4));
        if code.is_empty() || number.is_empty() || !stops.contains('-') {
            continue;
        }
        if !stops.split('-').all(|s| airports.contains(s)) {
            skipped += 1;
            continue;
        }
        by_callsign.insert(format!("{code}{number}"), vec![code, number, stops]);
    }
    Ok((by_callsign.into_values().collect(), skipped))
}

fn write_table(path: &Path, header: &str, rows: &[Vec<String>]) -> Result<()> {
    let mut text = String::with_capacity(rows.len() * 32);
    text.push_str(header);
    text.push('\n');
    for row in rows {
        text.push_str(&row.join("\t"));
        text.push('\n');
    }
    write(path, &text)
}

fn write(path: &Path, text: &str) -> Result<()> {
    fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

fn manifest(
    commit: &str,
    date: &str,
    routes: &[Vec<String>],
    airports: &[Vec<String>],
    airlines: &[Vec<String>],
) -> String {
    let built = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let manifest = serde_json::json!({
        "format": 1,
        "built_at_unix": built,
        "sources": {
            "vrs_standing_data": {
                "url": VRS_REPO,
                "commit": commit,
                "date": date,
                "license": "CC0-1.0",
            },
            "time_zones": {
                "via": "tzf-rs (bundled tzf-dist data)",
                "from": "timezone-boundary-builder, © OpenStreetMap contributors",
                "license": "ODbL-1.0",
            },
            "regions": {
                "url": NATURAL_EARTH,
                "files": [NE_COUNTRIES, NE_SEAS],
                "license": "public domain (Natural Earth)",
            },
        },
        "counts": {
            "routes": routes.len(),
            "airports": airports.len(),
            "airlines": airlines.len(),
        },
    });
    let mut text = serde_json::to_string_pretty(&manifest).unwrap_or_default();
    let _ = writeln!(text);
    text
}

const NOTICE: &str = "\
# Passing Over standing data

Built by `cargo xtask data`; see manifest.json for versions.

- Routes, airports and airlines: Virtual Radar Server standing data
  (https://github.com/vradarserver/standing-data), CC0 1.0. Route data
  contributors are credited in that repository.
- Countries and seas (`regions.tsv`): Natural Earth
  (https://www.naturalearthdata.com/), public domain. Made with Natural
  Earth.
- Airport time zones were computed from timezone-boundary-builder data
  (https://github.com/evansiroky/timezone-boundary-builder),
  © OpenStreetMap contributors, via tzf-rs.

Because the time zone column is derived from ODbL data, `airports.tsv` is
made available under the Open Database License 1.0
(https://opendatacommons.org/licenses/odbl/1-0/). `regions.tsv` is public
domain; the other files are CC0.
These files are not covered by Passing Over's MIT/Apache-2.0 code license.
";

/// Download a Natural Earth file once into `dir` (it's pinned to a release).
fn download(dir: &Path, name: &str) -> Result<PathBuf> {
    let path = dir.join(name);
    if path.exists() {
        return Ok(path);
    }
    fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let status = Command::new("curl")
        .args(["-sSfL", "-o"])
        .arg(&path)
        .arg(format!("{NATURAL_EARTH}/{name}"))
        .status()
        .map_err(|e| format!("running curl: {e}"))?;
    if !status.success() {
        let _ = fs::remove_file(&path);
        return Err(format!("downloading {name} failed"));
    }
    Ok(path)
}

/// One line per polygon: kind, display name, ISO 3166 code (countries),
/// rings (see passingover_core::regions). Countries from the 1:110m admin-0 layer, seas
/// from the 1:50m marine layer, simplified.
fn read_regions(countries: &Path, seas: &Path) -> Result<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    for (path, kind) in [(countries, "c"), (seas, "w")] {
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let json: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        let features = json["features"].as_array().ok_or("no features")?;
        for feature in features {
            let props = &feature["properties"];
            let name = if kind == "c" {
                props["NAME_EN"].as_str().map(country_name)
            } else {
                props["name_en"]
                    .as_str()
                    .or(props["name"].as_str())
                    .map(sea_name)
            };
            let Some(name) = name.filter(|n| !n.is_empty()) else {
                continue;
            };
            let geometry = &feature["geometry"];
            let polygons = match geometry["type"].as_str() {
                Some("Polygon") => vec![&geometry["coordinates"]],
                Some("MultiPolygon") => geometry["coordinates"]
                    .as_array()
                    .map(|p| p.iter().collect())
                    .unwrap_or_default(),
                _ => continue,
            };
            // ISO_A2_EH fixes ISO_A2's "-99" for e.g. France and Norway.
            let iso = props["ISO_A2_EH"]
                .as_str()
                .or(props["ISO_A2"].as_str())
                .filter(|c| kind == "c" && c.len() == 2)
                .unwrap_or("")
                .to_owned();
            let tolerance = if kind == "w" { SEA_TOLERANCE_DEG } else { 0.0 };
            for polygon in polygons {
                let rings: Vec<String> = polygon
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|ring| ring_text(ring, tolerance))
                    .collect();
                if !rings.is_empty() {
                    rows.push(vec![
                        kind.to_owned(),
                        name.clone(),
                        iso.clone(),
                        rings.join(";"),
                    ]);
                }
            }
        }
    }
    Ok(rows)
}

/// A country's name as it reads in a sentence.
fn country_name(name: &str) -> String {
    let short = match name {
        "United States of America" => "the United States",
        "People's Republic of China" => "China",
        "Turkish Republic of Northern Cyprus" => "Northern Cyprus",
        "Czech Republic" => "Czechia",
        "The Bahamas" => "the Bahamas",
        "The Gambia" => "the Gambia",
        _ => name,
    };
    let needs_the = [
        "Central African Republic",
        "Democratic Republic of the Congo",
        "Dominican Republic",
        "Falkland Islands",
        "French Southern and Antarctic Lands",
        "Netherlands",
        "Philippines",
        "Republic of the Congo",
        "Solomon Islands",
        "United Arab Emirates",
        "United Kingdom",
    ];
    if needs_the.contains(&short) {
        format!("the {short}")
    } else {
        short.to_owned()
    }
}

/// A sea's name as it reads in a sentence: "the North Sea".
fn sea_name(name: &str) -> String {
    let name = name.trim();
    let titled = if name.chars().any(char::is_lowercase) {
        name.to_owned()
    } else {
        // Natural Earth writes some oceans in capitals: "INDIAN OCEAN".
        name.split(' ')
            .map(|w| {
                let lower = w.to_lowercase();
                let mut c = lower.chars();
                c.next()
                    .map(|f| f.to_uppercase().chain(c).collect())
                    .unwrap_or_default()
            })
            .collect::<Vec<String>>()
            .join(" ")
    };
    if titled.starts_with("Lake ") {
        titled
    } else {
        format!("the {titled}")
    }
}

/// A GeoJSON ring as `lon,lat lon,lat …` with two decimals, simplified.
fn ring_text(ring: &serde_json::Value, tolerance: f64) -> Option<String> {
    let points: Vec<(f64, f64)> = ring
        .as_array()?
        .iter()
        .filter_map(|p| Some((p[0].as_f64()?, p[1].as_f64()?)))
        .collect();
    let mut kept = simplify(&points, tolerance);
    kept.dedup_by(|a, b| format!("{:.2},{:.2}", a.0, a.1) == format!("{:.2},{:.2}", b.0, b.1));
    (kept.len() >= 4).then(|| {
        kept.iter()
            .map(|(lon, lat)| format!("{lon:.2},{lat:.2}"))
            .collect::<Vec<_>>()
            .join(" ")
    })
}

/// Douglas–Peucker simplification in degrees.
fn simplify(points: &[(f64, f64)], tolerance: f64) -> Vec<(f64, f64)> {
    if tolerance <= 0.0 || points.len() < 5 {
        return points.to_vec();
    }
    let last = points.len() - 1;
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[last] = true;
    let mut stack = vec![(0, last)];
    while let Some((a, b)) = stack.pop() {
        let (ax, ay) = points[a];
        let (bx, by) = points[b];
        let (dx, dy) = (bx - ax, by - ay);
        let len = dx.hypot(dy);
        let mut farthest = (0.0, a);
        for (i, &(px, py)) in points.iter().enumerate().take(b).skip(a + 1) {
            let d = if len == 0.0 {
                (px - ax).hypot(py - ay)
            } else {
                ((px - ax) * dy - (py - ay) * dx).abs() / len
            };
            if d > farthest.0 {
                farthest = (d, i);
            }
        }
        if farthest.0 > tolerance {
            keep[farthest.1] = true;
            stack.push((a, farthest.1));
            stack.push((farthest.1, b));
        }
    }
    points
        .iter()
        .zip(keep)
        .filter_map(|(p, k)| k.then_some(*p))
        .collect()
}
