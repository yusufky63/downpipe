use std::time::{Duration, SystemTime};

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
