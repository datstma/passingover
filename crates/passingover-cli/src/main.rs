//! `passingover` — where the planes above you are going.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Instant, SystemTime};

use clap::{Parser, Subcommand, ValueEnum};
use passingover_core::Sky;
use passingover_core::credits;
use passingover_core::data::StandingData;
use passingover_core::geo::{Coordinate, Radius};
use passingover_core::http::{UreqClient, user_agent};
use passingover_core::log::DaydreamLog;
use passingover_core::position::{SourceId, choose_primary};
use passingover_core::regions::{REGIONS_FILE, Regions};
use passingover_core::story::{Units, dreamy_line, journey, log_summary};

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
    /// Show what the daydream log remembers.
    Log(LogArgs),
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
    /// Show temperatures in °F instead of °C.
    #[arg(long)]
    imperial: bool,
    /// Remember the flights in this daydream log file (only flight facts,
    /// never your location).
    #[arg(long)]
    log: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
struct LogArgs {
    /// The daydream log file written by `now --log`.
    #[arg(long)]
    log: PathBuf,
    #[arg(long, value_enum, default_value_t = Period::Month)]
    period: Period,
    /// Where you are, for "the farthest" (not stored).
    #[arg(long, allow_hyphen_values = true, requires = "lon")]
    lat: Option<f64>,
    #[arg(long, allow_hyphen_values = true, requires = "lat")]
    lon: Option<f64>,
    /// Show distances in miles.
    #[arg(long)]
    imperial: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Period {
    Week,
    Month,
    All,
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
        Command::Log(args) => show_log(&args),
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
    let primary = args
        .source
        .map_or_else(|| choose_primary(fastrand::f64()), SourceId::from);
    let mut sky = Sky::new(primary, fastrand::u64(..));
    if let Some(dir) = data_dir(args.data.as_ref()) {
        match StandingData::load(&dir) {
            Ok(data) => sky.set_data(data),
            Err(e) => eprintln!("Couldn't load route data ({e}); looking routes up online."),
        }
        if dir.join(REGIONS_FILE).exists() {
            match Regions::load(&dir) {
                Ok(regions) => sky.set_regions(regions),
                Err(e) => eprintln!("Couldn't load countries and seas ({e})."),
            }
        }
    }
    let units = if args.imperial {
        Units::Imperial
    } else {
        Units::Metric
    };
    let client = UreqClient::new(&user_agent("PassingOver-CLI"));
    let overhead = sky
        .poll(&client, here, radius, SystemTime::now(), Instant::now())
        .map_err(|e| format!("Couldn't reach the aircraft data: {e}"))?;

    if let Some(path) = &args.log {
        DaydreamLog::open(path)
            .and_then(|mut log| log.record(&overhead, SystemTime::now()))
            .map_err(|e| format!("Couldn't write the daydream log: {e}"))?;
    }

    let wall_clock = SystemTime::now();
    if overhead.flights.is_empty() {
        println!("The sky above you is quiet right now.");
    }
    for flight in &overhead.flights {
        println!("{}", dreamy_line(flight, wall_clock, units));
        if let Some(story) = journey(flight) {
            println!("    {story}");
        }
    }

    let attribution = overhead.source.attribution();
    let mut credits = format!("{}, {}", attribution.text, attribution.url);
    let route_sources = overhead.route_sources();
    if !route_sources.is_empty() {
        let names: Vec<&str> = route_sources.iter().map(|s| s.name()).collect();
        credits.push_str(&format!(". Routes: {}", names.join(", ")));
    }
    for source in overhead.weather_sources() {
        let c = credits::for_weather_source(source);
        credits.push_str(&format!(". {}, {}", c.text, c.url));
    }
    println!("\n{credits}.");
    Ok(())
}

fn show_log(args: &LogArgs) -> Result<(), String> {
    let log = DaydreamLog::open(&args.log).map_err(|e| e.to_string())?;
    let now_ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64);
    const DAY_MS: i64 = 24 * 60 * 60 * 1000;
    let (since, label) = match args.period {
        Period::Week => (now_ms - 7 * DAY_MS, "In the last 7 days"),
        Period::Month => (now_ms - 30 * DAY_MS, "In the last 30 days"),
        Period::All => (0, "So far"),
    };
    let here = match (args.lat, args.lon) {
        (Some(lat), Some(lon)) => Some(Coordinate::new(lat, lon).map_err(|e| e.to_string())?),
        _ => None,
    };
    let units = if args.imperial {
        Units::Imperial
    } else {
        Units::Metric
    };
    for line in log_summary(&log.stats(since, here), label, units) {
        println!("{line}");
    }
    Ok(())
}

/// The data directory: `--data`, `PASSINGOVER_DATA`, or the workspace's
/// `data/` in development builds.
fn data_dir(dir: Option<&PathBuf>) -> Option<PathBuf> {
    let dev_dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data"));
    let dir = dir.cloned().or_else(|| dev_dir.is_dir().then_some(dev_dir));
    if dir.is_none() {
        eprintln!(
            "No route data found (build it with `cargo xtask data`); looking routes up online."
        );
    }
    dir
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
