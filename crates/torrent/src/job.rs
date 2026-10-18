//! A torrent download behind a [`DownloadHandle`], and its seeding after.
//!
//! Pausing takes the torrent out of the session and keeps its files;
//! starting again adds it back, librqbit checks the pieces already on disk
//! and fetches the rest. Cancelling deletes the files it wrote. Once every
//! chosen file is complete the job returns `Completed`, and the torrent
//! keeps uploading in the background until a seeding limit is reached (see
//! [`SeedPolicy`]), which [`Torrents::seeding`] reports.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use downpipe_http::{
    DownloadError, DownloadHandle, JobContext, Outcome, Phase, Progress, TorrentProgress,
};
use futures_util::StreamExt;
use librqbit::api::TorrentIdOrHash;
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, ManagedTorrent, Session, TorrentStats,
    TorrentStatsState,
};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use crate::magnet::{self, Magnet};
use crate::meta::{self, MAX_TORRENT_FILE, Metadata};
use crate::session::{SeedStatus, Seeder, Torrents};

const TICK: Duration = Duration::from_millis(500);
/// How long [`Torrents::metadata`] waits for peers to send a magnet link's
/// file list before giving up.
pub const METADATA_TIMEOUT: Duration = Duration::from_secs(120);

/// When a finished torrent stops uploading. Seeding stops at whichever limit
/// comes first; with neither set it goes on until stopped.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeedPolicy {
    /// Upload at all after finishing.
    pub enabled: bool,
    /// Stop once this many times the downloaded size was uploaded.
    pub ratio: Option<f64>,
    /// Stop after seeding this long.
    pub time: Option<Duration>,
}

impl SeedPolicy {
    pub const NONE: Self = Self {
        enabled: false,
        ratio: None,
        time: None,
    };

    fn reached(&self, uploaded: u64, size: u64, seeding_for: Duration) -> bool {
        !self.enabled
            || self
                .ratio
                .is_some_and(|r| uploaded as f64 >= r * size.max(1) as f64)
            || self.time.is_some_and(|t| seeding_for >= t)
    }
}

/// Where a torrent's metadata comes from.
#[derive(Debug, Clone)]
pub struct Source {
    /// A `magnet:` link, or the http(s) link of a `.torrent` file.
    pub link: String,
    /// Sent when fetching a `.torrent` over http (the browser's Referer and
    /// the like). Memory only.
    pub headers: Vec<(String, String)>,
    /// For fetching a `.torrent` over http: the engine's client, so the
    /// proxy and user agent apply.
    pub client: Option<reqwest::Client>,
    /// The torrent's `.torrent` file in Downpipe's data folder: read when it
    /// exists, written once the metadata is known.
    pub metainfo: PathBuf,
}

/// One torrent to download.
#[derive(Debug, Clone)]
pub struct TorrentRequest {
    pub source: Source,
    /// The folder chosen for it; a multi-file torrent gets a folder of its
    /// own inside (see [`Metadata::output_dir`]).
    pub dir: PathBuf,
    /// Indices of the files to download; `None` for all.
    pub only_files: Option<Vec<usize>>,
    pub seed: SeedPolicy,
    /// Bytes uploaded in earlier runs, so the ratio counts them.
    pub uploaded: u64,
}

impl Torrents {
    /// The torrent's metadata: read from its `.torrent` file in the data
    /// folder, else fetched (an http link) or asked from peers (a magnet
    /// link, up to [`METADATA_TIMEOUT`]) and kept there. Downloads nothing
    /// else.
    pub async fn metadata(&self, source: &Source) -> Result<Metadata, DownloadError> {
        let resolved = tokio::time::timeout(METADATA_TIMEOUT, self.resolve(source))
            .await
            .map_err(|_| {
                DownloadError::Torrent(
                    "no peer sent the torrent's file list in time; try again later, or check that DHT is on in Settings".into(),
                )
            })??;
        Ok(resolved.0)
    }

    /// Start (or resume) a torrent on the current Tokio runtime.
    pub fn start(&self, request: TorrentRequest) -> DownloadHandle {
        let torrents = self.clone();
        DownloadHandle::spawn(move |ctx| async move { torrents.run(request, ctx).await })
    }

    /// Metadata, its `.torrent` bytes and peers already met while finding it.
    async fn resolve(
        &self,
        source: &Source,
    ) -> Result<(Metadata, bytes::Bytes, Vec<SocketAddr>), DownloadError> {
        if let Ok(bytes) = tokio::fs::read(&source.metainfo).await {
            let meta = Metadata::parse(&bytes)?;
            return Ok((meta, bytes.into(), Vec::new()));
        }
        let (bytes, peers) = if magnet::is_magnet(&source.link) {
            self.ask_peers(&source.link).await?
        } else {
            (fetch(source).await?, Vec::new())
        };
        let meta = Metadata::parse(&bytes)?;
        write_atomic(&source.metainfo, &bytes).await?;
        Ok((meta, bytes, peers))
    }

    async fn ask_peers(
        &self,
        link: &str,
    ) -> Result<(bytes::Bytes, Vec<SocketAddr>), DownloadError> {
        let magnet = Magnet::parse(link).map_err(|e| DownloadError::Torrent(e.to_string()))?;
        let session = self.session().await?;
        let response = session
            .add_torrent(
                AddTorrent::from_url(link.to_string()),
                Some(AddTorrentOptions {
                    list_only: true,
                    initial_peers: (!magnet.peers.is_empty()).then(|| magnet.peers.clone()),
                    ..Default::default()
                }),
            )
            .await
            .map_err(|e| no_peers(&e))?;
        match response {
            AddTorrentResponse::ListOnly(list) => {
                let mut peers = magnet.peers;
                peers.extend(list.seen_peers);
                Ok((list.torrent_bytes, peers))
            }
            // Running already (seeding, or a second entry for the same
            // torrent): its metadata is known.
            AddTorrentResponse::AlreadyManaged(_, handle)
            | AddTorrentResponse::Added(_, handle) => {
                let bytes = handle
                    .with_metadata(|m| m.torrent_bytes.clone())
                    .map_err(|e| DownloadError::Torrent(format!("{e:#}")))?;
                Ok((bytes, magnet.peers))
            }
        }
    }

    async fn run(self, req: TorrentRequest, ctx: JobContext) -> Result<Outcome, DownloadError> {
        let mut progress = Progress::starting();
        progress.resumable = true;
        progress.torrent = Some(TorrentProgress {
            uploaded: req.uploaded,
            fetching_metadata: true,
            ..Default::default()
        });
        let _ = ctx.progress.send(progress.clone());

        let (meta, bytes, mut peers) = tokio::select! {
            r = self.resolve(&req.source) => r?,
            () = ctx.stopped() => return Ok(stopped(&ctx, &progress, None)),
        };
        if let Ok(m) = Magnet::parse(&req.source.link) {
            peers.extend(m.peers);
        }
        let output = meta.output_dir(&req.dir);
        let size = meta.selected_size(req.only_files.as_deref());
        progress.file_name = Some(display_name(&meta, &req.dir));
        progress.path = Some(meta.output_path(&req.dir));
        progress.total = Some(size);
        if let Some(t) = progress.torrent.as_mut() {
            t.fetching_metadata = false;
        }
        let _ = ctx.progress.send(progress.clone());

        // The same torrent may still be seeding from before (downloading it
        // again): take it over rather than run it twice.
        self.end_seeding(&meta.info_hash).await;
        let session = self.session().await?;
        let added = session
            .add_torrent(
                AddTorrent::from_bytes(bytes),
                Some(AddTorrentOptions {
                    output_folder: Some(output.to_string_lossy().into_owned()),
                    only_files: req.only_files.clone(),
                    // The files are this torrent's: continue from what is on
                    // disk after a pause or a restart.
                    overwrite: true,
                    initial_peers: (!peers.is_empty()).then_some(peers),
                    ..Default::default()
                }),
            )
            .await
            .map_err(|e| DownloadError::Torrent(format!("the torrent could not start: {e:#}")))?;
        let handle = match added {
            AddTorrentResponse::Added(_, h) | AddTorrentResponse::AlreadyManaged(_, h) => h,
            AddTorrentResponse::ListOnly(_) => {
                return Err(DownloadError::Torrent("the torrent did not start".into()));
            }
        };
        if let Some(only) = &req.only_files {
            // A torrent that was still in the session keeps its old choice
            // otherwise.
            let set = only.iter().copied().collect();
            let _ = session.update_only_files(&handle, &set).await;
        }

        let mut tick = tokio::time::interval(TICK);
        loop {
            tokio::select! {
                _ = tick.tick() => {}
                () = ctx.stopped() => {
                    let cancel = ctx.is_cancel();
                    let _ = session.delete(TorrentIdOrHash::Id(handle.id()), cancel).await;
                    if cancel {
                        remove_empty_dirs(&output, &req.dir).await;
                    }
                    return Ok(stopped(&ctx, &progress, Some(&handle)));
                }
            }
            let stats = handle.stats();
            if let Some(error) = &stats.error {
                let _ = session
                    .delete(TorrentIdOrHash::Id(handle.id()), false)
                    .await;
                return Err(DownloadError::Torrent(error_message(error)));
            }
            publish(&ctx, &mut progress, &stats, req.uploaded);
            if stats.finished && matches!(stats.state, TorrentStatsState::Live) {
                break;
            }
        }

        let path = meta.output_path(&req.dir);
        progress.phase = Phase::Done;
        progress.speed = 0;
        progress.downloaded = size;
        let _ = ctx.progress.send(progress.clone());
        let uploaded = req.uploaded + handle.stats().uploaded_bytes;
        if req.seed.reached(uploaded, size, Duration::ZERO) {
            let _ = session
                .delete(TorrentIdOrHash::Id(handle.id()), false)
                .await;
        } else {
            self.seed(
                session,
                handle,
                meta.info_hash.clone(),
                size,
                req.uploaded,
                req.seed,
            );
        }
        Ok(Outcome::Completed {
            path,
            bytes: size,
            verified: None,
        })
    }

    /// Keep a finished torrent uploading until its policy says stop.
    fn seed(
        &self,
        session: Arc<Session>,
        handle: Arc<ManagedTorrent>,
        info_hash: String,
        size: u64,
        uploaded_before: u64,
        policy: SeedPolicy,
    ) {
        let started = Instant::now();
        let first = SeedStatus {
            seeding: true,
            uploaded: uploaded_before + handle.stats().uploaded_bytes,
            ..Default::default()
        };
        let (tx, rx) = watch::channel(first);
        let stop = CancellationToken::new();
        self.inner.seeders.lock().insert(
            info_hash.clone(),
            Seeder {
                status: rx,
                stop: stop.clone(),
            },
        );
        let inner = self.inner.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(1));
            let mut status = tx.borrow().clone();
            loop {
                tokio::select! {
                    _ = tick.tick() => {}
                    () = stop.cancelled() => break,
                }
                let stats = handle.stats();
                if stats.error.is_some() {
                    break;
                }
                let live = stats.live.as_ref();
                status.uploaded = uploaded_before + stats.uploaded_bytes;
                status.upload_speed = live.map_or(0, |l| l.upload_speed.as_bytes());
                status.peers = live.map_or(0, |l| l.snapshot.peer_stats.live);
                status.seconds = started.elapsed().as_secs();
                let _ = tx.send(status.clone());
                if policy.reached(status.uploaded, size, started.elapsed()) {
                    break;
                }
            }
            let _ = session
                .delete(TorrentIdOrHash::Id(handle.id()), false)
                .await;
            inner.seeders.lock().remove(&info_hash);
            status.seeding = false;
            status.upload_speed = 0;
            status.peers = 0;
            let _ = tx.send(status);
        });
    }

    /// Stop a torrent's seeding and wait until it left the session.
    async fn end_seeding(&self, info_hash: &str) {
        let Some(mut rx) = self.seeding(info_hash) else {
            return;
        };
        self.stop_seeding(info_hash);
        let _ = tokio::time::timeout(Duration::from_secs(10), rx.wait_for(|s| !s.seeding)).await;
    }
}

fn publish(ctx: &JobContext, progress: &mut Progress, stats: &TorrentStats, uploaded_before: u64) {
    let live = stats.live.as_ref();
    if let TorrentStatsState::Initializing { .. } = stats.state {
        // Checking the pieces already on disk: librqbit counts checked
        // bytes then, not bytes it has, so the last known amount stays.
        progress.phase = Phase::Starting;
    } else {
        progress.phase = Phase::Downloading;
        progress.downloaded = stats.progress_bytes;
        progress.total = Some(stats.total_bytes);
    }
    progress.speed = live.map_or(0, |l| l.download_speed.as_bytes());
    let peers = live.map_or(0, |l| l.snapshot.peer_stats.live);
    progress.connections = peers as usize;
    progress.target_connections = 0;
    progress.torrent = Some(TorrentProgress {
        peers,
        known_peers: live.map_or(0, |l| l.snapshot.peer_stats.seen),
        upload_speed: live.map_or(0, |l| l.upload_speed.as_bytes()),
        uploaded: uploaded_before + stats.uploaded_bytes,
        fetching_metadata: false,
    });
    let _ = ctx.progress.send(progress.clone());
}

fn stopped(ctx: &JobContext, progress: &Progress, handle: Option<&Arc<ManagedTorrent>>) -> Outcome {
    if ctx.is_cancel() {
        return Outcome::Canceled;
    }
    // While checking, librqbit's count is of checked bytes, not data.
    let downloaded = handle
        .map(|h| h.stats())
        .filter(|s| !matches!(s.state, TorrentStatsState::Initializing { .. }))
        .map_or(progress.downloaded, |s| s.progress_bytes);
    Outcome::Paused {
        path: progress.path.clone(),
        downloaded: downloaded.max(progress.downloaded),
        total: progress.total,
        resumable: true,
    }
}

/// The list's name for a torrent: its folder, or its one file.
fn display_name(meta: &Metadata, dir: &Path) -> String {
    meta.output_path(dir)
        .file_name()
        .map_or_else(|| meta.name.clone(), |n| n.to_string_lossy().into_owned())
}

fn no_peers(err: &anyhow::Error) -> DownloadError {
    let text = format!("{err:#}");
    if text.contains("no known way to resolve peers") {
        return DownloadError::Torrent(
            "there is no way to find peers for this magnet link: it names no tracker or peer, and DHT is off in Settings"
                .into(),
        );
    }
    DownloadError::Torrent(format!(
        "the torrent's file list could not be fetched: {text}"
    ))
}

fn error_message(error: &str) -> String {
    let error = error.trim();
    if error.to_ascii_lowercase().contains("no space") {
        return "the disk is full".into();
    }
    format!("the torrent stopped: {error}")
}

/// Fetch a `.torrent` file over http(s), refusing anything too large to be
/// one.
async fn fetch(source: &Source) -> Result<bytes::Bytes, DownloadError> {
    let client = source
        .client
        .clone()
        .ok_or_else(|| DownloadError::Torrent("no way to fetch the .torrent file".into()))?;
    let mut request = client.get(&source.link);
    for (name, value) in &source.headers {
        request = request.header(name, value);
    }
    let response = request.send().await.map_err(|e| {
        DownloadError::Network(format!("the .torrent file could not be fetched: {e}"))
    })?;
    let status = response.status().as_u16();
    if !response.status().is_success() {
        return Err(match status {
            404 | 410 => DownloadError::NotFound { status },
            403 => DownloadError::Forbidden,
            _ => DownloadError::HttpStatus { status },
        });
    }
    if response
        .content_length()
        .is_some_and(|n| n > MAX_TORRENT_FILE as u64)
    {
        return Err(meta::invalid("the file is too large to be a torrent"));
    }
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| DownloadError::Network(e.to_string()))?;
        if body.len() + chunk.len() > MAX_TORRENT_FILE {
            return Err(meta::invalid("the file is too large to be a torrent"));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body.into())
}

async fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), DownloadError> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let tmp = path.with_extension("torrent.tmp");
    tokio::fs::write(&tmp, bytes).await?;
    tokio::fs::rename(&tmp, path).await?;
    Ok(())
}

/// After a cancel: the torrent's own folder, and folders inside it, once
/// empty. Never `dir` itself or anything above it.
async fn remove_empty_dirs(output: &Path, dir: &Path) {
    if output == dir {
        return;
    }
    let output = output.to_path_buf();
    let _ = tokio::task::spawn_blocking(move || prune(&output)).await;
}

fn prune(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    let mut empty = true;
    for entry in entries.flatten() {
        let path = entry.path();
        let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());
        if !(is_dir && prune(&path)) {
            empty = false;
        }
    }
    empty && std::fs::remove_dir(dir).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_policy_limits() {
        let ratio = SeedPolicy {
            enabled: true,
            ratio: Some(1.0),
            time: Some(Duration::from_secs(60)),
        };
        assert!(!ratio.reached(99, 100, Duration::from_secs(1)));
        assert!(ratio.reached(100, 100, Duration::from_secs(1)));
        assert!(ratio.reached(0, 100, Duration::from_secs(60)));
        assert!(SeedPolicy::NONE.reached(0, 100, Duration::ZERO));
        let forever = SeedPolicy {
            enabled: true,
            ratio: None,
            time: None,
        };
        assert!(!forever.reached(u64::MAX, 1, Duration::MAX));
    }

    #[test]
    fn prunes_only_empty_folders() {
        let root = tempfile::tempdir().unwrap();
        let out = root.path().join("t");
        std::fs::create_dir_all(out.join("a/b")).unwrap();
        assert!(prune(&out));
        assert!(!out.exists());
        std::fs::create_dir_all(out.join("a")).unwrap();
        std::fs::write(out.join("keep.txt"), b"x").unwrap();
        assert!(!prune(&out));
        assert!(out.join("keep.txt").exists());
        assert!(!out.join("a").exists());
    }
}
