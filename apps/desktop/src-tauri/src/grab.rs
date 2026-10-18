//! The site grabber and the feed subscriptions, as commands and events.
//!
//! A grab runs on the async runtime and reports through `downpipe://grab`
//! (`progress`, then `done` or `error`), tagged with the id `start_grab`
//! returned, so a late event of a closed grab is ignored by the page. Feeds
//! send their whole list on `downpipe://feeds` after every change.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use downpipe_core::Manager;
use downpipe_core::crawl::{GrabOptions, GrabPhase, GrabProgress, GrabReport};
use downpipe_core::feeds::{Feed, FeedCheck, FeedInput, Feeds, FeedsError};
use parking_lot::Mutex;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;

use crate::CommandError;

const GRAB_EVENT: &str = "downpipe://grab";
const FEEDS_EVENT: &str = "downpipe://feeds";
/// Progress reaches the page at most this often; the last one always does.
const PROGRESS_EVERY: Duration = Duration::from_millis(150);

/// Running grabs by id, to stop them.
#[derive(Debug, Default)]
pub struct Grabs {
    running: Mutex<HashMap<String, CancellationToken>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum GrabEvent {
    Progress {
        id: String,
        progress: GrabProgress,
    },
    Done {
        id: String,
        report: GrabReport,
    },
    Error {
        id: String,
        code: &'static str,
        message: String,
    },
}

impl From<FeedsError> for CommandError {
    fn from(err: FeedsError) -> Self {
        Self::new(err.code(), err.to_string())
    }
}

#[tauri::command]
pub fn start_grab(
    app: AppHandle,
    manager: State<'_, Manager>,
    grabs: State<'_, Grabs>,
    options: GrabOptions,
) -> String {
    let id = uuid_like();
    let cancel = CancellationToken::new();
    grabs.running.lock().insert(id.clone(), cancel.clone());
    let client = manager.http_client();
    let task_id = id.clone();
    tauri::async_runtime::spawn(async move {
        let last = Mutex::new(None::<(Instant, GrabPhase)>);
        let progress = |p: &GrabProgress| {
            let mut last = last.lock();
            // A new phase always shows; within one, a few updates a second do.
            if last.is_some_and(|(t, phase)| phase == p.phase && t.elapsed() < PROGRESS_EVERY) {
                return;
            }
            *last = Some((Instant::now(), p.phase));
            let _ = app.emit(
                GRAB_EVENT,
                GrabEvent::Progress {
                    id: task_id.clone(),
                    progress: p.clone(),
                },
            );
        };
        let result = downpipe_core::crawl::grab(&client, options, &progress, cancel).await;
        let event = match result {
            Ok(report) => GrabEvent::Done {
                id: task_id.clone(),
                report,
            },
            Err(err) => GrabEvent::Error {
                id: task_id.clone(),
                code: err.code(),
                message: err.to_string(),
            },
        };
        let _ = app.emit(GRAB_EVENT, event);
        use tauri::Manager as _;
        app.state::<Grabs>().running.lock().remove(&task_id);
    });
    id
}

/// Stop a grab; it still reports what it found until then.
#[tauri::command]
pub fn cancel_grab(grabs: State<'_, Grabs>, id: String) {
    if let Some(token) = grabs.running.lock().get(&id) {
        token.cancel();
    }
}

#[tauri::command]
pub fn list_feeds(feeds: State<'_, Feeds>) -> Vec<Feed> {
    feeds.list()
}

#[derive(Debug, Serialize)]
pub struct Subscribed {
    feed: Feed,
    check: FeedCheck,
}

#[tauri::command]
pub async fn add_feed(
    feeds: State<'_, Feeds>,
    input: FeedInput,
) -> Result<Subscribed, CommandError> {
    let (feed, check) = feeds.add(input).await?;
    Ok(Subscribed { feed, check })
}

#[tauri::command]
pub fn update_feed(
    feeds: State<'_, Feeds>,
    id: String,
    input: FeedInput,
) -> Result<Feed, CommandError> {
    Ok(feeds.update(&id, input)?)
}

#[tauri::command]
pub fn remove_feed(feeds: State<'_, Feeds>, id: String) -> Result<(), CommandError> {
    Ok(feeds.remove(&id)?)
}

#[tauri::command]
pub async fn check_feed(feeds: State<'_, Feeds>, id: String) -> Result<FeedCheck, CommandError> {
    Ok(feeds.check(&id).await?)
}

/// Start checking feeds on their intervals and forward their changes.
pub fn spawn_feeds(app: AppHandle, feeds: Feeds) {
    let mut changes = feeds.subscribe();
    tauri::async_runtime::spawn(async move {
        loop {
            match changes.recv().await {
                Ok(list) => {
                    let _ = app.emit(FEEDS_EVENT, list);
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => return,
            }
        }
    });
    tauri::async_runtime::spawn(feeds.run());
}

/// Unique enough for an in-memory key: time plus a counter.
fn uuid_like() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    format!("grab-{nanos:x}-{}", SEQ.fetch_add(1, Ordering::Relaxed))
}
