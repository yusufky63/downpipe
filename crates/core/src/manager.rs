//! The download queue: starts downloads up to the concurrency limit, tracks
//! their state, persists it, and broadcasts snapshots to UIs.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use downpipe_http::{
    Checksum, DownloadControl, DownloadError, DownloadHandle, DownloadRequest, Engine,
    EngineConfig, Outcome, RateLimiter, filename, meta,
};
use parking_lot::Mutex;
use serde::Serialize;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use crate::settings::Settings;
use crate::store::Store;
use crate::task::{AddOptions, Live, TaskId, TaskRecord, TaskSnapshot, TaskState};

const DB_FILE: &str = "downpipe.db";
/// Progress is written to SQLite at most this often while a task runs; the
/// meta file next to the download is the precise record anyway.
const PERSIST_EVERY: Duration = Duration::from_secs(5);
const SHUTDOWN_GRACE: Duration = Duration::from_secs(10);

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error(transparent)]
    Download(#[from] DownloadError),
    #[error("database error: {0}")]
    Store(#[from] rusqlite::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("no download with id {0}")]
    NotFound(TaskId),
    #[error("this URL is already in the list")]
    Duplicate(TaskId),
    #[error("cannot {action} a download that is {state}")]
    InvalidState {
        action: &'static str,
        state: &'static str,
    },
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
}

impl CoreError {
    /// Stable name UIs and the browser bridge branch on.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Duplicate(_) => "duplicate",
            Self::NotFound(_) => "not_found",
            Self::InvalidState { .. } => "invalid_state",
            Self::InvalidUrl(_) => "invalid_url",
            Self::Download(e) => e.kind(),
            Self::Store(_) | Self::Io(_) => "internal",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ManagerEvent {
    // Boxed: a snapshot is large and events are moved through a channel.
    Updated { task: Box<TaskSnapshot> },
    Removed { id: TaskId },
    Settings { settings: Settings },
}

#[derive(Debug, Clone)]
pub struct Manager {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    engine: Engine,
    /// UI frameworks call into the manager from threads that are not part of
    /// any Tokio runtime (Tauri runs sync commands on the main thread), so
    /// downloads are always spawned on the runtime the manager was opened on.
    runtime: tokio::runtime::Handle,
    state: Mutex<State>,
    events: broadcast::Sender<ManagerEvent>,
}

#[derive(Debug)]
struct State {
    store: Store,
    settings: Settings,
    tasks: Vec<Entry>,
    shutting_down: bool,
}

#[derive(Debug)]
struct Entry {
    record: TaskRecord,
    live: Live,
    /// The URL as given, credentials included. Memory only.
    url: String,
    /// Memory only, like `url`.
    headers: Vec<(String, String)>,
    control: Option<DownloadControl>,
    supervisor: Option<JoinHandle<()>>,
    after_stop: AfterStop,
    last_persist: Instant,
}

/// What a stop request should turn into once the engine has stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AfterStop {
    Pause,
    Requeue,
    Cancel,
    Remove,
}

impl Manager {
    /// Open (or create) the catalog in `data_dir` and restart the queue.
    pub async fn open(data_dir: &Path) -> Result<Self, CoreError> {
        Self::open_with(data_dir, EngineConfig::default()).await
    }

    pub async fn open_with(data_dir: &Path, config: EngineConfig) -> Result<Self, CoreError> {
        let path = data_dir.join(DB_FILE);
        let (store, settings, records) = tokio::task::spawn_blocking(move || {
            let store = Store::open(&path)?;
            let settings = store.load_settings()?.unwrap_or_default().normalized();
            let records = store.all()?;
            Ok::<_, rusqlite::Error>((store, settings, records))
        })
        .await
        .map_err(|e| CoreError::Io(std::io::Error::other(e)))??;

        let limiter = Arc::new(RateLimiter::new(settings.speed_limit));
        let engine = Engine::with_limiter(config, limiter)?;

        let mut tasks = Vec::with_capacity(records.len());
        for mut record in records {
            // A task that was running when the app stopped (or crashed)
            // goes back into the queue; its meta file knows the real progress.
            if record.state.is_running() {
                record.state = TaskState::Queued;
            }
            refresh_from_meta(&mut record);
            store.upsert(&record)?;
            tasks.push(Entry {
                url: record.url.clone(),
                record,
                live: Live::default(),
                headers: Vec::new(),
                control: None,
                supervisor: None,
                after_stop: AfterStop::Pause,
                last_persist: Instant::now(),
            });
        }

        let (events, _) = broadcast::channel(512);
        let manager = Self {
            inner: Arc::new(Inner {
                engine,
                runtime: tokio::runtime::Handle::current(),
                state: Mutex::new(State {
                    store,
                    settings,
                    tasks,
                    shutting_down: false,
                }),
                events,
            }),
        };
        manager.inner.schedule();
        Ok(manager)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ManagerEvent> {
        self.inner.events.subscribe()
    }

    pub fn list(&self) -> Vec<TaskSnapshot> {
        let st = self.inner.state.lock();
        st.tasks.iter().map(Entry::snapshot).collect()
    }

    pub fn get(&self, id: &str) -> Option<TaskSnapshot> {
        let st = self.inner.state.lock();
        st.tasks
            .iter()
            .find(|e| e.record.id == id)
            .map(Entry::snapshot)
    }

    pub fn settings(&self) -> Settings {
        self.inner.state.lock().settings.clone()
    }

    pub fn add(&self, opts: AddOptions) -> Result<TaskSnapshot, CoreError> {
        let url =
            url::Url::parse(opts.url.trim()).map_err(|e| CoreError::InvalidUrl(e.to_string()))?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(CoreError::InvalidUrl(format!(
                "`{}` links are not supported; use http or https",
                url.scheme()
            )));
        }
        let checksum = match opts
            .checksum
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            Some(raw) => Some(raw.parse::<Checksum>()?.to_string()),
            None => None,
        };
        downpipe_http::build_headers(&opts.headers)?;
        let key = downpipe_http::redact_url(&url);

        let snapshot = {
            let mut st = self.inner.state.lock();
            if let Some(dup) = st
                .tasks
                .iter()
                .find(|e| e.record.url == key && !e.record.state.is_finished())
            {
                return Err(CoreError::Duplicate(dup.record.id.clone()));
            }
            let now = unix_now();
            let record = TaskRecord {
                id: uuid::Uuid::new_v4().to_string(),
                url: key,
                dir: opts.dir.unwrap_or_else(|| st.settings.download_dir.clone()),
                file_name: opts
                    .file_name
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(filename::sanitize),
                path: None,
                state: if opts.paused {
                    TaskState::Paused
                } else {
                    TaskState::Queued
                },
                total: None,
                downloaded: 0,
                error: None,
                error_kind: None,
                checksum,
                max_connections: opts.max_connections.map(|c| c.clamp(1, 32)),
                created_at: now,
                updated_at: now,
                completed_at: None,
            };
            st.store.upsert(&record)?;
            let entry = Entry {
                url: url.to_string(),
                record,
                live: Live::default(),
                headers: opts.headers,
                control: None,
                supervisor: None,
                after_stop: AfterStop::Pause,
                last_persist: Instant::now(),
            };
            let snapshot = entry.snapshot();
            st.tasks.push(entry);
            snapshot
        };
        self.inner.emit(ManagerEvent::Updated {
            task: Box::new(snapshot.clone()),
        });
        self.inner.schedule();
        Ok(snapshot)
    }

    pub fn pause(&self, id: &str) -> Result<(), CoreError> {
        self.inner.with_entry(id, |st, entry| {
            if let Some(control) = &entry.control {
                entry.after_stop = AfterStop::Pause;
                control.pause();
                return Ok(None);
            }
            match entry.record.state {
                TaskState::Queued => {
                    entry.set_state(TaskState::Paused);
                    st.store.upsert(&entry.record)?;
                    Ok(Some(entry.snapshot()))
                }
                TaskState::Paused => Ok(None),
                state => Err(invalid("pause", state)),
            }
        })
    }

    /// Queue a paused, failed or canceled download again.
    pub fn resume(&self, id: &str) -> Result<(), CoreError> {
        self.inner.with_entry(id, |st, entry| {
            if entry.control.is_some() {
                // Stopping right now: make it come straight back.
                entry.after_stop = AfterStop::Requeue;
                return Ok(None);
            }
            match entry.record.state {
                TaskState::Queued => Ok(None),
                s if s.can_resume() => {
                    entry.record.error = None;
                    entry.record.error_kind = None;
                    entry.set_state(TaskState::Queued);
                    st.store.upsert(&entry.record)?;
                    Ok(Some(entry.snapshot()))
                }
                state => Err(invalid("resume", state)),
            }
        })?;
        self.inner.schedule();
        Ok(())
    }

    /// Stop and delete partial data. Completed files are left alone.
    pub fn cancel(&self, id: &str) -> Result<(), CoreError> {
        self.inner.with_entry(id, |st, entry| {
            if let Some(control) = &entry.control {
                entry.after_stop = AfterStop::Cancel;
                control.cancel();
                return Ok(None);
            }
            match entry.record.state {
                TaskState::Completed => Err(invalid("cancel", TaskState::Completed)),
                TaskState::Canceled => Ok(None),
                _ => {
                    remove_partial(&entry.record);
                    entry.record.downloaded = 0;
                    entry.live = Live::default();
                    entry.set_state(TaskState::Canceled);
                    st.store.upsert(&entry.record)?;
                    Ok(Some(entry.snapshot()))
                }
            }
        })
    }

    /// Forget a download; with `delete_files`, also delete what it wrote.
    pub fn remove(&self, id: &str, delete_files: bool) -> Result<(), CoreError> {
        let removed = {
            let mut st = self.inner.state.lock();
            let idx = st
                .tasks
                .iter()
                .position(|e| e.record.id == id)
                .ok_or_else(|| CoreError::NotFound(id.to_string()))?;
            let entry = &mut st.tasks[idx];
            if let Some(control) = &entry.control {
                entry.after_stop = AfterStop::Remove;
                if delete_files {
                    control.cancel();
                } else {
                    control.pause();
                }
                false
            } else {
                if delete_files {
                    match (&entry.record.state, &entry.record.path) {
                        (TaskState::Completed, Some(path)) => {
                            let _ = std::fs::remove_file(path);
                        }
                        _ => remove_partial(&entry.record),
                    }
                }
                st.store.delete(id)?;
                st.tasks.remove(idx);
                true
            }
        };
        if removed {
            self.inner
                .emit(ManagerEvent::Removed { id: id.to_string() });
        }
        Ok(())
    }

    pub fn pause_all(&self) {
        let ids = self.ids(|s| s.is_running() || s == TaskState::Queued);
        for id in ids {
            let _ = self.pause(&id);
        }
    }

    pub fn resume_all(&self) {
        let ids = self.ids(|s| s == TaskState::Paused || s == TaskState::Failed);
        for id in ids {
            let _ = self.resume(&id);
        }
    }

    /// Remove finished (completed or canceled) entries from the list.
    pub fn clear_finished(&self) -> Result<(), CoreError> {
        for id in self.ids(TaskState::is_finished) {
            self.remove(&id, false)?;
        }
        Ok(())
    }

    pub fn update_settings(&self, settings: Settings) -> Result<Settings, CoreError> {
        let settings = settings.normalized();
        {
            let mut st = self.inner.state.lock();
            st.store.save_settings(&settings)?;
            st.settings = settings.clone();
        }
        self.inner.engine.limiter().set_rate(settings.speed_limit);
        self.inner.emit(ManagerEvent::Settings {
            settings: settings.clone(),
        });
        self.inner.schedule();
        Ok(settings)
    }

    /// Stop everything cleanly so it resumes on the next start.
    pub async fn shutdown(&self) {
        let supervisors: Vec<JoinHandle<()>> = {
            let mut st = self.inner.state.lock();
            st.shutting_down = true;
            st.tasks
                .iter_mut()
                .filter_map(|e| {
                    let control = e.control.as_ref()?;
                    e.after_stop = AfterStop::Requeue;
                    control.pause();
                    e.supervisor.take()
                })
                .collect()
        };
        let all = futures_join_all(supervisors);
        if tokio::time::timeout(SHUTDOWN_GRACE, all).await.is_err() {
            tracing::warn!("some downloads did not stop within {SHUTDOWN_GRACE:?}");
        }
    }

    fn ids(&self, pred: impl Fn(TaskState) -> bool) -> Vec<TaskId> {
        let st = self.inner.state.lock();
        st.tasks
            .iter()
            .filter(|e| pred(e.record.state))
            .map(|e| e.record.id.clone())
            .collect()
    }
}

impl Inner {
    fn emit(&self, event: ManagerEvent) {
        // No receivers is fine (headless use).
        let _ = self.events.send(event);
    }

    fn with_entry(
        &self,
        id: &str,
        f: impl FnOnce(&mut StateParts<'_>, &mut Entry) -> Result<Option<TaskSnapshot>, CoreError>,
    ) -> Result<(), CoreError> {
        let snapshot = {
            let mut st = self.state.lock();
            let State { store, tasks, .. } = &mut *st;
            let entry = tasks
                .iter_mut()
                .find(|e| e.record.id == id)
                .ok_or_else(|| CoreError::NotFound(id.to_string()))?;
            f(&mut StateParts { store }, entry)?
        };
        if let Some(task) = snapshot {
            self.emit(ManagerEvent::Updated {
                task: Box::new(task),
            });
        }
        Ok(())
    }

    /// Start queued tasks while there are free slots.
    fn schedule(self: &Arc<Self>) {
        let _runtime = self.runtime.enter();
        let mut started = Vec::new();
        {
            let mut st = self.state.lock();
            if st.shutting_down {
                return;
            }
            let limit = st.settings.max_concurrent_downloads as usize;
            let per_download = st.settings.max_connections_per_download;
            let mut running = st.tasks.iter().filter(|e| e.control.is_some()).count();
            let State { store, tasks, .. } = &mut *st;
            for entry in tasks.iter_mut() {
                if running >= limit {
                    break;
                }
                if entry.record.state != TaskState::Queued || entry.control.is_some() {
                    continue;
                }
                let handle = self.engine.start(entry.request(per_download));
                entry.control = Some(handle.control());
                entry.after_stop = AfterStop::Pause;
                entry.record.error = None;
                entry.record.error_kind = None;
                entry.set_state(TaskState::Starting);
                if let Err(e) = store.upsert(&entry.record) {
                    tracing::warn!("could not persist task: {e}");
                }
                let id = entry.record.id.clone();
                let inner = self.clone();
                entry.supervisor = Some(tokio::spawn(
                    async move { inner.supervise(id, handle).await },
                ));
                started.push(entry.snapshot());
                running += 1;
            }
        }
        for task in started {
            self.emit(ManagerEvent::Updated {
                task: Box::new(task),
            });
        }
    }

    async fn supervise(self: Arc<Self>, id: TaskId, handle: DownloadHandle) {
        let mut rx = handle.progress();
        let forward = {
            let inner = self.clone();
            let id = id.clone();
            async move {
                while rx.changed().await.is_ok() {
                    let progress = rx.borrow_and_update().clone();
                    inner.on_progress(&id, &progress);
                }
            }
        };
        let ((), result) = tokio::join!(forward, handle.wait());
        self.on_finished(&id, result);
        self.schedule();
    }

    fn on_progress(&self, id: &str, p: &downpipe_http::Progress) {
        let snapshot = {
            let mut st = self.state.lock();
            let State { store, tasks, .. } = &mut *st;
            let Some(entry) = tasks.iter_mut().find(|e| e.record.id == id) else {
                return;
            };
            if entry.control.is_none() {
                return;
            }
            let state = TaskState::from_phase(p.phase);
            let state_changed = entry.record.state != state;
            let r = &mut entry.record;
            r.state = state;
            if p.file_name.is_some() {
                r.file_name.clone_from(&p.file_name);
            }
            if p.path.is_some() {
                r.path.clone_from(&p.path);
            }
            r.total = p.total.or(r.total);
            r.downloaded = p.downloaded;
            entry.live = Live::from_progress(p);
            if state_changed || entry.last_persist.elapsed() >= PERSIST_EVERY {
                entry.record.updated_at = unix_now();
                entry.last_persist = Instant::now();
                if let Err(e) = store.upsert(&entry.record) {
                    tracing::warn!("could not persist progress: {e}");
                }
            }
            entry.snapshot()
        };
        self.emit(ManagerEvent::Updated {
            task: Box::new(snapshot),
        });
    }

    fn on_finished(&self, id: &str, result: Result<Outcome, DownloadError>) {
        let event = {
            let mut st = self.state.lock();
            let State { store, tasks, .. } = &mut *st;
            let Some(idx) = tasks.iter().position(|e| e.record.id == id) else {
                return;
            };
            let entry = &mut tasks[idx];
            entry.control = None;
            entry.supervisor = None;
            entry.live.speed = 0;
            entry.live.connections = 0;
            let after = entry.after_stop;

            if after == AfterStop::Remove && !matches!(result, Ok(Outcome::Completed { .. })) {
                if let Err(e) = store.delete(id) {
                    tracing::warn!("could not delete task: {e}");
                }
                tasks.remove(idx);
                drop(st);
                self.emit(ManagerEvent::Removed { id: id.to_string() });
                return;
            }

            let r = &mut entry.record;
            match result {
                Ok(Outcome::Completed { path, bytes, .. }) => {
                    r.file_name = path.file_name().map(|n| n.to_string_lossy().into_owned());
                    r.path = Some(path);
                    r.total = Some(bytes);
                    r.downloaded = bytes;
                    r.completed_at = Some(unix_now());
                    r.error = None;
                    r.error_kind = None;
                    entry.live.segments.clear();
                    entry.set_state(TaskState::Completed);
                }
                Ok(Outcome::Paused {
                    downloaded, total, ..
                }) => {
                    r.downloaded = downloaded;
                    r.total = total.or(r.total);
                    let next = if after == AfterStop::Requeue {
                        TaskState::Queued
                    } else {
                        TaskState::Paused
                    };
                    entry.set_state(next);
                }
                Ok(Outcome::Canceled) => {
                    r.downloaded = 0;
                    entry.live = Live::default();
                    entry.set_state(TaskState::Canceled);
                }
                Err(err) => {
                    tracing::info!(task = id, kind = err.kind(), "download failed: {err}");
                    r.error = Some(err.to_string());
                    r.error_kind = Some(err.kind().to_string());
                    entry.set_state(TaskState::Failed);
                }
            }
            if let Err(e) = store.upsert(&entry.record) {
                tracing::warn!("could not persist task: {e}");
            }
            ManagerEvent::Updated {
                task: Box::new(entry.snapshot()),
            }
        };
        self.emit(event);
    }
}

/// Borrowed pieces of [`State`] handed to entry callbacks.
struct StateParts<'a> {
    store: &'a mut Store,
}

impl Entry {
    fn snapshot(&self) -> TaskSnapshot {
        TaskSnapshot::new(&self.record, &self.live)
    }

    fn set_state(&mut self, state: TaskState) {
        self.record.state = state;
        self.record.updated_at = unix_now();
        self.last_persist = Instant::now();
    }

    fn request(&self, per_download: u32) -> DownloadRequest {
        let mut req = DownloadRequest::new(self.url.clone(), self.record.dir.clone());
        // Once the engine picked a name, keep it: that is how the resume
        // data next to the file is found again.
        req.file_name.clone_from(&self.record.file_name);
        req.checksum = self.record.checksum.as_deref().and_then(|c| c.parse().ok());
        req.headers.clone_from(&self.headers);
        req.max_connections = Some(self.record.max_connections.unwrap_or(per_download) as usize);
        req
    }
}

fn invalid(action: &'static str, state: TaskState) -> CoreError {
    CoreError::InvalidState {
        action,
        state: state.as_str(),
    }
}

fn final_path(record: &TaskRecord) -> Option<PathBuf> {
    record
        .path
        .clone()
        .or_else(|| record.file_name.as_ref().map(|n| record.dir.join(n)))
}

fn remove_partial(record: &TaskRecord) {
    if let Some(path) = final_path(record) {
        let _ = std::fs::remove_file(meta::part_path(&path));
        meta::remove(&path);
    }
}

/// Bring the displayed progress in line with what is actually on disk.
fn refresh_from_meta(record: &mut TaskRecord) {
    if record.state.is_finished() {
        return;
    }
    let Some(path) = final_path(record) else {
        return;
    };
    if let Ok(m) = meta::load(&meta::meta_path(&path)) {
        record.downloaded = m.completed_bytes();
        record.total = Some(m.total_size);
    } else if !meta::part_path(&path).exists() {
        record.downloaded = 0;
    }
}

async fn futures_join_all(handles: Vec<JoinHandle<()>>) {
    for handle in handles {
        let _ = handle.await;
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}
