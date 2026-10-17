use std::path::PathBuf;
use std::time::Duration;

use crate::checksum::Checksum;

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
    /// Attempts per segment before the download fails.
    pub max_retries: u32,
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
            max_retries: 8,
            checkpoint_interval: Duration::from_secs(5),
            ramp_window: Duration::from_secs(2),
            ramp_gain: 1.10,
            retry_base_delay: Duration::from_millis(500),
            retry_max_delay: Duration::from_secs(30),
            user_agent: concat!("Downpipe/", env!("CARGO_PKG_VERSION")).to_string(),
            proxy: None,
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
        }
    }
}
