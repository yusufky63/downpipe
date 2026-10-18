//! Slows down token guessing: a peer that failed too often recently is
//! refused without its token even being checked.

use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::time::{Duration, Instant};

/// Failed attempts a peer may make within [`WINDOW`].
pub(crate) const MAX_FAILURES: usize = 10;
pub(crate) const WINDOW: Duration = Duration::from_secs(10 * 60);
/// Peers remembered at once, so a flood of addresses cannot exhaust memory.
const MAX_PEERS: usize = 4096;

#[derive(Debug, Default)]
pub(crate) struct FailureLimiter {
    peers: HashMap<IpAddr, VecDeque<Instant>>,
}

impl FailureLimiter {
    /// How long `ip` has to wait, if it used up its attempts.
    pub(crate) fn blocked(&mut self, ip: IpAddr, now: Instant) -> Option<Duration> {
        let key = key(ip);
        let failures = self.peers.get_mut(&key)?;
        forget_old(failures, now);
        if failures.is_empty() {
            self.peers.remove(&key);
            return None;
        }
        (failures.len() >= MAX_FAILURES).then(|| {
            let oldest = failures[0];
            WINDOW
                .saturating_sub(now.duration_since(oldest))
                .max(Duration::from_secs(1))
        })
    }

    pub(crate) fn failed(&mut self, ip: IpAddr, now: Instant) {
        let key = key(ip);
        if !self.peers.contains_key(&key) && self.peers.len() >= MAX_PEERS {
            self.peers.retain(|_, failures| {
                forget_old(failures, now);
                !failures.is_empty()
            });
            if self.peers.len() >= MAX_PEERS {
                // Still full of live entries: drop the quietest peer.
                if let Some(quietest) = self
                    .peers
                    .iter()
                    .min_by_key(|(_, f)| f.back().copied())
                    .map(|(ip, _)| *ip)
                {
                    self.peers.remove(&quietest);
                }
            }
        }
        let failures = self.peers.entry(key).or_default();
        forget_old(failures, now);
        failures.push_back(now);
        while failures.len() > MAX_FAILURES {
            failures.pop_front();
        }
    }

    /// A correct token clears the peer's record.
    pub(crate) fn succeeded(&mut self, ip: IpAddr) {
        self.peers.remove(&key(ip));
    }
}

fn forget_old(failures: &mut VecDeque<Instant>, now: Instant) {
    while failures
        .front()
        .is_some_and(|t| now.duration_since(*t) >= WINDOW)
    {
        failures.pop_front();
    }
}

/// IPv4 by address; IPv6 by its /64, which one device can fill with
/// addresses of its own.
fn key(ip: IpAddr) -> IpAddr {
    match ip.to_canonical() {
        IpAddr::V6(v6) => {
            let s = v6.segments();
            IpAddr::from([s[0], s[1], s[2], s[3], 0, 0, 0, 0])
        }
        v4 => v4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: IpAddr = IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 20));
    const B: IpAddr = IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 21));

    #[test]
    fn blocks_after_too_many_failures_until_they_age_out() {
        let mut limiter = FailureLimiter::default();
        let start = Instant::now();
        for i in 0..MAX_FAILURES {
            assert_eq!(limiter.blocked(A, start), None, "attempt {i} is allowed");
            limiter.failed(A, start);
        }
        let wait = limiter.blocked(A, start).expect("blocked");
        assert_eq!(wait, WINDOW);
        assert_eq!(
            limiter.blocked(B, start),
            None,
            "other peers are unaffected"
        );
        let later = start + WINDOW;
        assert_eq!(limiter.blocked(A, later), None, "old failures expire");
    }

    #[test]
    fn success_clears_the_record() {
        let mut limiter = FailureLimiter::default();
        let now = Instant::now();
        for _ in 0..MAX_FAILURES - 1 {
            limiter.failed(A, now);
        }
        limiter.succeeded(A);
        limiter.failed(A, now);
        assert_eq!(limiter.blocked(A, now), None);
    }

    #[test]
    fn ipv6_peers_share_their_prefix() {
        let mut limiter = FailureLimiter::default();
        let now = Instant::now();
        for i in 0..MAX_FAILURES as u16 {
            limiter.failed(IpAddr::from([0xfd00, 1, 2, 3, 0, 0, 0, i]), now);
        }
        assert!(
            limiter
                .blocked(IpAddr::from([0xfd00, 1, 2, 3, 9, 9, 9, 9]), now)
                .is_some()
        );
        assert!(
            limiter
                .blocked(IpAddr::from([0xfd00, 1, 2, 4, 0, 0, 0, 1]), now)
                .is_none()
        );
    }

    #[test]
    fn memory_stays_bounded() {
        let mut limiter = FailureLimiter::default();
        let now = Instant::now();
        for i in 0..(MAX_PEERS as u32 + 100) {
            limiter.failed(IpAddr::from(i.to_be_bytes()), now);
        }
        assert!(limiter.peers.len() <= MAX_PEERS);
    }
}
