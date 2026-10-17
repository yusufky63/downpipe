//! Everything that reacts to the queue while the window may be hidden:
//! forwarding events to the web view, finish notifications, the tray
//! tooltip, the time-of-day schedule and "when all downloads finish".

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use downpipe_core::schedule::{DoneWatcher, LocalStamp, ScheduleAction, due};
use downpipe_core::{Manager, ManagerEvent, TaskSnapshot, TaskState};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager as _};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::broadcast::error::RecvError;

use crate::{power, tray};

/// Engine events are re-emitted to the web view under this name.
pub const EVENT: &str = "downpipe://event";
/// Sent when the web view fell behind and should reload the full list.
pub const RESYNC: &str = "downpipe://resync";
/// The "when done" action or its countdown changed.
pub const WHEN_DONE: &str = "downpipe://when-done";

const COUNTDOWN: Duration = Duration::from_secs(60);
const SCHEDULE_EVERY: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WhenDone {
    #[default]
    Nothing,
    Quit,
    Sleep,
    Shutdown,
}

/// What the web view shows: the armed action and, while counting down, the
/// seconds left.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct WhenDoneView {
    pub action: WhenDone,
    pub seconds_left: Option<u64>,
}

#[derive(Debug, Default)]
struct WhenDoneState {
    action: WhenDone,
    watcher: DoneWatcher,
    /// Bumped on every change, so a stale countdown task can tell it was
    /// canceled or superseded.
    generation: u64,
    deadline: Option<std::time::Instant>,
}

/// Shared state, managed by Tauri.
#[derive(Debug, Default)]
pub struct Background {
    when_done: Mutex<WhenDoneState>,
    /// Last known state and speed per download, from the event stream.
    tasks: Mutex<HashMap<String, (TaskState, u64)>>,
    told_about_tray: AtomicBool,
}

impl Background {
    pub fn when_done(&self) -> WhenDoneView {
        let st = self.when_done.lock();
        WhenDoneView {
            action: st.action,
            seconds_left: st.deadline.map(|d| {
                d.saturating_duration_since(std::time::Instant::now())
                    .as_secs()
            }),
        }
    }

    pub fn set_when_done(&self, app: &AppHandle, action: WhenDone) {
        let active = self.active();
        {
            let mut st = self.when_done.lock();
            st.action = action;
            st.generation += 1;
            st.deadline = None;
            st.watcher.arm(active);
        }
        self.publish_when_done(app);
    }

    /// Run the armed action now instead of waiting for the countdown.
    pub fn when_done_now(&self, app: &AppHandle) {
        let action = {
            let mut st = self.when_done.lock();
            if st.deadline.is_none() {
                return;
            }
            st.generation += 1;
            st.deadline = None;
            std::mem::take(&mut st.action)
        };
        self.publish_when_done(app);
        perform(app, action);
    }

    /// The first time the window is closed into the tray, say so: otherwise
    /// it looks as if Downpipe quit while it keeps downloading.
    pub fn closed_to_tray(&self, app: &AppHandle) {
        if !self.told_about_tray.swap(true, Ordering::Relaxed) {
            let _ = app
                .notification()
                .builder()
                .title("Downpipe is still running")
                .body("Downloads continue and the browser can still send links. Quit from the tray icon.")
                .show();
        }
    }

    fn active(&self) -> usize {
        self.tasks
            .lock()
            .values()
            .filter(|(s, _)| s.is_running() || *s == TaskState::Queued)
            .count()
    }

    fn seed(&self, tasks: &[TaskSnapshot]) {
        let mut map = self.tasks.lock();
        map.clear();
        for t in tasks {
            map.insert(t.record.id.clone(), (t.record.state, t.live.speed));
        }
    }

    fn publish_when_done(&self, app: &AppHandle) {
        let _ = app.emit(WHEN_DONE, self.when_done());
    }

    /// Feed the active count to the watcher; start the countdown when it fires.
    fn check_when_done(&self, app: &AppHandle) {
        let active = self.active();
        let (action, generation) = {
            let mut st = self.when_done.lock();
            if st.action == WhenDone::Nothing
                || st.deadline.is_some()
                || !st.watcher.observe(active)
            {
                return;
            }
            st.generation += 1;
            st.deadline = Some(std::time::Instant::now() + COUNTDOWN);
            (st.action, st.generation)
        };
        self.publish_when_done(app);
        // The countdown must be visible to be cancelled.
        tray::show_main(app);
        let what = match action {
            WhenDone::Nothing => return,
            WhenDone::Quit => "quit",
            WhenDone::Sleep => "put this computer to sleep",
            WhenDone::Shutdown => "shut down this computer",
        };
        let _ = app
            .notification()
            .builder()
            .title("All downloads finished")
            .body(format!(
                "Downpipe will {what} in {} seconds. Open Downpipe to cancel.",
                COUNTDOWN.as_secs()
            ))
            .show();
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(COUNTDOWN).await;
            let bg = app.state::<Background>();
            let action = {
                let mut st = bg.when_done.lock();
                if st.generation != generation || st.deadline.is_none() {
                    return;
                }
                st.generation += 1;
                st.deadline = None;
                std::mem::take(&mut st.action)
            };
            bg.publish_when_done(&app);
            perform(&app, action);
        });
    }
}

fn perform(app: &AppHandle, action: WhenDone) {
    let result = match action {
        WhenDone::Nothing => Ok(()),
        WhenDone::Quit => {
            app.exit(0);
            Ok(())
        }
        WhenDone::Sleep => power::sleep(),
        WhenDone::Shutdown => power::shutdown(),
    };
    if let Err(err) = result {
        tracing::warn!("when-done action {action:?} failed: {err}");
        let _ = app
            .notification()
            .builder()
            .title("Downpipe could not finish the job")
            .body(format!("The system did not allow it: {err}"))
            .show();
    }
}

/// Start the background loop. Call once, after `Background` is managed.
pub fn spawn(app: AppHandle, manager: Manager) {
    let mut events = manager.subscribe();
    app.state::<Background>().seed(&manager.list());
    tauri::async_runtime::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut last_tooltip = String::new();
        let mut last_schedule_check = std::time::Instant::now();
        let mut prev_stamp = LocalStamp::now();
        loop {
            tokio::select! {
                event = events.recv() => match event {
                    Ok(event) => on_event(&app, &manager, &event),
                    Err(RecvError::Lagged(_)) => {
                        app.state::<Background>().seed(&manager.list());
                        let _ = app.emit(RESYNC, ());
                    }
                    Err(RecvError::Closed) => break,
                },
                _ = tick.tick() => {
                    let tooltip = tooltip_text(&app.state::<Background>());
                    if tooltip != last_tooltip {
                        tray::set_tooltip(&app, &tooltip);
                        last_tooltip = tooltip;
                    }
                    if last_schedule_check.elapsed() >= SCHEDULE_EVERY {
                        last_schedule_check = std::time::Instant::now();
                        let now = LocalStamp::now();
                        run_schedule(&app, &manager, prev_stamp, now);
                        prev_stamp = now;
                    }
                }
            }
        }
    });
}

fn on_event(app: &AppHandle, manager: &Manager, event: &ManagerEvent) {
    let _ = app.emit(EVENT, event);
    let bg = app.state::<Background>();
    match event {
        ManagerEvent::Updated { task } => {
            let state = task.record.state;
            let prev = bg
                .tasks
                .lock()
                .insert(task.record.id.clone(), (state, task.live.speed))
                .map(|(s, _)| s);
            let finished_now = matches!(state, TaskState::Completed | TaskState::Failed)
                && prev.is_some_and(|p| p != state);
            if finished_now && manager.settings().notify_on_finish && !window_in_front(app) {
                notify_finished(app, task);
            }
        }
        ManagerEvent::Removed { id } => {
            bg.tasks.lock().remove(id);
        }
        ManagerEvent::Settings { .. } => {}
    }
    bg.check_when_done(app);
}

fn window_in_front(app: &AppHandle) -> bool {
    app.get_webview_window("main").is_some_and(|w| {
        w.is_visible().unwrap_or(false)
            && !w.is_minimized().unwrap_or(false)
            && w.is_focused().unwrap_or(false)
    })
}

fn notify_finished(app: &AppHandle, task: &TaskSnapshot) {
    let r = &task.record;
    let name = r.file_name.clone().unwrap_or_else(|| r.url.clone());
    let (title, body) = if r.state == TaskState::Completed {
        let size = r.total.map(format_bytes).unwrap_or_default();
        let place = r
            .dir
            .file_name()
            .map(|d| d.to_string_lossy().into_owned())
            .unwrap_or_default();
        (
            "Download complete".to_string(),
            [name, size, place]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" · "),
        )
    } else {
        let why = r.error.clone().unwrap_or_default();
        ("Download failed".to_string(), format!("{name}: {why}"))
    };
    let _ = app.notification().builder().title(title).body(body).show();
}

fn run_schedule(app: &AppHandle, manager: &Manager, prev: LocalStamp, now: LocalStamp) {
    let settings = manager.settings();
    let (title, body) = match due(&settings.schedule, prev, now) {
        Some(ScheduleAction::Start) => {
            manager.resume_all();
            (
                "Scheduled downloads started",
                "Paused and failed downloads are running again.",
            )
        }
        Some(ScheduleAction::Stop) => {
            manager.pause_all();
            (
                "Scheduled stop",
                "All downloads are paused until the next start time.",
            )
        }
        None => return,
    };
    tracing::info!("{title}");
    let _ = app.notification().builder().title(title).body(body).show();
}

fn tooltip_text(bg: &Background) -> String {
    let (running, speed) = bg
        .tasks
        .lock()
        .values()
        .filter(|(s, _)| s.is_running())
        .fold((0usize, 0u64), |(n, total), (_, speed)| {
            (n + 1, total + speed)
        });
    match running {
        0 => "Downpipe".to_string(),
        1 => format!("Downpipe: 1 download, {}/s", format_bytes(speed)),
        n => format!("Downpipe: {n} downloads, {}/s", format_bytes(speed)),
    }
}

/// Binary units with one decimal, matching the UI.
pub fn format_bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    if n < 1024 {
        return format!("{n} B");
    }
    let mut value = n as f64;
    let mut unit = 0;
    while value >= 1023.95 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1536), "1.5 KiB");
        assert_eq!(format_bytes(1024 * 1024 - 1), "1.0 MiB");
    }

    #[test]
    fn tooltip_counts_running_downloads() {
        let bg = Background::default();
        assert_eq!(tooltip_text(&bg), "Downpipe");
        bg.tasks
            .lock()
            .insert("a".into(), (TaskState::Downloading, 2 * 1024 * 1024));
        bg.tasks.lock().insert("b".into(), (TaskState::Paused, 0));
        assert_eq!(tooltip_text(&bg), "Downpipe: 1 download, 2.0 MiB/s");
        bg.tasks
            .lock()
            .insert("c".into(), (TaskState::Waiting, 1024 * 1024));
        assert_eq!(tooltip_text(&bg), "Downpipe: 2 downloads, 3.0 MiB/s");
        assert_eq!(bg.active(), 2);
    }

    #[test]
    fn when_done_round_trips_as_snake_case() {
        assert_eq!(
            serde_json::to_string(&WhenDone::Shutdown).unwrap(),
            "\"shutdown\""
        );
        let parsed: WhenDone = serde_json::from_str("\"sleep\"").unwrap();
        assert_eq!(parsed, WhenDone::Sleep);
    }
}
