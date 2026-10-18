//! Downpipe's HTTP download engine.
//!
//! * Probes with a real `Range: bytes=0-` request and keeps that connection
//!   as the first worker, so no request is wasted on a separate `HEAD`.
//! * Splits work adaptively: an idle connection takes half of the segment
//!   with the most bytes left (work stealing), and the connection count only
//!   grows while measurements show it helps.
//! * Writes positionally into `<name>.part`, checkpoints completed ranges to
//!   `<name>.part.meta` after syncing the data, validates resumes with
//!   `If-Range` + ETag/size, and renames to the final name only at the end.
//!
//! ```no_run
//! # async fn demo() -> Result<(), downpipe_http::DownloadError> {
//! use downpipe_http::{DownloadRequest, Engine, EngineConfig};
//!
//! let engine = Engine::new(EngineConfig::default())?;
//! let handle = engine.start(DownloadRequest::new("https://example.org/file.iso", "."));
//! let outcome = handle.wait().await?;
//! println!("{outcome:?}");
//! # Ok(()) }
//! ```

mod checksum;
mod config;
mod engine;
mod error;
pub mod filename;
pub mod hosts;
mod http;
pub mod media;
pub mod meta;
mod progress;
mod ratelimit;
mod retry;
pub mod segments;
mod worker;
mod writer;

pub use checksum::{Checksum, HashAlgo, hash_file};
pub use config::{DownloadRequest, EngineConfig, KIB, MIB};
pub use engine::{DownloadControl, DownloadHandle, Engine, JobContext, Outcome};
pub use error::{DownloadError, SITE_KINDS};
pub use http::{
    ContentRange, ResourceInfo, build_headers, display_url, parse_content_range, redact_url,
};
pub use meta::ResumeMeta;
pub use progress::{MediaProgress, Phase, Progress, TorrentProgress, WaitReason};
pub use ratelimit::{RateLimiter, parse_rate};
pub use segments::{ByteRange, SegmentView};

/// Run blocking file-system work off the async threads.
pub(crate) async fn blocking<T, E>(
    f: impl FnOnce() -> Result<T, E> + Send + 'static,
) -> Result<T, DownloadError>
where
    T: Send + 'static,
    E: Into<DownloadError> + Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| DownloadError::Io(std::io::Error::other(e)))?
        .map_err(Into::into)
}
