//! The download controller: opens or resumes a download, runs the adaptive
//! connection loop, checkpoints, verifies and atomically publishes the file.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use parking_lot::Mutex;
use reqwest::header::{ETAG, HeaderMap};
use reqwest::{Client, RequestBuilder, Response, StatusCode};
use tokio::sync::watch;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use url::Url;

use crate::checksum::{self, Checksum};
use crate::config::{DownloadRequest, EngineConfig};
use crate::error::status_error;
use crate::http::{self, ResourceInfo, header_str};
use crate::meta::{self, META_VERSION, ResumeMeta};
use crate::progress::{Phase, Progress, SpeedMeter};
use crate::ratelimit::RateLimiter;
use crate::segments::{Assignment, SegmentId, SegmentMap};
use crate::worker::{self, Buffer, Shared, WorkerCtx, WorkerEnd};
use crate::writer::PartFile;
use crate::{DownloadError, blocking, filename, retry};

const TICK: Duration = Duration::from_millis(250);
const SPEED_WINDOW: Duration = Duration::from_secs(2);
/// The first request is retried less than segments: if the host is
/// unreachable the user should hear about it quickly.
const MAX_INITIAL_ATTEMPTS: u32 = 5;
const MAX_CONNECTIONS_CAP: usize = 32;

type WorkerSet = JoinSet<(SegmentId, Result<WorkerEnd, DownloadError>)>;

/// Cheap to clone; every clone shares the HTTP connection pool and limiter.
#[derive(Debug, Clone)]
pub struct Engine {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    client: Client,
    config: EngineConfig,
    limiter: Arc<RateLimiter>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Completed {
        path: PathBuf,
        bytes: u64,
        verified: Option<Checksum>,
    },
    /// Stopped on request. With `resumable`, starting the same request again
    /// continues where it left off.
    Paused {
        path: Option<PathBuf>,
        downloaded: u64,
        total: Option<u64>,
        resumable: bool,
    },
    /// Stopped and the partial data deleted.
    Canceled,
}

#[derive(Debug, Default)]
struct Control {
    stop: CancellationToken,
    cancel: AtomicBool,
}

/// A running download.
#[derive(Debug)]
pub struct DownloadHandle {
    progress: watch::Receiver<Progress>,
    control: Arc<Control>,
    task: tokio::task::JoinHandle<Result<Outcome, DownloadError>>,
}

/// Cloneable pause/cancel switch, for code that does not own the handle.
#[derive(Debug, Clone)]
pub struct DownloadControl {
    control: Arc<Control>,
}

impl DownloadControl {
    pub fn pause(&self) {
        self.control.stop.cancel();
    }

    pub fn cancel(&self) {
        self.control.cancel.store(true, Ordering::SeqCst);
        self.control.stop.cancel();
    }
}

impl DownloadHandle {
    pub fn progress(&self) -> watch::Receiver<Progress> {
        self.progress.clone()
    }

    pub fn control(&self) -> DownloadControl {
        DownloadControl {
            control: self.control.clone(),
        }
    }

    /// Stop, keeping resume data.
    pub fn pause(&self) {
        self.control.stop.cancel();
    }

    /// Stop and delete the partial file.
    pub fn cancel(&self) {
        self.control.cancel.store(true, Ordering::SeqCst);
        self.control.stop.cancel();
    }

    pub fn is_finished(&self) -> bool {
        self.task.is_finished()
    }

    pub async fn wait(self) -> Result<Outcome, DownloadError> {
        self.task
            .await
            .map_err(|e| DownloadError::Io(std::io::Error::other(e)))?
    }
}

impl Engine {
    pub fn new(config: EngineConfig) -> Result<Self, DownloadError> {
        Self::with_limiter(config, Arc::new(RateLimiter::unlimited()))
    }

    /// Share one limiter between engines to enforce a global speed limit.
    pub fn with_limiter(
        config: EngineConfig,
        limiter: Arc<RateLimiter>,
    ) -> Result<Self, DownloadError> {
        let client = http::build_client(&config)?;
        Ok(Self {
            inner: Arc::new(Inner {
                client,
                config,
                limiter,
            }),
        })
    }

    pub fn config(&self) -> &EngineConfig {
        &self.inner.config
    }

    pub fn limiter(&self) -> &Arc<RateLimiter> {
        &self.inner.limiter
    }

    /// Start (or resume) a download on the current Tokio runtime.
    pub fn start(&self, request: DownloadRequest) -> DownloadHandle {
        let (tx, rx) = watch::channel(Progress::starting());
        let control = Arc::new(Control::default());
        let job = Job::new(self.inner.clone(), request, control.clone(), tx);
        let task = tokio::spawn(job.run());
        DownloadHandle {
            progress: rx,
            control,
            task,
        }
    }

    /// Ask the server about a URL without downloading it (one tiny range).
    pub async fn probe(
        &self,
        url: &str,
        headers: &[(String, String)],
    ) -> Result<ResourceInfo, DownloadError> {
        let url = http::parse_url(url)?;
        let headers = http::build_headers(headers)?;
        let resp = http::range_get(&self.inner.client, &url, &headers, 0, Some(1), None)
            .send()
            .await
            .map_err(|e| DownloadError::from_reqwest(&e))?;
        let (accepts, total) = match resp.status() {
            StatusCode::PARTIAL_CONTENT => {
                let total = http::content_range(&resp).and_then(|cr| cr.complete);
                (total.is_some(), total)
            }
            StatusCode::RANGE_NOT_SATISFIABLE => (true, Some(0)),
            s if s.is_success() => (false, resp.content_length()),
            s => return Err(status_error(s.as_u16(), http::retry_after(&resp))),
        };
        Ok(ResourceInfo::from_response(&resp, accepts, total))
    }
}

/// An opened download, ready to transfer bytes.
#[derive(Debug)]
struct Session {
    final_path: PathBuf,
    part: Arc<PartFile>,
    /// Present when the download can be resumed (ranges + known size).
    meta: Option<ResumeMeta>,
    total: Option<u64>,
    /// URL after redirects, used by every connection of this session.
    work_url: Url,
    etag: Option<String>,
    body: Body,
}

#[derive(Debug)]
enum Body {
    Segmented {
        map: SegmentMap,
        first: Option<(Response, Assignment)>,
    },
    Stream {
        first: Response,
    },
}

enum Opened {
    Ready(Box<Session>),
    /// The resume data no longer matches the server; start over.
    Changed(String),
    Stopped,
}

enum Flow {
    Complete,
    Stopped,
}

struct Job {
    inner: Arc<Inner>,
    req: DownloadRequest,
    control: Arc<Control>,
    tx: watch::Sender<Progress>,
    max_connections: usize,
    /// Final path of the session in progress, for cleanup.
    current: Option<PathBuf>,
}

impl Job {
    fn new(
        inner: Arc<Inner>,
        req: DownloadRequest,
        control: Arc<Control>,
        tx: watch::Sender<Progress>,
    ) -> Self {
        let max_connections = req
            .max_connections
            .unwrap_or(inner.config.max_connections)
            .clamp(1, MAX_CONNECTIONS_CAP);
        Self {
            inner,
            req,
            control,
            tx,
            max_connections,
            current: None,
        }
    }

    async fn run(mut self) -> Result<Outcome, DownloadError> {
        let url = http::parse_url(&self.req.url)?;
        let headers = http::build_headers(&self.req.headers)?;
        let dir = self.req.output_dir.clone();
        blocking(move || std::fs::create_dir_all(&dir)).await?;

        let mut restarted = false;
        loop {
            match self.attempt(&url, &headers).await {
                Err(DownloadError::ResourceChanged(why)) if !restarted => {
                    restarted = true;
                    self.notice(format!("The remote file changed ({why}); starting over"));
                    self.discard_current().await;
                }
                Err(err) => {
                    tracing::warn!(url = %http::display_url(&url), kind = err.kind(), "download failed: {err}");
                    return Err(err);
                }
                Ok(outcome) => return Ok(outcome),
            }
        }
    }

    async fn attempt(&mut self, url: &Url, headers: &HeaderMap) -> Result<Outcome, DownloadError> {
        let mut session = None;
        if let Some((final_path, meta)) = self.find_resume(url).await {
            match self.open_resume(url, headers, final_path, meta).await? {
                Opened::Ready(s) => session = Some(s),
                Opened::Changed(why) => {
                    self.notice(format!("Could not resume ({why}); starting over"));
                    self.discard_current().await;
                }
                Opened::Stopped => return Ok(self.stopped_outcome(None).await),
            }
        }
        let mut session = match session {
            Some(s) => s,
            None => match self.open_fresh(url, headers).await? {
                Opened::Ready(s) => s,
                Opened::Changed(why) => return Err(DownloadError::ResourceChanged(why)),
                Opened::Stopped => return Ok(self.stopped_outcome(None).await),
            },
        };

        let body = std::mem::replace(
            &mut session.body,
            Body::Segmented {
                map: SegmentMap::new(0, &[]),
                first: None,
            },
        );
        let flow = match body {
            Body::Segmented { map, first } => {
                self.run_segmented(&mut session, headers, map, first)
                    .await?
            }
            Body::Stream { first } => self.run_stream(&session, url, headers, first).await?,
        };
        match flow {
            Flow::Complete => self.finalize(*session).await,
            Flow::Stopped => Ok(self.stopped_outcome(Some(*session)).await),
        }
    }

    // ---------------------------------------------------------------- open

    async fn find_resume(&self, url: &Url) -> Option<(PathBuf, ResumeMeta)> {
        let key = http::redact_url(url);
        let dir = self.req.output_dir.clone();
        let wanted = self.req.file_name.as_deref().map(filename::sanitize);
        tokio::task::spawn_blocking(move || match wanted {
            Some(name) => {
                let path = dir.join(name);
                let meta = meta::load(&meta::meta_path(&path)).ok()?;
                (meta.url == key).then_some((path, meta))
            }
            None => meta::find_for_url(&dir, &key),
        })
        .await
        .ok()
        .flatten()
    }

    async fn open_fresh(
        &mut self,
        url: &Url,
        headers: &HeaderMap,
    ) -> Result<Opened, DownloadError> {
        self.set_phase(Phase::Starting);
        let client = self.inner.client.clone();
        let Some(mut resp) = self
            .send_initial(|| http::range_get(&client, url, headers, 0, None, None))
            .await?
        else {
            return Ok(Opened::Stopped);
        };

        let mut range = None;
        match resp.status() {
            StatusCode::PARTIAL_CONTENT => match http::content_range(&resp) {
                Some(cr) if cr.first == 0 && cr.complete.is_some() => range = cr.complete,
                // A partial answer we cannot place; fetch the whole thing.
                _ => {
                    let Some(r) = self
                        .send_initial(|| http::plain_get(&client, url, headers))
                        .await?
                    else {
                        return Ok(Opened::Stopped);
                    };
                    resp = r;
                }
            },
            // `bytes=0-` on an empty file. Ask again without a range.
            StatusCode::RANGE_NOT_SATISFIABLE => {
                let Some(r) = self
                    .send_initial(|| http::plain_get(&client, url, headers))
                    .await?
                else {
                    return Ok(Opened::Stopped);
                };
                resp = r;
            }
            _ => {}
        }
        if !resp.status().is_success() {
            return Err(status_error(
                resp.status().as_u16(),
                http::retry_after(&resp),
            ));
        }
        let accepts = range.is_some();
        let total = range.or_else(|| {
            (resp.status() == StatusCode::OK)
                .then(|| resp.content_length())
                .flatten()
        });

        let info = ResourceInfo::from_response(&resp, accepts, total);
        let name = self
            .req
            .file_name
            .as_deref()
            .map(filename::sanitize)
            .unwrap_or_else(|| info.file_name.clone());
        let dir = self.req.output_dir.clone();
        let final_path = {
            let name = name.clone();
            blocking(move || Ok::<_, std::io::Error>(filename::unique_path(&dir, &name))).await?
        };
        let part = {
            let path = meta::part_path(&final_path);
            Arc::new(blocking(move || PartFile::create(&path, total)).await?)
        };
        self.current = Some(final_path.clone());

        let resumable = info.is_resumable();
        if !accepts {
            self.notice("The server does not support byte ranges: using one connection, and an interrupted download restarts from zero".into());
        } else if info.etag.is_none() && info.last_modified.is_none() {
            self.notice("The server sends no ETag or Last-Modified, so a resume cannot detect if the file changed".into());
        }

        let meta = match total {
            Some(total) if resumable => {
                let now = meta::unix_now();
                let meta = ResumeMeta {
                    version: META_VERSION,
                    url: http::redact_url(url),
                    final_url: info.final_url.clone(),
                    file_name: file_name_of(&final_path),
                    total_size: total,
                    etag: info.etag.clone(),
                    last_modified: info.last_modified.clone(),
                    content_type: info.content_type.clone(),
                    completed: Vec::new(),
                    checksum: self.req.checksum.as_ref().map(ToString::to_string),
                    created_at: now,
                    updated_at: now,
                };
                let path = meta::meta_path(&final_path);
                let m = meta.clone();
                blocking(move || meta::save(&path, &m)).await?;
                Some(meta)
            }
            _ => None,
        };

        self.tx.send_modify(|p| {
            p.file_name = Some(file_name_of(&final_path));
            p.path = Some(final_path.clone());
            p.total = total;
            p.resumable = meta.is_some();
        });

        let work_url = resp.url().clone();
        let body = match total {
            Some(total) if resumable => {
                let mut map = SegmentMap::new(total, &[]);
                let first = map
                    .assign(self.inner.config.min_split_size)
                    .ok_or_else(|| DownloadError::CorruptMeta("nothing to download".into()))?;
                Body::Segmented {
                    map,
                    first: Some((resp, first)),
                }
            }
            _ => Body::Stream { first: resp },
        };
        Ok(Opened::Ready(Box::new(Session {
            final_path,
            part,
            total,
            work_url,
            etag: meta.as_ref().and_then(|m| m.etag.clone()),
            meta,
            body,
        })))
    }

    async fn open_resume(
        &mut self,
        url: &Url,
        headers: &HeaderMap,
        final_path: PathBuf,
        meta: ResumeMeta,
    ) -> Result<Opened, DownloadError> {
        self.set_phase(Phase::Starting);
        self.current = Some(final_path.clone());
        let total = meta.total_size;
        let part = {
            let path = meta::part_path(&final_path);
            match blocking(move || PartFile::open_existing(&path, total)).await {
                Ok(part) => Arc::new(part),
                Err(_) => {
                    return Ok(Opened::Changed(
                        "the partial file is missing or truncated".into(),
                    ));
                }
            }
        };
        let mut map = SegmentMap::new(total, &meta.completed);
        self.tx.send_modify(|p| {
            p.file_name = Some(file_name_of(&final_path));
            p.path = Some(final_path.clone());
            p.total = Some(total);
            p.downloaded = map.written_bytes();
            p.resumable = true;
            p.segments = map.views();
        });

        let work_url;
        let first = match map.assign(self.inner.config.min_split_size) {
            // Everything was on disk already; only verification is left.
            None => {
                work_url = url.clone();
                None
            }
            Some(a) => {
                let validator = meta.if_range_validator().map(str::to_owned);
                if validator.is_none() {
                    self.notice("Resuming without a validator: the server gives no way to confirm the file is unchanged".into());
                }
                let client = self.inner.client.clone();
                let Some(resp) = self
                    .send_initial(|| {
                        http::range_get(
                            &client,
                            url,
                            headers,
                            a.start,
                            Some(a.end),
                            validator.as_deref(),
                        )
                    })
                    .await?
                else {
                    return Ok(Opened::Stopped);
                };
                match resp.status() {
                    StatusCode::PARTIAL_CONTENT => {
                        let cr = http::content_range(&resp);
                        if cr.map(|c| c.first) != Some(a.start) {
                            return Ok(Opened::Changed(
                                "the server returned a different range".into(),
                            ));
                        }
                        if cr.and_then(|c| c.complete) != Some(total) {
                            return Ok(Opened::Changed("the file size changed".into()));
                        }
                        if let (Some(old), Some(new)) =
                            (&meta.etag, header_str(resp.headers(), &ETAG))
                            && *old != new
                        {
                            return Ok(Opened::Changed("the ETag changed".into()));
                        }
                    }
                    StatusCode::OK => {
                        return Ok(Opened::Changed(
                            "the server sent the whole file instead of the missing part".into(),
                        ));
                    }
                    StatusCode::RANGE_NOT_SATISFIABLE => {
                        return Ok(Opened::Changed("the file got shorter".into()));
                    }
                    s => return Err(status_error(s.as_u16(), http::retry_after(&resp))),
                }
                work_url = resp.url().clone();
                Some((resp, a))
            }
        };

        Ok(Opened::Ready(Box::new(Session {
            final_path,
            part,
            total: Some(total),
            work_url,
            etag: meta.etag.clone(),
            meta: Some(meta),
            body: Body::Segmented { map, first },
        })))
    }

    /// Send the request that opens a session, retrying transient failures.
    /// `Ok(None)` means the download was stopped meanwhile.
    async fn send_initial<F>(&self, build: F) -> Result<Option<Response>, DownloadError>
    where
        F: Fn() -> RequestBuilder,
    {
        let cfg = &self.inner.config;
        let attempts = (cfg.max_retries + 1).min(MAX_INITIAL_ATTEMPTS);
        let mut last = None;
        for attempt in 0..attempts {
            let sent = tokio::select! {
                biased;
                () = self.control.stop.cancelled() => return Ok(None),
                r = build().send() => r,
            };
            let err = match sent {
                Ok(resp)
                    if resp.status().is_success()
                        || resp.status() == StatusCode::RANGE_NOT_SATISFIABLE =>
                {
                    self.set_phase(Phase::Downloading);
                    return Ok(Some(resp));
                }
                Ok(resp) => status_error(resp.status().as_u16(), http::retry_after(&resp)),
                Err(e) => DownloadError::from_reqwest(&e),
            };
            if !err.is_retryable() {
                return Err(err);
            }
            let delay = err.retry_after().unwrap_or_else(|| {
                retry::backoff(attempt, cfg.retry_base_delay, cfg.retry_max_delay)
            });
            tracing::debug!(attempt, ?delay, "initial request failed: {err}");
            self.set_phase(Phase::Waiting);
            tokio::select! {
                biased;
                () = self.control.stop.cancelled() => return Ok(None),
                () = tokio::time::sleep(delay) => {}
            }
            last = Some(err);
        }
        Err(DownloadError::RetriesExhausted {
            attempts,
            last: Box::new(last.unwrap_or(DownloadError::PrematureEnd)),
        })
    }

    // ------------------------------------------------------------ transfer

    async fn run_segmented(
        &self,
        session: &mut Session,
        headers: &HeaderMap,
        map: SegmentMap,
        first: Option<(Response, Assignment)>,
    ) -> Result<Flow, DownloadError> {
        let cfg = &self.inner.config;
        let max = self.max_connections;
        let min_split = cfg.min_split_size;
        let shared = Arc::new(Shared {
            downloaded: AtomicU64::new(map.written_bytes()),
            map: Mutex::new(map),
            part: session.part.clone(),
        });
        let ctx = Arc::new(WorkerCtx {
            client: self.inner.client.clone(),
            url: session.work_url.clone(),
            headers: headers.clone(),
            total: session.total.unwrap_or(0),
            etag: session.etag.clone(),
            limiter: self.inner.limiter.clone(),
            buffer_size: cfg.buffer_size,
            stall_timeout: cfg.stall_timeout,
        });
        let stop_workers = self.control.stop.child_token();
        let spawn = |workers: &mut WorkerSet, a: Assignment, first: Option<Response>| {
            workers.spawn(worker::run(
                shared.clone(),
                ctx.clone(),
                a,
                first,
                stop_workers.clone(),
            ));
        };

        let mut workers = WorkerSet::new();
        let mut target = cfg.initial_connections.clamp(1, max);
        let mut ramp = Ramp::new(cfg.ramp_window, cfg.ramp_gain);
        let mut retries: Vec<(Instant, SegmentId)> = Vec::new();
        let mut hold_until: Option<Instant> = None;
        let mut meter = SpeedMeter::new(SPEED_WINDOW);
        let mut tick = tokio::time::interval(TICK);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut last_checkpoint = Instant::now();
        // fsync can take a while under heavy writes; it runs beside the loop
        // so connection management never waits for the disk.
        let mut checkpointing: Option<CheckpointTask> = None;
        let mut stopping = false;
        let mut fatal: Option<DownloadError> = None;

        if let Some((resp, a)) = first {
            spawn(&mut workers, a, Some(resp));
        }

        loop {
            if !stopping && hold_until.is_none_or(|t| Instant::now() >= t) {
                hold_until = None;
                while workers.len() < target {
                    let Some(a) = shared.map.lock().assign(min_split) else {
                        break;
                    };
                    spawn(&mut workers, a, None);
                }
            }
            if workers.is_empty() {
                let mut map = shared.map.lock();
                if stopping || map.is_complete() {
                    break;
                }
                if retries.is_empty() && hold_until.is_none() && !map.has_assignable_work(min_split)
                {
                    // Every remaining segment is marked failed without a
                    // retry scheduled; make them eligible again.
                    map.rearm_all();
                }
            }

            tokio::select! {
                Some(joined) = workers.join_next(), if !workers.is_empty() => {
                    let (id, result) = match joined {
                        Ok(v) => v,
                        Err(e) => {
                            if fatal.is_none() {
                                fatal = Some(DownloadError::Io(std::io::Error::other(e)));
                            }
                            stopping = true;
                            stop_workers.cancel();
                            continue;
                        }
                    };
                    let err = match result {
                        Ok(WorkerEnd::Done | WorkerEnd::Stopped) => {
                            shared.map.lock().release(id, false);
                            continue;
                        }
                        Err(err) => err,
                    };
                    let failures = shared.map.lock().release(id, true);
                    if stopping {
                        continue;
                    }
                    if !err.is_retryable() || failures > cfg.max_retries {
                        tracing::debug!(segment = id, failures, "segment failed: {err}");
                        fatal = Some(if err.is_retryable() {
                            DownloadError::RetriesExhausted { attempts: failures, last: Box::new(err) }
                        } else {
                            err
                        });
                        stopping = true;
                        stop_workers.cancel();
                        continue;
                    }
                    let delay = err.retry_after().unwrap_or_else(|| {
                        retry::backoff(failures.saturating_sub(1), cfg.retry_base_delay, cfg.retry_max_delay)
                    });
                    if matches!(err, DownloadError::Throttled { .. }) {
                        let reduced = workers.len().max(1);
                        if reduced < target {
                            self.notice(format!("The server is rate limiting; reduced to {reduced} connection(s)"));
                        }
                        target = reduced.min(target);
                        ramp.disable();
                        hold_until = Some(Instant::now() + delay);
                    }
                    tracing::debug!(segment = id, failures, ?delay, "segment will retry: {err}");
                    retries.push((Instant::now() + delay, id));
                }
                _ = tick.tick() => {
                    let now = Instant::now();
                    retries.retain(|&(at, id)| {
                        if at <= now {
                            shared.map.lock().rearm(id);
                            false
                        } else {
                            true
                        }
                    });
                    let downloaded = shared.downloaded.load(Ordering::Relaxed);
                    let speed = meter.record(now, downloaded);
                    if !stopping {
                        ramp.observe(now, downloaded, workers.len(), &mut target, max);
                    }
                    if checkpointing.as_ref().is_some_and(tokio::task::JoinHandle::is_finished)
                        && let Some(task) = checkpointing.take()
                        && let Err(err) = finish_checkpoint(session, task).await
                    {
                        fatal.get_or_insert(err);
                        stopping = true;
                        stop_workers.cancel();
                    }
                    if checkpointing.is_none() && now.duration_since(last_checkpoint) >= cfg.checkpoint_interval {
                        checkpointing = spawn_checkpoint(session, &shared);
                        last_checkpoint = now;
                    }
                    let segments = shared.map.lock().views();
                    let phase = if workers.is_empty() { Phase::Waiting } else { Phase::Downloading };
                    self.tx.send_modify(|p| {
                        p.phase = phase;
                        p.downloaded = downloaded;
                        p.speed = speed;
                        p.connections = workers.len();
                        p.target_connections = target;
                        p.segments = segments;
                    });
                }
                () = self.control.stop.cancelled(), if !stopping => {
                    stopping = true;
                }
            }
        }

        if let Some(task) = checkpointing.take() {
            finish_checkpoint(session, task).await?;
        }
        if let Some(task) = spawn_checkpoint(session, &shared) {
            finish_checkpoint(session, task).await?;
        }
        let (complete, downloaded, segments) = {
            let map = shared.map.lock();
            (map.is_complete(), map.written_bytes(), map.views())
        };
        self.tx.send_modify(|p| {
            p.downloaded = downloaded;
            p.speed = 0;
            p.connections = 0;
            p.segments = segments;
        });
        if let Some(err) = fatal {
            return Err(err);
        }
        Ok(if complete {
            Flow::Complete
        } else {
            Flow::Stopped
        })
    }

    /// Single connection, start to end. Used when the server ignores ranges
    /// or does not announce a size; a failure restarts from zero.
    async fn run_stream(
        &self,
        session: &Session,
        url: &Url,
        headers: &HeaderMap,
        first: Response,
    ) -> Result<Flow, DownloadError> {
        let cfg = &self.inner.config;
        let client = self.inner.client.clone();
        let mut next = Some(first);
        let mut attempt = 0;
        loop {
            let resp = match next.take() {
                Some(resp) => resp,
                None => {
                    let Some(resp) = self
                        .send_initial(|| http::plain_get(&client, url, headers))
                        .await?
                    else {
                        return Ok(Flow::Stopped);
                    };
                    if resp.status() != StatusCode::OK {
                        return Err(status_error(
                            resp.status().as_u16(),
                            http::retry_after(&resp),
                        ));
                    }
                    let part = session.part.clone();
                    blocking(move || part.truncate(0)).await?;
                    resp
                }
            };
            match self.stream_body(session, resp).await {
                Err(err) if err.is_retryable() && attempt < cfg.max_retries => {
                    let delay = retry::backoff(attempt, cfg.retry_base_delay, cfg.retry_max_delay);
                    attempt += 1;
                    tracing::debug!(attempt, ?delay, "stream failed, restarting: {err}");
                    self.set_phase(Phase::Waiting);
                    tokio::select! {
                        biased;
                        () = self.control.stop.cancelled() => return Ok(Flow::Stopped),
                        () = tokio::time::sleep(delay) => {}
                    }
                }
                other => return other,
            }
        }
    }

    async fn stream_body(&self, session: &Session, resp: Response) -> Result<Flow, DownloadError> {
        let cfg = &self.inner.config;
        let total = session.total;
        let mut stream = resp.bytes_stream();
        let mut buf = Buffer::new(cfg.buffer_size);
        let mut received: u64 = 0;
        let mut meter = SpeedMeter::new(SPEED_WINDOW);
        let mut tick = tokio::time::interval(TICK);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        self.tx.send_modify(|p| {
            p.phase = Phase::Downloading;
            p.downloaded = 0;
            p.connections = 1;
            p.target_connections = 1;
        });

        loop {
            tokio::select! {
                biased;
                () = self.control.stop.cancelled() => {
                    buf.write(&session.part).await?;
                    return Ok(Flow::Stopped);
                }
                _ = tick.tick() => {
                    let speed = meter.record(Instant::now(), received);
                    self.tx.send_modify(|p| {
                        p.downloaded = received;
                        p.speed = speed;
                    });
                }
                next = tokio::time::timeout(cfg.stall_timeout, stream.next()) => {
                    let chunk = match next {
                        Err(_) => return Err(DownloadError::Stalled(cfg.stall_timeout)),
                        Ok(Some(Err(e))) => return Err(DownloadError::from_reqwest(&e)),
                        Ok(None) => {
                            buf.write(&session.part).await?;
                            if total.is_some_and(|t| received < t) {
                                return Err(DownloadError::PrematureEnd);
                            }
                            self.tx.send_modify(|p| {
                                p.downloaded = received;
                                p.speed = 0;
                                p.connections = 0;
                                p.total.get_or_insert(received);
                            });
                            return Ok(Flow::Complete);
                        }
                        Ok(Some(Ok(chunk))) => chunk,
                    };
                    self.inner.limiter.acquire(chunk.len()).await;
                    // Never write past an announced size.
                    let take = total.map_or(chunk.len() as u64, |t| (t - received).min(chunk.len() as u64));
                    buf.push(received, &chunk[..take as usize]);
                    received += take;
                    if buf.is_full() {
                        buf.write(&session.part).await?;
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------- finish

    async fn finalize(&mut self, session: Session) -> Result<Outcome, DownloadError> {
        let Session {
            final_path, part, ..
        } = session;
        {
            let part = part.clone();
            blocking(move || part.sync()).await?;
        }
        // Windows will not rename a file that still has open handles.
        drop(part);
        let part_path = meta::part_path(&final_path);

        let verified = match self.req.checksum.clone() {
            Some(expected) => {
                self.set_phase(Phase::Verifying);
                let path = part_path.clone();
                let actual = blocking(move || checksum::hash_file(&path, expected.algo)).await?;
                if actual != expected.hex {
                    return Err(DownloadError::ChecksumMismatch {
                        expected: expected.to_string(),
                        actual: format!("{}:{actual}", expected.algo.name()),
                    });
                }
                Some(expected)
            }
            None => None,
        };

        self.set_phase(Phase::Finalizing);
        let dest = {
            let final_path = final_path.clone();
            let part_path = part_path.clone();
            blocking(move || {
                let dest = if final_path.exists() {
                    let dir = final_path.parent().unwrap_or(Path::new("."));
                    filename::unique_path(dir, &file_name_of(&final_path))
                } else {
                    final_path.clone()
                };
                std::fs::rename(&part_path, &dest)?;
                meta::remove(&final_path);
                Ok::<_, std::io::Error>(dest)
            })
            .await?
        };
        let bytes = {
            let dest = dest.clone();
            blocking(move || std::fs::metadata(&dest).map(|m| m.len())).await?
        };
        self.current = None;
        self.tx.send_modify(|p| {
            p.phase = Phase::Done;
            p.file_name = Some(file_name_of(&dest));
            p.path = Some(dest.clone());
            p.downloaded = bytes;
            p.total = Some(bytes);
            p.speed = 0;
            p.connections = 0;
        });
        Ok(Outcome::Completed {
            path: dest,
            bytes,
            verified,
        })
    }

    async fn stopped_outcome(&mut self, session: Option<Session>) -> Outcome {
        let canceled = self.control.cancel.load(Ordering::SeqCst);
        let resumable = session.as_ref().is_some_and(|s| s.meta.is_some());
        let total = session.as_ref().and_then(|s| s.total);
        drop(session);
        if canceled || !resumable {
            // Without resume data the partial file is useless: the next
            // start begins from zero and would otherwise pick a new name.
            self.discard_current().await;
        }
        let progress = self.tx.borrow().clone();
        if canceled {
            return Outcome::Canceled;
        }
        Outcome::Paused {
            path: progress.path.filter(|_| resumable),
            downloaded: if resumable { progress.downloaded } else { 0 },
            total,
            resumable,
        }
    }

    async fn discard_current(&mut self) {
        if let Some(final_path) = self.current.take() {
            let _ = blocking(move || {
                let _ = std::fs::remove_file(meta::part_path(&final_path));
                meta::remove(&final_path);
                Ok::<_, std::io::Error>(())
            })
            .await;
        }
        self.tx.send_modify(|p| {
            p.downloaded = 0;
            p.segments.clear();
        });
    }

    fn set_phase(&self, phase: Phase) {
        self.tx.send_if_modified(|p| {
            let changed = p.phase != phase;
            p.phase = phase;
            changed
        });
    }

    fn notice(&self, text: String) {
        tracing::info!("{text}");
        self.tx.send_modify(|p| {
            if !p.notices.contains(&text) {
                p.notices.push(text);
            }
        });
    }
}

/// Decides when more connections are worth opening.
///
/// Starts at `initial_connections`, measures throughput over a window once
/// all connections are busy, then doubles. If a doubling did not improve
/// throughput by at least `gain`, it goes back to the previous count and
/// stops probing: more connections would only add load on the server.
#[derive(Debug)]
struct Ramp {
    enabled: bool,
    window: Duration,
    gain: f64,
    phase_start: Option<(Instant, u64)>,
    prev_rate: Option<f64>,
    prev_target: usize,
}

impl Ramp {
    fn new(window: Duration, gain: f64) -> Self {
        Self {
            enabled: true,
            window,
            gain,
            phase_start: None,
            prev_rate: None,
            prev_target: 1,
        }
    }

    fn disable(&mut self) {
        self.enabled = false;
    }

    fn observe(
        &mut self,
        now: Instant,
        downloaded: u64,
        active: usize,
        target: &mut usize,
        max: usize,
    ) {
        if !self.enabled {
            return;
        }
        if *target >= max {
            self.enabled = false;
            return;
        }
        if active < *target {
            // Not saturated (connecting, or no work to split): measure later.
            self.phase_start = None;
            return;
        }
        let Some((t0, b0)) = self.phase_start else {
            self.phase_start = Some((now, downloaded));
            return;
        };
        let elapsed = now.duration_since(t0);
        if elapsed < self.window {
            return;
        }
        let rate = downloaded.saturating_sub(b0) as f64 / elapsed.as_secs_f64();
        if let Some(prev) = self.prev_rate
            && rate < prev * self.gain
        {
            tracing::debug!(
                rate,
                prev,
                target = *target,
                "more connections did not help"
            );
            *target = self.prev_target;
            self.enabled = false;
            return;
        }
        tracing::debug!(rate, target = *target, "ramping up connections");
        self.prev_rate = Some(rate);
        self.prev_target = *target;
        *target = (*target * 2).min(max);
        self.phase_start = None;
    }
}

type CheckpointTask = tokio::task::JoinHandle<Result<ResumeMeta, DownloadError>>;

/// Sync the data, then record it in the meta file — in that order, so the
/// meta never claims bytes that a power cut could still lose.
fn spawn_checkpoint(session: &Session, shared: &Shared) -> Option<CheckpointTask> {
    let meta = session.meta.as_ref()?;
    let ranges = shared.map.lock().completed_ranges();
    if ranges == meta.completed {
        return None;
    }
    let mut next = meta.clone();
    next.completed = ranges;
    next.touch();
    let part = session.part.clone();
    let path = meta::meta_path(&session.final_path);
    Some(tokio::task::spawn_blocking(move || {
        part.sync()?;
        meta::save(&path, &next)?;
        Ok(next)
    }))
}

async fn finish_checkpoint(
    session: &mut Session,
    task: CheckpointTask,
) -> Result<(), DownloadError> {
    let saved = task
        .await
        .map_err(|e| DownloadError::Io(std::io::Error::other(e)))??;
    session.meta = Some(saved);
    Ok(())
}

fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ramp_doubles_while_it_helps_then_backs_off() {
        let mut ramp = Ramp::new(Duration::from_secs(1), 1.1);
        let mut target = 2;
        let t = Instant::now();
        let s = |secs: u64| t + Duration::from_secs(secs);

        ramp.observe(s(0), 0, 2, &mut target, 8);
        ramp.observe(s(1), 100, 2, &mut target, 8);
        assert_eq!(target, 4, "first window always ramps");

        ramp.observe(s(2), 100, 4, &mut target, 8);
        ramp.observe(s(3), 300, 4, &mut target, 8);
        assert_eq!(target, 8, "200 B/s beats 100 B/s");

        let mut target = 4;
        let mut ramp = Ramp::new(Duration::from_secs(1), 1.1);
        ramp.observe(s(0), 0, 4, &mut target, 16);
        ramp.observe(s(1), 1000, 4, &mut target, 16);
        assert_eq!(target, 8);
        ramp.observe(s(2), 1000, 8, &mut target, 16);
        ramp.observe(s(3), 2050, 8, &mut target, 16);
        assert_eq!(target, 4, "5 % gain is not worth 4 more connections");
        ramp.observe(s(10), 99_999, 4, &mut target, 16);
        assert_eq!(target, 4, "ramp stays off");
    }

    #[test]
    fn ramp_waits_for_saturation() {
        let mut ramp = Ramp::new(Duration::from_secs(1), 1.1);
        let mut target = 2;
        let t = Instant::now();
        ramp.observe(t, 0, 1, &mut target, 8);
        ramp.observe(t + Duration::from_secs(5), 500, 1, &mut target, 8);
        assert_eq!(target, 2);
    }
}
