//! Downloading a stream: fragments in parallel into the work folder, then
//! joined per track and merged (or moved) into the final file.
//!
//! The same guarantees as a single-file download hold: progress survives a
//! pause, a restart or a crash (a fragment is renamed to `.frag` only after
//! its bytes are synced), and the final name appears only when the file is
//! complete.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use reqwest::header::{HeaderMap, RANGE};
use reqwest::{Client, StatusCode};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::watch;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use url::Url;

use super::resolve::{self, Fetcher, Plan, TrackRole};
use super::state::{self, WorkState};
use super::{Container, MediaRequest, Segment, crypto, mux};
use crate::config::DownloadRequest;
use crate::engine::{Control, Inner, Outcome};
use crate::error::status_error;
use crate::progress::{MediaProgress, Phase, Progress, SpeedMeter, WaitReason};
use crate::ratelimit::Limits;
use crate::segments::SegmentView;
use crate::{DownloadError, blocking, filename, http, retry};

const TICK: Duration = Duration::from_millis(250);
const SPEED_WINDOW: Duration = Duration::from_secs(2);
/// An AES key is 16 bytes; anything much bigger is not a key.
const MAX_KEY_BYTES: usize = 64;
/// Extensions a name hint may carry that say nothing about the result.
const STREAM_EXTENSIONS: &[&str] = &[
    "m3u8", "m3u", "mpd", "mp4", "m4a", "m4v", "ts", "webm", "mkv", "aac", "mp3",
];

pub(crate) async fn run(
    inner: Arc<Inner>,
    req: DownloadRequest,
    media: MediaRequest,
    control: Arc<Control>,
    tx: watch::Sender<Progress>,
) -> Result<Outcome, DownloadError> {
    let url = http::parse_url(&req.url)?;
    let mut job = MediaJob {
        inner,
        req,
        media,
        control,
        tx,
        final_path: None,
    };
    let result = job.run(&url).await;
    if let Err(err) = &result {
        tracing::warn!(url = %http::display_url(&url), kind = err.kind(), "stream download failed: {err}");
    }
    result
}

struct MediaJob {
    inner: Arc<Inner>,
    req: DownloadRequest,
    media: MediaRequest,
    control: Arc<Control>,
    tx: watch::Sender<Progress>,
    /// Where the result goes; its work folder is `<final>.part`.
    final_path: Option<PathBuf>,
}

/// One request's worth of a track, by its place in the joined file.
#[derive(Debug, Clone)]
struct Fragment {
    track: usize,
    index: usize,
    segment: Segment,
}

enum FragEnd {
    Done(u64),
    Stopped,
}

enum Flow {
    Complete,
    Stopped,
}

/// What the finished download is made of: one file, or video and audio
/// merged by FFmpeg, or both kept separately when there is no FFmpeg.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Assembly {
    Move,
    Remux,
    Merge,
    Separate,
}

impl MediaJob {
    async fn run(&mut self, url: &Url) -> Result<Outcome, DownloadError> {
        let headers = http::build_headers(&self.req.headers)?;
        let dir = self.req.output_dir.clone();
        blocking(move || std::fs::create_dir_all(&dir)).await?;
        self.set_phase(Phase::Starting);
        self.show_resumable_bytes().await;

        let fetcher = Fetcher {
            client: self.inner.client.clone(),
            headers: headers.clone(),
            stop: self.control.stop.clone(),
            retry_base: self.inner.config.retry_base_delay,
            retry_max: self.inner.config.retry_max_delay,
        };
        let Some(source) = resolve::load(&fetcher, url, self.media.kind).await? else {
            return self.stopped(None, 0).await;
        };
        let Some(plan) = resolve::plan(&fetcher, source, &self.media.selection).await? else {
            return self.stopped(None, 0).await;
        };
        let ffmpeg = self.media.ffmpeg.clone().filter(|p| p.is_file());
        let assembly = assembly(&plan, ffmpeg.is_some());
        let mut work_state = WorkState::new(http::redact_url(url), self.media.kind, &plan);
        let final_path = self.place(&plan, assembly, &work_state).await?;
        let work = state::work_dir(&final_path);
        self.final_path = Some(final_path.clone());
        work_state = self.open_work(&work, work_state).await?;

        let fragments: Vec<Fragment> = plan
            .tracks
            .iter()
            .enumerate()
            .flat_map(|(t, track)| {
                track.fragments().enumerate().map(move |(i, s)| Fragment {
                    track: t,
                    index: i,
                    segment: s.clone(),
                })
            })
            .collect();
        let estimate = plan
            .tracks
            .iter()
            .map(|t| t.estimated_bytes)
            .sum::<Option<u64>>();
        self.tx.send_modify(|p| {
            p.file_name = Some(file_name_of(&final_path));
            p.path = Some(final_path.clone());
            p.resumable = true;
            p.total = estimate;
            p.media = Some(MediaProgress {
                quality: plan.quality.clone(),
                selection: plan.selection.clone(),
                fragments: fragments.len(),
                fragments_done: 0,
            });
        });

        let ctx = Arc::new(FragCtx {
            client: self.inner.client.clone(),
            headers,
            limiter: Limits::new(self.inner.limiter.clone(), self.req.limiter.clone()),
            stall_timeout: self.inner.config.stall_timeout,
            response_timeout: self.inner.config.response_timeout,
            buffer_size: self.inner.config.buffer_size,
            work: work.clone(),
            fetcher,
            keys: tokio::sync::Mutex::new(HashMap::new()),
            in_flight: AtomicU64::new(0),
        });
        let (flow, bytes) = self
            .download(&ctx, &fragments, &work_state, estimate)
            .await?;
        match flow {
            Flow::Stopped => self.stopped(estimate, bytes).await,
            Flow::Complete => {
                self.finish(&plan, &mut work_state, assembly, ffmpeg.as_deref())
                    .await
            }
        }
    }

    /// Before the manifest answers, show what an earlier session left on disk.
    async fn show_resumable_bytes(&self) {
        let Some(name) = self.req.file_name.as_deref() else {
            return;
        };
        let work = state::work_dir(&self.req.output_dir.join(filename::sanitize(name)));
        let bytes = blocking(move || Ok::<_, std::io::Error>(fragment_bytes(&work)))
            .await
            .unwrap_or(0);
        if bytes > 0 {
            self.tx.send_modify(|p| p.downloaded = bytes);
        }
    }

    /// The final path: the folder of an earlier session of this stream when
    /// there is one, else a free name with the right extension.
    async fn place(
        &self,
        plan: &Plan,
        assembly: Assembly,
        fresh: &WorkState,
    ) -> Result<PathBuf, DownloadError> {
        let dir = self.req.output_dir.clone();
        let ext = output_extension(plan, assembly).to_string();
        let hint = self
            .req
            .file_name
            .as_deref()
            .map(filename::sanitize)
            .or_else(|| {
                http::parse_url(&self.req.url)
                    .ok()
                    .and_then(|u| filename::from_url(&u))
            })
            .unwrap_or_else(|| "video".to_string());
        let fresh = fresh.clone();
        // A link the user refreshed is allowed to differ; the fragment
        // lists still have to match (see `open_work`).
        let relink = self.req.relink;
        blocking(move || {
            let wanted = with_extension(&hint, &ext);
            // An earlier session under the recorded (or default) name, as it
            // is and with the extension this run would give it. Running the
            // same CLI command again finds its folder this way too.
            for name in [hint.as_str(), wanted.as_str()] {
                let path = dir.join(name);
                if state::load(&state::work_dir(&path))
                    .is_some_and(|s| relink || s.url == fresh.url)
                {
                    return Ok::<_, std::io::Error>(path);
                }
            }
            Ok(filename::unique_path(&dir, &wanted))
        })
        .await
    }

    /// Create the work folder, or check that the one found holds this
    /// stream. Returns the state to continue with.
    async fn open_work(&self, work: &Path, fresh: WorkState) -> Result<WorkState, DownloadError> {
        let work = work.to_path_buf();
        let (state, changed) = blocking(move || {
            std::fs::create_dir_all(&work)?;
            let (state, changed) = match state::load(&work) {
                Some(old) if old.same_stream(&fresh) => {
                    // Keep what was joined; take the current link.
                    let state = WorkState {
                        url: fresh.url,
                        ..old
                    };
                    (state, false)
                }
                Some(_) => {
                    state::clear(&work)?;
                    (fresh, true)
                }
                None => (fresh, false),
            };
            state::save(&work, &state)?;
            Ok::<_, DownloadError>((state, changed))
        })
        .await?;
        if changed {
            self.notice("The stream changed since the download was paused; starting over".into());
        }
        Ok(state)
    }

    async fn download(
        &self,
        ctx: &Arc<FragCtx>,
        fragments: &[Fragment],
        work_state: &WorkState,
        estimate: Option<u64>,
    ) -> Result<(Flow, u64), DownloadError> {
        let cfg = &self.inner.config;
        let total_count = fragments.len();
        let joined: Vec<bool> = work_state.tracks.iter().map(|t| t.joined).collect();
        // What earlier sessions finished: whole tracks already joined, and
        // single fragments.
        let work = ctx.work.clone();
        let layout: Vec<(usize, usize)> = fragments.iter().map(|f| (f.track, f.index)).collect();
        let containers: Vec<Container> = work_state.tracks.iter().map(|t| t.container).collect();
        let found = blocking(move || {
            Ok::<_, std::io::Error>(
                layout
                    .iter()
                    .map(|&(t, i)| {
                        if joined.get(t).copied().unwrap_or(false) {
                            let track = state::track_path(&work, t, containers[t]);
                            // Counted once, on the track's first fragment.
                            return Some(if i == 0 {
                                file_len(&track).unwrap_or(0)
                            } else {
                                0
                            });
                        }
                        file_len(&state::fragment_path(&work, t, i))
                    })
                    .collect::<Vec<Option<u64>>>(),
            )
        })
        .await?;
        let mut done = vec![false; total_count];
        let mut done_bytes = 0u64;
        let mut done_count = 0usize;
        let mut pending = VecDeque::new();
        for (i, size) in found.into_iter().enumerate() {
            match size {
                Some(size) => {
                    done[i] = true;
                    done_bytes += size;
                    done_count += 1;
                }
                None => pending.push_back(i),
            }
        }

        let max = self
            .req
            .max_connections
            .unwrap_or(cfg.max_connections)
            .clamp(1, 32);
        let mut target = max;
        let stop_workers = self.control.stop.child_token();
        let mut workers: JoinSet<(usize, Result<FragEnd, DownloadError>)> = JoinSet::new();
        let mut failures = vec![0u32; total_count];
        let mut retries: Vec<(Instant, usize, bool)> = Vec::new();
        let mut hold_until: Option<Instant> = None;
        let mut budget = retry::Budget::new(self.req.retries(cfg), cfg.offline_grace);
        // The latest failure and when it happened: what a waiting download
        // shows, unless nothing failed since the bytes stopped.
        let mut last_failure = (Instant::now(), WaitReason::Network);
        let mut last_bytes = (Instant::now(), done_bytes);
        let no_data_after = Duration::from_secs(10).min(cfg.stall_timeout / 2);
        let mut meter = SpeedMeter::new(SPEED_WINDOW);
        let mut tick = tokio::time::interval(TICK);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut stopping = false;
        let mut fatal: Option<DownloadError> = None;
        self.set_phase(Phase::Downloading);

        loop {
            if !stopping && hold_until.is_none_or(|t| Instant::now() >= t) {
                hold_until = None;
                while workers.len() < target {
                    let Some(i) = pending.pop_front() else { break };
                    let ctx = ctx.clone();
                    let fragment = fragments[i].clone();
                    let stop = stop_workers.clone();
                    workers.spawn(async move { (i, fetch_fragment(&ctx, &fragment, &stop).await) });
                }
            }
            if workers.is_empty() && (stopping || (pending.is_empty() && retries.is_empty())) {
                break;
            }
            tokio::select! {
                Some(joined) = workers.join_next(), if !workers.is_empty() => {
                    let (i, result) = match joined {
                        Ok(v) => v,
                        Err(e) => {
                            fatal.get_or_insert(DownloadError::Io(std::io::Error::other(e)));
                            stopping = true;
                            stop_workers.cancel();
                            continue;
                        }
                    };
                    match result {
                        Ok(FragEnd::Done(size)) => {
                            done[i] = true;
                            done_bytes += size;
                            done_count += 1;
                        }
                        Ok(FragEnd::Stopped) => {}
                        Err(_) if stopping => {}
                        Err(err) => {
                            failures[i] += 1;
                            if !budget.allows(&err, failures[i], Instant::now()) {
                                tracing::debug!(fragment = i, failures = failures[i], "fragment failed: {err}");
                                fatal = Some(if err.is_retryable() {
                                    DownloadError::RetriesExhausted { attempts: failures[i], last: Box::new(err) }
                                } else {
                                    err
                                });
                                stopping = true;
                                stop_workers.cancel();
                                continue;
                            }
                            let delay = err.retry_after().unwrap_or_else(|| {
                                retry::backoff(failures[i] - 1, cfg.retry_base_delay, cfg.retry_max_delay)
                            });
                            if matches!(err, DownloadError::Throttled { .. }) {
                                let reduced = (target / 2).max(1);
                                if reduced < target {
                                    self.notice(format!("The server is rate limiting; reduced to {reduced} connection(s)"));
                                }
                                target = reduced;
                                hold_until = Some(Instant::now() + delay);
                            }
                            tracing::debug!(fragment = i, failures = failures[i], ?delay, "fragment will retry: {err}");
                            last_failure = (Instant::now(), err.wait_reason());
                            retries.push((Instant::now() + delay, i, err.is_connectivity()));
                        }
                    }
                }
                _ = tick.tick() => {
                    let now = Instant::now();
                    let downloaded = done_bytes + ctx.in_flight.load(Ordering::Relaxed);
                    if downloaded > last_bytes.1 {
                        last_bytes = (now, downloaded);
                        budget.progressed(now);
                        // Back online: no need to sit out the outage's backoff.
                        for retry in retries.iter_mut().filter(|r| r.2) {
                            retry.0 = retry.0.min(now);
                        }
                    }
                    // In-flight bytes are taken out again when an attempt
                    // fails, so the count can go down.
                    last_bytes.1 = last_bytes.1.min(downloaded);
                    retries.retain(|&(at, i, _)| {
                        if at <= now {
                            pending.push_front(i);
                            false
                        } else {
                            true
                        }
                    });
                    let speed = meter.record(now, downloaded);
                    let total = estimated_total(done_bytes, done_count, total_count, estimate);
                    let views = fragment_views(&done, total);
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
                        p.total = total;
                        p.speed = speed;
                        p.connections = workers.len();
                        p.target_connections = target;
                        p.segments = views;
                        if let Some(m) = p.media.as_mut() {
                            m.fragments_done = done_count;
                        }
                    });
                }
                () = self.control.stop.cancelled(), if !stopping => {
                    stopping = true;
                }
            }
        }

        let total = estimated_total(done_bytes, done_count, total_count, estimate);
        self.tx.send_modify(|p| {
            p.downloaded = done_bytes;
            p.total = total;
            p.speed = 0;
            p.connections = 0;
            p.segments = fragment_views(&done, total);
            if let Some(m) = p.media.as_mut() {
                m.fragments_done = done_count;
            }
        });
        if let Some(err) = fatal {
            return Err(err);
        }
        Ok((
            if done_count == total_count {
                Flow::Complete
            } else {
                Flow::Stopped
            },
            done_bytes,
        ))
    }

    /// Join each track's fragments, then merge or move the tracks into place.
    async fn finish(
        &mut self,
        plan: &Plan,
        work_state: &mut WorkState,
        assembly: Assembly,
        ffmpeg: Option<&Path>,
    ) -> Result<Outcome, DownloadError> {
        self.set_phase(Phase::Finalizing);
        let final_path = self
            .final_path
            .clone()
            .ok_or_else(|| std::io::Error::other("no final path"))?;
        let work = state::work_dir(&final_path);
        for (t, track) in plan.tracks.iter().enumerate() {
            if work_state.tracks[t].joined {
                continue;
            }
            let count = track.fragment_count();
            let container = track.container;
            let w = work.clone();
            blocking(move || join_track(&w, t, count, container)).await?;
            work_state.tracks[t].joined = true;
            let (w, s) = (work.clone(), work_state.clone());
            blocking(move || state::save(&w, &s)).await?;
            // Only now, with the joined track synced and recorded, are its
            // fragments redundant.
            let w = work.clone();
            blocking(move || {
                for i in 0..count {
                    let _ = std::fs::remove_file(state::fragment_path(&w, t, i));
                }
                Ok::<_, std::io::Error>(())
            })
            .await?;
        }

        let inputs: Vec<(PathBuf, Container)> = plan
            .tracks
            .iter()
            .enumerate()
            .map(|(t, track)| {
                (
                    state::track_path(&work, t, track.container),
                    track.container,
                )
            })
            .collect();
        let mut result = match (assembly, ffmpeg) {
            (Assembly::Remux | Assembly::Merge, Some(ffmpeg)) => {
                let out = work.join(format!("output.{}", extension_of(&final_path)));
                let merged = tokio::select! {
                    biased;
                    () = self.control.stop.cancelled() => return self.stopped(None, 0).await,
                    r = mux::run(ffmpeg, &inputs, &out) => r,
                };
                match merged {
                    Ok(()) => Ok(vec![out]),
                    Err(why) => Err(why),
                }
            }
            _ => Err(String::new()),
        };
        if let Err(why) = &result
            && matches!(assembly, Assembly::Remux | Assembly::Merge)
        {
            self.notice(format!(
                "Could not merge with FFmpeg ({why}); saved the tracks as they came"
            ));
        }
        let separate = inputs.len() > 1 && result.is_err();
        if separate && assembly == Assembly::Separate {
            self.notice(
                "FFmpeg was not found, so the video and its sound were saved as two files. \
                 Install FFmpeg, or set its path in Settings, to have them merged."
                    .into(),
            );
        }
        if result.is_err() {
            result = Ok(inputs.iter().map(|(p, _)| p.clone()).collect());
        }
        let sources = result.unwrap_or_default();

        let moves: Vec<(PathBuf, PathBuf)> = {
            let dir = final_path.parent().unwrap_or(Path::new(".")).to_path_buf();
            let stem = stem_of(&final_path);
            sources
                .iter()
                .enumerate()
                .map(|(i, src)| {
                    let src_ext = src
                        .extension()
                        .map(|e| e.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    let name = if i == 0 {
                        // The joined track's own extension when it was not
                        // remuxed (a transport stream stays `.ts`).
                        let ext = if src
                            .file_name()
                            .is_some_and(|n| n.to_string_lossy().starts_with("output."))
                        {
                            extension_of(&final_path)
                        } else {
                            src_ext
                        };
                        format!("{stem}.{ext}")
                    } else {
                        format!("{stem}.audio.{src_ext}")
                    };
                    (src.clone(), dir.join(name))
                })
                .collect()
        };
        let placed = blocking(move || {
            let mut placed = Vec::new();
            for (src, wanted) in moves {
                let dest = if wanted.exists() {
                    let dir = wanted.parent().unwrap_or(Path::new(".")).to_path_buf();
                    filename::unique_path(&dir, &file_name_of(&wanted))
                } else {
                    wanted
                };
                std::fs::rename(&src, &dest)?;
                placed.push(dest);
            }
            Ok::<_, std::io::Error>(placed)
        })
        .await?;
        let main = placed
            .first()
            .cloned()
            .ok_or_else(|| std::io::Error::other("nothing to save"))?;
        let w = work.clone();
        blocking(move || std::fs::remove_dir_all(&w)).await?;
        let paths = placed.clone();
        let bytes = blocking(move || {
            Ok::<_, std::io::Error>(paths.iter().filter_map(|p| file_len(p)).sum::<u64>())
        })
        .await?;
        if placed.len() > 1 {
            let names: Vec<String> = placed.iter().map(|p| file_name_of(p)).collect();
            self.notice(format!("Saved as {}", names.join(" and ")));
        }
        self.final_path = None;
        self.tx.send_modify(|p| {
            p.phase = Phase::Done;
            p.file_name = Some(file_name_of(&main));
            p.path = Some(main.clone());
            p.downloaded = bytes;
            p.total = Some(bytes);
            p.speed = 0;
            p.connections = 0;
            p.segments.clear();
            if let Some(m) = p.media.as_mut() {
                m.fragments_done = m.fragments;
            }
        });
        Ok(Outcome::Completed {
            path: main,
            bytes,
            verified: None,
        })
    }

    async fn stopped(
        &mut self,
        total: Option<u64>,
        downloaded: u64,
    ) -> Result<Outcome, DownloadError> {
        let canceled = self.control.cancel.load(Ordering::SeqCst);
        let final_path = self.final_path.take().or_else(|| {
            self.req
                .file_name
                .as_deref()
                .map(|n| self.req.output_dir.join(filename::sanitize(n)))
        });
        if canceled {
            if let Some(path) = final_path {
                blocking(move || {
                    state::remove_work_dir(&path);
                    Ok::<_, std::io::Error>(())
                })
                .await?;
            }
            self.tx.send_modify(|p| {
                p.downloaded = 0;
                p.segments.clear();
            });
            return Ok(Outcome::Canceled);
        }
        let progress = self.tx.borrow().clone();
        Ok(Outcome::Paused {
            path: progress.path.or(final_path),
            downloaded: if downloaded > 0 {
                downloaded
            } else {
                progress.downloaded
            },
            total: total.or(progress.total),
            resumable: true,
        })
    }

    fn set_phase(&self, phase: Phase) {
        self.tx.send_if_modified(|p| {
            let changed = p.phase != phase || p.waiting.is_some();
            p.phase = phase;
            p.waiting = None;
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

/// How the tracks of a plan become the final file.
fn assembly(plan: &Plan, ffmpeg: bool) -> Assembly {
    match (plan.tracks.as_slice(), ffmpeg) {
        ([_, _], true) => Assembly::Merge,
        ([_, _], false) => Assembly::Separate,
        ([track], true) if track.container == Container::Ts => Assembly::Remux,
        _ => Assembly::Move,
    }
}

fn output_extension(plan: &Plan, assembly: Assembly) -> &'static str {
    let video = plan.tracks.iter().find(|t| t.role != TrackRole::Audio);
    let audio = plan.tracks.iter().find(|t| t.role == TrackRole::Audio);
    match (assembly, video, audio) {
        (Assembly::Merge, Some(v), Some(a)) => mux::merged_container(v.container, a.container),
        (Assembly::Remux, _, _) => "mp4",
        (_, Some(v), _) => v.container.extension(),
        (_, None, Some(a)) => a.container.extension(),
        (_, None, None) => "mp4",
    }
}

/// `name` with a stream-ish extension replaced by `ext`.
fn with_extension(name: &str, ext: &str) -> String {
    let stem = match name.rsplit_once('.') {
        Some((stem, old))
            if !stem.is_empty()
                && STREAM_EXTENSIONS.contains(&old.to_ascii_lowercase().as_str()) =>
        {
            stem
        }
        _ => name,
    };
    filename::sanitize(&format!("{stem}.{ext}"))
}

/// Total size from what the finished fragments weigh on average, else the
/// manifest's estimate.
fn estimated_total(
    done_bytes: u64,
    done: usize,
    total: usize,
    estimate: Option<u64>,
) -> Option<u64> {
    if done == total {
        return Some(done_bytes);
    }
    if done >= 2 {
        let average = done_bytes / done as u64;
        return Some(done_bytes + average * (total - done) as u64);
    }
    estimate.map(|e| e.max(done_bytes))
}

/// The progress bar's view: each fragment an equal slice of the estimated
/// size, runs of finished ones drawn as one segment.
fn fragment_views(done: &[bool], total: Option<u64>) -> Vec<SegmentView> {
    let Some(total) = total.filter(|t| *t > 0) else {
        return Vec::new();
    };
    let n = done.len() as u64;
    if n == 0 {
        return Vec::new();
    }
    let edge = |i: u64| (u128::from(total) * u128::from(i) / u128::from(n)) as u64;
    let mut views = Vec::new();
    let mut i = 0usize;
    while i < done.len() {
        if done[i] {
            let start = i;
            while i < done.len() && done[i] {
                i += 1;
            }
            views.push(SegmentView {
                start: edge(start as u64),
                end: edge(i as u64),
                written: edge(i as u64),
                active: false,
            });
        } else {
            i += 1;
        }
    }
    views
}

fn file_len(path: &Path) -> Option<u64> {
    std::fs::metadata(path)
        .ok()
        .filter(|m| m.is_file())
        .map(|m| m.len())
}

/// Bytes of finished fragments and joined tracks in a work folder.
fn fragment_bytes(work: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(work) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .filter(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.ends_with(".frag") || name.starts_with("track")
        })
        .filter_map(|e| e.metadata().ok().map(|m| m.len()))
        .sum()
}

fn join_track(
    work: &Path,
    track: usize,
    count: usize,
    container: Container,
) -> std::io::Result<()> {
    use std::io::Write;
    let path = state::track_path(work, track, container);
    let mut out = std::io::BufWriter::with_capacity(1 << 20, std::fs::File::create(&path)?);
    for i in 0..count {
        let mut part = std::fs::File::open(state::fragment_path(work, track, i))?;
        std::io::copy(&mut part, &mut out)?;
    }
    out.flush()?;
    out.into_inner().map_err(|e| e.into_error())?.sync_all()
}

fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn stem_of(path: &Path) -> String {
    path.file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "video".into())
}

fn extension_of(path: &Path) -> String {
    path.extension()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "mp4".into())
}

/// What every fragment request needs.
struct FragCtx {
    client: Client,
    headers: HeaderMap,
    limiter: Limits,
    stall_timeout: Duration,
    response_timeout: Duration,
    buffer_size: usize,
    work: PathBuf,
    /// For AES keys, which are small and fetched once each. An async lock,
    /// held while a key is fetched, so fragments that need the same key
    /// wait for the first request instead of each sending their own.
    fetcher: Fetcher,
    keys: tokio::sync::Mutex<HashMap<Url, [u8; 16]>>,
    /// Bytes received for fragments still in flight, for progress.
    in_flight: AtomicU64,
}

impl FragCtx {
    async fn key(&self, uri: &Url) -> Result<Option<[u8; 16]>, DownloadError> {
        let mut keys = self.keys.lock().await;
        if let Some(key) = keys.get(uri) {
            return Ok(Some(*key));
        }
        let Some(bytes) = self.fetcher.bytes(uri, MAX_KEY_BYTES).await? else {
            return Ok(None);
        };
        let key = crypto::key(&bytes)?;
        keys.insert(uri.clone(), key);
        Ok(Some(key))
    }
}

/// Counts bytes into the shared in-flight total and takes them out again
/// when the attempt ends, however it ends.
struct InFlight<'a> {
    counter: &'a AtomicU64,
    bytes: u64,
}

impl InFlight<'_> {
    fn add(&mut self, n: u64) {
        self.bytes += n;
        self.counter.fetch_add(n, Ordering::Relaxed);
    }
}

impl Drop for InFlight<'_> {
    fn drop(&mut self) {
        self.counter.fetch_sub(self.bytes, Ordering::Relaxed);
    }
}

/// Download one fragment into `.tmp`, continuing a partial one with a range
/// request, then decrypt it if needed and rename it to `.frag`.
async fn fetch_fragment(
    ctx: &FragCtx,
    fragment: &Fragment,
    stop: &CancellationToken,
) -> Result<FragEnd, DownloadError> {
    let tmp = state::temp_path(&ctx.work, fragment.track, fragment.index);
    let have = tokio::fs::metadata(&tmp)
        .await
        .map(|m| m.len())
        .unwrap_or(0);
    let segment = &fragment.segment;
    let complete = segment.range.is_some_and(|r| have >= r.length);
    if !complete {
        match stream_fragment(ctx, segment, &tmp, have, stop).await? {
            FragEnd::Stopped => return Ok(FragEnd::Stopped),
            FragEnd::Done(_) => {}
        }
    }
    let key = match &segment.key {
        Some(k) => match ctx.key(&k.uri).await? {
            Some(key) => Some((key, k.iv)),
            None => return Ok(FragEnd::Stopped),
        },
        None => None,
    };
    let dest = state::fragment_path(&ctx.work, fragment.track, fragment.index);
    let size = blocking(move || {
        if let Some((key, iv)) = key {
            let mut data = std::fs::read(&tmp)?;
            crypto::decrypt(&key, &iv, &mut data)?;
            let clear = tmp.with_extension("dec");
            {
                use std::io::Write;
                let mut file = std::fs::File::create(&clear)?;
                file.write_all(&data)?;
                file.sync_all()?;
            }
            std::fs::rename(&clear, &dest)?;
            std::fs::remove_file(&tmp)?;
            Ok::<_, DownloadError>(data.len() as u64)
        } else {
            std::fs::rename(&tmp, &dest)?;
            Ok(std::fs::metadata(&dest)?.len())
        }
    })
    .await?;
    Ok(FragEnd::Done(size))
}

async fn stream_fragment(
    ctx: &FragCtx,
    segment: &Segment,
    tmp: &Path,
    have: u64,
    stop: &CancellationToken,
) -> Result<FragEnd, DownloadError> {
    // The bytes wanted: the segment's own range, minus what is already on disk.
    let (start, last) = match segment.range {
        Some(r) => (r.offset + have, Some(r.last())),
        None => (have, None),
    };
    let mut request = http::plain_get(&ctx.client, &segment.url, &ctx.headers);
    if start > 0 || last.is_some() {
        let end = last.map(|l| l.to_string()).unwrap_or_default();
        request = request.header(RANGE, format!("bytes={start}-{end}"));
    }
    let resp = tokio::select! {
        biased;
        () = stop.cancelled() => return Ok(FragEnd::Stopped),
        r = tokio::time::timeout(ctx.response_timeout, request.send()) => match r {
            Err(_) => return Err(DownloadError::Stalled(ctx.response_timeout)),
            Ok(r) => r.map_err(|e| DownloadError::from_reqwest(&e))?,
        },
    };
    let status = resp.status();
    // Media fragments are never web pages; one means the tokens in the
    // stream's URLs ran out.
    if status.is_success() && http::is_html(&resp) {
        return Err(http::page_error(&resp));
    }
    let (append, mut skip, mut take) = match status {
        StatusCode::PARTIAL_CONTENT => {
            if http::content_range(&resp).map(|c| c.first) != Some(start) {
                return Err(DownloadError::RangeNotHonoured);
            }
            (true, 0, last.map(|l| l + 1 - start))
        }
        // The whole resource: start the fragment over and cut its range out.
        StatusCode::OK => (
            false,
            segment.range.map_or(0, |r| r.offset),
            segment.range.map(|r| r.length),
        ),
        // The partial file already held all of it.
        StatusCode::RANGE_NOT_SATISFIABLE if have > 0 && segment.range.is_none() => {
            return Ok(FragEnd::Done(have));
        }
        s => return Err(status_error(s.as_u16(), http::retry_after(&resp))),
    };
    let expected = match (take, status) {
        (Some(t), _) => Some(t),
        (None, StatusCode::OK) => resp.content_length(),
        (None, _) => None,
    };
    let file = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(tmp)
        .await?;
    let mut out = BufWriter::with_capacity(ctx.buffer_size, file);
    let mut counted = InFlight {
        counter: &ctx.in_flight,
        bytes: 0,
    };
    let mut received = 0u64;
    let mut stream = resp.bytes_stream();
    loop {
        let next = tokio::select! {
            biased;
            () = stop.cancelled() => {
                out.flush().await?;
                return Ok(FragEnd::Stopped);
            }
            n = tokio::time::timeout(ctx.stall_timeout, stream.next()) => n,
        };
        let chunk = match next {
            Err(_) => return Err(DownloadError::Stalled(ctx.stall_timeout)),
            Ok(None) => break,
            Ok(Some(Err(e))) => return Err(DownloadError::from_reqwest(&e)),
            Ok(Some(Ok(chunk))) => chunk,
        };
        ctx.limiter.acquire(chunk.len()).await;
        let mut bytes = &chunk[..];
        if skip > 0 {
            let n = skip.min(bytes.len() as u64) as usize;
            bytes = &bytes[n..];
            skip -= n as u64;
        }
        if let Some(left) = take.as_mut() {
            let n = (*left).min(bytes.len() as u64) as usize;
            bytes = &bytes[..n];
            *left -= n as u64;
        }
        out.write_all(bytes).await?;
        received += bytes.len() as u64;
        counted.add(bytes.len() as u64);
        if take == Some(0) {
            break;
        }
    }
    out.flush().await?;
    if take.is_some_and(|t| t > 0) || expected.is_some_and(|e| received < e) {
        return Err(DownloadError::PrematureEnd);
    }
    // Synced before the rename that marks it finished.
    out.into_inner().sync_all().await?;
    Ok(FragEnd::Done(received))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_get_the_extension_of_the_result() {
        assert_eq!(
            with_extension("Clip of the week.mp4", "ts"),
            "Clip of the week.ts"
        );
        assert_eq!(with_extension("master.m3u8", "mp4"), "master.mp4");
        assert_eq!(with_extension("Mr. Robot", "mp4"), "Mr. Robot.mp4");
        assert_eq!(with_extension("a/b:c.mpd", "webm"), "b_c.webm");
    }

    #[test]
    fn totals_follow_the_fragments() {
        assert_eq!(estimated_total(0, 0, 10, None), None);
        assert_eq!(estimated_total(0, 0, 10, Some(500)), Some(500));
        assert_eq!(estimated_total(200, 2, 10, Some(500)), Some(1000));
        assert_eq!(estimated_total(900, 10, 10, Some(500)), Some(900));
    }

    #[test]
    fn finished_fragments_are_drawn_as_runs() {
        let views = fragment_views(&[true, true, false, true], Some(400));
        assert_eq!(
            views,
            [
                SegmentView {
                    start: 0,
                    end: 200,
                    written: 200,
                    active: false
                },
                SegmentView {
                    start: 300,
                    end: 400,
                    written: 400,
                    active: false
                },
            ]
        );
        assert!(fragment_views(&[true], None).is_empty());
    }
}
