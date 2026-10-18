//! Runs the optional remote control (`downpipe-remote`) while it is
//! switched on, and restarts it when its settings change. The web page it
//! serves is `remote.html` from this app's own bundle.

use std::net::IpAddr;
use std::sync::Arc;

use downpipe_core::{Manager, ManagerEvent, RemoteSettings};
use downpipe_remote::{Asset, Assets, Config, RemoteServer, is_servable, token};
use serde::Serialize;
use tauri::{AppHandle, Manager as _};
use tokio::sync::Mutex;
use tokio::sync::broadcast::error::RecvError;

/// The page's files, from the bundle (read from `dist` in development).
struct BundledAssets(AppHandle);

impl std::fmt::Debug for BundledAssets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BundledAssets")
    }
}

impl Assets for BundledAssets {
    fn get(&self, path: &str) -> Option<Asset> {
        if !is_servable(path) {
            return None;
        }
        let asset = self.0.asset_resolver().get(path.to_string())?;
        Some(Asset {
            bytes: asset.bytes,
            mime: asset.mime_type,
        })
    }
}

#[derive(Debug, Default)]
pub struct Remote {
    inner: Mutex<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    /// The settings the server (or its failure) belongs to.
    applied: Option<RemoteSettings>,
    server: Option<RemoteServer>,
    /// Why it is not running although switched on (`port_in_use`, ...).
    error: Option<&'static str>,
}

/// What the Remote settings page shows.
#[derive(Debug, Clone, Serialize)]
pub struct RemoteStatus {
    pub enabled: bool,
    pub bind: String,
    pub port: u16,
    pub has_token: bool,
    pub running: bool,
    pub error: Option<&'static str>,
    /// Where to open the page: this computer's network address when it
    /// listens on every interface, else the address it listens on.
    pub url: Option<String>,
    /// This computer's address on the local network, for the bind choice.
    pub lan_address: Option<String>,
}

impl Remote {
    /// Start, stop or restart the server to match `settings`. Does nothing
    /// when they did not change.
    pub async fn apply(&self, app: &AppHandle, manager: &Manager, settings: &RemoteSettings) {
        let mut inner = self.inner.lock().await;
        if inner.applied.as_ref() == Some(settings) {
            return;
        }
        if let Some(server) = inner.server.take() {
            server.stop().await;
        }
        inner.error = None;
        inner.applied = Some(settings.clone());
        let Some(config) = Config::from_settings(settings) else {
            return;
        };
        let assets: Arc<dyn Assets> = Arc::new(BundledAssets(app.clone()));
        match RemoteServer::start(manager.clone(), config, assets).await {
            Ok(server) => inner.server = Some(server),
            Err(err) => {
                tracing::warn!("remote control is off: {err}");
                inner.error = Some(err.code());
            }
        }
    }

    pub async fn status(&self, settings: &RemoteSettings) -> RemoteStatus {
        let inner = self.inner.lock().await;
        let lan = downpipe_remote::lan_address();
        let url = inner.server.as_ref().map(|server| {
            let addr = server.local_addr();
            let host = if addr.ip().is_unspecified() {
                lan.unwrap_or(addr.ip())
            } else {
                addr.ip()
            };
            page_url(host, addr.port())
        });
        RemoteStatus {
            enabled: settings.enabled,
            bind: settings.bind.clone(),
            port: settings.port,
            has_token: !settings.token_hash.is_empty(),
            running: inner.server.is_some(),
            error: inner.error,
            url,
            lan_address: lan.map(|ip| ip.to_string()),
        }
    }
}

fn page_url(ip: IpAddr, port: u16) -> String {
    match ip {
        IpAddr::V6(v6) => format!("http://[{v6}]:{port}/"),
        IpAddr::V4(v4) => format!("http://{v4}:{port}/"),
    }
}

/// Follow the settings for as long as the app runs.
pub fn spawn(app: AppHandle, manager: Manager) {
    tauri::async_runtime::spawn(async move {
        let mut events = manager.subscribe();
        let remote = app.state::<Remote>();
        remote
            .apply(&app, &manager, &manager.settings().remote)
            .await;
        loop {
            match events.recv().await {
                Ok(ManagerEvent::Settings { settings }) => {
                    remote.apply(&app, &manager, &settings.remote).await;
                }
                Ok(_) => {}
                Err(RecvError::Lagged(_)) => {
                    remote
                        .apply(&app, &manager, &manager.settings().remote)
                        .await;
                }
                Err(RecvError::Closed) => break,
            }
        }
    });
}

/// A new token: stored as its hash, returned once for the settings page to
/// show. The old one stops working at once.
pub fn new_token(remote: &mut RemoteSettings) -> String {
    let token = token::generate();
    remote.token_hash = token::hash(&token);
    token
}
