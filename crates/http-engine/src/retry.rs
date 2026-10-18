use std::time::{Duration, Instant, SystemTime};

use crate::DownloadError;

/// Longest server-requested pause we honour before treating it as a failure.
pub const MAX_RETRY_AFTER: Duration = Duration::from_secs(600);

/// Exponential backoff with ±20 % jitter so parallel connections that failed
/// together do not hammer the server in lockstep.
pub fn backoff(attempt: u32, base: Duration, max: Duration) -> Duration {
    let exp = base.saturating_mul(1u32 << attempt.min(16));
    let capped = exp.min(max);
    let jitter = 0.8 + fastrand::f64() * 0.4;
    capped.mul_f64(jitter)
}

/// Decides whether a failed request is tried again. Server failures get
/// `max_retries` attempts in a row; network failures are retried until no
/// byte has arrived from any connection for `grace`, because counting them
/// would let a two-minute Wi-Fi outage fail every download in it.
#[derive(Debug)]
pub(crate) struct Budget {
    max_retries: u32,
    grace: Duration,
    last_progress: Instant,
}

impl Budget {
    pub(crate) fn new(max_retries: u32, grace: Duration) -> Self {
        Self {
            max_retries,
            grace,
            last_progress: Instant::now(),
        }
    }

    /// Bytes arrived: the network works, the grace period starts over.
    pub(crate) fn progressed(&mut self, now: Instant) {
        self.last_progress = now;
    }

    /// `failures` counts this one.
    pub(crate) fn allows(&self, err: &DownloadError, failures: u32, now: Instant) -> bool {
        err.is_retryable()
            && (failures <= self.max_retries
                || (err.is_connectivity()
                    && now.saturating_duration_since(self.last_progress) < self.grace))
    }
}

/// `Retry-After` is either delta-seconds or an HTTP-date (RFC 9110 §10.2.3).
pub fn parse_retry_after(value: &str) -> Option<Duration> {
    let value = value.trim();
    let delay = if let Ok(secs) = value.parse::<u64>() {
        Duration::from_secs(secs)
    } else {
        let when = httpdate::parse_http_date(value).ok()?;
        when.duration_since(SystemTime::now())
            .unwrap_or(Duration::ZERO)
    };
    Some(delay.min(MAX_RETRY_AFTER))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_and_caps() {
        let base = Duration::from_millis(500);
        let max = Duration::from_secs(30);
        let first = backoff(0, base, max);
        assert!(first >= Duration::from_millis(400) && first <= Duration::from_millis(600));
        let fourth = backoff(3, base, max);
        assert!(fourth >= Duration::from_millis(3200) && fourth <= Duration::from_millis(4800));
        let huge = backoff(40, base, max);
        assert!(huge <= Duration::from_secs(36));
    }

    #[test]
    fn network_failures_last_until_the_grace_runs_out() {
        let budget = Budget::new(2, Duration::from_secs(60));
        let t = Instant::now();
        let net = DownloadError::Network("could not connect".into());
        let server = DownloadError::HttpStatus { status: 500 };
        assert!(budget.allows(&server, 2, t));
        assert!(!budget.allows(&server, 3, t), "server failures are counted");
        assert!(budget.allows(&net, 50, t + Duration::from_secs(59)));
        assert!(!budget.allows(&net, 50, t + Duration::from_secs(61)));
        assert!(!budget.allows(&DownloadError::Forbidden, 1, t));

        let mut budget = budget;
        budget.progressed(t + Duration::from_secs(61));
        assert!(
            budget.allows(&net, 50, t + Duration::from_secs(100)),
            "progress restarts the grace"
        );
    }

    #[test]
    fn retry_after_formats() {
        assert_eq!(parse_retry_after("3"), Some(Duration::from_secs(3)));
        assert_eq!(parse_retry_after("999999"), Some(MAX_RETRY_AFTER));
        let past = httpdate::fmt_http_date(SystemTime::now() - Duration::from_secs(60));
        assert_eq!(parse_retry_after(&past), Some(Duration::ZERO));
        let future = httpdate::fmt_http_date(SystemTime::now() + Duration::from_secs(120));
        let parsed = parse_retry_after(&future).unwrap();
        assert!(parsed > Duration::from_secs(100) && parsed <= Duration::from_secs(120));
        assert_eq!(parse_retry_after("soon"), None);
    }
}
