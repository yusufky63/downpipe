//! A newer Downpipe: found once a day while "Check for updates" is on,
//! shown in the window and announced once with a system notification, and on
//! Windows installed when the user asks: the installer is downloaded, checked
//! against the release's SHA-256 list and started, and Downpipe quits so it
//! can replace the program.

use std::time::Duration;

use downpipe_core::Manager;
use downpipe_core::providers::update::Update;
use parking_lot::Mutex;
use tauri::{AppHandle, Emitter, Manager as _};
use tauri_plugin_notification::NotificationExt;

use crate::i18n::{self, Lang};

/// The window hears of a found (or no longer offered) update here.
pub const EVENT: &str = "downpipe://update";
/// Not while the app is still starting and the network may not be up.
const FIRST_CHECK: Duration = Duration::from_secs(120);
const EVERY: Duration = Duration::from_secs(24 * 60 * 60);
/// After a failed check, or while checking is off (the setting is read again).
const RETRY: Duration = Duration::from_secs(60 * 60);

#[derive(Default)]
pub struct UpdateState {
    found: Mutex<Option<Update>>,
    /// The version the notification was shown for, so it is shown once.
    notified: Mutex<Option<String>>,
}

impl UpdateState {
    pub fn found(&self) -> Option<Update> {
        self.found.lock().clone()
    }
}

/// Check now and then for as long as the app runs.
pub fn spawn(app: AppHandle, manager: Manager) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK).await;
        loop {
            let wait = if manager.settings().check_updates {
                match manager.check_for_update().await {
                    Ok(found) => {
                        publish(&app, &manager, found);
                        EVERY
                    }
                    Err(err) => {
                        tracing::info!("could not check for updates: {err}");
                        RETRY
                    }
                }
            } else {
                RETRY
            };
            tokio::time::sleep(wait).await;
        }
    });
}

/// Remember what a check found, tell the window, and announce a new version
/// once.
pub fn publish(app: &AppHandle, manager: &Manager, found: Option<Update>) {
    let state = app.state::<UpdateState>();
    *state.found.lock() = found.clone();
    let _ = app.emit(EVENT, &found);
    let Some(update) = found else { return };
    let mut notified = state.notified.lock();
    if notified.as_deref() == Some(update.version.as_str()) {
        return;
    }
    *notified = Some(update.version.clone());
    let texts = Lang::resolve(&manager.settings().language).texts();
    let _ = app
        .notification()
        .builder()
        .title(i18n::fill(
            texts.update_title,
            &[("version", &update.version)],
        ))
        .body(texts.update_body)
        .show();
}
