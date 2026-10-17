use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use parking_lot::Mutex;

/// Token bucket shared by every connection that should respect one limit.
/// A rate of 0 means unlimited. The rate can change while downloads run.
#[derive(Debug)]
pub struct RateLimiter {
    rate: AtomicU64,
    state: Mutex<Bucket>,
}

#[derive(Debug)]
struct Bucket {
    /// May go negative: callers take what they need and sleep off the debt,
    /// which keeps large chunks from starving behind small ones.
    tokens: f64,
    last: Instant,
}

impl RateLimiter {
    pub fn new(bytes_per_sec: u64) -> Self {
        Self {
            rate: AtomicU64::new(bytes_per_sec),
            state: Mutex::new(Bucket {
                tokens: 0.0,
                last: Instant::now(),
            }),
        }
    }

    pub fn unlimited() -> Self {
        Self::new(0)
    }

    pub fn rate(&self) -> u64 {
        self.rate.load(Ordering::Relaxed)
    }

    pub fn set_rate(&self, bytes_per_sec: u64) {
        self.rate.store(bytes_per_sec, Ordering::Relaxed);
        let mut state = self.state.lock();
        state.tokens = 0.0;
        state.last = Instant::now();
    }

    pub async fn acquire(&self, bytes: usize) {
        if let Some(wait) = self.debit(bytes) {
            tokio::time::sleep(wait).await;
        }
    }

    fn debit(&self, bytes: usize) -> Option<Duration> {
        let rate = self.rate();
        if rate == 0 {
            return None;
        }
        let rate = rate as f64;
        // A quarter second of burst smooths out scheduling jitter.
        let capacity = (rate / 4.0).max(16.0 * 1024.0);
        let mut state = self.state.lock();
        let now = Instant::now();
        let elapsed = now.duration_since(state.last).as_secs_f64();
        state.last = now;
        state.tokens = (state.tokens + elapsed * rate).min(capacity);
        state.tokens -= bytes as f64;
        (state.tokens < 0.0).then(|| Duration::from_secs_f64(-state.tokens / rate))
    }
}

/// Parse `500K`, `2M`, `1.5MB`, `800kb`, `1048576` into bytes per second.
pub fn parse_rate(input: &str) -> Option<u64> {
    let s = input.trim().to_ascii_lowercase();
    let s = s.strip_suffix("/s").unwrap_or(&s);
    let s = s
        .strip_suffix("ib")
        .or_else(|| s.strip_suffix('b'))
        .unwrap_or(s);
    let (num, mult) = match s.chars().last()? {
        'k' => (&s[..s.len() - 1], 1024.0),
        'm' => (&s[..s.len() - 1], 1024.0 * 1024.0),
        'g' => (&s[..s.len() - 1], 1024.0 * 1024.0 * 1024.0),
        _ => (s, 1.0),
    };
    let value: f64 = num.trim().parse().ok()?;
    (value.is_finite() && value >= 0.0).then_some((value * mult) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rates() {
        assert_eq!(parse_rate("1048576"), Some(1_048_576));
        assert_eq!(parse_rate("500K"), Some(512_000));
        assert_eq!(parse_rate("2M"), Some(2 * 1024 * 1024));
        assert_eq!(parse_rate("1.5MB"), Some(1_572_864));
        assert_eq!(parse_rate("800kb/s"), Some(819_200));
        assert_eq!(parse_rate("1MiB"), Some(1_048_576));
        assert_eq!(parse_rate("0"), Some(0));
        assert_eq!(parse_rate("fast"), None);
        assert_eq!(parse_rate("-1M"), None);
        assert_eq!(parse_rate(""), None);
    }

    #[test]
    fn unlimited_never_waits() {
        let limiter = RateLimiter::unlimited();
        assert_eq!(limiter.debit(10 << 20), None);
    }

    #[test]
    fn limited_accumulates_debt() {
        let limiter = RateLimiter::new(1024 * 1024);
        let wait = limiter.debit(512 * 1024).unwrap();
        // Bucket starts empty, so 0.5 MiB at 1 MiB/s is ~0.5 s of debt.
        assert!(wait > Duration::from_millis(400) && wait <= Duration::from_millis(500));
        limiter.set_rate(0);
        assert_eq!(limiter.debit(1 << 30), None);
    }

    #[tokio::test]
    async fn enforces_rate_over_time() {
        let limiter = RateLimiter::new(200 * 1024);
        let start = Instant::now();
        for _ in 0..10 {
            limiter.acquire(20 * 1024).await;
        }
        // 200 KiB at 200 KiB/s: about a second.
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(850), "{elapsed:?}");
    }
}
