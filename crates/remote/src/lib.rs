//! Downpipe's optional remote control: a small HTTP API over the
//! [`Manager`] and the web page that uses it, for a phone on the same
//! network or a browser on the same computer.
//!
//! Off by default. Every API request needs the access token
//! (`Authorization: Bearer`), which the desktop app makes with
//! [`token::generate`], shows once and stores only as a hash. Peers that
//! guess wrong too often are locked out for a while; a network-facing
//! listener answers only peers on local networks; other sites' pages cannot
//! call the API (no CORS, `Origin` checked); bodies are capped. The API can
//! list, add, pause, resume and remove downloads and read harmless settings,
//! and nothing else: no paths, no file browsing, no commands. See
//! `docs/SECURITY_MODEL.md`.

mod api;
mod assets;
mod folder;
mod limiter;
mod peer;
pub mod token;

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use downpipe_core::{Manager, RemoteSettings};
use parking_lot::Mutex;
use tokio::net::TcpListener;
use tokio::sync::{Semaphore, oneshot};
use tokio::task::JoinHandle;

pub use assets::{Asset, Assets, DirAssets, NoAssets, PAGE, is_servable, mime_for};
pub use peer::{is_local_network, lan_address};

/// What the server needs from the settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub bind: IpAddr,
    pub port: u16,
    pub token_hash: [u8; 32],
}

impl Config {
    /// `None` while the remote control is off or has no token.
    pub fn from_settings(remote: &RemoteSettings) -> Option<Self> {
        if !remote.enabled {
            return None;
        }
        Some(Self {
            bind: remote.bind.parse().ok()?,
            port: remote.port,
            token_hash: token::parse_hash(&remote.token_hash)?,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StartError {
    #[error("port {0} is already in use")]
    PortInUse(u16),
    #[error("cannot listen on {addr}: {source}")]
    Bind {
        addr: SocketAddr,
        source: std::io::Error,
    },
}

impl StartError {
    /// Stable name the settings page branches on.
    pub fn code(&self) -> &'static str {
        match self {
            Self::PortInUse(_) => "port_in_use",
            Self::Bind { .. } => "bind_failed",
        }
    }
}

/// A running remote control. Dropping it stops the server too.
#[derive(Debug)]
pub struct RemoteServer {
    addr: SocketAddr,
    stop: Mutex<Option<oneshot::Sender<()>>>,
    task: Option<JoinHandle<()>>,
}

impl RemoteServer {
    /// Listen on `config.bind:config.port` and serve until stopped.
    pub async fn start(
        manager: Manager,
        config: Config,
        assets: Arc<dyn Assets>,
    ) -> Result<Self, StartError> {
        let wanted = SocketAddr::new(config.bind, config.port);
        let listener = TcpListener::bind(wanted).await.map_err(|source| {
            if source.kind() == std::io::ErrorKind::AddrInUse {
                StartError::PortInUse(config.port)
            } else {
                StartError::Bind {
                    addr: wanted,
                    source,
                }
            }
        })?;
        let addr = listener.local_addr().map_err(|source| StartError::Bind {
            addr: wanted,
            source,
        })?;
        let state = Arc::new(api::AppState {
            manager,
            token_hash: config.token_hash,
            limiter: Mutex::default(),
            assets,
            network: !config.bind.is_loopback(),
            in_flight: Semaphore::new(api::MAX_IN_FLIGHT),
        });
        let app = api::router(state).into_make_service_with_connect_info::<SocketAddr>();
        let (stop, stopped) = oneshot::channel::<()>();
        let task = tokio::spawn(async move {
            let shutdown = async {
                let _ = stopped.await;
            };
            if let Err(err) = axum::serve(listener, app)
                .with_graceful_shutdown(shutdown)
                .await
            {
                tracing::warn!(%err, "remote control stopped");
            }
        });
        tracing::info!(%addr, "remote control listening");
        Ok(Self {
            addr,
            stop: Mutex::new(Some(stop)),
            task: Some(task),
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    /// Stop listening and finish open requests, for at most two seconds.
    pub async fn stop(mut self) {
        self.signal();
        if let Some(mut task) = self.task.take()
            && tokio::time::timeout(Duration::from_secs(2), &mut task)
                .await
                .is_err()
        {
            task.abort();
        }
        tracing::info!(addr = %self.addr, "remote control stopped");
    }

    fn signal(&self) {
        if let Some(stop) = self.stop.lock().take() {
            let _ = stop.send(());
        }
    }
}

impl Drop for RemoteServer {
    fn drop(&mut self) {
        self.signal();
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}
