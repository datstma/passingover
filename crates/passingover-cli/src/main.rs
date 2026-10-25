//! `passingover` — where the planes above you are going.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Instant, SystemTime};

use clap::{Parser, Subcommand, ValueEnum};
use passingover_core::data::StandingData;
use passingover_core::flight::flights_overhead;
use passingover_core::geo::{Coordinate, QueryPoint, Radius};
use passingover_core::http::{UreqClient, user_agent};
use passingover_core::position::{Area, PositionFeed, SourceId, choose_primary};
use passingover_core::route::{RouteLookup, RouteSourceId};
use passingover_core::story::dreamy_line;

/// Largest radius the CLI accepts (25 nm, D15).
const MAX_RADIUS_KM: f64 = 46.3;

#[derive(Debug, Parser)]
#[command(
    name = "passingover",
    version,
    about = "Where the planes above you are going."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Show the flights passing over a place right now.
    Now(NowArgs),
    /// Check that the CLI is wired to the core.
    Hello,
}

#[derive(Debug, clap::Args)]
struct NowArgs {
    /// Latitude in decimal degrees. It is rounded to about 1 km before it is
    /// sent anywhere.
    #[arg(long, allow_hyphen_values = true)]
    lat: f64,
    /// Longitude in decimal degrees, rounded the same way.
    #[arg(long, allow_hyphen_values = true)]
    lon: f64,
    /// Search radius in kilometres (default 37, at most 46).
    #[arg(long)]
    radius_km: Option<f64>,
    /// Aircraft data source; picked at random when not given.
    #[arg(long, value_enum)]
    source: Option<Source>,
    /// Directory built by `cargo xtask data` (or set PASSINGOVER_DATA).
    #[arg(long, env = "PASSINGOVER_DATA")]
    data: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Source {
    #[value(name = "adsb.lol")]
    AdsbLol,
    #[value(name = "adsb.fi")]
    AdsbFi,
}

impl From<Source> for SourceId {
    fn from(s: Source) -> Self {
        match s {
            Source::AdsbLol => SourceId::AdsbLol,
            Source::AdsbFi => SourceId::AdsbFi,
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Now(args) => now(&args),
        Command::Hello => {
            println!("{}", passingover_core::hello());
            Ok(())
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn now(args: &NowArgs) -> Result<(), String> {
    let here = Coordinate::new(args.lat, args.lon).map_err(|e| e.to_string())?;
    let radius = match args.radius_km {
        Some(km) if km > MAX_RADIUS_KM => {
            return Err(format!("--radius-km can be at most {MAX_RADIUS_KM}"));
        }
        Some(km) => Radius::from_kilometres(km).map_err(|e| e.to_string())?,
        None => Radius::DEFAULT,
    };
    let area = Area {
        center: QueryPoint::from(here),
        radius,
    };
    let primary = args
        .source
        .map_or_else(|| choose_primary(fastrand::f64()), SourceId::from);
    let data = load_data(args.data.as_ref());
    let client = UreqClient::new(&user_agent("PassingOver-CLI"));

    let mut feed = PositionFeed::new(primary, fastrand::u64(..));
    let snapshot = feed
        .fetch(&client, &area, Instant::now())
        .map_err(|e| format!("Couldn't reach the aircraft data: {e}"))?;

    let mut routes = RouteLookup::new();
    let flights = flights_overhead(&snapshot, here, radius, |aircraft| {
        let callsign = aircraft.callsign.as_deref()?;
        routes.lookup(callsign, data.as_ref(), Some(&client), Instant::now())
    });

    let wall_clock = SystemTime::now();
    if flights.is_empty() {
        println!("The sky above you is quiet right now.");
    }
    for flight in &flights {
        println!("{}", dreamy_line(flight, wall_clock));
    }

    let attribution = snapshot.source.attribution();
    let mut credits = format!("{}, {}", attribution.text, attribution.url);
    let mut route_sources: Vec<RouteSourceId> =
        flights.iter().filter_map(|f| f.route_source).collect();
    route_sources.dedup();
    if !route_sources.is_empty() {
        let names: Vec<&str> = route_sources.iter().map(|s| s.name()).collect();
        credits.push_str(&format!(". Routes: {}", names.join(", ")));
    }
    println!("\n{credits}.");
    Ok(())
}

/// Load the standing data from `--data`, `PASSINGOVER_DATA`, or the
/// workspace's `data/` directory in development builds.
fn load_data(dir: Option<&PathBuf>) -> Option<StandingData> {
    let dev_dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data"));
    let dir = dir.cloned().or_else(|| dev_dir.is_dir().then_some(dev_dir));
    let Some(dir) = dir else {
        eprintln!(
            "No route data found (build it with `cargo xtask data`); looking routes up online."
        );
        return None;
    };
    match StandingData::load(&dir) {
        Ok(data) => Some(data),
        Err(e) => {
            eprintln!("Couldn't load route data ({e}); looking routes up online.");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn now_accepts_negative_coordinates_and_a_source() {
        let cli = Cli::try_parse_from([
            "passingover",
            "now",
            "--lat",
            "-33.86",
            "--lon",
            "151.21",
            "--source",
            "adsb.fi",
        ])
        .unwrap();
        let Command::Now(args) = cli.command else {
            panic!("expected now");
        };
        assert_eq!((args.lat, args.lon), (-33.86, 151.21));
        assert!(matches!(args.source, Some(Source::AdsbFi)));
    }
}
