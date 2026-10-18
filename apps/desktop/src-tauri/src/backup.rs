//! Settings → App: export a backup, import one, and import IDM's list.
//!
//! The web view never handles paths or file contents: Rust opens the file
//! dialogs, reads and checks the files, and keeps what it read until the
//! user picks what to import from the preview, which then names entries by
//! their position only.

use std::path::{Path, PathBuf};

use downpipe_core::Manager;
use downpipe_core::backup::{self, Backup, BackupError, BackupSummary, ImportChoice, ImportReport};
use downpipe_core::idm::{self, IdmEntry};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::{CmdResult, CommandError};

/// What was read for the preview, until it is imported or replaced.
#[derive(Debug, Default)]
pub struct PendingImports {
    backup: Mutex<Option<Backup>>,
    idm: Mutex<Vec<IdmEntry>>,
}

impl From<BackupError> for CommandError {
    fn from(err: BackupError) -> Self {
        CommandError::new(err.code(), err.to_string())
    }
}

fn io_error(err: impl std::fmt::Display) -> CommandError {
    CommandError::new("io", err.to_string())
}

fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

async fn save_dialog(
    window: &tauri::WebviewWindow,
    app: &AppHandle,
    name: &str,
) -> Option<PathBuf> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_parent(window)
        .set_file_name(name)
        .add_filter("JSON", &["json"])
        .save_file(move |path| {
            let _ = tx.send(path.and_then(|p| p.into_path().ok()));
        });
    rx.await.ok().flatten()
}

async fn open_dialog(
    window: &tauri::WebviewWindow,
    app: &AppHandle,
    filter: (&str, &[&str]),
) -> Option<PathBuf> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_parent(window)
        .add_filter(filter.0, filter.1)
        .pick_file(move |path| {
            let _ = tx.send(path.and_then(|p| p.into_path().ok()));
        });
    rx.await.ok().flatten()
}

/// Read a file of at most `limit` bytes, refusing a larger one before
/// reading it.
async fn read_capped(path: PathBuf, limit: u64) -> Result<Vec<u8>, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let len = std::fs::metadata(&path).map_err(io_error)?.len();
        if len > limit {
            return Err(CommandError::new(
                "file_too_large",
                format!("the file is larger than {} MB", limit / (1024 * 1024)),
            ));
        }
        std::fs::read(&path).map_err(io_error)
    })
    .await
    .map_err(|e| CommandError::new("internal", e.to_string()))?
}

#[derive(Debug, Serialize)]
pub struct Exported {
    file: String,
    downloads: usize,
}

/// Save the settings and the list to a JSON file the user picks.
#[tauri::command]
pub async fn export_backup(
    app: AppHandle,
    window: tauri::WebviewWindow,
    manager: State<'_, Manager>,
) -> CmdResult<Option<Exported>> {
    let backup = manager.backup(env!("CARGO_PKG_VERSION"));
    let name = format!(
        "downpipe-backup-{}.json",
        chrono::Local::now().format("%Y-%m-%d")
    );
    let Some(path) = save_dialog(&window, &app, &name).await else {
        return Ok(None);
    };
    let downloads = backup.downloads.len();
    let json = backup.to_json();
    let target = path.clone();
    tauri::async_runtime::spawn_blocking(move || std::fs::write(&target, json))
        .await
        .map_err(|e| CommandError::new("internal", e.to_string()))?
        .map_err(io_error)?;
    Ok(Some(Exported {
        file: file_label(&path),
        downloads,
    }))
}

#[derive(Debug, Serialize)]
pub struct BackupPreview {
    file: String,
    #[serde(flatten)]
    summary: BackupSummary,
}

/// Pick a backup and check it; the preview says what importing it brings.
#[tauri::command]
pub async fn inspect_backup(
    app: AppHandle,
    window: tauri::WebviewWindow,
    manager: State<'_, Manager>,
    pending: State<'_, PendingImports>,
) -> CmdResult<Option<BackupPreview>> {
    let Some(path) = open_dialog(&window, &app, ("JSON", &["json"])).await else {
        return Ok(None);
    };
    let bytes = read_capped(path.clone(), backup::MAX_BACKUP_BYTES).await?;
    let parsed = tauri::async_runtime::spawn_blocking(move || Backup::parse(&bytes))
        .await
        .map_err(|e| CommandError::new("internal", e.to_string()))??;
    let summary = manager.backup_summary(&parsed);
    *pending.backup.lock() = Some(parsed);
    Ok(Some(BackupPreview {
        file: file_label(&path),
        summary,
    }))
}

/// Import what the preview showed, as far as the user chose.
#[tauri::command]
pub fn import_backup(
    app: AppHandle,
    manager: State<'_, Manager>,
    pending: State<'_, PendingImports>,
    choice: ImportChoice,
) -> CmdResult<ImportReport> {
    let backup = pending.backup.lock().take().ok_or_else(|| {
        CommandError::new("not_found", "Pick the backup again; it is no longer open")
    })?;
    let report = manager.import_backup(&backup, choice)?;
    if report.settings {
        // As saving the settings does: the login item follows at once.
        crate::sync_autostart(&app, manager.settings().start_with_system)
            .map_err(|e| CommandError::new("autostart", e))?;
    }
    Ok(report)
}

/// Where to read IDM's list from.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdmSource {
    /// IDM's own list on this computer (Windows registry, read only).
    Computer,
    /// A list exported from IDM (text or `.ef2`).
    File,
}

/// One row of the IDM preview.
#[derive(Debug, Serialize)]
pub struct IdmRow {
    #[serde(flatten)]
    entry: IdmEntry,
    /// The link is in Downpipe's list already; importing skips it.
    listed: bool,
}

#[derive(Debug, Serialize)]
pub struct IdmList {
    /// The exported file's name; none for the computer's list.
    file: Option<String>,
    rows: Vec<IdmRow>,
}

/// Read IDM's list for the preview. `None` when the user closed the file
/// dialog.
#[tauri::command]
pub async fn read_idm_list(
    app: AppHandle,
    window: tauri::WebviewWindow,
    manager: State<'_, Manager>,
    pending: State<'_, PendingImports>,
    source: IdmSource,
) -> CmdResult<Option<IdmList>> {
    let (file, entries) = match source {
        IdmSource::Computer => {
            let entries = tauri::async_runtime::spawn_blocking(idm::read_registry)
                .await
                .map_err(|e| CommandError::new("internal", e.to_string()))?
                .map_err(|e| CommandError::new("idm_unreadable", e.to_string()))?;
            (None, entries)
        }
        IdmSource::File => {
            let Some(path) = open_dialog(&window, &app, ("IDM", &["txt", "ef2", "lst"])).await
            else {
                return Ok(None);
            };
            let bytes = read_capped(path.clone(), idm::MAX_LIST_BYTES as u64).await?;
            let entries = idm::parse_list(&idm::decode_text(&bytes));
            (Some(file_label(&path)), entries)
        }
    };
    let listed = manager.listed(entries.iter().map(|e| e.url.as_str()));
    let rows = entries
        .iter()
        .cloned()
        .zip(listed)
        .map(|(entry, listed)| IdmRow { entry, listed })
        .collect();
    *pending.idm.lock() = entries;
    Ok(Some(IdmList { file, rows }))
}

/// Add the chosen rows of the IDM preview, paused.
#[tauri::command]
pub fn import_idm(
    manager: State<'_, Manager>,
    pending: State<'_, PendingImports>,
    rows: Vec<usize>,
) -> CmdResult<ImportReport> {
    let entries = std::mem::take(&mut *pending.idm.lock());
    if entries.is_empty() {
        return Err(CommandError::new(
            "not_found",
            "Read IDM's list again; it is no longer open",
        ));
    }
    let mut chosen: Vec<usize> = rows.into_iter().filter(|&i| i < entries.len()).collect();
    chosen.sort_unstable();
    chosen.dedup();
    let downloads = chosen.iter().map(|&i| entries[i].to_download()).collect();
    Ok(manager.import_downloads(downloads)?)
}
