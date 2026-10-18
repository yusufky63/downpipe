//! BitTorrent downloads for Downpipe, through [librqbit]: magnet links and
//! `.torrent` files run behind the engine's [`downpipe_http::DownloadHandle`],
//! so the queue and the UIs treat them like any other download.
//!
//! [librqbit]: https://github.com/ikatson/rqbit

mod job;
pub mod magnet;
pub mod meta;
mod session;
#[cfg(feature = "testing")]
pub mod testing;

pub use job::{METADATA_TIMEOUT, SeedPolicy, Source, TorrentRequest};
pub use magnet::{Magnet, MagnetError, is_magnet};
pub use meta::{MAX_TORRENT_FILE, Metadata, TorrentFile, local_path};
pub use session::{Limits, NetConfig, SeedStatus, Torrents};

/// The librqbit release this build uses, for the About and diagnostics.
pub fn library_version() -> &'static str {
    librqbit::version()
}
