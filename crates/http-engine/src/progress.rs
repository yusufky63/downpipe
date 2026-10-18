use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::media::MediaSelection;
use crate::segments::SegmentView;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Opening the first connection / validating resume data.
    Starting,
    Downloading,
    /// No connection is active because of backoff or `Retry-After`.
    Waiting,
    Verifying,
    Finalizing,
    Done,
}

/// Why a running download is in [`Phase::Waiting`], so a UI can say more
/// than "retrying".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WaitReason {
    /// Connections are open but no bytes arrive; they are being replaced.
    NoData,
    /// Requests fail before reaching the server: offline, DNS, refused,
    /// reset. Retried until the network is back.
    Network,
    /// The server asked to slow down (`429`, `503`).
    Throttled,
    /// The server failed (`5xx`) or sent something unusable.
    Server,
}

/// Snapshot published on a `watch` channel several times per second.
#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    pub phase: Phase,
    /// Set while `phase` is [`Phase::Waiting`].
    pub waiting: Option<WaitReason>,
    pub file_name: Option<String>,
    /// Final destination (the data lives in `<path>.part` until done).
    pub path: Option<PathBuf>,
    pub total: Option<u64>,
    pub downloaded: u64,
    /// Bytes per second over the last couple of seconds.
    pub speed: u64,
    pub connections: usize,
    pub target_connections: usize,
    pub resumable: bool,
    pub segments: Vec<SegmentView>,
    /// Human-readable events worth surfacing ("server ignores ranges", ...).
    pub notices: Vec<String>,
    /// Set for stream downloads once their manifest is read.
    pub media: Option<MediaProgress>,
    /// Set for BitTorrent downloads.
    pub torrent: Option<TorrentProgress>,
}

/// Where a BitTorrent download stands, beyond bytes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct TorrentProgress {
    /// Peers connected right now.
    pub peers: u32,
    /// Peers heard of (trackers, DHT, PEX, local discovery), connected or
    /// not.
    pub known_peers: u32,
    /// Bytes per second sent to peers.
    pub upload_speed: u64,
    /// Bytes sent to peers in all, earlier runs included.
    pub uploaded: u64,
    /// Still fetching the file list from peers (a magnet link).
    pub fetching_metadata: bool,
}

/// Where a stream download stands, beyond bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MediaProgress {
    /// "1080p" and the like.
    pub quality: String,
    /// The tracks being downloaded, to be stored so a resume takes the same.
    pub selection: MediaSelection,
    /// Fragments in all tracks, init sections included.
    pub fragments: usize,
    pub fragments_done: usize,
}

impl Progress {
    /// Nothing known yet: the state a download reports before its first
    /// answer.
    pub fn starting() -> Self {
        Self {
            phase: Phase::Starting,
            waiting: None,
            file_name: None,
            path: None,
            total: None,
            downloaded: 0,
            speed: 0,
            connections: 0,
            target_connections: 0,
            resumable: false,
            segments: Vec::new(),
            notices: Vec::new(),
            media: None,
            torrent: None,
        }
    }

    pub fn fraction(&self) -> Option<f64> {
        match self.total {
            Some(0) => Some(1.0),
            Some(total) => Some(self.downloaded as f64 / total as f64),
            None => None,
        }
    }

    pub fn eta(&self) -> Option<Duration> {
        let remaining = self.total?.checked_sub(self.downloaded)?;
        (self.speed > 0).then(|| Duration::from_secs_f64(remaining as f64 / self.speed as f64))
    }
}

/// Sliding-window throughput meter.
#[derive(Debug)]
pub(crate) struct SpeedMeter {
    window: Duration,
    samples: VecDeque<(Instant, u64)>,
}

impl SpeedMeter {
    pub(crate) fn new(window: Duration) -> Self {
        Self {
            window,
            samples: VecDeque::new(),
        }
    }

    pub(crate) fn record(&mut self, now: Instant, total_bytes: u64) -> u64 {
        self.samples.push_back((now, total_bytes));
        while let Some(&(t, _)) = self.samples.front() {
            if now.duration_since(t) > self.window && self.samples.len() > 2 {
                self.samples.pop_front();
            } else {
                break;
            }
        }
        let (Some(&(t0, b0)), Some(&(t1, b1))) = (self.samples.front(), self.samples.back()) else {
            return 0;
        };
        let secs = t1.duration_since(t0).as_secs_f64();
        if secs <= 0.0 {
            0
        } else {
            (b1.saturating_sub(b0) as f64 / secs) as u64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_over_window() {
        let mut meter = SpeedMeter::new(Duration::from_secs(2));
        let t = Instant::now();
        assert_eq!(meter.record(t, 0), 0);
        assert_eq!(meter.record(t + Duration::from_secs(1), 1000), 1000);
        assert_eq!(meter.record(t + Duration::from_secs(2), 3000), 1500);
        // The first sample falls out of the window.
        assert_eq!(meter.record(t + Duration::from_secs(4), 3000), 0);
    }

    #[test]
    fn eta_and_fraction() {
        let mut p = Progress::starting();
        assert_eq!(p.fraction(), None);
        p.total = Some(1000);
        p.downloaded = 250;
        p.speed = 250;
        assert_eq!(p.fraction(), Some(0.25));
        assert_eq!(p.eta(), Some(Duration::from_secs(3)));
        p.total = Some(0);
        assert_eq!(p.fraction(), Some(1.0));
    }
}
