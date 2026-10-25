//! Politeness defaults (D15). The reasoning behind each number is in
//! docs/data-sources.md, "Politeness defaults".

use std::time::Duration;

/// Normal position poll interval while the view is visible.
pub const POLL_INTERVAL: Duration = Duration::from_secs(15);
/// Never poll positions more often than this.
pub const MIN_POLL_INTERVAL: Duration = Duration::from_secs(10);
/// Random spread added to each poll (±), so clients don't synchronise.
pub const POLL_JITTER: Duration = Duration::from_secs(2);
/// Poll interval in ambient mode, until a source agrees to more.
pub const AMBIENT_POLL_INTERVAL: Duration = Duration::from_secs(60);

pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Consecutive failures of the primary source before failing over.
pub const FAILURES_BEFORE_FAILOVER: u32 = 3;
/// While failed over, how often to give the primary another try.
pub const PRIMARY_RETRY_INTERVAL: Duration = Duration::from_secs(5 * 60);
/// First backoff step after a failure; doubles per consecutive failure.
pub const BACKOFF_BASE: Duration = Duration::from_secs(15);
/// Longest backoff.
pub const BACKOFF_CAP: Duration = Duration::from_secs(10 * 60);
/// Wait after a 429, or when the quota runs low, if the server gives no time.
pub const DEFAULT_RATE_LIMIT_WAIT: Duration = Duration::from_secs(60);
/// Below this share of a rate-limit window, rest the source until it resets.
pub const LOW_QUOTA_SHARE: f64 = 0.10;

/// Share of installs that use adsb.lol as their primary source; the rest use
/// adsb.fi (D17). Shift it in a release if an operator asks.
pub const ADSB_LOL_PRIMARY_SHARE: f64 = 0.5;
