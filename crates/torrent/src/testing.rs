//! A seeding peer on loopback for tests: a torrent made from files in a
//! temporary folder, served by a librqbit session of its own with no DHT,
//! no trackers and no local discovery, so tests never touch the internet.

use std::net::{Ipv4Addr, SocketAddr};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use librqbit::spawn_utils::BlockingSpawner;
use librqbit::{
    AddTorrent, AddTorrentOptions, CreateTorrentOptions, ListenerMode, ListenerOptions, Session,
    SessionOptions, create_torrent,
};

/// Deterministic test content: byte `i` of a file is a function of `i` and
/// `seed`, so a wrong offset shows.
pub fn content(len: usize, seed: u8) -> Vec<u8> {
    (0..len)
        .map(|i| (i as u32).wrapping_mul(31).wrapping_add(u32::from(seed)) as u8)
        .collect()
}

/// A peer that has every piece of one torrent.
pub struct LocalSeeder {
    session: Arc<Session>,
    pub addr: SocketAddr,
    pub torrent: Vec<u8>,
    pub info_hash: String,
    /// The folder the torrent was made from (its name is the torrent's).
    pub root: PathBuf,
    _dir: tempfile::TempDir,
}

impl std::fmt::Debug for LocalSeeder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalSeeder")
            .field("addr", &self.addr)
            .field("info_hash", &self.info_hash)
            .finish_non_exhaustive()
    }
}

impl LocalSeeder {
    /// Make a torrent named `name` from `files` (`/`-separated paths and
    /// their bytes) and seed it, uploading at most `upload_limit` bytes per
    /// second (0 for no limit) so tests can catch it half way.
    pub async fn start(name: &str, files: &[(&str, Vec<u8>)], upload_limit: u32) -> Self {
        let dir = tempfile::tempdir().expect("a temporary folder");
        let root = dir.path().join(name);
        for (path, bytes) in files {
            let path = root.join(crate::local_path(path));
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("a folder");
            std::fs::write(&path, bytes).expect("a test file");
        }
        // One file: the torrent is the file itself, not a folder.
        let source = if files.len() == 1 {
            root.join(crate::local_path(files[0].0))
        } else {
            root.clone()
        };
        let made = create_torrent(
            &source,
            CreateTorrentOptions {
                piece_length: Some(16 * 1024),
                ..Default::default()
            },
            &BlockingSpawner::new(1),
        )
        .await
        .expect("a torrent");
        let torrent = made.as_bytes().expect("torrent bytes").to_vec();
        let info_hash = made.info_hash().as_string();
        let session = Session::new_with_opts(
            dir.path().join("seeder-state"),
            SessionOptions {
                dht: None,
                disable_trackers: true,
                disable_local_service_discovery: true,
                ipv4_only: true,
                listen: Some(ListenerOptions {
                    mode: ListenerMode::TcpOnly,
                    listen_addr: (Ipv4Addr::LOCALHOST, 0).into(),
                    ipv4_only: true,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .await
        .expect("a seeding session");
        session
            .ratelimits
            .set_upload_bps(NonZeroU32::new(upload_limit));
        let output = if files.len() == 1 {
            source.parent().expect("a parent").to_path_buf()
        } else {
            root.clone()
        };
        let handle = session
            .add_torrent(
                AddTorrent::from_bytes(torrent.clone()),
                Some(AddTorrentOptions {
                    overwrite: true,
                    output_folder: Some(output.to_string_lossy().into_owned()),
                    ..Default::default()
                }),
            )
            .await
            .expect("the torrent added")
            .into_handle()
            .expect("a handle");
        handle
            .wait_until_initialized()
            .await
            .expect("the files checked");
        assert!(handle.stats().finished, "the seeder must have every piece");
        let addr = session.listen_addr().expect("a listening address");
        let addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), addr.port());
        Self {
            session,
            addr,
            torrent,
            info_hash,
            root,
            _dir: dir,
        }
    }

    /// A magnet link that names this seeder as its only peer.
    pub fn magnet(&self) -> String {
        format!(
            "magnet:?xt=urn:btih:{}&dn=test&x.pe={}",
            self.info_hash, self.addr
        )
    }

    /// The `.torrent` file, written into `dir`.
    pub fn write_torrent(&self, dir: &Path) -> PathBuf {
        let path = dir.join("test.torrent");
        std::fs::write(&path, &self.torrent).expect("the .torrent written");
        path
    }

    pub async fn stop(self) {
        self.session.stop().await;
    }
}
