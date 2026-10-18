use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::checksum::Checksum;
use crate::media::MediaRequest;
use crate::ratelimit::RateLimiter;

pub const KIB: u64 = 1024;
pub const MIB: u64 = 1024 * KIB;

/// Engine-wide tuning. Defaults are deliberately conservative: a polite
/// per-host ceiling of 8 connections that is only reached when measurements
/// show extra connections actually help.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Upper bound of parallel connections per download.
    pub max_connections: usize,
    /// Connections to start with before measuring whether more help.
    pub initial_connections: usize,
    /// Segments are never split into pieces smaller than this.
    pub min_split_size: u64,
    /// Bytes buffered per connection before a positional write.
    pub buffer_size: usize,
    pub connect_timeout: Duration,
    /// A connection that delivers no bytes for this long is restarted.
    pub stall_timeout: Duration,
    /// How long to wait for a response's headers. Longer than
    /// `stall_timeout` because servers that build the file on the fly (an
    /// archive of a folder) may think for a while before answering.
    pub response_timeout: Duration,
    /// Consecutive failed attempts per segment before the download fails.
    /// Network failures (offline, reset, silence) are not limited by this but
    /// by `offline_grace`.
    pub max_retries: u32,
    /// How long network failures are retried while no byte arrives at all,
    /// so a dropped Wi-Fi, a switch between networks or a VPN reconnect
    /// does not fail a download. Any byte that arrives starts it over.
    pub offline_grace: Duration,
    /// How often progress is synced to disk and recorded in the meta file.
    /// Only matters after a crash: pausing always writes a final checkpoint.
    pub checkpoint_interval: Duration,
    /// Measurement window used when deciding whether to add connections.
    pub ramp_window: Duration,
    /// Minimum relative throughput gain to keep doubling connections.
    pub ramp_gain: f64,
    /// Base delay for exponential backoff.
    pub retry_base_delay: Duration,
    pub retry_max_delay: Duration,
    pub user_agent: String,
    /// `http://`, `https://` or `socks5://` proxy URL.
    pub proxy: Option<String>,
    /// Without `proxy`: connect directly, ignoring the system proxy (a site
    /// rule that says "no proxy" for one host).
    pub no_proxy: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            max_connections: 8,
            initial_connections: 2,
            min_split_size: MIB,
            buffer_size: 256 * KIB as usize,
            connect_timeout: Duration::from_secs(15),
            stall_timeout: Duration::from_secs(30),
            response_timeout: Duration::from_secs(60),
            max_retries: 8,
            offline_grace: Duration::from_secs(10 * 60),
            checkpoint_interval: Duration::from_secs(5),
            ramp_window: Duration::from_secs(2),
            ramp_gain: 1.10,
            retry_base_delay: Duration::from_millis(500),
            retry_max_delay: Duration::from_secs(30),
            user_agent: concat!("Downpipe/", env!("CARGO_PKG_VERSION")).to_string(),
            proxy: None,
            no_proxy: false,
        }
    }
}

/// One thing to download.
#[derive(Debug, Clone)]
pub struct DownloadRequest {
    pub url: String,
    pub output_dir: PathBuf,
    /// Overrides the name derived from the server/URL. Still sanitized.
    pub file_name: Option<String>,
    pub checksum: Option<Checksum>,
    /// Extra request headers (e.g. `Referer`). Headers that would break range
    /// semantics are rejected. They are never written to the meta file.
    pub headers: Vec<(String, String)>,
    /// Per-download override of [`EngineConfig::max_connections`].
    pub max_connections: Option<usize>,
    /// `url` replaces the link the resume data was saved with (an expired
    /// link refreshed by the user). The resume must still prove it is the
    /// same file (size, ETag); if not, the download fails with
    /// [`DownloadError::DifferentFile`](crate::DownloadError::DifferentFile)
    /// and the partial data is kept.
    pub relink: bool,
    /// Download the HLS or DASH stream `url` points to instead of the file
    /// itself. `file_name` then names the result without its extension
    /// mattering; the stream decides it.
    pub media: Option<MediaRequest>,
    /// Per-download override of [`EngineConfig::max_retries`].
    pub max_retries: Option<u32>,
    /// A speed limit of this download's own, on top of the engine's global
    /// one. Shared rather than a number, so its owner can change the rate
    /// while the download runs.
    pub limiter: Option<Arc<RateLimiter>>,
    /// A third limit, shared by every download from one site (a site rule's
    /// speed limit). The download runs at the lowest of the three.
    pub site_limiter: Option<Arc<RateLimiter>>,
}

impl DownloadRequest {
    pub fn new(url: impl Into<String>, output_dir: impl Into<PathBuf>) -> Self {
        Self {
            url: url.into(),
            output_dir: output_dir.into(),
            file_name: None,
            checksum: None,
            headers: Vec::new(),
            max_connections: None,
            relink: false,
            media: None,
            max_retries: None,
            limiter: None,
            site_limiter: None,
        }
    }

    pub(crate) fn retries(&self, config: &EngineConfig) -> u32 {
        self.max_retries.unwrap_or(config.max_retries)
    }
}
