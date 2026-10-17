// No console window behind the app in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use downpipe_core::{AddOptions, CoreError, Manager, Settings, TaskSnapshot, TaskState};
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
        let (code, id) = match &err {
            CoreError::Duplicate(id) => ("duplicate", Some(id.clone())),
            CoreError::NotFound(_) => ("not_found", None),
            CoreError::InvalidState { .. } => ("invalid_state", None),
            CoreError::InvalidUrl(_) => ("invalid_url", None),
            CoreError::Download(e) => (e.kind(), None),
            CoreError::Store(_) | CoreError::Io(_) => ("internal", None),
        };
        Self {
            code,
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
fn update_settings(manager: State<'_, Manager>, settings: Settings) -> CmdResult<Settings> {
    Ok(manager.update_settings(settings)?)
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

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("DOWNPIPE_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let app = tauri::Builder::default()
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
