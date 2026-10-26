//! Polite fetching across the two position sources (D15, D17).
//!
//! Each [`PositionFeed::fetch`] sends at most one request per source. A source
//! that fails is rested with exponential backoff; one that answers 429, or
//! whose rate-limit window runs low, is rested until it resets; one that
//! answers 401 or 403 is left alone for the rest of the session. After
//! [`FAILURES_BEFORE_FAILOVER`] consecutive failures, or on 401/403/429, the
//! feed fails over to the other source and gives the primary another try
//! every [`PRIMARY_RETRY_INTERVAL`]. A source with a daily request limit
//! sits out the rest of the UTC day once it is used up ([`DailyUsage`]).

use std::time::{Duration, Instant, SystemTime};

use thiserror::Error;

use super::{AdsbFi, AdsbLol, Area, PositionSource, Snapshot, SourceId};
use crate::http::HttpClient;
use crate::polite::{
    ADSB_LOL_PRIMARY_SHARE, BACKOFF_BASE, BACKOFF_CAP, DEFAULT_RATE_LIMIT_WAIT,
    FAILURES_BEFORE_FAILOVER, LOW_QUOTA_SHARE, PRIMARY_RETRY_INTERVAL,
};

/// Pick a primary source from a uniformly random `roll` in `0.0..1.0` (D17).
/// The app rolls once and stores the result; the CLI rolls per run.
pub fn choose_primary(roll: f64) -> SourceId {
    if roll < ADSB_LOL_PRIMARY_SHARE {
        SourceId::AdsbLol
    } else {
        SourceId::AdsbFi
    }
}

/// The implementation for a source.
pub fn source_for(id: SourceId) -> Box<dyn PositionSource + Send + Sync> {
    match id {
        SourceId::AdsbLol => Box::new(AdsbLol),
        SourceId::AdsbFi => Box::new(AdsbFi),
    }
}

/// Why a fetch produced no snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FetchError {
    /// Every source is resting or disabled. `retry_after` is when the first
    /// one is available again, or `None` if none will be this session.
    #[error("no position source is available right now")]
    Unavailable { retry_after: Option<Duration> },
    /// The last source tried failed.
    #[error("{}: {reason}", from.name())]
    Failed { from: SourceId, reason: String },
}

/// Position requests sent per source on one UTC day (D15). The app saves it
/// and restores it on start, so restarting doesn't reset the count.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DailyUsage {
    /// Days since the Unix epoch, UTC.
    pub day: u32,
    pub adsb_lol: u32,
    pub adsb_fi: u32,
}

impl DailyUsage {
    pub fn count(&self, id: SourceId) -> u32 {
        match id {
            SourceId::AdsbLol => self.adsb_lol,
            SourceId::AdsbFi => self.adsb_fi,
        }
    }

    fn add(&mut self, id: SourceId) {
        match id {
            SourceId::AdsbLol => self.adsb_lol += 1,
            SourceId::AdsbFi => self.adsb_fi += 1,
        }
    }

    /// Start counting afresh on a new day.
    fn roll_to(&mut self, day: u32) {
        if self.day != day {
            *self = Self {
                day,
                ..Self::default()
            };
        }
    }

    fn exhausted(&self, id: SourceId) -> bool {
        id.daily_request_limit()
            .is_some_and(|limit| self.count(id) >= limit)
    }
}

/// Days since the Unix epoch, UTC.
pub fn utc_day(wall: SystemTime) -> u32 {
    let secs = wall
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    (secs / 86_400) as u32
}

fn until_next_utc_day(wall: SystemTime) -> Duration {
    let secs = wall
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    Duration::from_secs(86_400 - secs % 86_400)
}

/// Fetches snapshots from a primary source with the other as fallback.
pub struct PositionFeed {
    /// Index 0 is the primary, 1 the fallback.
    slots: [Slot; 2],
    /// Set while failed over to the fallback.
    failed_over_at: Option<Instant>,
    usage: DailyUsage,
    rng: fastrand::Rng,
}

struct Slot {
    source: Box<dyn PositionSource + Send + Sync>,
    failures: u32,
    resting_until: Option<Instant>,
    disabled: bool,
}

impl Slot {
    fn new(id: SourceId) -> Self {
        Self {
            source: source_for(id),
            failures: 0,
            resting_until: None,
            disabled: false,
        }
    }

    fn available(&self, now: Instant) -> bool {
        !self.disabled && self.resting_until.is_none_or(|t| now >= t)
    }
}

enum Outcome {
    Ok(Snapshot, Option<Duration>),
    Failed(String),
    RateLimited(Duration),
    Refused(u16),
}

impl std::fmt::Debug for PositionFeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PositionFeed")
            .field("primary", &self.primary())
            .field("failed_over_at", &self.failed_over_at)
            .finish_non_exhaustive()
    }
}

impl PositionFeed {
    /// `seed` drives the backoff jitter; use a random one outside tests.
    pub fn new(primary: SourceId, seed: u64) -> Self {
        let fallback = match primary {
            SourceId::AdsbLol => SourceId::AdsbFi,
            SourceId::AdsbFi => SourceId::AdsbLol,
        };
        Self {
            slots: [Slot::new(primary), Slot::new(fallback)],
            failed_over_at: None,
            usage: DailyUsage::default(),
            rng: fastrand::Rng::with_seed(seed),
        }
    }

    pub fn primary(&self) -> SourceId {
        self.slots[0].source.id()
    }

    /// Requests sent today, to save across restarts.
    pub fn usage(&self) -> DailyUsage {
        self.usage
    }

    /// Restore saved usage; ignored on later calls if it's from another day.
    pub fn set_usage(&mut self, usage: DailyUsage) {
        self.usage = usage;
    }

    /// Fetch the aircraft in `area`. `wall` (UTC clock, for daily limits) and
    /// `now` (monotonic, for backoff) are passed in so tests control time.
    pub fn fetch(
        &mut self,
        client: &dyn HttpClient,
        area: &Area,
        wall: SystemTime,
        now: Instant,
    ) -> Result<Snapshot, FetchError> {
        self.usage.roll_to(utc_day(wall));
        let order = match self.failed_over_at {
            Some(t) if now.duration_since(t) < PRIMARY_RETRY_INTERVAL => [1, 0],
            _ => [0, 1],
        };
        let mut last_error = None;
        for index in order {
            let id = self.slots[index].source.id();
            if !self.slots[index].available(now) || self.usage.exhausted(id) {
                continue;
            }
            self.usage.add(id);
            let source = &self.slots[index].source;
            let outcome = match client.get(&source.request(area)) {
                Err(e) => Outcome::Failed(e.to_string()),
                Ok(r) if r.status == 429 => {
                    Outcome::RateLimited(r.retry_after().unwrap_or(DEFAULT_RATE_LIMIT_WAIT))
                }
                Ok(r) if r.status == 401 || r.status == 403 => Outcome::Refused(r.status),
                Ok(r) if r.status != 200 => Outcome::Failed(format!("HTTP {}", r.status)),
                Ok(r) => match source.parse(&r.body) {
                    Ok(snapshot) => {
                        let low = r.quota_left().is_some_and(|q| q < LOW_QUOTA_SHARE);
                        let rest = low.then(|| r.retry_after().unwrap_or(DEFAULT_RATE_LIMIT_WAIT));
                        Outcome::Ok(snapshot, rest)
                    }
                    Err(e) => Outcome::Failed(e.to_string()),
                },
            };
            match self.record(index, outcome, now) {
                Ok(snapshot) => return Ok(snapshot),
                Err(e) => last_error = Some(e),
            }
        }
        Err(last_error.unwrap_or_else(|| FetchError::Unavailable {
            retry_after: self.next_available(wall, now),
        }))
    }

    /// Update the slot's state from an outcome.
    fn record(
        &mut self,
        index: usize,
        outcome: Outcome,
        now: Instant,
    ) -> Result<Snapshot, FetchError> {
        let is_primary = index == 0;
        let slot = &mut self.slots[index];
        let from = slot.source.id();
        let failover = match outcome {
            Outcome::Ok(snapshot, rest) => {
                slot.failures = 0;
                slot.resting_until = rest.map(|d| now + d);
                if is_primary {
                    self.failed_over_at = None;
                }
                return Ok(snapshot);
            }
            Outcome::Failed(reason) => {
                slot.failures += 1;
                slot.resting_until = Some(now + backoff(slot.failures, &mut self.rng));
                let failover = slot.failures >= FAILURES_BEFORE_FAILOVER;
                (failover, reason)
            }
            Outcome::RateLimited(wait) => {
                slot.failures += 1;
                slot.resting_until = Some(now + wait);
                (true, format!("rate limited for {}s", wait.as_secs()))
            }
            Outcome::Refused(status) => {
                slot.disabled = true;
                (true, format!("HTTP {status}; not retrying this session"))
            }
        };
        let (failover, reason) = failover;
        if is_primary && failover {
            self.failed_over_at = Some(now);
        }
        Err(FetchError::Failed { from, reason })
    }

    /// How long until some source is available, if any will be.
    fn next_available(&self, wall: SystemTime, now: Instant) -> Option<Duration> {
        self.slots
            .iter()
            .filter(|s| !s.disabled)
            .map(|s| {
                let resting = s
                    .resting_until
                    .map_or(Duration::ZERO, |t| t.saturating_duration_since(now));
                if self.usage.exhausted(s.source.id()) {
                    resting.max(until_next_utc_day(wall))
                } else {
                    resting
                }
            })
            .min()
    }
}

/// Exponential backoff with "equal jitter": half the step is fixed and half
/// random, so retries spread out but never come back immediately.
fn backoff(failures: u32, rng: &mut fastrand::Rng) -> Duration {
    let doublings = failures.saturating_sub(1).min(16);
    let step = BACKOFF_BASE.saturating_mul(1 << doublings).min(BACKOFF_CAP);
    step / 2 + step.mul_f64(rng.f64() / 2.0)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::VecDeque;

    use super::*;
    use crate::geo::{Coordinate, QueryPoint, Radius};
    use crate::http::{Request, Response, TransportError};

    const LOL: &str = "api.adsb.lol";
    const FI: &str = "opendata.adsb.fi";

    /// Answers each host from its own queue (empty queue: 200 with no
    /// aircraft) and records every requested host.
    type Answer = Result<Response, TransportError>;

    #[derive(Default)]
    struct FakeClient {
        answers: RefCell<Vec<(&'static str, VecDeque<Answer>)>>,
        requested: RefCell<Vec<&'static str>>,
    }

    impl FakeClient {
        fn answer(&self, host: &'static str, status: u16, headers: &[(&str, &str)]) {
            let headers = headers
                .iter()
                .map(|(n, v)| (n.to_string(), v.to_string()))
                .collect();
            let response = Response::new(status, headers, ok_body());
            self.push(host, Ok(response));
        }

        fn fail(&self, host: &'static str) {
            self.push(host, Err(TransportError("timed out".into())));
        }

        fn push(&self, host: &'static str, answer: Answer) {
            let mut answers = self.answers.borrow_mut();
            match answers.iter_mut().find(|(h, _)| *h == host) {
                Some((_, queue)) => queue.push_back(answer),
                None => answers.push((host, VecDeque::from([answer]))),
            }
        }

        fn take_requested(&self) -> Vec<&'static str> {
            self.requested.take()
        }
    }

    impl HttpClient for FakeClient {
        fn get(&self, request: &Request) -> Result<Response, TransportError> {
            let host = [LOL, FI]
                .into_iter()
                .find(|h| request.url.contains(h))
                .unwrap();
            self.requested.borrow_mut().push(host);
            let mut answers = self.answers.borrow_mut();
            let queued = answers
                .iter_mut()
                .find(|(h, _)| *h == host)
                .and_then(|(_, q)| q.pop_front());
            queued.unwrap_or_else(|| Ok(Response::new(200, Vec::new(), ok_body())))
        }
    }

    fn ok_body() -> Vec<u8> {
        br#"{"ac":[],"now":1792966614001}"#.to_vec()
    }

    fn area() -> Area {
        Area {
            center: QueryPoint::from(Coordinate::new(55.62, 12.66).unwrap()),
            radius: Radius::DEFAULT,
        }
    }

    /// 2026-10-25 22:16:54 UTC.
    fn wall() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_792_966_614)
    }

    fn feed() -> PositionFeed {
        PositionFeed::new(SourceId::AdsbLol, 7)
    }

    fn mins(m: u64) -> Duration {
        Duration::from_secs(m * 60)
    }

    #[test]
    fn healthy_primary_is_the_only_source_asked() {
        let (client, mut feed, t0) = (FakeClient::default(), feed(), Instant::now());
        let snap = feed.fetch(&client, &area(), wall(), t0).unwrap();
        assert_eq!(snap.source, SourceId::AdsbLol);
        assert_eq!(client.take_requested(), [LOL]);
    }

    #[test]
    fn a_failure_tries_the_fallback_once_and_rests_the_primary() {
        let (client, mut feed, t0) = (FakeClient::default(), feed(), Instant::now());
        client.answer(LOL, 503, &[]);
        let snap = feed.fetch(&client, &area(), wall(), t0).unwrap();
        assert_eq!(snap.source, SourceId::AdsbFi);
        assert_eq!(client.take_requested(), [LOL, FI]);

        // Primary rests for at least half of the 15 s first step...
        feed.fetch(&client, &area(), wall(), t0 + Duration::from_secs(5))
            .unwrap();
        assert_eq!(client.take_requested(), [FI]);
        // ...and is used again afterwards; one failure is no failover.
        feed.fetch(&client, &area(), wall(), t0 + Duration::from_secs(16))
            .unwrap();
        assert_eq!(client.take_requested(), [LOL]);
    }

    #[test]
    fn three_failures_fail_over_and_the_primary_is_retried_every_five_minutes() {
        let (client, mut feed, t0) = (FakeClient::default(), feed(), Instant::now());
        for i in 0..3 {
            client.fail(LOL);
            feed.fetch(&client, &area(), wall(), t0 + mins(20 * i))
                .unwrap();
            assert_eq!(client.take_requested(), [LOL, FI]);
        }
        let failed_over = t0 + mins(40);
        // Within five minutes only the fallback is asked, even though the
        // primary's backoff (at most 60 s here) is over.
        feed.fetch(&client, &area(), wall(), failed_over + mins(4))
            .unwrap();
        assert_eq!(client.take_requested(), [FI]);
        // Then the primary gets another try, and success ends the failover.
        let snap = feed
            .fetch(&client, &area(), wall(), failed_over + mins(5))
            .unwrap();
        assert_eq!(snap.source, SourceId::AdsbLol);
        feed.fetch(&client, &area(), wall(), failed_over + mins(6))
            .unwrap();
        assert_eq!(client.take_requested(), [LOL, LOL]);
    }

    #[test]
    fn rate_limit_rests_the_source_for_the_time_asked_and_fails_over() {
        let (client, mut feed, t0) = (FakeClient::default(), feed(), Instant::now());
        client.answer(LOL, 429, &[("Retry-After", "600")]);
        assert_eq!(
            feed.fetch(&client, &area(), wall(), t0).unwrap().source,
            SourceId::AdsbFi
        );
        feed.fetch(&client, &area(), wall(), t0 + mins(9)).unwrap();
        assert_eq!(client.take_requested(), [LOL, FI, FI]);
        feed.fetch(&client, &area(), wall(), t0 + mins(11)).unwrap();
        assert_eq!(client.take_requested(), [LOL]);
    }

    #[test]
    fn refusal_disables_the_source_for_the_session() {
        let (client, mut feed, t0) = (FakeClient::default(), feed(), Instant::now());
        client.answer(LOL, 403, &[]);
        feed.fetch(&client, &area(), wall(), t0).unwrap();
        feed.fetch(&client, &area(), wall(), t0 + mins(600))
            .unwrap();
        assert_eq!(client.take_requested(), [LOL, FI, FI]);
    }

    #[test]
    fn low_quota_rests_the_source_until_its_window_resets() {
        let (client, mut feed, t0) = (FakeClient::default(), feed(), Instant::now());
        let low = [
            ("x-ratelimit-limit", "60;w=60"),
            ("x-ratelimit-remaining", "2"),
            ("x-ratelimit-reset", "45"),
        ];
        client.answer(LOL, 200, &low);
        assert_eq!(
            feed.fetch(&client, &area(), wall(), t0).unwrap().source,
            SourceId::AdsbLol
        );
        feed.fetch(&client, &area(), wall(), t0 + Duration::from_secs(15))
            .unwrap();
        feed.fetch(&client, &area(), wall(), t0 + Duration::from_secs(46))
            .unwrap();
        assert_eq!(client.take_requested(), [LOL, FI, LOL]);
    }

    #[test]
    fn unavailable_when_every_source_rests_or_is_disabled() {
        let (client, mut feed, t0) = (FakeClient::default(), feed(), Instant::now());
        client.answer(LOL, 429, &[("Retry-After", "120")]);
        client.answer(FI, 429, &[("Retry-After", "60")]);
        assert!(matches!(
            feed.fetch(&client, &area(), wall(), t0),
            Err(FetchError::Failed {
                from: SourceId::AdsbFi,
                ..
            })
        ));
        assert_eq!(
            feed.fetch(&client, &area(), wall(), t0 + Duration::from_secs(10)),
            Err(FetchError::Unavailable {
                retry_after: Some(Duration::from_secs(50))
            })
        );

        let (client, mut feed) = (
            FakeClient::default(),
            PositionFeed::new(SourceId::AdsbFi, 1),
        );
        client.answer(LOL, 401, &[]);
        client.answer(FI, 403, &[]);
        assert!(feed.fetch(&client, &area(), wall(), t0).is_err());
        assert_eq!(
            feed.fetch(&client, &area(), wall(), t0 + mins(1)),
            Err(FetchError::Unavailable { retry_after: None })
        );
    }

    #[test]
    fn a_bad_body_counts_as_a_failure() {
        let (client, mut feed, t0) = (FakeClient::default(), feed(), Instant::now());
        client.push(LOL, Ok(Response::new(200, Vec::new(), b"<html>".to_vec())));
        assert_eq!(
            feed.fetch(&client, &area(), wall(), t0).unwrap().source,
            SourceId::AdsbFi
        );
    }

    #[test]
    fn backoff_doubles_with_jitter_up_to_the_cap() {
        let mut rng = fastrand::Rng::with_seed(3);
        for _ in 0..100 {
            let first = backoff(1, &mut rng);
            assert!(first >= Duration::from_millis(7_500) && first <= Duration::from_secs(15));
            let third = backoff(3, &mut rng);
            assert!(third >= Duration::from_secs(30) && third <= Duration::from_secs(60));
            let late = backoff(40, &mut rng);
            assert!(late >= mins(5) && late <= mins(10));
        }
    }

    #[test]
    fn a_used_up_daily_limit_moves_to_the_fallback_until_midnight_utc() {
        let (client, mut feed, t0) = (FakeClient::default(), feed(), Instant::now());
        let limit = SourceId::AdsbLol.daily_request_limit().unwrap();
        feed.set_usage(DailyUsage {
            day: utc_day(wall()),
            adsb_lol: limit - 1,
            adsb_fi: 0,
        });
        feed.fetch(&client, &area(), wall(), t0).unwrap();
        assert_eq!(
            feed.fetch(&client, &area(), wall(), t0).unwrap().source,
            SourceId::AdsbFi
        );
        assert_eq!(client.take_requested(), [LOL, FI]);
        assert_eq!(feed.usage().adsb_lol, limit);
        assert_eq!(feed.usage().adsb_fi, 1);

        // A new UTC day starts the count again.
        let tomorrow = wall() + Duration::from_secs(2 * 3600);
        feed.fetch(&client, &area(), tomorrow, t0).unwrap();
        assert_eq!(client.take_requested(), [LOL]);
        assert_eq!(
            feed.usage(),
            DailyUsage {
                day: utc_day(tomorrow),
                adsb_lol: 1,
                adsb_fi: 0
            }
        );
    }

    #[test]
    fn a_used_up_limit_with_the_fallback_down_waits_for_midnight() {
        let (client, mut feed, t0) = (FakeClient::default(), feed(), Instant::now());
        client.answer(FI, 403, &[]);
        let limit = SourceId::AdsbLol.daily_request_limit().unwrap();
        feed.set_usage(DailyUsage {
            day: utc_day(wall()),
            adsb_lol: limit,
            adsb_fi: 0,
        });
        assert!(feed.fetch(&client, &area(), wall(), t0).is_err());
        // 22:16:54 UTC: 1 h 43 min 6 s to midnight.
        assert_eq!(
            feed.fetch(&client, &area(), wall(), t0),
            Err(FetchError::Unavailable {
                retry_after: Some(Duration::from_secs(6186))
            })
        );
    }

    #[test]
    fn primary_choice_follows_the_share() {
        assert_eq!(choose_primary(0.0), SourceId::AdsbLol);
        assert_eq!(choose_primary(0.49), SourceId::AdsbLol);
        assert_eq!(choose_primary(0.5), SourceId::AdsbFi);
        assert_eq!(choose_primary(0.99), SourceId::AdsbFi);
        assert_eq!(
            PositionFeed::new(SourceId::AdsbFi, 0).primary(),
            SourceId::AdsbFi
        );
    }
}
