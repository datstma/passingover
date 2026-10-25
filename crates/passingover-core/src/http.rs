//! HTTP plumbing shared by every source (D21, D23).
//!
//! Sources only build [`Request`]s and parse bodies. An [`HttpClient`] sends
//! the requests; [`UreqClient`] is the default, blocking implementation, and
//! tests use fakes, so nothing here needs a network to be tested.

use std::time::Duration;

use thiserror::Error;

/// Project homepage, sent in the User-Agent so operators can reach us.
pub const PROJECT_URL: &str = "https://github.com/datstma/passingover";

/// The identifying User-Agent from D15, e.g.
/// `PassingOver/0.1.0 (+https://github.com/datstma/passingover)`.
pub fn user_agent(product: &str) -> String {
    format!("{product}/{} (+{PROJECT_URL})", crate::VERSION)
}

/// An HTTP GET request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub url: String,
}

/// A response with any status code; body already read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    /// Header names in lowercase.
    headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new(status: u16, headers: Vec<(String, String)>, body: Vec<u8>) -> Self {
        let headers = headers
            .into_iter()
            .map(|(name, value)| (name.to_ascii_lowercase(), value))
            .collect();
        Self {
            status,
            headers,
            body,
        }
    }

    /// The first value of a header, matched case-insensitively.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// How long the server asks us to wait, from `Retry-After` (seconds),
    /// OpenSky's `X-Rate-Limit-Retry-After-Seconds`, or the rate-limit reset
    /// header that adsb.lol sends (`x-ratelimit-reset`, seconds).
    pub fn retry_after(&self) -> Option<Duration> {
        [
            "retry-after",
            "x-rate-limit-retry-after-seconds",
            "x-ratelimit-reset",
        ]
        .iter()
        .find_map(|name| self.header(name)?.trim().parse::<u64>().ok())
        .map(Duration::from_secs)
    }

    /// Share of the current rate-limit window still available (0.0–1.0),
    /// from `x-ratelimit-remaining` and the first value of
    /// `x-ratelimit-limit` (e.g. `4, 4;w=1, 60;w=60, 500;w=86400`).
    pub fn quota_left(&self) -> Option<f64> {
        let remaining: f64 = self.header("x-ratelimit-remaining")?.trim().parse().ok()?;
        let limit: f64 = self
            .header("x-ratelimit-limit")?
            .split([',', ';'])
            .next()?
            .trim()
            .parse()
            .ok()?;
        (limit > 0.0).then(|| (remaining / limit).clamp(0.0, 1.0))
    }
}

/// The request never produced a response: DNS, connection, TLS or timeout.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("network error: {0}")]
pub struct TransportError(pub String);

/// Sends requests. Implementations must not treat non-2xx statuses as
/// errors; callers need to see 429s and 403s.
pub trait HttpClient {
    fn get(&self, request: &Request) -> Result<Response, TransportError>;
}

#[cfg(feature = "http")]
pub use ureq_client::UreqClient;

#[cfg(feature = "http")]
mod ureq_client {
    use super::{HttpClient, Request, Response, TransportError};
    use crate::polite::{CONNECT_TIMEOUT, REQUEST_TIMEOUT};

    /// Blocking HTTPS client with the politeness defaults from D15:
    /// identifying User-Agent, 5 s connect and 10 s total timeouts, gzip.
    #[derive(Debug, Clone)]
    pub struct UreqClient {
        agent: ureq::Agent,
    }

    impl UreqClient {
        /// `user_agent` should come from [`super::user_agent`].
        pub fn new(user_agent: &str) -> Self {
            let config = ureq::Agent::config_builder()
                .user_agent(user_agent)
                .timeout_connect(Some(CONNECT_TIMEOUT))
                .timeout_global(Some(REQUEST_TIMEOUT))
                .http_status_as_error(false)
                .https_only(true)
                .build();
            Self {
                agent: config.into(),
            }
        }
    }

    impl HttpClient for UreqClient {
        fn get(&self, request: &Request) -> Result<Response, TransportError> {
            let to_err = |e: ureq::Error| TransportError(e.to_string());
            let mut response = self.agent.get(&request.url).call().map_err(to_err)?;
            let status = response.status().as_u16();
            let headers = response
                .headers()
                .iter()
                .filter_map(|(name, value)| {
                    Some((name.as_str().to_owned(), value.to_str().ok()?.to_owned()))
                })
                .collect();
            let body = response.body_mut().read_to_vec().map_err(to_err)?;
            Ok(Response::new(status, headers, body))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(headers: &[(&str, &str)]) -> Response {
        let headers = headers
            .iter()
            .map(|(n, v)| (n.to_string(), v.to_string()))
            .collect();
        Response::new(200, headers, Vec::new())
    }

    #[test]
    fn user_agent_names_product_version_and_contact() {
        assert_eq!(
            user_agent("PassingOver-CLI"),
            format!(
                "PassingOver-CLI/{} (+https://github.com/datstma/passingover)",
                crate::VERSION
            )
        );
    }

    #[test]
    fn headers_match_case_insensitively() {
        let r = response(&[("Retry-After", "120")]);
        assert_eq!(r.header("retry-after"), Some("120"));
        assert_eq!(r.retry_after(), Some(Duration::from_secs(120)));
    }

    #[test]
    fn retry_after_falls_back_to_rate_limit_reset() {
        let r = response(&[("x-ratelimit-reset", "17")]);
        assert_eq!(r.retry_after(), Some(Duration::from_secs(17)));
        // An HTTP date is not understood; the caller uses its default wait.
        let r = response(&[("Retry-After", "Wed, 21 Oct 2026 07:28:00 GMT")]);
        assert_eq!(r.retry_after(), None);
    }

    #[test]
    fn quota_left_uses_the_current_window() {
        let r = response(&[
            ("x-ratelimit-limit", "4, 4;w=1, 60;w=60, 500;w=86400"),
            ("x-ratelimit-remaining", "3"),
        ]);
        assert_eq!(r.quota_left(), Some(0.75));
        let r = response(&[("x-ratelimit-limit", "500"), ("x-ratelimit-remaining", "0")]);
        assert_eq!(r.quota_left(), Some(0.0));
        assert_eq!(response(&[]).quota_left(), None);
    }
}
