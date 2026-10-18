//! The one librqbit session every torrent of a Downpipe runs in.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;

use downpipe_http::DownloadError;
use librqbit::dht::DhtPersistenceConfig;
use librqbit::{
    ConnectionOptions, DhtSessionConfig, ListenerMode, ListenerOptions, Session, SessionOptions,
};
use parking_lot::Mutex;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

/// How the session talks to the network. Changing any of these needs a new
/// session; see [`Torrents::configure`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetConfig {
    /// TCP port for incoming peers (and the DHT's UDP port); 0 picks a free
    /// one each time the session starts.
    pub listen_port: u16,
    /// Address to listen on; unspecified listens everywhere. Tests and
    /// side-by-side copies listen on loopback only.
    pub bind: IpAddr,
    /// Find peers in the distributed hash table (BEP 5).
    pub dht: bool,
    /// Find peers on the local network (BEP 14).
    pub lsd: bool,
    /// Ask the router to forward the listening port (UPnP).
    pub upnp: bool,
    /// `socks5://` proxy for connections to peers. Other proxies cannot carry
    /// peer connections and are not used for them.
    pub proxy: Option<String>,
    /// Where the DHT remembers nodes between runs.
    pub state_dir: PathBuf,
}

impl Default for NetConfig {
    fn default() -> Self {
        Self {
            listen_port: 0,
            bind: IpAddr::V6(Ipv6Addr::UNSPECIFIED),
            dht: true,
            lsd: true,
            upnp: false,
            proxy: None,
            state_dir: PathBuf::new(),
        }
    }
}

/// Speed limits in bytes per second, 0 for none. Applied at once.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Limits {
    pub download: u64,
    pub upload: u64,
}

/// What one seeding torrent reports.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SeedStatus {
    /// Still uploading; false once a limit was reached or it was stopped.
    pub seeding: bool,
    pub uploaded: u64,
    pub upload_speed: u64,
    pub peers: u32,
    /// Seconds spent seeding so far.
    pub seconds: u64,
}

/// BitTorrent for one Downpipe: a lazily started librqbit session, the
/// limits, and the torrents that seed after finishing. Cheap to clone.
#[derive(Debug, Clone)]
pub struct Torrents {
    pub(crate) inner: Arc<Inner>,
}

pub(crate) struct Inner {
    pub(crate) config: Mutex<(NetConfig, Limits)>,
    /// The running session and the configuration it was started with.
    pub(crate) session: tokio::sync::Mutex<Option<(NetConfig, Arc<Session>)>>,
    /// Seeding torrents by info hash.
    pub(crate) seeders: Mutex<HashMap<String, Seeder>>,
}

impl std::fmt::Debug for Inner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Torrents")
            .field("config", &*self.config.lock())
            .field("seeding", &self.seeders.lock().len())
            .finish_non_exhaustive()
    }
}

pub(crate) struct Seeder {
    pub(crate) status: watch::Receiver<SeedStatus>,
    pub(crate) stop: CancellationToken,
}

impl Torrents {
    pub fn new(net: NetConfig, limits: Limits) -> Self {
        Self {
            inner: Arc::new(Inner {
                config: Mutex::new((net, limits)),
                session: tokio::sync::Mutex::new(None),
                seeders: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// New settings. Limits apply at once. Network settings apply when the
    /// session next starts: right away when no torrent runs, otherwise once
    /// the running ones have stopped (a second session could not listen on
    /// the same port).
    pub fn configure(&self, net: NetConfig, limits: Limits) {
        *self.inner.config.lock() = (net, limits);
        if let Ok(slot) = self.inner.session.try_lock()
            && let Some((_, session)) = slot.as_ref()
        {
            apply_limits(session, limits);
        }
    }

    /// The session to add torrents to, started (or restarted with new
    /// network settings when idle) as needed.
    pub(crate) async fn session(&self) -> Result<Arc<Session>, DownloadError> {
        let (net, limits) = self.inner.config.lock().clone();
        let mut slot = self.inner.session.lock().await;
        if let Some((running, session)) = slot.as_ref() {
            let idle = session.with_torrents(|it| it.count()) == 0;
            if *running == net || !idle {
                apply_limits(session, limits);
                return Ok(session.clone());
            }
            session.stop().await;
            *slot = None;
        }
        let session = start_session(&net).await?;
        // The limits may have changed while it started.
        apply_limits(&session, self.inner.config.lock().1);
        *slot = Some((net, session.clone()));
        Ok(session)
    }

    /// The seeding status of a torrent that finished, while Downpipe still
    /// knows it (until it stops seeding).
    pub fn seeding(&self, info_hash: &str) -> Option<watch::Receiver<SeedStatus>> {
        self.inner
            .seeders
            .lock()
            .get(info_hash)
            .map(|s| s.status.clone())
    }

    /// Stop uploading a finished torrent. Its files stay.
    pub fn stop_seeding(&self, info_hash: &str) {
        if let Some(seeder) = self.inner.seeders.lock().get(info_hash) {
            seeder.stop.cancel();
        }
    }

    /// Stop every torrent, for quitting. Seeding does not continue after a
    /// restart.
    pub async fn shutdown(&self) {
        let stops: Vec<CancellationToken> = self
            .inner
            .seeders
            .lock()
            .values()
            .map(|s| s.stop.clone())
            .collect();
        for stop in stops {
            stop.cancel();
        }
        if let Some((_, session)) = self.inner.session.lock().await.take() {
            session.stop().await;
        }
    }

    /// The port the session listens on, once it runs.
    pub async fn listen_addr(&self) -> Option<SocketAddr> {
        self.inner
            .session
            .lock()
            .await
            .as_ref()
            .and_then(|(_, s)| s.listen_addr())
    }
}

fn apply_limits(session: &Session, limits: Limits) {
    session.ratelimits.set_download_bps(bps(limits.download));
    session.ratelimits.set_upload_bps(bps(limits.upload));
}

fn bps(value: u64) -> Option<NonZeroU32> {
    NonZeroU32::new(u32::try_from(value).unwrap_or(u32::MAX))
}

async fn start_session(net: &NetConfig) -> Result<Arc<Session>, DownloadError> {
    let ipv4_only = net.bind.is_ipv4();
    let dht = net.dht.then(|| DhtSessionConfig {
        port: (net.listen_port != 0).then_some(net.listen_port),
        persistence: (!net.state_dir.as_os_str().is_empty()).then(|| DhtPersistenceConfig {
            config_filename: Some(net.state_dir.join("dht.json")),
            ..Default::default()
        }),
        ..Default::default()
    });
    let socks = net
        .proxy
        .as_deref()
        .filter(|p| p.starts_with("socks5://") || p.starts_with("socks5h://"))
        .map(|p| p.replacen("socks5h://", "socks5://", 1));
    let options = SessionOptions {
        dht,
        listen: Some(ListenerOptions {
            mode: ListenerMode::TcpOnly,
            listen_addr: SocketAddr::new(net.bind, net.listen_port),
            enable_upnp_port_forwarding: net.upnp,
            ipv4_only,
            ..Default::default()
        }),
        connect: Some(ConnectionOptions {
            proxy_url: socks,
            ..Default::default()
        }),
        disable_local_service_discovery: !net.lsd,
        ipv4_only,
        // Downpipe keeps its own list; librqbit must not bring torrents back
        // by itself.
        persistence: None,
        fastresume: false,
        ..Default::default()
    };
    let folder = if net.state_dir.as_os_str().is_empty() {
        std::env::temp_dir()
    } else {
        net.state_dir.clone()
    };
    Session::new_with_opts(folder, options).await.map_err(|e| {
        DownloadError::Torrent(if net.listen_port != 0 {
            format!(
                "BitTorrent could not start on port {}; another program may use it ({e:#})",
                net.listen_port
            )
        } else {
            format!("BitTorrent could not start ({e:#})")
        })
    })
}
