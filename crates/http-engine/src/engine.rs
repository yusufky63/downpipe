//! The download controller: opens or resumes a download, runs the adaptive
//! connection loop, checkpoints, verifies and atomically publishes the file.

use std::collections::HashMap;
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
use crate::hosts;
use crate::http::{self, ResourceInfo, header_str};
use crate::media::resolve::{self, Fetcher};
use crate::media::{MediaKind, MediaOptions};
use crate::meta::{self, META_VERSION, ResumeMeta};
use crate::progress::{Phase, Progress, SpeedMeter, WaitReason};
use crate::ratelimit::{Limits, RateLimiter};
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
/// A download that received nothing for this long shows as waiting, not
/// downloading (capped at half the stall timeout, so it shows before the
/// silent connections are replaced).
const NO_DATA_AFTER: Duration = Duration::from_secs(10);

type WorkerSet = JoinSet<(SegmentId, Result<WorkerEnd, DownloadError>)>;

/// Cheap to clone; every clone shares the HTTP connection pool and limiter.
#[derive(Debug, Clone)]
pub struct Engine {
    inner: Arc<Inner>,
}

#[derive(Debug)]
pub(crate) struct Inner {
    pub(crate) client: Client,
    pub(crate) config: EngineConfig,
    pub(crate) limiter: Arc<RateLimiter>,
    /// Connections a host accepted when it refused more (`host:port` to
    /// count), so later downloads from it start there instead of tripping
    /// the limit again. Lives as long as the engine: a session.
    hosts: Mutex<HashMap<String, usize>>,
}

impl Inner {
    fn host_limit(&self, url: &Url) -> Option<usize> {
        self.hosts.lock().get(&http::host_key(url)).copied()
    }

    fn remember_host_limit(&self, url: &Url, connections: usize) {
        let connections = connections.max(1);
        self.hosts
            .lock()
            .entry(http::host_key(url))
            .and_modify(|n| *n = (*n).min(connections))
            .or_insert(connections);
    }
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
pub(crate) struct Control {
    pub(crate) stop: CancellationToken,
    pub(crate) cancel: AtomicBool,
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

    /// Run a download the engine does not perform itself (a site provider
    /// driving an external program) behind the same handle, so queues and
    /// UIs treat it like any other. `job` publishes its progress on the
    /// context and returns when it finished or was stopped. Must be called
    /// inside a Tokio runtime.
    pub fn spawn<F, Fut>(job: F) -> Self
    where
        F: FnOnce(JobContext) -> Fut,
        Fut: std::future::Future<Output = Result<Outcome, DownloadError>> + Send + 'static,
    {
        let (tx, rx) = watch::channel(Progress::starting());
        let control = Arc::new(Control::default());
        let task = tokio::spawn(job(JobContext {
            progress: tx,
            control: control.clone(),
        }));
        DownloadHandle {
            progress: rx,
            control,
            task,
        }
    }
}

/// What a job started with [`DownloadHandle::spawn`] reports through and
/// listens to.
#[derive(Debug)]
pub struct JobContext {
    /// Publish snapshots here; the handle's watchers see them.
    pub progress: watch::Sender<Progress>,
    control: Arc<Control>,
}

impl JobContext {
    /// Resolves once a pause or cancel was asked for.
    pub async fn stopped(&self) {
        self.control.stop.cancelled().await;
    }

    pub fn is_stopped(&self) -> bool {
        self.control.stop.is_cancelled()
    }

    /// The stop was a cancel: the job should delete its partial data and
    /// return [`Outcome::Canceled`].
    pub fn is_cancel(&self) -> bool {
        self.control.cancel.load(Ordering::SeqCst)
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
                hosts: Mutex::new(HashMap::new()),
            }),
        })
    }

    /// Connections the host of `url` accepted when it refused more during
    /// this session; downloads from it never open more than this.
    pub fn connection_limit(&self, url: &str) -> Option<usize> {
        self.inner.host_limit(&http::parse_url(url).ok()?)
    }

    pub fn config(&self) -> &EngineConfig {
        &self.inner.config
    }

    pub fn limiter(&self) -> &Arc<RateLimiter> {
        &self.inner.limiter
    }

    /// The HTTP client with this engine's proxy and user agent, for helper
    /// downloads that are not user downloads (a site provider fetching its
    /// tools).
    pub fn client(&self) -> &Client {
        &self.inner.client
    }

    /// Start (or resume) a download on the current Tokio runtime.
    pub fn start(&self, request: DownloadRequest) -> DownloadHandle {
        let (tx, rx) = watch::channel(Progress::starting());
        let control = Arc::new(Control::default());
        let task = match request.media.clone() {
            Some(media) => tokio::spawn(crate::media::job::run(
                self.inner.clone(),
                request,
                media,
                control.clone(),
                tx,
            )),
            None => tokio::spawn(Job::new(self.inner.clone(), request, control.clone(), tx).run()),
        };
        DownloadHandle {
            progress: rx,
            control,
            task,
        }
    }

    /// What a stream offers: its qualities and audio tracks, best first.
    /// Fetches the manifest (and nothing else for DASH or a master playlist
    /// that names its qualities), so it is quick and downloads no media.
    pub async fn media_options(
        &self,
        url: &str,
        kind: MediaKind,
        headers: &[(String, String)],
    ) -> Result<MediaOptions, DownloadError> {
        let url = http::parse_url(url)?;
        let fetcher = Fetcher {
            client: self.inner.client.clone(),
            headers: http::build_headers(headers)?,
            stop: CancellationToken::new(),
            retry_base: self.inner.config.retry_base_delay,
            retry_max: self.inner.config.retry_max_delay,
        };
        let source = resolve::load(&fetcher, &url, kind)
            .await?
            .ok_or(DownloadError::PrematureEnd)?;
        Ok(resolve::options(&source))
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
        Ok(ResourceInfo::from_response(&resp, &url, accepts, total))
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
    /// The file is a web page itself, so `text/html` answers are not a
    /// sign of an expired link.
    accept_html: bool,
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
    /// The partial file on disk is unusable; start over.
    Gone(String),
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
    /// The link the resume data is filed under: the one the user gave, also
    /// when a file host's share link is downloaded from another address.
    key: String,
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
            key: String::new(),
        }
    }

    fn limits(&self) -> Limits {
        Limits::new(self.inner.limiter.clone(), self.req.limiter.clone())
    }

    async fn run(mut self) -> Result<Outcome, DownloadError> {
        let given = http::parse_url(&self.req.url)?;
        let headers = http::build_headers(&self.req.headers)?;
        self.key = http::redact_url(&given);
        // Asked on every start: a host's confirmation token expires.
        // Stopped meanwhile: go on with the link as given, so the usual
        // paths below see the stop and keep or delete the partial data.
        let url = tokio::select! {
            biased;
            r = hosts::resolve(&self.inner.client, &given, &headers) => r?,
            _ = self.control.stop.cancelled() => given.clone(),
        };
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
        if let Some((final_path, meta)) = self.find_resume().await {
            match self.open_resume(url, headers, final_path, meta).await? {
                Opened::Ready(s) => session = Some(s),
                // A refreshed link must lead to the same file; starting over
                // would throw away the data the user asked to keep.
                Opened::Changed(why) if self.req.relink => {
                    self.current = None;
                    return Err(DownloadError::DifferentFile(why));
                }
                Opened::Changed(why) | Opened::Gone(why) => {
                    self.notice(format!("Could not resume ({why}); starting over"));
                    self.discard_current().await;
                }
                // Paused before the server answered: the data from earlier
                // sessions is untouched and must survive.
                Opened::Stopped => return Ok(self.stopped_while_resuming().await),
            }
        }
        let mut session = match session {
            Some(s) => s,
            None => match self.open_fresh(url, headers).await? {
                Opened::Ready(s) => s,
                Opened::Changed(why) | Opened::Gone(why) => {
                    return Err(DownloadError::ResourceChanged(why));
                }
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
            Body::Stream { first } => match self.run_stream(&session, url, headers, first).await {
                Ok(flow) => flow,
                Err(err) => {
                    // Without resume data the partial file is useless, and
                    // left behind it would push the retry to `name (1)`.
                    drop(session);
                    self.discard_current().await;
                    return Err(err);
                }
            },
        };
        match flow {
            Flow::Complete => self.finalize(*session).await,
            Flow::Stopped => Ok(self.stopped_outcome(Some(*session)).await),
        }
    }

    // ---------------------------------------------------------------- open

    async fn find_resume(&self) -> Option<(PathBuf, ResumeMeta)> {
        let key = self.key.clone();
        let dir = self.req.output_dir.clone();
        let wanted = self.req.file_name.as_deref().map(filename::sanitize);
        let relink = self.req.relink;
        tokio::task::spawn_blocking(move || match wanted {
            Some(name) => {
                let path = dir.join(name);
                let meta = meta::load(&meta::meta_path(&path)).ok()?;
                // A refreshed link is allowed to differ; open_resume then
                // proves it is still the same file.
                (relink || meta.url == key).then_some((path, meta))
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
            .send_initial(
                || http::range_get(&client, url, headers, 0, None, None),
                false,
            )
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
                        .send_initial(|| http::plain_get(&client, url, headers), false)
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
                    .send_initial(|| http::plain_get(&client, url, headers), false)
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
        // A link to `setup.exe` that answers with a web page is a sign-in
        // or "link expired" page; saving it as setup.exe helps nobody.
        if http::is_html(&resp) && self.expects_file(url) {
            return Err(http::page_error(&resp));
        }
        let accepts = range.is_some();
        let total = range.or_else(|| {
            (resp.status() == StatusCode::OK)
                .then(|| resp.content_length())
                .flatten()
        });

        let info = ResourceInfo::from_response(&resp, url, accepts, total);
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
                    url: self.key.clone(),
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
        let accept_html = info.content_type.as_deref().is_some_and(http::is_html_type);
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
            accept_html,
            meta,
            body,
        })))
    }

    /// The name the download will get says it is not a web page.
    fn expects_file(&self, url: &Url) -> bool {
        self.req
            .file_name
            .clone()
            .or_else(|| filename::from_url(url))
            .is_some_and(|name| filename::is_binary_name(&name))
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
                    return Ok(Opened::Gone(
                        "the partial file is missing or truncated".into(),
                    ));
                }
            }
        };
        let mut map = SegmentMap::new(total, &meta.completed);
        let accept_html = meta.content_type.as_deref().is_some_and(http::is_html_type);
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
                // Patient: the host served this file before, so a network
                // failure here is an outage to wait out, not a typo.
                let Some(resp) = self
                    .send_initial(
                        || {
                            http::range_get(
                                &client,
                                url,
                                headers,
                                a.start,
                                Some(a.end),
                                validator.as_deref(),
                            )
                        },
                        true,
                    )
                    .await?
                else {
                    return Ok(Opened::Stopped);
                };
                // Checked before anything else: a sign-in page answering
                // `200` would otherwise read as "the file changed" and throw
                // the partial data away.
                if resp.status().is_success() && !accept_html && http::is_html(&resp) {
                    return Err(http::page_error(&resp));
                }
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

        let mut meta = meta;
        if self.req.relink {
            // Proven to be the same file: from now on the resume data belongs
            // to the new link, so later resumes need no special handling.
            meta.url = self.key.clone();
            meta.final_url = http::redact_url(&work_url);
            meta.touch();
            let path = meta::meta_path(&final_path);
            let saved = meta.clone();
            blocking(move || meta::save(&path, &saved)).await?;
        }

        Ok(Opened::Ready(Box::new(Session {
            final_path,
            part,
            total: Some(total),
            work_url,
            etag: meta.etag.clone(),
            accept_html,
            meta: Some(meta),
            body: Body::Segmented { map, first },
        })))
    }

    /// Send the request that opens a session, retrying transient failures.
    /// `Ok(None)` means the download was stopped meanwhile.
    ///
    /// A new download gives up after a few attempts, so a mistyped or dead
    /// host is reported quickly. A `patient` one (a resume, a restart) waits
    /// out network failures for `offline_grace`: the host worked before.
    async fn send_initial<F>(
        &self,
        build: F,
        patient: bool,
    ) -> Result<Option<Response>, DownloadError>
    where
        F: Fn() -> RequestBuilder,
    {
        let cfg = &self.inner.config;
        let attempts = (cfg.max_retries + 1).min(MAX_INITIAL_ATTEMPTS);
        let budget = retry::Budget::new(attempts - 1, cfg.offline_grace);
        let mut failures = 0;
        loop {
            // connect_timeout only covers the TCP connect; a server that
            // accepts and never answers would otherwise hang here forever.
            let sent = tokio::select! {
                biased;
                () = self.control.stop.cancelled() => return Ok(None),
                r = tokio::time::timeout(cfg.response_timeout, build().send()) => r,
            };
            let err = match sent {
                Err(_) => DownloadError::Stalled(cfg.response_timeout),
                Ok(Ok(resp))
                    if resp.status().is_success()
                        || resp.status() == StatusCode::RANGE_NOT_SATISFIABLE =>
                {
                    self.set_phase(Phase::Downloading);
                    return Ok(Some(resp));
                }
                Ok(Ok(resp)) => status_error(resp.status().as_u16(), http::retry_after(&resp)),
                Ok(Err(e)) => DownloadError::from_reqwest(&e),
            };
            failures += 1;
            let allowed = if patient {
                budget.allows(&err, failures, Instant::now())
            } else {
                err.is_retryable() && failures < attempts
            };
            if !allowed {
                return Err(if err.is_retryable() {
                    DownloadError::RetriesExhausted {
                        attempts: failures,
                        last: Box::new(err),
                    }
                } else {
                    err
                });
            }
            let delay = err.retry_after().unwrap_or_else(|| {
                retry::backoff(failures - 1, cfg.retry_base_delay, cfg.retry_max_delay)
            });
            tracing::debug!(failures, ?delay, "initial request failed: {err}");
            self.set_waiting(err.wait_reason());
            tokio::select! {
                biased;
                () = self.control.stop.cancelled() => return Ok(None),
                () = tokio::time::sleep(delay) => {}
            }
        }
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
        let mut max = self.max_connections;
        if let Some(limit) = self.inner.host_limit(&session.work_url)
            && limit < max
        {
            max = limit;
            self.notice(format!(
                "This server refused more than {limit} connection(s) earlier; using {limit}"
            ));
        }
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
            limits: self.limits(),
            buffer_size: cfg.buffer_size,
            stall_timeout: cfg.stall_timeout,
            response_timeout: cfg.response_timeout,
            accept_html: session.accept_html,
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
        let mut budget = retry::Budget::new(cfg.max_retries, cfg.offline_grace);
        // (due, segment, failed on the network): network retries are pulled
        // forward as soon as bytes flow again.
        let mut retries: Vec<(Instant, SegmentId, bool)> = Vec::new();
        let mut hold_until: Option<Instant> = None;
        // The host refused extra connections during this download.
        let mut limited = false;
        // The latest failure and when it happened: what a waiting download
        // shows, unless nothing failed since the bytes stopped.
        let mut last_failure = (Instant::now(), WaitReason::Network);
        let mut last_bytes = (Instant::now(), shared.downloaded.load(Ordering::Relaxed));
        let no_data_after = no_data_after(cfg.stall_timeout);
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
                    let others = workers.len();
                    if !stopping && others > 0 && is_refusal(&err) {
                        // Refused while other connections of this download
                        // are being served: the host caps connections per
                        // client. Not a failure; continue with the ones it
                        // accepts and hand this segment to the next free one.
                        shared.map.lock().release(id, false);
                        limited = true;
                        ramp.disable();
                        if others < target {
                            self.notice(format!(
                                "The server allows only {others} connection(s) at a time; continuing with {others}"
                            ));
                            self.inner.remember_host_limit(&session.work_url, others);
                        }
                        target = target.min(others);
                        max = max.min(others);
                        if let Some(after) = err.retry_after() {
                            hold_until = Some(Instant::now() + after);
                        }
                        tracing::debug!(segment = id, others, "connection refused, host limit: {err}");
                        continue;
                    }
                    let failures = shared.map.lock().release(id, true);
                    if stopping {
                        continue;
                    }
                    let now = Instant::now();
                    // A refusal with no other connection left is usually the
                    // server still counting one that just closed, once it
                    // showed a limit; retried like a server failure. A link
                    // that really expired runs out of retries and fails.
                    let lingering = limited && matches!(err, DownloadError::Forbidden);
                    let allowed = if lingering {
                        failures <= cfg.max_retries
                    } else {
                        budget.allows(&err, failures, now)
                    };
                    if !allowed {
                        tracing::debug!(segment = id, failures, "segment failed: {err}");
                        fatal = Some(if err.is_retryable() || lingering {
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
                        let reduced = others.max(1);
                        if reduced < target {
                            self.notice(format!("The server is rate limiting; reduced to {reduced} connection(s)"));
                        }
                        target = reduced.min(target);
                        ramp.disable();
                        hold_until = Some(now + delay);
                    }
                    last_failure = (Instant::now(), err.wait_reason());
                    tracing::debug!(segment = id, failures, ?delay, "segment will retry: {err}");
                    retries.push((now + delay, id, err.is_connectivity()));
                }
                _ = tick.tick() => {
                    let now = Instant::now();
                    let downloaded = shared.downloaded.load(Ordering::Relaxed);
                    if downloaded > last_bytes.1 {
                        last_bytes = (now, downloaded);
                        budget.progressed(now);
                        // The network is back: segments sitting out a long
                        // backoff from the outage need not wait any longer.
                        for retry in retries.iter_mut().filter(|r| r.2) {
                            retry.0 = retry.0.min(now);
                        }
                    }
                    retries.retain(|&(at, id, _)| {
                        if at <= now {
                            shared.map.lock().rearm(id);
                            false
                        } else {
                            true
                        }
                    });
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
                    // Never "downloading" while nothing moves: either every
                    // connection is backing off, or the open ones are silent.
                    let waiting = if workers.is_empty() {
                        Some(last_failure.1)
                    } else if now.duration_since(last_bytes.0) >= no_data_after {
                        // Connections are open (or opening) but nothing comes:
                        // silence, unless they keep failing, as while offline.
                        Some(if last_failure.0 > last_bytes.0 { last_failure.1 } else { WaitReason::NoData })
                    } else {
                        None
                    };
                    self.tx.send_modify(|p| {
                        p.phase = if waiting.is_some() { Phase::Waiting } else { Phase::Downloading };
                        p.waiting = waiting;
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
                    // Patient: an outage is waited out here, while failures
                    // mid-body are counted below, since each one starts over.
                    let Some(resp) = self
                        .send_initial(|| http::plain_get(&client, url, headers), true)
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
                    if !session.accept_html && http::is_html(&resp) {
                        return Err(http::page_error(&resp));
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
                    self.set_waiting(err.wait_reason());
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
        let no_data_after = no_data_after(cfg.stall_timeout);
        let mut last_bytes = (Instant::now(), 0u64);
        let limits = self.limits();
        self.tx.send_modify(|p| {
            p.phase = Phase::Downloading;
            p.waiting = None;
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
                    let now = Instant::now();
                    if received > last_bytes.1 {
                        last_bytes = (now, received);
                    }
                    let silent = now.duration_since(last_bytes.0) >= no_data_after;
                    let speed = meter.record(now, received);
                    self.tx.send_modify(|p| {
                        p.phase = if silent { Phase::Waiting } else { Phase::Downloading };
                        p.waiting = silent.then_some(WaitReason::NoData);
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
                    limits.acquire(chunk.len()).await;
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

    async fn stopped_while_resuming(&mut self) -> Outcome {
        if self.control.cancel.load(Ordering::SeqCst) {
            self.discard_current().await;
            return Outcome::Canceled;
        }
        self.current = None;
        let progress = self.tx.borrow().clone();
        Outcome::Paused {
            path: progress.path,
            downloaded: progress.downloaded,
            total: progress.total,
            resumable: true,
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
            let changed = p.phase != phase || p.waiting.is_some();
            p.phase = phase;
            p.waiting = None;
            changed
        });
    }

    fn set_waiting(&self, reason: WaitReason) {
        self.tx.send_if_modified(|p| {
            let changed = p.phase != Phase::Waiting || p.waiting != Some(reason);
            p.phase = Phase::Waiting;
            p.waiting = Some(reason);
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

/// `403` or `429`/`503`: what hosts that cap connections per client answer
/// to one connection too many.
fn is_refusal(err: &DownloadError) -> bool {
    matches!(
        err,
        DownloadError::Forbidden | DownloadError::Throttled { .. }
    )
}

fn no_data_after(stall_timeout: Duration) -> Duration {
    NO_DATA_AFTER.min(stall_timeout / 2)
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
