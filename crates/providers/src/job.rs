//! A site download: yt-dlp run behind a [`DownloadHandle`].
//!
//! Pausing ends yt-dlp (and the helpers it started); starting again runs it
//! with `--continue`, which picks up its `.part` files and skips formats it
//! already has. Cancelling also deletes those files. The stem of the file
//! name is fixed before the first start (see [`crate::files`]).

use std::path::PathBuf;
use std::process::Stdio;

use downpipe_http::media::MediaSelection;
use downpipe_http::{
    DownloadError, DownloadHandle, JobContext, MediaProgress, Outcome, Phase, Progress,
};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

use crate::args::{self, DownloadArgs};
use crate::errors;
use crate::files;
use crate::info;
use crate::process::{self, Tree};
use crate::progress::{Tracker, parse_line};
use crate::tools::Tools;

/// yt-dlp's error lines kept to explain a failure.
const STDERR_LINES: usize = 40;
const MAX_NOTICES: usize = 3;

/// One page to download.
#[derive(Debug, Clone)]
pub struct SiteRequest {
    /// The page, as [`crate::pages::recognize`] normalized it.
    pub url: String,
    pub dir: PathBuf,
    /// The name kept so far (`Clip.mp4`); `None` before the first start,
    /// when the page title names it.
    pub file_name: Option<String>,
    /// From the quality chooser; `None` takes the best.
    pub selection: Option<MediaSelection>,
    pub tools: Tools,
    /// `None`: yt-dlp's own choice (the system proxy). An empty string
    /// connects directly.
    pub proxy: Option<String>,
    /// Bytes per second, 0 for no limit.
    pub rate_limit: u64,
}

/// Start (or resume) a site download on the current Tokio runtime.
pub fn start(request: SiteRequest) -> DownloadHandle {
    DownloadHandle::spawn(move |ctx| run(request, ctx))
}

async fn run(req: SiteRequest, ctx: JobContext) -> Result<Outcome, DownloadError> {
    let yt_dlp = req.tools.yt_dlp.clone().ok_or_else(errors::no_yt_dlp)?;
    if args::needs_merge(req.selection.as_ref()) && req.tools.ffmpeg.is_none() {
        return Err(errors::no_ffmpeg());
    }

    let stem = match &req.file_name {
        Some(name) => files::stem_of(name),
        None => {
            // No name yet (added without the confirmation dialog): the
            // title names it. Reading the page first also refuses live and
            // protected videos before anything is written.
            let info = tokio::select! {
                info = info::inspect(&req.tools, &req.url, req.proxy.as_deref()) => info?,
                () = ctx.stopped() => return Ok(stopped(&ctx, None, &Tracker::default())),
            };
            files::stem_from_title(info.title.as_deref().unwrap_or("video"))
        }
    };
    tokio::fs::create_dir_all(&req.dir).await?;
    let dir = req.dir.clone();
    let stem = {
        let dir = dir.clone();
        tokio::task::spawn_blocking(move || files::unique_stem(&dir, &stem))
            .await
            .map_err(|e| DownloadError::Io(std::io::Error::other(e)))?
    };
    let guess = if is_audio_only(req.selection.as_ref()) {
        "m4a"
    } else {
        "mp4"
    };
    let file_name = format!("{stem}.{guess}");

    let mut progress = Progress::starting();
    progress.file_name = Some(file_name);
    progress.resumable = true;
    let _ = ctx.progress.send(progress.clone());

    let mut command = process::command(&yt_dlp);
    command
        .args(args::download(
            &DownloadArgs {
                url: &req.url,
                dir: &dir,
                stem: &stem,
                selection: req.selection.as_ref(),
                proxy: req.proxy.as_deref(),
                rate_limit: req.rate_limit,
            },
            &req.tools,
        ))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|e| errors::spawn_failed(&yt_dlp, &e))?;
    let tree = Tree::adopt(&child);

    // Both pipes are read to the end on their own tasks, so yt-dlp never
    // blocks on a full pipe while this loop waits for the other.
    let (tx, mut lines) = mpsc::channel::<Line>(256);
    if let Some(out) = child.stdout.take() {
        let tx = tx.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(out).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                if tx.send(Line::Out(line)).await.is_err() {
                    break;
                }
            }
        });
    }
    if let Some(err) = child.stderr.take() {
        let tx = tx.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(err).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                if tx.send(Line::Err(line)).await.is_err() {
                    break;
                }
            }
        });
    }
    drop(tx);

    let mut tracker = Tracker::default();
    let mut stderr: Vec<String> = Vec::new();
    loop {
        tokio::select! {
            line = lines.recv() => match line {
                Some(Line::Out(line)) => {
                    if let Some(event) = parse_line(&line) {
                        tracker.apply(event);
                        publish(&ctx, &mut progress, &tracker);
                    }
                }
                Some(Line::Err(line)) => {
                    tracing::debug!(target: "yt-dlp", "{line}");
                    if let Some(notice) = errors::notice(&line)
                        && !progress.notices.contains(&notice)
                        && progress.notices.len() < MAX_NOTICES
                    {
                        progress.notices.push(notice);
                        let _ = ctx.progress.send(progress.clone());
                    }
                    if stderr.len() == STDERR_LINES {
                        stderr.remove(0);
                    }
                    stderr.push(line);
                }
                // Both pipes closed: yt-dlp is exiting.
                None => break,
            },
            () = ctx.stopped() => {
                tree.kill(&mut child);
                let _ = child.wait().await;
                if ctx.is_cancel() {
                    let (dir, stem) = (dir.clone(), stem.clone());
                    let _ = tokio::task::spawn_blocking(move || files::remove_partials(&dir, &stem)).await;
                }
                return Ok(stopped(&ctx, Some(&progress), &tracker));
            }
        }
    }
    let status = child.wait().await?;
    if ctx.is_stopped() {
        return Ok(stopped(&ctx, Some(&progress), &tracker));
    }
    // 101: `--max-downloads` was reached, which is the normal end of one
    // video for some yt-dlp versions.
    let ok = status.success() || status.code() == Some(101);
    match tracker.done.clone() {
        Some(path) if ok => {
            let bytes = tokio::fs::metadata(&path).await?.len();
            progress.phase = Phase::Done;
            progress.file_name = path.file_name().map(|n| n.to_string_lossy().into_owned());
            progress.path = Some(path.clone());
            progress.total = Some(bytes);
            progress.downloaded = bytes;
            progress.speed = 0;
            let _ = ctx.progress.send(progress);
            Ok(Outcome::Completed {
                path,
                bytes,
                verified: None,
            })
        }
        None if ok => Err(DownloadError::Extractor(
            "yt-dlp finished without downloading anything; live streams and protected videos are skipped"
                .into(),
        )),
        _ => Err(errors::classify(&stderr, status.code(), req.tools.ffmpeg.is_some())),
    }
}

enum Line {
    Out(String),
    Err(String),
}

fn is_audio_only(selection: Option<&MediaSelection>) -> bool {
    selection
        .and_then(|s| s.video.as_ref())
        .is_some_and(|v| v.height.is_none())
        && selection.is_some_and(|s| s.audio.is_none())
}

fn publish(ctx: &JobContext, progress: &mut Progress, tracker: &Tracker) {
    progress.phase = if tracker.finishing() {
        Phase::Finalizing
    } else if tracker.started() {
        Phase::Downloading
    } else {
        Phase::Starting
    };
    progress.downloaded = tracker.downloaded();
    progress.total = tracker.total();
    progress.speed = tracker.speed;
    progress.connections = usize::from(tracker.started() && !tracker.finishing());
    progress.target_connections = 1;
    if let (Some(quality), Some(selection)) = (tracker.quality(), tracker.selection()) {
        progress.media = Some(MediaProgress {
            quality,
            selection,
            // Not counted in fragments: yt-dlp downloads whole formats.
            fragments: 0,
            fragments_done: 0,
        });
    }
    let _ = ctx.progress.send(progress.clone());
}

fn stopped(ctx: &JobContext, progress: Option<&Progress>, tracker: &Tracker) -> Outcome {
    if ctx.is_cancel() {
        return Outcome::Canceled;
    }
    Outcome::Paused {
        path: progress.and_then(|p| p.path.clone()),
        downloaded: tracker.downloaded(),
        total: tracker.total(),
        resumable: true,
    }
}
