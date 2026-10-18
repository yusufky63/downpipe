//! The download queue: starts downloads up to the concurrency limit, tracks
//! their state, persists it, and broadcasts snapshots to UIs.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use downpipe_http::media::{self, MediaKind, MediaRequest};
use downpipe_http::{
    Checksum, DownloadControl, DownloadError, DownloadHandle, DownloadRequest, Engine,
    EngineConfig, Outcome, RateLimiter, filename, meta,
};
use downpipe_providers::install::{self, Tool};
use downpipe_providers::{SiteRequest, ToolConfig, Tools, ToolsStatus, files as site_files, pages};
use parking_lot::Mutex;
use serde::Serialize;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use crate::rules::{self, RefusedHost, RuleProxy, SiteRule};
use crate::settings::{MAIN_QUEUE, Settings};
use crate::store::Store;
use crate::task::{
    AddOptions, ConfirmOptions, Live, MediaChoices, MediaInfo, OnDuplicate, QueueMove, SiteChoices,
    SiteInfo, SiteOptions, Swarm, TaskId, TaskRecord, TaskSnapshot, TaskState, TorrentChoices,
    TorrentInfo,
};
use crate::torrent::{self, MAX_TORRENT_FILE, Parsed, SeedState};

const DB_FILE: &str = "downpipe.db";
/// Folder in the data folder where yt-dlp and Deno go when the user has
/// Downpipe download them.
const TOOLS_DIR: &str = "tools";
/// Progress is written to SQLite at most this often while a task runs; the
/// meta file next to the download is the precise record anyway.
const PERSIST_EVERY: Duration = Duration::from_secs(5);
const SHUTDOWN_GRACE: Duration = Duration::from_secs(10);
/// How often downloads paused until a time are checked. Times are compared
/// with the wall clock, so a computer that slept through one still resumes
/// the download when it wakes.
const RESUME_CHECK_EVERY: Duration = Duration::from_secs(5);

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
    #[error("invalid setting: {0}")]
    InvalidSetting(String),
}

impl CoreError {
    /// Stable name UIs and the browser bridge branch on.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Duplicate(_) => "duplicate",
            Self::NotFound(_) => "not_found",
            Self::InvalidState { .. } => "invalid_state",
            Self::InvalidUrl(_) => "invalid_url",
            Self::InvalidSetting(_) => "invalid_setting",
            Self::Download(e) => e.kind(),
            Self::Store(_) | Self::Io(_) => "internal",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ManagerEvent {
    // Boxed: snapshots and settings are large and events are moved
    // through a channel.
    Updated { task: Box<TaskSnapshot> },
    Removed { id: TaskId },
    Settings { settings: Box<Settings> },
}

#[derive(Debug, Clone)]
pub struct Manager {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    /// Rebuilt when the proxy or user agent changes; downloads already
    /// running keep the engine (and connections) they started with.
    engine: Mutex<Engine>,
    base_config: EngineConfig,
    limiter: Arc<RateLimiter>,
    /// UI frameworks call into the manager from threads that are not part of
    /// any Tokio runtime (Tauri runs sync commands on the main thread), so
    /// downloads are always spawned on the runtime the manager was opened on.
    runtime: tokio::runtime::Handle,
    state: Mutex<State>,
    events: broadcast::Sender<ManagerEvent>,
    /// Downpipe's own copies of the site provider's tools.
    tools_dir: PathBuf,
    /// BitTorrent: the session, `.torrent` files and seeding.
    torrents: torrent::Service,
    /// Engines for site rules with a proxy of their own (`""` for none),
    /// derived from `engine` and dropped when it is rebuilt.
    proxied: Mutex<HashMap<String, Engine>>,
    /// One limiter per site rule with a speed limit, by rule id, shared by
    /// every download the rule applies to.
    site_limiters: Mutex<HashMap<String, Arc<RateLimiter>>>,
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
    /// The URL was replaced by the user; the next start must prove the
    /// server still has the same file before resuming.
    relink: bool,
    /// This download's own speed limit, shared with its running job so a
    /// change applies at once.
    limiter: Arc<RateLimiter>,
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
        let torrents = torrent::Service::new(data_dir, &settings);
        let engine = Engine::with_limiter(engine_config(&config, &settings), limiter.clone())?;

        let mut records = records;
        // Positions are only an order: renumber them so they are distinct and
        // dense, and send downloads of queues that no longer exist to main.
        records.sort_by_key(|r| (r.position, r.created_at));
        for (i, r) in records.iter_mut().enumerate() {
            r.position = i as i64;
            if settings.queue(&r.queue).is_none() {
                r.queue = MAIN_QUEUE.into();
            }
        }
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
                limiter: Arc::new(RateLimiter::new(record.speed_limit.unwrap_or(0))),
                record,
                live: Live::default(),
                headers: Vec::new(),
                control: None,
                supervisor: None,
                after_stop: AfterStop::Pause,
                last_persist: Instant::now(),
                relink: false,
            });
        }

        let (events, _) = broadcast::channel(512);
        let manager = Self {
            inner: Arc::new(Inner {
                engine: Mutex::new(engine),
                base_config: config,
                limiter,
                runtime: tokio::runtime::Handle::current(),
                state: Mutex::new(State {
                    store,
                    settings,
                    tasks,
                    shutting_down: false,
                }),
                events,
                tools_dir: data_dir.join(TOOLS_DIR),
                torrents,
                proxied: Mutex::default(),
                site_limiters: Mutex::default(),
            }),
        };
        manager.inner.schedule();
        // Weak, so the timer never keeps a closed manager (and its database)
        // alive.
        let weak = Arc::downgrade(&manager.inner);
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(RESUME_CHECK_EVERY);
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tick.tick().await;
                let Some(inner) = weak.upgrade() else { break };
                Manager { inner }.resume_due(unix_now());
            }
        });
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

    /// The HTTP client downloads use, with the proxy and user agent of the
    /// settings and no cookie store: for the site grabber and feeds.
    pub fn http_client(&self) -> reqwest::Client {
        self.inner.engine.lock().client().clone()
    }

    pub fn add(&self, opts: AddOptions) -> Result<TaskSnapshot, CoreError> {
        // A magnet link, or a `.torrent` file the browser or a link points
        // at: a BitTorrent download. Without BitTorrent support a `.torrent`
        // link is an ordinary file.
        let torrent_name = opts
            .file_name
            .as_deref()
            .is_some_and(torrent::is_torrent_name);
        let magnet = is_magnet(&opts.url);
        if magnet || torrent::AVAILABLE && (torrent_name || torrent::is_torrent_link(&opts.url)) {
            return self.add_torrent(opts, None);
        }
        let url = parse_web_url(&opts.url)?;
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
        // A video page (YouTube) goes to the site provider under one link
        // per video, whatever tracking or timestamp the link carried.
        let page = pages::recognize(&url);
        let url = page.as_ref().map_or(url, |p| p.url.clone());
        let key = downpipe_http::redact_url(&url);
        let site = page.as_ref().map(|p| SiteInfo::new(p.site));
        let media = match site {
            Some(_) => None,
            None => opts
                .media
                .or_else(|| MediaKind::from_url(&url))
                .map(MediaInfo::new),
        };

        let snapshot = {
            let mut st = self.inner.state.lock();
            let same = |e: &&Entry| e.record.url == key;
            // A question about this link already waits: never a second one.
            if let Some(asking) = st
                .tasks
                .iter()
                .filter(same)
                .find(|e| e.live.awaiting_confirmation)
            {
                return Err(CoreError::Duplicate(asking.record.id.clone()));
            }
            let duplicate_of = match (st.tasks.iter().find(same), opts.on_duplicate) {
                (Some(dup), OnDuplicate::Refuse) => {
                    return Err(CoreError::Duplicate(dup.record.id.clone()));
                }
                (Some(dup), OnDuplicate::Ask) => Some(dup.record.id.clone()),
                (_, _) => None,
            };
            let confirm = opts.confirm || duplicate_of.is_some();
            let now = unix_now();
            let file_name = opts
                .file_name
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(filename::sanitize);
            // The real name is known only once the server answers; the hint
            // from the URL or the browser is good enough to pick a folder,
            // and the finished file moves if it turns out to be different.
            let hint = file_name.clone().or_else(|| filename::from_url(&url));
            // A stream ends up as a video whatever its manifest is called,
            // and so does a video page (until its title names it).
            let hint = if site.is_some() {
                Some(video_name(file_name.as_deref().unwrap_or("video")))
            } else if media.is_some() {
                Some(video_name(hint.as_deref().unwrap_or("video")))
            } else {
                hint
            };
            let category = hint
                .as_deref()
                .and_then(|name| st.settings.category_for(name))
                .cloned();
            let rule = st.settings.site_rule_for(&url).cloned();
            let rule_dir = rule
                .as_ref()
                .filter(|r| !r.download_dir.is_empty())
                .map(|r| st.settings.resolve_dir(&r.download_dir));
            // A rule's folder is chosen, not guessed, so the finished file
            // stays there whatever its type.
            let auto_dir = opts.dir.is_none() && rule_dir.is_none();
            let dir = match (opts.dir, rule_dir, &category) {
                (Some(dir), _, _) | (None, Some(dir), _) => dir,
                (None, None, Some(c)) => st.settings.category_dir(c),
                (None, None, None) => st.settings.download_dir.clone(),
            };
            let queue = opts
                .queue
                .filter(|q| st.settings.queue(q).is_some())
                .or_else(|| {
                    rule.as_ref()
                        .map(|r| r.queue.clone())
                        .filter(|q| st.settings.queue(q).is_some())
                })
                .unwrap_or_else(|| MAIN_QUEUE.into());
            let record = TaskRecord {
                id: uuid::Uuid::new_v4().to_string(),
                url: key,
                dir,
                file_name,
                path: None,
                state: if opts.paused || confirm {
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
                category: category.map(|c| c.name),
                auto_dir,
                queue,
                position: next_position(&st.tasks),
                created_at: now,
                updated_at: now,
                completed_at: None,
                media,
                site,
                speed_limit: None,
                resume_at: None,
                site_rule: rule.map(|r| r.pattern),
                torrent: None,
            };
            st.store.upsert(&record)?;
            let entry = Entry {
                limiter: Arc::new(RateLimiter::unlimited()),
                url: url.to_string(),
                record,
                live: Live::default(),
                headers: opts.headers,
                control: None,
                supervisor: None,
                after_stop: AfterStop::Pause,
                last_persist: Instant::now(),
                relink: false,
            };
            let mut entry = entry;
            entry.live.awaiting_confirmation = confirm;
            entry.live.duplicate_of = duplicate_of;
            entry.live.from_app = opts.from_app;
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
        if self.seeding_hash(id).is_some() {
            return self.stop_seeding(id);
        }
        self.inner.with_entry(id, |st, entry| {
            // Paused by hand: no longer until a time.
            let had_timer = entry.record.resume_at.take().is_some();
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
                TaskState::Paused | TaskState::Failed if had_timer => {
                    st.store.upsert(&entry.record)?;
                    Ok(Some(entry.snapshot()))
                }
                TaskState::Paused => Ok(None),
                state => Err(invalid("pause", state)),
            }
        })
    }

    /// Pause a download until `at` (Unix seconds); it is queued again by
    /// itself then. A running download stops with a checkpoint like any
    /// pause; a failed one keeps its error until it is retried at that time.
    pub fn pause_until(&self, id: &str, at: i64) -> Result<(), CoreError> {
        self.inner.with_entry(id, |st, entry| {
            if entry.live.awaiting_confirmation || entry.record.state.is_finished() {
                return Err(invalid("pause", entry.record.state));
            }
            entry.record.resume_at = Some(at);
            if let Some(control) = &entry.control {
                // Stored with the paused state once the job has stopped.
                entry.after_stop = AfterStop::Pause;
                control.pause();
                return Ok(Some(entry.snapshot()));
            }
            if entry.record.state == TaskState::Queued {
                entry.set_state(TaskState::Paused);
            }
            st.store.upsert(&entry.record)?;
            Ok(Some(entry.snapshot()))
        })
    }

    /// Queue the downloads paused until `now` or earlier. The manager calls
    /// this itself every few seconds with the current time.
    pub fn resume_due(&self, now: i64) {
        let due: Vec<TaskId> = {
            let st = self.inner.state.lock();
            st.tasks
                .iter()
                .filter(|e| {
                    e.record.resume_at.is_some_and(|at| at <= now)
                        && e.control.is_none()
                        && e.record.state.can_resume()
                        && !e.live.awaiting_confirmation
                })
                .map(|e| e.record.id.clone())
                .collect()
        };
        for id in due {
            if let Err(e) = self.resume(&id) {
                tracing::warn!(task = id, "could not resume at its time: {e}");
            }
        }
    }

    /// Give a download a speed limit of its own (bytes per second), under the
    /// global one; `None` or 0 removes it. A running download follows the
    /// change at once, except a video page, whose yt-dlp takes it at its next
    /// start.
    pub fn set_speed_limit(&self, id: &str, limit: Option<u64>) -> Result<(), CoreError> {
        let limit = limit.filter(|&l| l > 0);
        self.inner.with_entry(id, |st, entry| {
            // librqbit limits the whole session; a torrent follows the
            // download limit and the BitTorrent upload limit instead.
            if entry.record.torrent.is_some() {
                return Err(CoreError::InvalidSetting(
                    "a torrent has no speed limit of its own; it follows the limits in Settings"
                        .into(),
                ));
            }
            entry.record.speed_limit = limit;
            entry.limiter.set_rate(limit.unwrap_or(0));
            st.store.upsert(&entry.record)?;
            Ok(Some(entry.snapshot()))
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
                    // Resuming one held for confirmation by hand accepts it
                    // as it is.
                    entry.live.awaiting_confirmation = false;
                    entry.record.resume_at = None;
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
                    self.inner.remove_partial(&entry.record);
                    entry.record.downloaded = 0;
                    entry.record.resume_at = None;
                    entry.live = Live::default();
                    entry.set_state(TaskState::Canceled);
                    st.store.upsert(&entry.record)?;
                    Ok(Some(entry.snapshot()))
                }
            }
        })
    }

    /// Give a paused or failed download a new link (IDM's "refresh download
    /// address") and queue it. The partial data is reused only if the new
    /// link serves the same file; otherwise the download fails with
    /// `different_file` and keeps what it has.
    pub fn change_url(&self, id: &str, new_url: &str) -> Result<(), CoreError> {
        let url = parse_web_url(new_url)?;
        let key = downpipe_http::redact_url(&url);
        let snapshot = {
            let mut st = self.inner.state.lock();
            if let Some(dup) = st
                .tasks
                .iter()
                .find(|e| e.record.id != id && e.record.url == key && !e.record.state.is_finished())
            {
                return Err(CoreError::Duplicate(dup.record.id.clone()));
            }
            let State { store, tasks, .. } = &mut *st;
            let entry = tasks
                .iter_mut()
                .find(|e| e.record.id == id)
                .ok_or_else(|| CoreError::NotFound(id.to_string()))?;
            if entry.control.is_some() || entry.record.state == TaskState::Completed {
                return Err(invalid("change the link of", entry.record.state));
            }
            // A video page's address does not expire; another page is
            // another download.
            if entry.record.site.is_some() {
                return Err(CoreError::InvalidUrl(
                    "a video page keeps its address; add the other page as a new download".into(),
                ));
            }
            // A torrent is found by its hash, not by a link that expires.
            if entry.record.torrent.is_some() {
                return Err(CoreError::InvalidUrl(
                    "a torrent keeps its link; add another torrent as a new download".into(),
                ));
            }
            entry.url = url.to_string();
            entry.record.url = key;
            // Without a file name the engine never started, so there is no
            // partial data to protect.
            entry.relink = entry.record.file_name.is_some();
            entry.record.error = None;
            entry.record.error_kind = None;
            entry.set_state(TaskState::Queued);
            store.upsert(&entry.record)?;
            entry.snapshot()
        };
        self.inner.emit(ManagerEvent::Updated {
            task: Box::new(snapshot),
        });
        self.inner.schedule();
        Ok(())
    }

    /// Forget a download; with `delete_files`, also delete what it wrote.
    pub fn remove(&self, id: &str, delete_files: bool) -> Result<(), CoreError> {
        if let Some(hash) = self.seeding_hash(id) {
            self.inner.torrents.stop_seeding(&hash);
        }
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
                        // Only the torrent's own files: its path may be a
                        // folder the user put other things in.
                        (TaskState::Completed, Some(_)) if entry.record.torrent.is_some() => {
                            self.inner.remove_partial(&entry.record);
                        }
                        (TaskState::Completed, Some(path)) => {
                            let _ = std::fs::remove_file(path);
                        }
                        _ => self.inner.remove_partial(&entry.record),
                    }
                }
                if entry.record.torrent.is_some() {
                    self.inner.torrents.forget(id);
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
        // Downloads waiting for confirmation start only when confirmed.
        let ids: Vec<TaskId> = {
            let st = self.inner.state.lock();
            st.tasks
                .iter()
                .filter(|e| {
                    matches!(e.record.state, TaskState::Paused | TaskState::Failed)
                        && !e.live.awaiting_confirmation
                })
                .map(|e| e.record.id.clone())
                .collect()
        };
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

    /// Apply what the user chose for a download held for confirmation.
    pub fn confirm(&self, id: &str, choice: ConfirmOptions) -> Result<(), CoreError> {
        let start = choice.start;
        self.inner.with_entry(id, |st, entry| {
            if !entry.live.awaiting_confirmation {
                return Err(invalid("confirm", entry.record.state));
            }
            if let Some(name) = choice
                .file_name
                .as_deref()
                .map(str::trim)
                .filter(|n| !n.is_empty())
            {
                entry.record.file_name = Some(filename::sanitize(name));
            }
            if let Some(dir) = choice.dir.filter(|d| !d.as_os_str().is_empty()) {
                entry.record.dir = dir;
                entry.record.auto_dir = false;
            }
            if let Some(queue) = choice.queue.filter(|q| st.settings.queue(q).is_some()) {
                entry.record.queue = queue;
            }
            if let Some(files) = choice.files
                && let Some(t) = entry.record.torrent.as_mut()
            {
                if files.is_empty() {
                    return Err(CoreError::InvalidSetting(
                        "choose at least one file of the torrent".into(),
                    ));
                }
                let mut files = files;
                files.sort_unstable();
                files.dedup();
                t.only_files = (t.files == 0 || files.len() < t.files).then_some(files);
            }
            if let Some(selection) = choice.media {
                // Named again once the stream or page is read.
                if let Some(info) = entry.record.media.as_mut() {
                    info.selection = Some(selection);
                    info.quality = None;
                } else if let Some(site) = entry.record.site.as_mut() {
                    site.selection = Some(selection);
                    site.quality = None;
                }
            }
            entry.live.awaiting_confirmation = false;
            entry.set_state(if start {
                TaskState::Queued
            } else {
                TaskState::Paused
            });
            st.store.upsert(&entry.record)?;
            Ok(Some(entry.snapshot()))
        })?;
        self.inner.schedule();
        Ok(())
    }

    /// Download a finished file again from the same link. With `replace`,
    /// the old file is deleted first and the new one takes its name;
    /// otherwise the new file gets a free name next to it.
    pub fn redownload(&self, id: &str, replace: bool) -> Result<(), CoreError> {
        self.inner.with_entry(id, |st, entry| {
            if entry.control.is_some() || entry.record.state != TaskState::Completed {
                return Err(invalid("download again", entry.record.state));
            }
            if let Some(t) = entry.record.torrent.as_ref() {
                // A torrent is downloaded into the same place again: the
                // pieces still on disk are checked and kept unless replaced.
                self.inner.torrents.stop_seeding(&t.info_hash);
                if replace {
                    self.inner.remove_partial(&entry.record);
                }
            } else if replace {
                if let Some(path) = entry.record.path.as_ref().filter(|p| p.exists()) {
                    std::fs::remove_file(path)?;
                }
            } else {
                entry.record.file_name = None;
            }
            let r = &mut entry.record;
            r.path = None;
            r.total = None;
            r.downloaded = 0;
            r.completed_at = None;
            r.error = None;
            r.error_kind = None;
            entry.live = Live::default();
            entry.set_state(TaskState::Queued);
            st.store.upsert(&entry.record)?;
            Ok(Some(entry.snapshot()))
        })?;
        self.inner.schedule();
        Ok(())
    }

    /// What a stream download can be: its qualities and audio tracks, read
    /// from the manifest with the download's own headers. No media is
    /// downloaded. For the quality chooser in the confirmation dialog.
    pub async fn inspect_media(&self, id: &str) -> Result<MediaChoices, CoreError> {
        let (url, headers, info, engine) = {
            let st = self.inner.state.lock();
            let entry = st
                .tasks
                .iter()
                .find(|e| e.record.id == id)
                .ok_or_else(|| CoreError::NotFound(id.to_string()))?;
            let info = entry
                .record
                .media
                .clone()
                .ok_or_else(|| invalid("inspect the stream of", entry.record.state))?;
            let rule = entry.rule(&st.settings);
            let engine = self.inner.engine.lock().clone();
            let engine = self.inner.engine_for(&engine, &st.settings, rule)?;
            (
                entry.url.clone(),
                rule_headers(&entry.headers, rule),
                info,
                engine,
            )
        };
        let options = engine.media_options(&url, info.kind, &headers).await?;
        Ok(MediaChoices {
            options,
            selection: info.selection,
            ffmpeg: self.ffmpeg().is_some(),
        })
    }

    /// The FFmpeg stream downloads would use now: the one set in the
    /// settings, else `ffmpeg` on `PATH`.
    pub fn ffmpeg(&self) -> Option<PathBuf> {
        let configured = self.inner.state.lock().settings.ffmpeg_path.clone();
        find_ffmpeg(&configured)
    }

    /// What a video page offers, read by yt-dlp (which fetches the page and
    /// its player data, no video). For the quality chooser.
    pub async fn inspect_site(&self, id: &str) -> Result<SiteChoices, CoreError> {
        let (url, site, settings, proxy) = {
            let st = self.inner.state.lock();
            let entry = st
                .tasks
                .iter()
                .find(|e| e.record.id == id)
                .ok_or_else(|| CoreError::NotFound(id.to_string()))?;
            let site = entry
                .record
                .site
                .clone()
                .ok_or_else(|| invalid("inspect the page of", entry.record.state))?;
            let proxy = site_proxy(&st.settings, entry.rule(&st.settings));
            (entry.url.clone(), site, st.settings.clone(), proxy)
        };
        let tools = downpipe_providers::tools::find(&self.inner.tool_config(&settings));
        let info = downpipe_providers::info::inspect(&tools, &url, proxy.as_deref()).await?;
        Ok(SiteChoices {
            site: site.site,
            title: info.title,
            options: SiteOptions {
                videos: info.videos,
                audios: info.audios,
                duration_ms: info.duration_ms,
            },
            selection: site.selection,
            ffmpeg: tools.ffmpeg.is_some(),
        })
    }

    /// The site provider's tools as the settings find them, with versions.
    pub async fn site_tools(&self) -> ToolsStatus {
        let settings = self.settings();
        downpipe_providers::tools::status(&self.inner.tool_config(&settings)).await
    }

    /// Download the latest official yt-dlp and/or Deno into Downpipe's tools
    /// folder, checked against their published SHA-256. Only ever called
    /// when the user asks for it.
    pub async fn install_site_tools(&self, tools: &[Tool]) -> Result<ToolsStatus, CoreError> {
        let client = self.inner.engine.lock().client().clone();
        let base = url::Url::parse(install::GITHUB).expect("a valid URL");
        for &tool in tools {
            let installed = install::install(&client, &base, &self.inner.tools_dir, tool).await?;
            tracing::info!(
                tool = tool.name(),
                release = installed.release,
                sha256 = installed.sha256,
                "installed into {}",
                installed.path.display()
            );
        }
        Ok(self.site_tools().await)
    }

    /// Reorder downloads inside their queues. Several ids move together and
    /// keep their relative order.
    pub fn move_tasks(&self, ids: &[TaskId], to: QueueMove) -> Result<(), CoreError> {
        let changed = {
            let mut st = self.inner.state.lock();
            let State { store, tasks, .. } = &mut *st;
            let mut order: Vec<usize> = (0..tasks.len()).collect();
            order.sort_by_key(|&i| tasks[i].record.position);
            let queues: Vec<String> = ids
                .iter()
                .filter_map(|id| tasks.iter().find(|e| &e.record.id == id))
                .map(|e| e.record.queue.clone())
                .collect();
            for queue in dedup(queues) {
                // Slots this queue occupies in the global order stay the same;
                // only which download sits in them changes.
                let slots: Vec<usize> = (0..order.len())
                    .filter(|&at| tasks[order[at]].record.queue == queue)
                    .collect();
                let picked = |i: usize| ids.contains(&tasks[i].record.id);
                let mut members: Vec<usize> = slots.iter().map(|&at| order[at]).collect();
                reorder(&mut members, &picked, to);
                for (&at, member) in slots.iter().zip(members) {
                    order[at] = member;
                }
            }
            let mut changed = Vec::new();
            for (pos, &i) in order.iter().enumerate() {
                let pos = pos as i64;
                if tasks[i].record.position != pos {
                    tasks[i].record.position = pos;
                    store.upsert(&tasks[i].record)?;
                    changed.push(tasks[i].snapshot());
                }
            }
            changed
        };
        for task in changed {
            self.inner.emit(ManagerEvent::Updated {
                task: Box::new(task),
            });
        }
        self.inner.schedule();
        Ok(())
    }

    /// Move downloads to the end of another queue.
    pub fn set_queue(&self, ids: &[TaskId], queue: &str) -> Result<(), CoreError> {
        let changed = {
            let mut st = self.inner.state.lock();
            if st.settings.queue(queue).is_none() {
                return Err(CoreError::InvalidSetting(format!(
                    "there is no queue `{queue}`"
                )));
            }
            let mut next = next_position(&st.tasks);
            let State { store, tasks, .. } = &mut *st;
            let mut changed = Vec::new();
            for entry in tasks.iter_mut().filter(|e| ids.contains(&e.record.id)) {
                if entry.record.queue == queue {
                    continue;
                }
                entry.record.queue = queue.to_string();
                entry.record.position = next;
                next += 1;
                store.upsert(&entry.record)?;
                changed.push(entry.snapshot());
            }
            changed
        };
        for task in changed {
            self.inner.emit(ManagerEvent::Updated {
                task: Box::new(task),
            });
        }
        self.inner.schedule();
        Ok(())
    }

    /// Resume the paused and failed downloads of one queue.
    pub fn start_queue(&self, queue: &str) {
        for id in self.ids_in(queue, |s| s == TaskState::Paused || s == TaskState::Failed) {
            let _ = self.resume(&id);
        }
    }

    /// Pause everything running or waiting in one queue.
    pub fn stop_queue(&self, queue: &str) {
        for id in self.ids_in(queue, |s| s.is_running() || s == TaskState::Queued) {
            let _ = self.pause(&id);
        }
    }

    fn ids_in(&self, queue: &str, pred: impl Fn(TaskState) -> bool) -> Vec<TaskId> {
        let st = self.inner.state.lock();
        st.tasks
            .iter()
            .filter(|e| {
                e.record.queue == queue && pred(e.record.state) && !e.live.awaiting_confirmation
            })
            .map(|e| e.record.id.clone())
            .collect()
    }

    /// Hosts that refused extra connections this session and that no rule
    /// holds to that many connections yet, for "make it a rule".
    pub fn refused_hosts(&self) -> Vec<RefusedHost> {
        let rules = self.settings().site_rules;
        let engine = self.inner.engine.lock().clone();
        let mut out: Vec<RefusedHost> = Vec::new();
        for (host, _port, connections) in engine.connection_limits() {
            let connections = u32::try_from(connections).unwrap_or(u32::MAX);
            let covered = rules::rule_for_host(&rules, &host)
                .and_then(|r| r.max_connections)
                .is_some_and(|max| max <= connections);
            if covered {
                continue;
            }
            // One row per host, whatever ports it was reached on.
            match out.iter_mut().find(|h| h.host == host) {
                Some(seen) => seen.connections = seen.connections.min(connections),
                None => out.push(RefusedHost { host, connections }),
            }
        }
        out
    }

    pub fn update_settings(&self, settings: Settings) -> Result<Settings, CoreError> {
        let settings = settings.normalized();
        validate_proxy(&settings.proxy)?;
        if let Some(problem) = rules::problems(&settings.site_rules).first() {
            let pattern = settings
                .site_rules
                .iter()
                .find(|r| r.id == problem.id)
                .map_or("", |r| r.pattern.as_str());
            return Err(CoreError::InvalidSetting(format!(
                "site rule `{pattern}`: {}",
                rules::describe(problem.code)
            )));
        }
        if settings.user_agent.chars().any(char::is_control) {
            return Err(CoreError::InvalidSetting(
                "the user agent cannot contain control characters".into(),
            ));
        }
        let previous = self.settings();
        // Build the new client before saving, so a value the HTTP stack
        // rejects never reaches the catalog.
        let rebuilt =
            if previous.proxy != settings.proxy || previous.user_agent != settings.user_agent {
                let config = engine_config(&self.inner.base_config, &settings);
                // Derived, so what hosts refused is not forgotten.
                let current = self.inner.engine.lock().clone();
                Some(
                    current
                        .derive(config)
                        .map_err(|e| CoreError::InvalidSetting(e.to_string()))?,
                )
            } else {
                None
            };
        {
            let mut st = self.inner.state.lock();
            st.store.save_settings(&settings)?;
            st.settings = settings.clone();
            let State { store, tasks, .. } = &mut *st;
            for entry in tasks
                .iter_mut()
                .filter(|e| settings.queue(&e.record.queue).is_none())
            {
                entry.record.queue = MAIN_QUEUE.into();
                store.upsert(&entry.record)?;
            }
        }
        if let Some(engine) = rebuilt {
            *self.inner.engine.lock() = engine;
            self.inner.proxied.lock().clear();
        } else {
            // Engines are kept by proxy; drop those no rule uses any more.
            let wanted: Vec<String> = settings.site_rules.iter().filter_map(proxy_key).collect();
            self.inner
                .proxied
                .lock()
                .retain(|key, _| wanted.contains(key));
        }
        self.inner.limiter.set_rate(settings.speed_limit);
        self.inner.torrents.configure(&settings);
        // Running downloads follow a changed site limit at once.
        self.inner.site_limiters.lock().retain(|id, limiter| {
            let rate = settings
                .site_rules
                .iter()
                .find(|r| &r.id == id)
                .and_then(|r| r.speed_limit);
            limiter.set_rate(rate.unwrap_or(0));
            rate.is_some()
        });
        self.inner.emit(ManagerEvent::Settings {
            settings: Box::new(settings.clone()),
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
        // Seeding stops with the app; what was uploaded is kept.
        self.inner.torrents.shutdown().await;
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

    fn tool_config(&self, settings: &Settings) -> ToolConfig {
        ToolConfig {
            yt_dlp: PathBuf::from(&settings.yt_dlp_path),
            deno: PathBuf::from(&settings.deno_path),
            ffmpeg: PathBuf::from(&settings.ffmpeg_path),
            managed_dir: Some(self.tools_dir.clone()),
        }
    }

    /// The engine for downloads `rule` applies to: the main one, or one
    /// derived for the rule's proxy (or for no proxy).
    fn engine_for(
        &self,
        engine: &Engine,
        settings: &Settings,
        rule: Option<&SiteRule>,
    ) -> Result<Engine, DownloadError> {
        let Some(key) = rule.and_then(proxy_key) else {
            return Ok(engine.clone());
        };
        let mut cache = self.proxied.lock();
        if let Some(engine) = cache.get(&key) {
            return Ok(engine.clone());
        }
        let mut config = engine_config(&self.base_config, settings);
        config.no_proxy = key.is_empty();
        config.proxy = (!key.is_empty()).then(|| key.clone());
        let derived = engine.derive(config)?;
        cache.insert(key, derived.clone());
        Ok(derived)
    }

    fn site_limiter(&self, rule: &SiteRule) -> Option<Arc<RateLimiter>> {
        let rate = rule.speed_limit.filter(|&r| r > 0)?;
        let mut limiters = self.site_limiters.lock();
        let limiter = limiters
            .entry(rule.id.clone())
            .or_insert_with(|| Arc::new(RateLimiter::new(rate)));
        if limiter.rate() != rate {
            limiter.set_rate(rate);
        }
        Some(limiter.clone())
    }

    fn with_entry(
        &self,
        id: &str,
        f: impl FnOnce(&mut StateParts<'_>, &mut Entry) -> Result<Option<TaskSnapshot>, CoreError>,
    ) -> Result<(), CoreError> {
        let snapshot = {
            let mut st = self.state.lock();
            let State {
                store,
                tasks,
                settings,
                ..
            } = &mut *st;
            let entry = tasks
                .iter_mut()
                .find(|e| e.record.id == id)
                .ok_or_else(|| CoreError::NotFound(id.to_string()))?;
            f(&mut StateParts { store, settings }, entry)?
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
            let per_download = st.settings.max_connections_per_download;
            let engine = self.engine.lock().clone();
            // Looked up once per pass, and only when a stream starts.
            let mut ffmpeg: Option<Option<PathBuf>> = None;
            let mut site_tools: Option<Tools> = None;
            let State {
                store,
                tasks,
                settings,
                ..
            } = &mut *st;
            // Each queue runs up to its own limit, in its own order.
            let mut running: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for e in tasks.iter().filter(|e| e.control.is_some()) {
                *running.entry(e.record.queue.clone()).or_default() += 1;
            }
            let mut order: Vec<usize> = (0..tasks.len()).collect();
            order.sort_by_key(|&i| tasks[i].record.position);
            for i in order {
                let entry = &mut tasks[i];
                if entry.record.state != TaskState::Queued || entry.control.is_some() {
                    continue;
                }
                let busy = running.entry(entry.record.queue.clone()).or_default();
                if *busy >= settings.queue_limit(&entry.record.queue) {
                    continue;
                }
                *busy += 1;
                // Looked up at every start, so a rule changed while the
                // download was paused applies to its resume.
                let rule = entry.rule(settings);
                entry.record.site_rule = rule.map(|r| r.pattern.clone());
                let handle = if entry.record.torrent.is_some() {
                    self.torrents
                        .start(entry.torrent_request(&engine), settings)
                } else if entry.record.site.is_some() {
                    let tools = site_tools
                        .get_or_insert_with(|| {
                            downpipe_providers::tools::find(&self.tool_config(settings))
                        })
                        .clone();
                    downpipe_providers::job::start(entry.site_request(settings, tools, rule))
                } else {
                    let tool = match &entry.record.media {
                        Some(_) => ffmpeg
                            .get_or_insert_with(|| find_ffmpeg(&settings.ffmpeg_path))
                            .clone(),
                        None => None,
                    };
                    let limiter = rule.and_then(|r| self.site_limiter(r));
                    let request = entry.request(per_download, tool, rule, limiter);
                    match self.engine_for(&engine, settings, rule) {
                        Ok(engine) => engine.start(request),
                        // Validated when saved, so only a proxy the HTTP
                        // stack refuses gets here; ignoring the rule would
                        // send the download past the proxy it asked for.
                        Err(err) => DownloadHandle::spawn(move |_| async move { Err(err) }),
                    }
                };
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
        let completed = matches!(result, Ok(Outcome::Completed { .. }));
        self.on_finished(&id, result);
        if completed {
            self.follow_seeding(&id);
        }
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
            // The tracks a stream download resolved to are kept, so that a
            // resume after a restart takes the same quality.
            let mut media_changed = false;
            if let (Some(info), Some(m)) = (r.media.as_mut(), p.media.as_ref())
                && (info.quality.as_deref() != Some(m.quality.as_str())
                    || info.selection.as_ref() != Some(&m.selection))
            {
                info.quality = Some(m.quality.clone());
                info.selection = Some(m.selection.clone());
                media_changed = true;
            }
            if let (Some(site), Some(m)) = (r.site.as_mut(), p.media.as_ref())
                && (site.quality.as_deref() != Some(m.quality.as_str())
                    || site.selection.as_ref() != Some(&m.selection))
            {
                site.quality = Some(m.quality.clone());
                site.selection = Some(m.selection.clone());
                media_changed = true;
            }
            if let (Some(t), Some(tp)) = (r.torrent.as_mut(), p.torrent.as_ref()) {
                t.uploaded = tp.uploaded;
            }
            entry.live = Live::from_progress(p);
            if state_changed || media_changed || entry.last_persist.elapsed() >= PERSIST_EVERY {
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
            let State {
                store,
                tasks,
                settings,
                ..
            } = &mut *st;
            let Some(idx) = tasks.iter().position(|e| e.record.id == id) else {
                return;
            };
            let entry = &mut tasks[idx];
            entry.control = None;
            entry.supervisor = None;
            entry.live.speed = 0;
            entry.live.connections = 0;
            // Peers belong to a running torrent; seeding sets them again.
            entry.live.swarm = None;
            let after = entry.after_stop;
            if result.is_ok() {
                entry.relink = false;
            }

            if after == AfterStop::Remove && !matches!(result, Ok(Outcome::Completed { .. })) {
                if entry.record.torrent.is_some() {
                    self.torrents.forget(id);
                }
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
                    // A torrent's files stay where they are: they are what
                    // it seeds from.
                    let (path, category) = if r.torrent.is_some() {
                        (path, r.category.clone())
                    } else {
                        sort_into_category(settings, r, path)
                    };
                    r.category = category;
                    if let Some(dir) = path.parent() {
                        r.dir = dir.to_path_buf();
                    }
                    r.file_name = path.file_name().map(|n| n.to_string_lossy().into_owned());
                    r.path = Some(path);
                    r.total = Some(bytes);
                    r.downloaded = bytes;
                    r.completed_at = Some(unix_now());
                    r.error = None;
                    r.error_kind = None;
                    r.resume_at = None;
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
                    r.resume_at = None;
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
    settings: &'a Settings,
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

    /// The site rule that applies to this download under `settings`.
    fn rule<'a>(&self, settings: &'a Settings) -> Option<&'a SiteRule> {
        settings.site_rule_for(&url::Url::parse(&self.url).ok()?)
    }

    fn request(
        &self,
        per_download: u32,
        ffmpeg: Option<PathBuf>,
        rule: Option<&SiteRule>,
        site_limiter: Option<Arc<RateLimiter>>,
    ) -> DownloadRequest {
        let mut req = DownloadRequest::new(self.url.clone(), self.record.dir.clone());
        // Once the engine picked a name, keep it: that is how the resume
        // data next to the file is found again.
        req.file_name.clone_from(&self.record.file_name);
        req.checksum = self.record.checksum.as_deref().and_then(|c| c.parse().ok());
        req.headers = rule_headers(&self.headers, rule);
        // A number set for this one download beats the site's, which beats
        // the global one.
        let connections = self
            .record
            .max_connections
            .or_else(|| rule.and_then(|r| r.max_connections))
            .unwrap_or(per_download);
        req.max_connections = Some(connections as usize);
        req.max_retries = rule.and_then(|r| r.max_retries);
        req.site_limiter = site_limiter;
        req.relink = self.relink;
        req.limiter = Some(self.limiter.clone());
        req.media = self.record.media.as_ref().map(|info| MediaRequest {
            kind: info.kind,
            selection: info.selection.clone().unwrap_or_default(),
            ffmpeg,
        });
        req
    }

    fn site_request(
        &self,
        settings: &Settings,
        tools: Tools,
        rule: Option<&SiteRule>,
    ) -> SiteRequest {
        // yt-dlp has one limit per process: the lowest of the global, the
        // download's own and the site's; 0 when none is set.
        let rate_limit = [
            Some(settings.speed_limit),
            self.record.speed_limit,
            rule.and_then(|r| r.speed_limit),
        ]
        .into_iter()
        .flatten()
        .filter(|&l| l > 0)
        .min()
        .unwrap_or(0);
        SiteRequest {
            url: self.url.clone(),
            dir: self.record.dir.clone(),
            file_name: self.record.file_name.clone(),
            selection: self.record.site.as_ref().and_then(|s| s.selection.clone()),
            tools,
            proxy: site_proxy(settings, rule),
            rate_limit,
        }
    }
}

/// The proxy cache key of a rule that sets one: its URL, or `""` for none.
fn proxy_key(rule: &SiteRule) -> Option<String> {
    match rule.proxy {
        RuleProxy::Inherit => None,
        RuleProxy::Direct => Some(String::new()),
        RuleProxy::Custom => Some(rule.proxy_url.clone()),
    }
}

/// yt-dlp's `--proxy` for a download: `""` connects directly.
fn site_proxy(settings: &Settings, rule: Option<&SiteRule>) -> Option<String> {
    match rule.and_then(proxy_key) {
        Some(key) => Some(key),
        None => (!settings.proxy.is_empty()).then(|| settings.proxy.clone()),
    }
}

/// The download's headers with the rule's user agent and referer in place
/// of any the browser passed on.
fn rule_headers(headers: &[(String, String)], rule: Option<&SiteRule>) -> Vec<(String, String)> {
    let mut out = headers.to_vec();
    let Some(rule) = rule else {
        return out;
    };
    for (name, value) in [("User-Agent", &rule.user_agent), ("Referer", &rule.referer)] {
        if value.is_empty() {
            continue;
        }
        out.retain(|(n, _)| !n.trim().eq_ignore_ascii_case(name));
        out.push((name.to_string(), value.clone()));
    }
    out
}

fn next_position(tasks: &[Entry]) -> i64 {
    tasks
        .iter()
        .map(|e| e.record.position + 1)
        .max()
        .unwrap_or(0)
}

fn dedup(items: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for item in items {
        if !out.contains(&item) {
            out.push(item);
        }
    }
    out
}

/// Reorder `members` (task indices in queue order) so the picked ones move
/// as asked; relative order inside both groups is kept.
fn reorder(members: &mut Vec<usize>, picked: &impl Fn(usize) -> bool, to: QueueMove) {
    match to {
        QueueMove::Top | QueueMove::Bottom => {
            let (mut sel, rest): (Vec<usize>, Vec<usize>) =
                members.iter().partition(|&&i| picked(i));
            let mut out = if to == QueueMove::Top {
                sel.extend(rest);
                sel
            } else {
                let mut r = rest;
                r.extend(sel);
                r
            };
            std::mem::swap(members, &mut out);
        }
        QueueMove::Up => {
            for k in 1..members.len() {
                if picked(members[k]) && !picked(members[k - 1]) {
                    members.swap(k, k - 1);
                }
            }
        }
        QueueMove::Down => {
            for k in (0..members.len().saturating_sub(1)).rev() {
                if picked(members[k]) && !picked(members[k + 1]) {
                    members.swap(k, k + 1);
                }
            }
        }
    }
}

fn parse_web_url(input: &str) -> Result<url::Url, CoreError> {
    let url = url::Url::parse(input.trim()).map_err(|e| CoreError::InvalidUrl(e.to_string()))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(CoreError::InvalidUrl(format!(
            "`{}` links are not supported; use http or https",
            url.scheme()
        )));
    }
    Ok(url)
}

fn engine_config(base: &EngineConfig, settings: &Settings) -> EngineConfig {
    let mut config = base.clone();
    if !settings.proxy.is_empty() {
        config.proxy = Some(settings.proxy.clone());
    }
    if !settings.user_agent.is_empty() {
        config.user_agent = settings.user_agent.clone();
    }
    config
}

fn validate_proxy(proxy: &str) -> Result<(), CoreError> {
    if proxy.is_empty() {
        return Ok(());
    }
    let bad = || {
        CoreError::InvalidSetting(format!(
            "`{proxy}` is not a proxy address; use http://, https:// or socks5://host:port"
        ))
    };
    let url = url::Url::parse(proxy).map_err(|_| bad())?;
    let scheme_ok = matches!(url.scheme(), "http" | "https" | "socks5" | "socks5h");
    if scheme_ok && url.host_str().is_some_and(|h| !h.is_empty()) {
        Ok(())
    } else {
        Err(bad())
    }
}

/// Move a finished download whose folder Downpipe picked into the folder of
/// its real type. The type guessed from the URL can be wrong (a `download?id=`
/// link that turns out to be an ISO), and only now is the name certain.
/// Returns the final path and category. A failed move leaves the file where
/// it is: it is complete either way.
fn sort_into_category(
    settings: &Settings,
    record: &TaskRecord,
    path: PathBuf,
) -> (PathBuf, Option<String>) {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let Some(category) = settings.category_for(&name).cloned() else {
        return (path, None);
    };
    if !record.auto_dir {
        return (path, Some(category.name));
    }
    let target = settings.category_dir(&category);
    if path.parent() == Some(target.as_path()) {
        return (path, Some(category.name));
    }
    match move_into(&path, &target, &name) {
        Ok(moved) => (moved, Some(category.name)),
        Err(err) => {
            tracing::warn!(
                "could not move {} into {}: {err}",
                path.display(),
                target.display()
            );
            (path, Some(category.name))
        }
    }
}

fn move_into(path: &Path, dir: &Path, name: &str) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let dest = filename::unique_path(dir, name);
    std::fs::rename(path, &dest)?;
    Ok(dest)
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

fn is_magnet(link: &str) -> bool {
    link.trim()
        .get(..7)
        .is_some_and(|s| s.eq_ignore_ascii_case("magnet:"))
}

fn remove_partial(record: &TaskRecord) {
    if record.site.is_some() {
        if let Some(name) = &record.file_name {
            site_files::remove_partials(&record.dir, &site_files::stem_of(name));
        }
        return;
    }
    if let Some(path) = final_path(record) {
        if record.media.is_some() {
            media::remove_work_dir(&path);
        } else {
            let _ = std::fs::remove_file(meta::part_path(&path));
            meta::remove(&path);
        }
    }
}

/// `configured` when set (the settings field), else `ffmpeg` on `PATH`.
fn find_ffmpeg(configured: &str) -> Option<PathBuf> {
    let configured = Path::new(configured);
    media::mux::find_ffmpeg((!configured.as_os_str().is_empty()).then_some(configured))
}

/// A name for a stream before its container is known: the hint without a
/// manifest's extension, as a video. The engine sets the real extension.
fn video_name(hint: &str) -> String {
    let stem = match hint.rsplit_once('.') {
        Some((stem, ext))
            if !stem.is_empty()
                && ["m3u8", "m3u", "mpd", "mp4", "ts", "m4s"]
                    .contains(&ext.to_ascii_lowercase().as_str()) =>
        {
            stem
        }
        _ => hint,
    };
    format!("{stem}.mp4")
}

/// Bring the displayed progress in line with what is actually on disk.
fn refresh_from_meta(record: &mut TaskRecord) {
    // Site downloads keep no meta file; yt-dlp finds its own parts again,
    // and librqbit checks a torrent's pieces when it starts.
    if record.state.is_finished() || record.site.is_some() || record.torrent.is_some() {
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

// BitTorrent. Torrents share the list, the queues and the state machine
// with everything else; what differs lives here.

impl Manager {
    /// Add a torrent from the bytes of a `.torrent` file (dropped on the
    /// window or picked in the Add dialog). `opts.url` is ignored.
    pub fn add_torrent_file(
        &self,
        bytes: &[u8],
        opts: AddOptions,
    ) -> Result<TaskSnapshot, CoreError> {
        if bytes.len() > MAX_TORRENT_FILE {
            return Err(CoreError::InvalidUrl(
                "that file is too large to be a torrent".into(),
            ));
        }
        self.add_torrent(opts, Some(bytes))
    }

    fn add_torrent(
        &self,
        opts: AddOptions,
        file: Option<&[u8]>,
    ) -> Result<TaskSnapshot, CoreError> {
        let (parsed, memory_url) = match file {
            Some(bytes) => {
                let parsed = torrent::parse_file(bytes)?;
                (parsed.clone(), parsed.link)
            }
            None if is_magnet(&opts.url) => {
                let parsed = torrent::parse_magnet(&opts.url)?;
                (parsed.clone(), parsed.link)
            }
            None => {
                // A `.torrent` on a web server: what it holds is known once
                // it is fetched (in the confirmation dialog or at start).
                let url = parse_web_url(&opts.url)?;
                downpipe_http::build_headers(&opts.headers)?;
                let name = opts
                    .file_name
                    .as_deref()
                    .map(str::trim)
                    .filter(|n| !n.is_empty())
                    .map(str::to_string)
                    .or_else(|| filename::from_url(&url));
                let parsed = Parsed {
                    info_hash: String::new(),
                    name: name.map(|n| strip_torrent_extension(&n)),
                    link: downpipe_http::redact_url(&url),
                    files: 0,
                    total: None,
                    private: false,
                };
                (parsed, url.to_string())
            }
        };
        let snapshot = {
            let mut st = self.inner.state.lock();
            let same = |e: &&Entry| {
                e.record.url == parsed.link
                    || (!parsed.info_hash.is_empty()
                        && e.record
                            .torrent
                            .as_ref()
                            .is_some_and(|t| t.info_hash == parsed.info_hash))
            };
            if let Some(asking) = st
                .tasks
                .iter()
                .filter(same)
                .find(|e| e.live.awaiting_confirmation)
            {
                return Err(CoreError::Duplicate(asking.record.id.clone()));
            }
            let duplicate_of = match (st.tasks.iter().find(same), opts.on_duplicate) {
                (Some(dup), OnDuplicate::Refuse) => {
                    return Err(CoreError::Duplicate(dup.record.id.clone()));
                }
                // Two entries would write the same files and seed them
                // twice; the one in the list is the torrent.
                (Some(_), OnDuplicate::AddCopy) => {
                    return Err(CoreError::InvalidUrl(
                        "this torrent is already in the list; a second copy would write the same files"
                            .into(),
                    ));
                }
                (Some(dup), OnDuplicate::Ask) => Some(dup.record.id.clone()),
                (None, _) => None,
            };
            let confirm = opts.confirm || duplicate_of.is_some();
            let now = unix_now();
            // One file goes into the folder of its type; a folder of files
            // has no single type and goes into the download folder.
            let category = parsed
                .name
                .as_deref()
                .filter(|_| parsed.files <= 1)
                .and_then(|name| st.settings.category_for(name))
                .cloned();
            let auto_dir = opts.dir.is_none();
            let dir = match (opts.dir, &category) {
                (Some(dir), _) => dir,
                (None, Some(c)) => st.settings.category_dir(c),
                (None, None) => st.settings.download_dir.clone(),
            };
            let record = TaskRecord {
                id: uuid::Uuid::new_v4().to_string(),
                url: parsed.link.clone(),
                dir,
                file_name: parsed
                    .name
                    .as_deref()
                    .filter(|_| parsed.files > 0)
                    .map(filename::sanitize),
                path: None,
                state: if opts.paused || confirm {
                    TaskState::Paused
                } else {
                    TaskState::Queued
                },
                total: parsed.total,
                downloaded: 0,
                error: None,
                error_kind: None,
                checksum: None,
                max_connections: None,
                category: category.map(|c| c.name),
                auto_dir,
                queue: opts
                    .queue
                    .filter(|q| st.settings.queue(q).is_some())
                    .unwrap_or_else(|| MAIN_QUEUE.into()),
                position: next_position(&st.tasks),
                created_at: now,
                updated_at: now,
                completed_at: None,
                media: None,
                site: None,
                speed_limit: None,
                resume_at: None,
                // Site rules are about web hosts; a torrent has none.
                site_rule: None,
                torrent: Some(TorrentInfo {
                    info_hash: parsed.info_hash.clone(),
                    name: parsed.name.clone(),
                    files: parsed.files,
                    only_files: None,
                    uploaded: 0,
                    private: parsed.private,
                }),
            };
            if let Some(bytes) = file {
                self.inner.torrents.keep_file(&record.id, bytes)?;
            }
            if let Err(e) = st.store.upsert(&record) {
                self.inner.torrents.forget(&record.id);
                return Err(e.into());
            }
            let mut entry = Entry {
                limiter: Arc::new(RateLimiter::unlimited()),
                url: memory_url,
                record,
                live: Live::default(),
                headers: opts.headers,
                control: None,
                supervisor: None,
                after_stop: AfterStop::Pause,
                last_persist: Instant::now(),
                relink: false,
            };
            entry.live.awaiting_confirmation = confirm;
            entry.live.duplicate_of = duplicate_of;
            entry.live.from_app = opts.from_app;
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

    /// A torrent's files, for choosing which to download: read from its
    /// `.torrent` file, or fetched first (a link to one) or asked from peers
    /// (a magnet link, which can take up to two minutes). Nothing else is
    /// downloaded. The download learns the torrent's name and size.
    pub async fn inspect_torrent(&self, id: &str) -> Result<TorrentChoices, CoreError> {
        let request = {
            let st = self.inner.state.lock();
            let entry = st
                .tasks
                .iter()
                .find(|e| e.record.id == id)
                .ok_or_else(|| CoreError::NotFound(id.to_string()))?;
            if entry.record.torrent.is_none() {
                return Err(invalid("list the files of", entry.record.state));
            }
            let engine = self.inner.engine.lock().clone();
            entry.torrent_request(&engine)
        };
        let (choices, parsed) = self.inner.torrents.choices(&request).await?;
        self.inner.with_entry(id, |st, entry| {
            let r = &mut entry.record;
            let Some(t) = r.torrent.as_mut() else {
                return Ok(None);
            };
            t.info_hash = parsed.info_hash;
            t.name = parsed.name.clone();
            t.files = parsed.files;
            t.private = parsed.private;
            if entry.control.is_none() && !r.state.is_finished() {
                r.file_name = parsed.name.as_deref().map(filename::sanitize);
                r.total = parsed.total;
                // Now that the files are known, one file may belong to a
                // category the magnet link's name did not tell.
                if r.auto_dir && parsed.files == 1 && r.downloaded == 0 {
                    let category = parsed
                        .name
                        .as_deref()
                        .and_then(|name| st.settings.category_for(name))
                        .cloned();
                    r.dir = match &category {
                        Some(c) => st.settings.category_dir(c),
                        None => st.settings.download_dir.clone(),
                    };
                    r.category = category.map(|c| c.name);
                }
            }
            st.store.upsert(&entry.record)?;
            Ok(Some(entry.snapshot()))
        })?;
        Ok(choices)
    }

    /// Stop uploading a finished torrent. Its files stay.
    pub fn stop_seeding(&self, id: &str) -> Result<(), CoreError> {
        let hash = self.seeding_hash(id).ok_or_else(|| {
            let state = self
                .get(id)
                .map_or(TaskState::Completed, |t| t.record.state);
            invalid("stop seeding", state)
        })?;
        self.inner.torrents.stop_seeding(&hash);
        Ok(())
    }

    /// The info hash of a download that seeds right now.
    fn seeding_hash(&self, id: &str) -> Option<String> {
        let st = self.inner.state.lock();
        let entry = st.tasks.iter().find(|e| e.record.id == id)?;
        let seeding = entry.live.swarm.as_ref().is_some_and(|s| s.seeding);
        let hash = entry.record.torrent.as_ref()?.info_hash.clone();
        seeding.then_some(hash)
    }
}

impl Inner {
    /// After a torrent finished: show its seeding in the list until it
    /// stops, and keep what it uploaded.
    fn follow_seeding(self: &Arc<Self>, id: &str) {
        let hash = {
            let st = self.state.lock();
            st.tasks
                .iter()
                .find(|e| e.record.id == id && e.record.state == TaskState::Completed)
                .and_then(|e| e.record.torrent.as_ref())
                .map(|t| t.info_hash.clone())
        };
        let Some(hash) = hash else {
            return;
        };
        let inner = Arc::downgrade(self);
        let id = id.to_string();
        self.torrents.watch_seeding(&hash, move |s| {
            if let Some(inner) = inner.upgrade() {
                inner.on_seeding(&id, &s);
            }
        });
    }

    fn on_seeding(&self, id: &str, s: &SeedState) {
        let snapshot = {
            let mut st = self.state.lock();
            let State { store, tasks, .. } = &mut *st;
            let Some(entry) = tasks.iter_mut().find(|e| e.record.id == id) else {
                return;
            };
            if entry.record.state != TaskState::Completed {
                return;
            }
            entry.live.swarm = s.seeding.then_some(Swarm {
                peers: s.peers,
                known_peers: 0,
                upload_speed: s.upload_speed,
                seeding: true,
                fetching_metadata: false,
            });
            if let Some(t) = entry.record.torrent.as_mut() {
                t.uploaded = s.uploaded;
            }
            if !s.seeding || entry.last_persist.elapsed() >= PERSIST_EVERY {
                entry.last_persist = Instant::now();
                if let Err(e) = store.upsert(&entry.record) {
                    tracing::warn!("could not persist seeding: {e}");
                }
            }
            entry.snapshot()
        };
        self.emit(ManagerEvent::Updated {
            task: Box::new(snapshot),
        });
    }

    /// Delete what an unfinished download wrote; a torrent's own files only.
    fn remove_partial(&self, record: &TaskRecord) {
        match &record.torrent {
            // Never started: the files there, if any, are not its own.
            Some(_) if record.downloaded == 0 && record.state != TaskState::Completed => {}
            Some(t) => {
                self.torrents
                    .delete_files(&record.id, &record.dir, t.only_files.as_deref());
            }
            None => remove_partial(record),
        }
    }
}

impl Entry {
    fn torrent_request(&self, engine: &Engine) -> torrent::Request {
        let t = self.record.torrent.as_ref();
        torrent::Request {
            id: self.record.id.clone(),
            link: self.url.clone(),
            headers: self.headers.clone(),
            client: engine.client().clone(),
            dir: self.record.dir.clone(),
            only_files: t.and_then(|t| t.only_files.clone()),
            uploaded: t.map_or(0, |t| t.uploaded),
        }
    }
}

/// `ubuntu.iso.torrent` -> `ubuntu.iso`: the name of what it downloads.
fn strip_torrent_extension(name: &str) -> String {
    match name.len().checked_sub(".torrent".len()) {
        Some(at) if name.is_char_boundary(at) && name[at..].eq_ignore_ascii_case(".torrent") => {
            name[..at].to_string()
        }
        _ => name.to_string(),
    }
}
