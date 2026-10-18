//! BitTorrent downloads for the manager, when built with the `torrent`
//! feature (the desktop app is). Without it the same functions exist and
//! refuse torrent links, so the rest of the core does not care.

use std::path::{Path, PathBuf};

use downpipe_http::DownloadError;

use crate::settings::Settings;

/// This build downloads torrents.
pub const AVAILABLE: bool = cfg!(feature = "torrent");

/// Largest `.torrent` file accepted; real ones are far smaller.
pub const MAX_TORRENT_FILE: usize = 16 * 1024 * 1024;

/// Folder in the data folder that keeps each torrent's `.torrent` file (and
/// the DHT's node list).
const TORRENTS_DIR: &str = "torrents";

/// A magnet link, or an http(s) link whose path ends in `.torrent`.
pub fn is_torrent_link(link: &str) -> bool {
    let link = link.trim();
    if link
        .get(..7)
        .is_some_and(|s| s.eq_ignore_ascii_case("magnet:"))
    {
        return true;
    }
    url::Url::parse(link).is_ok_and(|u| {
        matches!(u.scheme(), "http" | "https")
            && u.path().to_ascii_lowercase().ends_with(".torrent")
    })
}

/// A file name the browser gave for a `.torrent` download.
pub(crate) fn is_torrent_name(name: &str) -> bool {
    name.trim().to_ascii_lowercase().ends_with(".torrent")
}

/// What is known about a torrent when it is added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Parsed {
    pub info_hash: String,
    pub name: Option<String>,
    /// How the list stores it: a canonical magnet link.
    pub link: String,
    /// From a `.torrent` file: the number of files, its size and whether
    /// it is private.
    pub files: usize,
    pub total: Option<u64>,
    pub private: bool,
}

/// What a running or seeding torrent reports to the list.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SeedState {
    pub seeding: bool,
    pub uploaded: u64,
    pub upload_speed: u64,
    pub peers: u32,
}

/// One torrent to start.
#[cfg_attr(not(feature = "torrent"), allow(dead_code))]
#[derive(Debug, Clone)]
pub(crate) struct Request {
    pub id: String,
    /// The magnet link, or the http(s) link of the `.torrent`, as given.
    pub link: String,
    pub headers: Vec<(String, String)>,
    pub client: reqwest::Client,
    pub dir: PathBuf,
    pub only_files: Option<Vec<usize>>,
    pub uploaded: u64,
}

fn data_dir(data: &Path) -> PathBuf {
    data.join(TORRENTS_DIR)
}

#[cfg(feature = "torrent")]
mod imp {
    use std::net::IpAddr;
    use std::time::Duration;

    use downpipe_http::DownloadHandle;
    use downpipe_torrent::{
        Limits, Magnet, Metadata, NetConfig, SeedPolicy, Source, TorrentRequest, Torrents,
    };

    use super::*;
    use crate::task::{TorrentChoices, TorrentFileChoice};

    /// Address torrents listen on instead of every interface: tests and a
    /// second copy run beside the user's (see `DOWNPIPE_DATA_DIR`) set it to
    /// `127.0.0.1`.
    const BIND_ENV: &str = "DOWNPIPE_TORRENT_BIND";

    #[derive(Debug, Clone)]
    pub(crate) struct Service {
        torrents: Torrents,
        dir: PathBuf,
    }

    impl Service {
        pub fn new(data: &Path, settings: &Settings) -> Self {
            let dir = data_dir(data);
            let (net, limits) = config(&dir, settings);
            Self {
                torrents: Torrents::new(net, limits),
                dir,
            }
        }

        pub fn configure(&self, settings: &Settings) {
            let (net, limits) = config(&self.dir, settings);
            self.torrents.configure(net, limits);
        }

        pub fn metainfo(&self, id: &str) -> PathBuf {
            self.dir.join(format!("{id}.torrent"))
        }

        pub fn start(&self, req: Request, settings: &Settings) -> DownloadHandle {
            let request = TorrentRequest {
                source: self.source(&req),
                dir: req.dir,
                only_files: req.only_files,
                seed: policy(settings),
                uploaded: req.uploaded,
            };
            self.torrents.start(request)
        }

        /// The torrent's files, fetched or asked from peers if need be.
        pub async fn choices(
            &self,
            req: &Request,
        ) -> Result<(TorrentChoices, Parsed), DownloadError> {
            let meta = self.torrents.metadata(&self.source(req)).await?;
            let parsed = parsed_from(&meta);
            let choices = TorrentChoices {
                info_hash: meta.info_hash.clone(),
                name: meta.name.clone(),
                files: meta
                    .files
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| !f.padding)
                    .map(|(index, f)| TorrentFileChoice {
                        index,
                        path: f.path.clone(),
                        size: f.size,
                    })
                    .collect(),
                total: meta.total,
                selection: req.only_files.clone(),
                folder: meta.is_folder(),
                private: meta.private,
            };
            Ok((choices, parsed))
        }

        /// Keep the `.torrent` file a download was added from.
        pub fn keep_file(&self, id: &str, bytes: &[u8]) -> std::io::Result<()> {
            std::fs::create_dir_all(&self.dir)?;
            std::fs::write(self.metainfo(id), bytes)
        }

        /// Forget a download's `.torrent` file.
        pub fn forget(&self, id: &str) {
            let _ = std::fs::remove_file(self.metainfo(id));
        }

        /// Delete what a torrent wrote into `dir` (only its own files), for
        /// removing it with its files while it does not run.
        pub fn delete_files(&self, id: &str, dir: &Path, only: Option<&[usize]>) {
            let Ok(bytes) = std::fs::read(self.metainfo(id)) else {
                return;
            };
            let Ok(meta) = Metadata::parse(&bytes) else {
                return;
            };
            let out = meta.output_dir(dir);
            for (i, file) in meta.files.iter().enumerate() {
                let path = out.join(downpipe_torrent::local_path(&file.path));
                // librqbit creates files it was not asked for, empty, when a
                // piece it needs ends there; those go too, other ones stay.
                let chosen = only.is_none_or(|o| o.contains(&i)) || file.padding;
                if chosen || std::fs::metadata(&path).is_ok_and(|m| m.len() == 0) {
                    let _ = std::fs::remove_file(path);
                }
            }
            if out != dir {
                remove_empty(&out);
            }
        }

        /// Follow a finished torrent's seeding: `update` gets every change,
        /// the last one with `seeding` false. Nothing happens when it does
        /// not seed.
        pub fn watch_seeding(
            &self,
            info_hash: &str,
            update: impl Fn(SeedState) + Send + 'static,
        ) -> bool {
            let Some(mut rx) = self.torrents.seeding(info_hash) else {
                return false;
            };
            tokio::spawn(async move {
                loop {
                    let s = rx.borrow_and_update().clone();
                    let done = !s.seeding;
                    update(SeedState {
                        seeding: s.seeding,
                        uploaded: s.uploaded,
                        upload_speed: s.upload_speed,
                        peers: s.peers,
                    });
                    if done || rx.changed().await.is_err() {
                        break;
                    }
                }
            });
            true
        }

        pub fn stop_seeding(&self, info_hash: &str) {
            self.torrents.stop_seeding(info_hash);
        }

        pub async fn shutdown(&self) {
            self.torrents.shutdown().await;
        }

        fn source(&self, req: &Request) -> Source {
            Source {
                link: req.link.clone(),
                headers: req.headers.clone(),
                client: Some(req.client.clone()),
                metainfo: self.metainfo(&req.id),
            }
        }
    }

    pub(crate) fn parse_magnet(link: &str) -> Result<Parsed, DownloadError> {
        let magnet = Magnet::parse(link).map_err(|e| DownloadError::InvalidUrl(e.to_string()))?;
        Ok(Parsed {
            link: magnet.to_link(),
            info_hash: magnet.info_hash,
            name: magnet.name,
            files: 0,
            total: None,
            private: false,
        })
    }

    pub(crate) fn parse_file(bytes: &[u8]) -> Result<Parsed, DownloadError> {
        let meta = Metadata::parse(bytes)?;
        Ok(parsed_from(&meta))
    }

    fn parsed_from(meta: &Metadata) -> Parsed {
        // The trackers stay in the `.torrent` file, which is what starts it.
        let magnet = Magnet {
            info_hash: meta.info_hash.clone(),
            name: Some(meta.name.clone()),
            trackers: Vec::new(),
            peers: Vec::new(),
        };
        Parsed {
            link: magnet.to_link(),
            info_hash: meta.info_hash.clone(),
            name: Some(meta.name.clone()),
            files: meta.files.iter().filter(|f| !f.padding).count(),
            total: Some(meta.total),
            private: meta.private,
        }
    }

    fn config(dir: &Path, settings: &Settings) -> (NetConfig, Limits) {
        let t = &settings.torrent;
        let bind = std::env::var(BIND_ENV)
            .ok()
            .and_then(|v| v.trim().parse::<IpAddr>().ok())
            .unwrap_or(NetConfig::default().bind);
        let net = NetConfig {
            listen_port: t.listen_port,
            bind,
            dht: t.dht,
            lsd: t.lsd,
            upnp: t.upnp,
            proxy: (!settings.proxy.is_empty()).then(|| settings.proxy.clone()),
            state_dir: dir.to_path_buf(),
        };
        let limits = Limits {
            download: settings.speed_limit,
            upload: t.upload_limit,
        };
        (net, limits)
    }

    fn policy(settings: &Settings) -> SeedPolicy {
        let t = &settings.torrent;
        SeedPolicy {
            enabled: t.seed,
            ratio: (t.seed_ratio_percent > 0).then(|| f64::from(t.seed_ratio_percent) / 100.0),
            time: (t.seed_minutes > 0).then(|| Duration::from_secs(u64::from(t.seed_minutes) * 60)),
        }
    }

    fn remove_empty(dir: &Path) -> bool {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return false;
        };
        let mut empty = true;
        for entry in entries.flatten() {
            let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());
            if !(is_dir && remove_empty(&entry.path())) {
                empty = false;
            }
        }
        empty && std::fs::remove_dir(dir).is_ok()
    }
}

#[cfg(not(feature = "torrent"))]
mod imp {
    use downpipe_http::DownloadHandle;

    use super::*;
    use crate::task::TorrentChoices;

    fn unsupported() -> DownloadError {
        DownloadError::Torrent("this build of Downpipe has no BitTorrent support".into())
    }

    #[derive(Debug, Clone)]
    pub(crate) struct Service {
        dir: PathBuf,
    }

    impl Service {
        pub fn new(data: &Path, _settings: &Settings) -> Self {
            Self {
                dir: data_dir(data),
            }
        }

        pub fn configure(&self, _settings: &Settings) {}

        pub fn metainfo(&self, id: &str) -> PathBuf {
            self.dir.join(format!("{id}.torrent"))
        }

        pub fn start(&self, _req: Request, _settings: &Settings) -> DownloadHandle {
            DownloadHandle::spawn(|_| async { Err(unsupported()) })
        }

        pub async fn choices(
            &self,
            _req: &Request,
        ) -> Result<(TorrentChoices, Parsed), DownloadError> {
            Err(unsupported())
        }

        pub fn keep_file(&self, _id: &str, _bytes: &[u8]) -> std::io::Result<()> {
            Err(std::io::Error::other(unsupported()))
        }

        pub fn forget(&self, id: &str) {
            let _ = std::fs::remove_file(self.metainfo(id));
        }

        pub fn delete_files(&self, _id: &str, _dir: &Path, _only: Option<&[usize]>) {}

        pub fn watch_seeding(
            &self,
            _info_hash: &str,
            _update: impl Fn(SeedState) + Send + 'static,
        ) -> bool {
            false
        }

        pub fn stop_seeding(&self, _info_hash: &str) {}

        pub async fn shutdown(&self) {}
    }

    pub(crate) fn parse_magnet(_link: &str) -> Result<Parsed, DownloadError> {
        Err(unsupported())
    }

    pub(crate) fn parse_file(_bytes: &[u8]) -> Result<Parsed, DownloadError> {
        Err(unsupported())
    }
}

pub(crate) use imp::{Service, parse_file, parse_magnet};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_torrent_links() {
        assert!(is_torrent_link("magnet:?xt=urn:btih:abc"));
        assert!(is_torrent_link(" MAGNET:?xt"));
        assert!(is_torrent_link(
            "https://example.org/files/ubuntu.iso.torrent?x=1"
        ));
        assert!(is_torrent_link("http://example.org/A.TORRENT"));
        assert!(!is_torrent_link("https://example.org/a.torrent.zip"));
        assert!(!is_torrent_link("https://example.org/?f=a.torrent"));
        assert!(!is_torrent_link("ftp://example.org/a.torrent"));
        assert!(is_torrent_name("Ubuntu.torrent"));
        assert!(!is_torrent_name("notes.txt"));
    }
}
