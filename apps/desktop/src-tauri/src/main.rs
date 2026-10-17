// No console window behind the app in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use downpipe_core::{AddOptions, CoreError, Manager, Settings, TaskSnapshot, TaskState};
use downpipe_native_host::ipc::Endpoint;
use downpipe_native_host::manifest::{self, Layout, Stale, Status};
use downpipe_native_host::server::BridgeServer;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager as _, RunEvent, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
use tokio::sync::broadcast::error::RecvError;

/// Engine events are re-emitted to the web view under this name.
const EVENT: &str = "downpipe://event";
/// Sent when the web view fell behind and should reload the full list.
const RESYNC: &str = "downpipe://resync";

/// Error shape the UI can branch on (`code`) and show (`message`).
#[derive(Debug, Serialize)]
struct CommandError {
    code: &'static str,
    message: String,
    /// For `duplicate`: the existing download.
    id: Option<String>,
}

impl From<CoreError> for CommandError {
    fn from(err: CoreError) -> Self {
        let id = match &err {
            CoreError::Duplicate(id) => Some(id.clone()),
            _ => None,
        };
        Self {
            code: err.code(),
            message: err.to_string(),
            id,
        }
    }
}

impl CommandError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            id: None,
        }
    }
}

type CmdResult<T> = Result<T, CommandError>;

#[tauri::command]
fn list_downloads(manager: State<'_, Manager>) -> Vec<TaskSnapshot> {
    manager.list()
}

#[tauri::command]
fn add_download(manager: State<'_, Manager>, options: AddOptions) -> CmdResult<TaskSnapshot> {
    Ok(manager.add(options)?)
}

#[tauri::command]
fn pause_download(manager: State<'_, Manager>, id: String) -> CmdResult<()> {
    Ok(manager.pause(&id)?)
}

#[tauri::command]
fn resume_download(manager: State<'_, Manager>, id: String) -> CmdResult<()> {
    Ok(manager.resume(&id)?)
}

#[tauri::command]
fn cancel_download(manager: State<'_, Manager>, id: String) -> CmdResult<()> {
    Ok(manager.cancel(&id)?)
}

#[tauri::command]
fn remove_download(manager: State<'_, Manager>, id: String, delete_files: bool) -> CmdResult<()> {
    Ok(manager.remove(&id, delete_files)?)
}

#[tauri::command]
fn pause_all(manager: State<'_, Manager>) {
    manager.pause_all();
}

#[tauri::command]
fn resume_all(manager: State<'_, Manager>) {
    manager.resume_all();
}

#[tauri::command]
fn clear_finished(manager: State<'_, Manager>) -> CmdResult<()> {
    Ok(manager.clear_finished()?)
}

#[tauri::command]
fn get_settings(manager: State<'_, Manager>) -> Settings {
    manager.settings()
}

#[tauri::command]
async fn update_settings(manager: State<'_, Manager>, settings: Settings) -> CmdResult<Settings> {
    let was_registering = manager.settings().register_browser_host;
    let saved = manager.update_settings(settings)?;
    if saved.register_browser_host && !was_registering {
        // Switching it on fixes the registration now rather than at the
        // next start, so the extension connects without restarting the app.
        let _ = tauri::async_runtime::spawn_blocking(register_browser_host).await;
    }
    Ok(saved)
}

/// Native folder picker. Returns `None` when the user closes it.
#[tauri::command]
async fn pick_folder(app: AppHandle, initial: Option<String>) -> Option<String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let mut dialog = app.dialog().file();
    if let Some(dir) = initial.map(PathBuf::from).filter(|d| d.is_dir()) {
        dialog = dialog.set_directory(dir);
    }
    dialog.pick_folder(move |folder| {
        let _ = tx.send(folder.and_then(|f| f.into_path().ok()));
    });
    rx.await
        .ok()
        .flatten()
        .map(|p| p.to_string_lossy().into_owned())
}

/// Show the download in the system file manager. Works by id so the web
/// view can never ask for an arbitrary path.
#[tauri::command]
fn reveal_download(app: AppHandle, manager: State<'_, Manager>, id: String) -> CmdResult<()> {
    let task = manager
        .get(&id)
        .ok_or_else(|| CommandError::new("not_found", "That download no longer exists"))?;
    let record = task.record;
    let target = match (record.state, record.path) {
        (TaskState::Completed, Some(path)) if path.exists() => path,
        (_, Some(path)) => {
            let part = downpipe_core::http::meta::part_path(&path);
            if part.exists() { part } else { record.dir }
        }
        (_, None) => record.dir,
    };
    let result = if target.is_dir() {
        app.opener()
            .open_path(target.to_string_lossy(), None::<&str>)
    } else {
        app.opener().reveal_item_in_dir(&target)
    };
    result.map_err(|e| CommandError::new("open_failed", e.to_string()))
}

/// Open a completed download with its default application.
#[tauri::command]
fn open_download(app: AppHandle, manager: State<'_, Manager>, id: String) -> CmdResult<()> {
    let task = manager
        .get(&id)
        .ok_or_else(|| CommandError::new("not_found", "That download no longer exists"))?;
    match (task.record.state, task.record.path) {
        (TaskState::Completed, Some(path)) if path.exists() => app
            .opener()
            .open_path(path.to_string_lossy(), None::<&str>)
            .map_err(|e| CommandError::new("open_failed", e.to_string())),
        _ => Err(CommandError::new(
            "invalid_state",
            "Only finished downloads can be opened",
        )),
    }
}

/// The browser bridge's registration as the Settings dialog shows it.
#[derive(Debug, Serialize)]
struct BridgeRegistration {
    /// The native host this copy of the app ships.
    host: String,
    /// False when that file is missing: a damaged install, or a development
    /// build started without `pnpm sidecar`.
    host_found: bool,
    browsers: Vec<BrowserRegistration>,
}

#[derive(Debug, Serialize)]
struct BrowserRegistration {
    browser: &'static str,
    /// `registered`, `not_installed`, `missing`, `invalid`, `other_host`,
    /// `extension_not_allowed` or `failed`.
    state: &'static str,
    /// The other host's path, or what failed.
    detail: Option<String>,
}

fn bundled_host() -> std::io::Result<PathBuf> {
    Ok(manifest::host_beside(&std::env::current_exe()?))
}

fn bridge_registration() -> std::io::Result<BridgeRegistration> {
    let host = bundled_host()?;
    let browsers = manifest::check(&Layout::current()?, &host)
        .into_iter()
        .map(|(browser, status)| {
            let (state, detail) = match status {
                Status::NotInstalled => ("not_installed", None),
                Status::Current | Status::Updated(_) => ("registered", None),
                Status::Stale(Stale::Missing) => ("missing", None),
                Status::Stale(Stale::Invalid) => ("invalid", None),
                Status::Stale(Stale::OtherHost(other)) => {
                    ("other_host", Some(other.display().to_string()))
                }
                Status::Stale(Stale::ExtensionNotAllowed) => ("extension_not_allowed", None),
                Status::Failed(err) => ("failed", Some(err)),
            };
            BrowserRegistration {
                browser: browser.name(),
                state,
                detail,
            }
        })
        .collect();
    Ok(BridgeRegistration {
        host_found: host.is_file(),
        host: host.display().to_string(),
        browsers,
    })
}

/// Compare each browser's registration with this app's host, read-only.
#[tauri::command]
async fn browser_bridge_status() -> CmdResult<BridgeRegistration> {
    tauri::async_runtime::spawn_blocking(bridge_registration)
        .await
        .map_err(|e| CommandError::new("internal", e.to_string()))?
        .map_err(|e| CommandError::new("bridge_status", e.to_string()))
}

/// Register now, whatever the start-up setting says, and report the result.
#[tauri::command]
async fn register_browser_bridge() -> CmdResult<BridgeRegistration> {
    tauri::async_runtime::spawn_blocking(|| {
        register_browser_host();
        bridge_registration()
    })
    .await
    .map_err(|e| CommandError::new("internal", e.to_string()))?
    .map_err(|e| CommandError::new("bridge_status", e.to_string()))
}

/// Point every browser's Native Messaging registration for this user at the
/// host next to this executable, so the extension works without running
/// `downpipe-native-host install`. Machine-wide registrations are never
/// touched. Blocking: on Windows every registry access runs `reg.exe`.
fn register_browser_host() {
    let host = match bundled_host() {
        Ok(host) if host.is_file() => host,
        Ok(host) => {
            tracing::warn!(
                "browser bridge not registered: {} does not exist",
                host.display()
            );
            return;
        }
        Err(err) => {
            tracing::warn!("browser bridge not registered: {err}");
            return;
        }
    };
    let layout = match Layout::current() {
        Ok(layout) => layout,
        Err(err) => {
            tracing::warn!("browser bridge not registered: {err}");
            return;
        }
    };
    for (browser, status) in manifest::sync(&layout, &host) {
        let browser = browser.name();
        match status {
            Status::Updated(why) => {
                let why = match why {
                    Stale::Missing => "it was not registered".to_string(),
                    Stale::Invalid => "its manifest was unusable".to_string(),
                    Stale::OtherHost(other) => format!("it started {}", other.display()),
                    Stale::ExtensionNotAllowed => {
                        "it did not allow the Downpipe extension".to_string()
                    }
                };
                tracing::info!("registered the browser bridge for {browser}: {why}");
            }
            Status::Failed(err) => {
                tracing::warn!("could not register the browser bridge for {browser}: {err}");
            }
            Status::NotInstalled | Status::Current | Status::Stale(_) => {}
        }
    }
}

/// Serve the browser extension's native host on the local pipe or socket.
/// Without it the app still works; the extension then reports that
/// Downpipe is not running.
fn start_browser_bridge(manager: Manager) {
    tauri::async_runtime::spawn(async move {
        let endpoint = Endpoint::from_env_or_default();
        match BridgeServer::bind(endpoint) {
            Ok(server) => {
                tracing::info!("browser bridge listening on {}", server.endpoint());
                server.serve(manager, env!("CARGO_PKG_VERSION")).await;
            }
            Err(err) => tracing::warn!("browser bridge is off: {err}"),
        }
    });
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("DOWNPIPE_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let mut builder = tauri::Builder::default();
    // A second copy would find the catalog and the bridge's pipe in use, so
    // starting Downpipe again brings the running window forward instead.
    // Runs with their own DOWNPIPE_DATA_DIR (tests, portable copies, a
    // development build next to an installed one) stay separate on purpose.
    if std::env::var_os("DOWNPIPE_DATA_DIR").is_none() {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }));
    }

    let app = builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // DOWNPIPE_DATA_DIR keeps test runs and portable setups away from
            // the real catalog in the OS app-data folder.
            let data_dir = match std::env::var_os("DOWNPIPE_DATA_DIR") {
                Some(dir) => PathBuf::from(dir),
                None => app.path().app_data_dir()?,
            };
            let manager = tauri::async_runtime::block_on(Manager::open(&data_dir))?;
            if manager.settings().register_browser_host {
                tauri::async_runtime::spawn_blocking(register_browser_host);
            }
            start_browser_bridge(manager.clone());
            let mut events = manager.subscribe();
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    match events.recv().await {
                        Ok(event) => {
                            let _ = handle.emit(EVENT, event);
                        }
                        Err(RecvError::Lagged(_)) => {
                            let _ = handle.emit(RESYNC, ());
                        }
                        Err(RecvError::Closed) => break,
                    }
                }
            });
            app.manage(manager);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_downloads,
            add_download,
            pause_download,
            resume_download,
            cancel_download,
            remove_download,
            pause_all,
            resume_all,
            clear_finished,
            get_settings,
            update_settings,
            browser_bridge_status,
            register_browser_bridge,
            pick_folder,
            reveal_download,
            open_download,
        ])
        .build(tauri::generate_context!());

    let app = match app {
        Ok(app) => app,
        Err(err) => {
            eprintln!("Downpipe could not start: {err}");
            std::process::exit(1);
        }
    };

    app.run(|handle, event| {
        if let RunEvent::Exit = event {
            // Write final checkpoints so running downloads continue on the
            // next launch instead of losing the last seconds of progress.
            let manager = handle.state::<Manager>().inner().clone();
            tauri::async_runtime::block_on(manager.shutdown());
        }
    });
}
