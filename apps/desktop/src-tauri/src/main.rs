// No console window behind the app in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod background;
mod clipboard;
mod confirm;
mod grab;
mod i18n;
mod power;
mod progress;
mod remote;
mod tray;

use std::path::PathBuf;

use background::{Background, WhenDone, WhenDoneView};
use downpipe_core::feeds::Feeds;
use downpipe_core::providers::ToolsStatus;
use downpipe_core::providers::install::Tool;
use downpipe_core::{
    AddListOptions, AddListResult, AddOptions, ConfirmOptions, CoreError, ListChoices, Manager,
    MediaChoices, QueueMove, RefusedHost, RuleProblem, RuleTest, Settings, SiteChoices, SiteRule,
    TaskSnapshot, TaskState, rules,
};
use downpipe_native_host::ipc::Endpoint;
use downpipe_native_host::manifest::{self, Layout, Stale, Status};
use downpipe_native_host::server::BridgeServer;
use serde::Serialize;
use tauri::{AppHandle, Manager as _, RunEvent, State, WindowEvent};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

/// Passed by the login item so Downpipe starts in the tray.
const HIDDEN_ARG: &str = "--hidden";

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
        // "not_found" is also the engine's HTTP 404; a download that left
        // the list gets a code of its own so the UI words the two apart.
        let code = match &err {
            CoreError::NotFound(_) => GONE,
            _ => err.code(),
        };
        Self {
            code,
            message: err.to_string(),
            id,
        }
    }
}

/// The download a command names is no longer in the list.
const GONE: &str = "gone";

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

/// "Pause until": `at` is Unix seconds; the core queues it again then.
#[tauri::command]
fn pause_download_until(manager: State<'_, Manager>, id: String, at: i64) -> CmdResult<()> {
    Ok(manager.pause_until(&id, at)?)
}

/// Bytes per second; null or 0 leaves only the global limit.
#[tauri::command]
fn set_download_speed_limit(
    manager: State<'_, Manager>,
    id: String,
    limit: Option<u64>,
) -> CmdResult<()> {
    Ok(manager.set_speed_limit(&id, limit)?)
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
async fn update_settings(
    app: AppHandle,
    manager: State<'_, Manager>,
    mut settings: Settings,
) -> CmdResult<Settings> {
    let current = manager.settings();
    // The remote control changes only through its own commands, which make
    // its token; a page saving other settings cannot switch it on or set
    // the token hash, even from stale settings.
    settings.remote = current.remote;
    let was_registering = current.register_browser_host;
    let saved = manager.update_settings(settings)?;
    sync_autostart(&app, saved.start_with_system).map_err(|e| CommandError::new("autostart", e))?;
    if saved.register_browser_host && !was_registering {
        // Switching it on fixes the registration now rather than at the
        // next start, so the extension connects without restarting the app.
        let _ = tauri::async_runtime::spawn_blocking(register_browser_host).await;
    }
    Ok(saved)
}

#[derive(Debug, Serialize)]
struct SiteRulesCheck {
    problems: Vec<RuleProblem>,
    /// Present when an address was given.
    test: Option<RuleTest>,
}

/// The settings dialog's draft rules, before they are saved: what is wrong
/// with them, and which one `address` would get. The same code decides at
/// download time, so the answer cannot differ from what happens.
#[tauri::command]
fn check_site_rules(rules: Vec<SiteRule>, address: Option<String>) -> SiteRulesCheck {
    SiteRulesCheck {
        problems: rules::problems(&rules),
        test: address
            .filter(|a| !a.trim().is_empty())
            .map(|a| rules::test_address(&rules, &a)),
    }
}

#[tauri::command]
fn refused_hosts(manager: State<'_, Manager>) -> Vec<RefusedHost> {
    manager.refused_hosts()
}

/// What the Remote settings page shows: whether it runs, and where.
#[tauri::command]
async fn remote_status(
    manager: State<'_, Manager>,
    remote: State<'_, remote::Remote>,
) -> CmdResult<remote::RemoteStatus> {
    Ok(remote.status(&manager.settings().remote).await)
}

/// The remote control's state after a change, and a new token when one was
/// made: the only time it is ever shown.
#[derive(Debug, Serialize)]
struct RemoteUpdate {
    status: remote::RemoteStatus,
    token: Option<String>,
}

/// Switch the remote control and choose where it listens. Switching it on
/// the first time makes its token.
#[tauri::command]
async fn set_remote(
    app: AppHandle,
    manager: State<'_, Manager>,
    remote: State<'_, remote::Remote>,
    enabled: bool,
    bind: String,
    port: u16,
) -> CmdResult<RemoteUpdate> {
    let mut settings = manager.settings();
    settings.remote.enabled = enabled;
    settings.remote.bind = bind;
    settings.remote.port = port;
    let token = (enabled && settings.remote.token_hash.is_empty())
        .then(|| remote::new_token(&mut settings.remote));
    save_remote(&app, &manager, &remote, settings, token).await
}

/// Replace the token; the old one stops working at once.
#[tauri::command]
async fn new_remote_token(
    app: AppHandle,
    manager: State<'_, Manager>,
    remote: State<'_, remote::Remote>,
) -> CmdResult<RemoteUpdate> {
    let mut settings = manager.settings();
    let token = remote::new_token(&mut settings.remote);
    save_remote(&app, &manager, &remote, settings, Some(token)).await
}

async fn save_remote(
    app: &AppHandle,
    manager: &Manager,
    remote: &remote::Remote,
    settings: Settings,
    token: Option<String>,
) -> CmdResult<RemoteUpdate> {
    let saved = manager.update_settings(settings)?;
    // Applied here as well as from the settings event, so the answer
    // already says whether the server could start.
    remote.apply(app, manager, &saved.remote).await;
    Ok(RemoteUpdate {
        status: remote.status(&saved.remote).await,
        token,
    })
}

/// Name, folder and queue for a download the browser sent, then start it
/// or keep it for later.
#[tauri::command]
fn confirm_download(
    manager: State<'_, Manager>,
    id: String,
    options: ConfirmOptions,
) -> CmdResult<()> {
    Ok(manager.confirm(&id, options)?)
}

/// The qualities and audio tracks of a stream download, read from its
/// manifest, for the quality chooser. Downloads no media.
#[tauri::command]
async fn inspect_media(manager: State<'_, Manager>, id: String) -> CmdResult<MediaChoices> {
    Ok(manager.inspect_media(&id).await?)
}

/// The FFmpeg stream downloads would use now, for the settings.
#[tauri::command]
fn ffmpeg_status(manager: State<'_, Manager>) -> Option<PathBuf> {
    manager.ffmpeg()
}

/// Asks yt-dlp what a video page offers; fetches the page, no video.
#[tauri::command]
async fn inspect_site(manager: State<'_, Manager>, id: String) -> CmdResult<SiteChoices> {
    Ok(manager.inspect_site(&id).await?)
}

/// Asks yt-dlp for the videos of a playlist or channel; fetches the list
/// pages, no video.
#[tauri::command]
async fn inspect_list(manager: State<'_, Manager>, url: String) -> CmdResult<ListChoices> {
    Ok(manager.inspect_list(&url).await?)
}

/// Adds the videos chosen from a playlist or channel.
#[tauri::command]
fn add_list(manager: State<'_, Manager>, options: AddListOptions) -> CmdResult<AddListResult> {
    Ok(manager.add_list(options)?)
}

/// yt-dlp, Deno (or Node) and FFmpeg as the settings find them.
#[tauri::command]
async fn site_tools(manager: State<'_, Manager>) -> CmdResult<ToolsStatus> {
    Ok(manager.site_tools().await)
}

/// Download yt-dlp and/or Deno from their official releases. Only the
/// settings' button calls this.
#[tauri::command]
async fn install_site_tools(
    manager: State<'_, Manager>,
    tools: Vec<Tool>,
) -> CmdResult<ToolsStatus> {
    Ok(manager.install_site_tools(&tools).await?)
}

/// Download a finished file again; `replace` deletes the old copy first.
#[tauri::command]
fn redownload(manager: State<'_, Manager>, id: String, replace: bool) -> CmdResult<()> {
    Ok(manager.redownload(&id, replace)?)
}

#[tauri::command]
fn move_downloads(manager: State<'_, Manager>, ids: Vec<String>, to: QueueMove) -> CmdResult<()> {
    Ok(manager.move_tasks(&ids, to)?)
}

#[tauri::command]
fn set_download_queue(
    manager: State<'_, Manager>,
    ids: Vec<String>,
    queue: String,
) -> CmdResult<()> {
    Ok(manager.set_queue(&ids, &queue)?)
}

#[tauri::command]
fn start_queue(manager: State<'_, Manager>, queue: String) {
    manager.start_queue(&queue);
}

#[tauri::command]
fn stop_queue(manager: State<'_, Manager>, queue: String) {
    manager.stop_queue(&queue);
}

/// IDM's "refresh download address": a new link for a paused or failed
/// download, used only if it serves the same file.
#[tauri::command]
fn change_download_url(manager: State<'_, Manager>, id: String, url: String) -> CmdResult<()> {
    Ok(manager.change_url(&id, &url)?)
}

/// Save every link in the list to a text file, one per line. Credentials
/// were never stored, so none can leak into the file.
#[tauri::command]
async fn export_downloads(app: AppHandle, manager: State<'_, Manager>) -> CmdResult<Option<usize>> {
    let urls: Vec<String> = manager.list().into_iter().map(|t| t.record.url).collect();
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_file_name("downpipe-links.txt")
        .add_filter("Text", &["txt"])
        .save_file(move |path| {
            let _ = tx.send(path.and_then(|p| p.into_path().ok()));
        });
    let Some(path) = rx.await.ok().flatten() else {
        return Ok(None);
    };
    let mut text = urls.join("\n");
    text.push('\n');
    std::fs::write(&path, text).map_err(|e| CommandError::new("io", e.to_string()))?;
    Ok(Some(urls.len()))
}

/// Drag a finished file out of the window into another app. By id, like
/// every other file action.
#[tauri::command]
fn start_drag(
    window: tauri::WebviewWindow,
    manager: State<'_, Manager>,
    id: String,
) -> CmdResult<()> {
    let task = manager
        .get(&id)
        .ok_or_else(|| CommandError::new(GONE, "That download no longer exists"))?;
    let path = match (task.record.state, task.record.path) {
        (TaskState::Completed, Some(path)) if path.is_file() => path,
        _ => {
            return Err(CommandError::new(
                "invalid_state",
                "Only finished downloads can be dragged",
            ));
        }
    };
    // The system drag loop waits for the button to be released. Started
    // after a quick click instead of a drag, it would wait for the next
    // click anywhere on screen and drop the file there.
    if !primary_button_down() {
        return Ok(());
    }
    let icon = drag::Image::Raw(include_bytes!("../icons/64x64.png").to_vec());
    let target = window.clone();
    // OLE drag and drop must start on the UI thread, while the button is down.
    window
        .run_on_main_thread(move || {
            let item = drag::DragItem::Files(vec![path]);
            #[cfg(target_os = "linux")]
            let handle = target.gtk_window();
            #[cfg(not(target_os = "linux"))]
            let handle = &target;
            #[cfg(target_os = "linux")]
            let handle = match handle {
                Ok(h) => h,
                Err(_) => return,
            };
            #[cfg(target_os = "linux")]
            let handle = &handle;
            if let Err(err) =
                drag::start_drag(handle, item, icon, |_, _| {}, drag::Options::default())
            {
                tracing::warn!("drag failed: {err}");
            }
        })
        .map_err(|e| CommandError::new("drag", e.to_string()))
}

#[cfg(windows)]
fn primary_button_down() -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_SWAPBUTTON};
    // SAFETY: both functions only read input state and take plain values.
    unsafe {
        // GetAsyncKeyState reports physical buttons; with swapped buttons
        // the primary one is the physical right button.
        let key = if GetSystemMetrics(SM_SWAPBUTTON) != 0 {
            VK_RBUTTON
        } else {
            VK_LBUTTON
        };
        (GetAsyncKeyState(i32::from(key)) as u16) & 0x8000 != 0
    }
}

#[cfg(not(windows))]
fn primary_button_down() -> bool {
    true
}

/// The confirmation page drew a download's question: show its window.
/// Async, as every confirmation window command: a missing window is built
/// again, which must not happen on the main thread.
#[tauri::command]
async fn confirm_window_drawn(app: AppHandle) {
    confirm::drawn(&app);
}

/// The calling dialog window's page (the confirmation window or a progress
/// window) needs this height, in CSS pixels at its `devicePixelRatio`.
#[tauri::command]
async fn fit_window(window: tauri::WebviewWindow, height: f64, scale: f64) {
    confirm::fit(&window, height, scale);
}

/// After "Start download" in the confirmation window: follow the download
/// in a progress window of its own.
#[tauri::command]
async fn watch_download(app: AppHandle, id: String) {
    progress::open(&app, &id);
}

/// A progress page drew its download: show its window.
#[tauri::command]
async fn progress_window_drawn(window: tauri::WebviewWindow) {
    progress::drawn(&window);
}

/// A progress page names its window after its download's progress.
#[tauri::command]
async fn set_progress_title(window: tauri::WebviewWindow, title: String) {
    progress::set_title(&window, &title);
}

/// A progress page's Hide, Close and Cancel: its window goes, the download
/// stays.
#[tauri::command]
async fn close_progress_window(window: tauri::WebviewWindow) {
    progress::close(&window);
}

/// The confirmation page's Settings links: the main window opens on
/// Settings, Media, and the question waits until those close.
#[tauri::command]
async fn open_media_settings(app: AppHandle) {
    confirm::to_settings(&app);
}

/// The main window's Settings closed; a question waiting for them returns.
#[tauri::command]
async fn settings_closed(app: AppHandle) {
    confirm::back_from_settings(&app);
}

#[tauri::command]
fn get_when_done(background: State<'_, Background>) -> WhenDoneView {
    background.when_done()
}

#[tauri::command]
fn set_when_done(
    app: AppHandle,
    background: State<'_, Background>,
    action: WhenDone,
) -> WhenDoneView {
    background.set_when_done(&app, action);
    background.when_done()
}

#[tauri::command]
fn when_done_now(app: AppHandle, background: State<'_, Background>) {
    background.when_done_now(&app);
}

/// Keep the login item in step with the setting. Also rewrites it when the
/// executable moved (an update to a new folder).
fn sync_autostart(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let launcher = app.autolaunch();
    let is_enabled = launcher.is_enabled().unwrap_or(false);
    let result = if enabled {
        launcher.enable()
    } else if is_enabled {
        launcher.disable()
    } else {
        Ok(())
    };
    result.map_err(|e| e.to_string())
}

/// Native folder picker. Returns `None` when the user closes it.
#[tauri::command]
async fn pick_folder(
    app: AppHandle,
    window: tauri::WebviewWindow,
    initial: Option<String>,
) -> Option<String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    // Owned by the window that asked, so it opens in front of it rather
    // than behind it (the confirmation window has just come over the browser).
    let mut dialog = app.dialog().file().set_parent(&window);
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
        .ok_or_else(|| CommandError::new(GONE, "That download no longer exists"))?;
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
        .ok_or_else(|| CommandError::new(GONE, "That download no longer exists"))?;
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
            tray::show_main(app);
        }));
    }

    let app = builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![HIDDEN_ARG]),
        ))
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
            let feeds = Feeds::open(&data_dir, manager.clone())?;
            grab::spawn_feeds(app.handle().clone(), feeds.clone());
            app.manage(feeds);
            app.manage(grab::Grabs::default());
            app.manage(Background::default());
            app.manage(confirm::ConfirmWindow::default());
            background::spawn(app.handle().clone(), manager.clone());
            clipboard::spawn(app.handle().clone(), manager.clone());
            app.manage(remote::Remote::default());
            remote::spawn(app.handle().clone(), manager.clone());
            if let Err(err) = tray::create(app.handle()) {
                tracing::warn!("no tray icon: {err}");
            }
            if let Err(err) = sync_autostart(app.handle(), manager.settings().start_with_system) {
                tracing::warn!("could not update the login item: {err}");
            }
            app.manage(manager);
            // The window starts hidden (tauri.conf.json) so a login start
            // stays in the tray without a flash; normal starts show it now.
            if !std::env::args().any(|a| a == HIDDEN_ARG) {
                tray::show_main(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                if progress::id_of(window.label()).is_some() {
                    // Closing a progress window hides the download's
                    // progress only; the download goes on.
                } else if window.label() == confirm::LABEL {
                    // Kept for the next question; the page decides what
                    // closing means for the download shown.
                    api.prevent_close();
                    confirm::close_requested(app);
                } else if app.state::<Manager>().settings().close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                    app.state::<Background>().closed_to_tray(app);
                    // Settings left open in the hidden window would keep
                    // a question waiting for them out of sight.
                    confirm::back_from_settings(app);
                } else {
                    // The hidden confirmation window would otherwise keep
                    // Downpipe running without a window.
                    app.exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_downloads,
            add_download,
            pause_download,
            resume_download,
            pause_download_until,
            set_download_speed_limit,
            cancel_download,
            remove_download,
            pause_all,
            resume_all,
            clear_finished,
            get_settings,
            update_settings,
            check_site_rules,
            refused_hosts,
            remote_status,
            set_remote,
            new_remote_token,
            browser_bridge_status,
            register_browser_bridge,
            pick_folder,
            reveal_download,
            open_download,
            change_download_url,
            confirm_download,
            inspect_media,
            ffmpeg_status,
            inspect_site,
            inspect_list,
            add_list,
            site_tools,
            install_site_tools,
            redownload,
            move_downloads,
            set_download_queue,
            start_queue,
            stop_queue,
            export_downloads,
            start_drag,
            get_when_done,
            set_when_done,
            when_done_now,
            confirm_window_drawn,
            fit_window,
            watch_download,
            progress_window_drawn,
            set_progress_title,
            close_progress_window,
            open_media_settings,
            settings_closed,
            grab::start_grab,
            grab::cancel_grab,
            grab::list_feeds,
            grab::add_feed,
            grab::update_feed,
            grab::remove_feed,
            grab::check_feed,
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
            handle.state::<Feeds>().stop();
            let manager = handle.state::<Manager>().inner().clone();
            tauri::async_runtime::block_on(manager.shutdown());
        }
    });
}
