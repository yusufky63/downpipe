use std::path::PathBuf;

use downpipe_http::media::{AudioOption, MediaKind, MediaOptions, MediaSelection, VideoOption};
use downpipe_http::{Phase, Progress, SegmentView, WaitReason};
use serde::{Deserialize, Serialize};

pub type TaskId = String;

/// Lifecycle of a download as the user sees it.
///
/// ```text
/// Queued -> Starting -> Downloading <-> Waiting
///                           |  \-> Paused -> Queued
///                           |  \-> Failed -> Queued (retry)
///                           v
///                       Verifying -> Completed
/// any unfinished state -> Canceled
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Queued,
    Starting,
    Downloading,
    /// Running but every connection is backing off (`Retry-After`, errors).
    Waiting,
    Verifying,
    Paused,
    Completed,
    Failed,
    Canceled,
}

impl TaskState {
    /// The engine is working on it right now.
    pub fn is_running(self) -> bool {
        matches!(
            self,
            Self::Starting | Self::Downloading | Self::Waiting | Self::Verifying
        )
    }

    pub fn is_finished(self) -> bool {
        matches!(self, Self::Completed | Self::Canceled)
    }

    /// May be (re)started with `resume`.
    pub fn can_resume(self) -> bool {
        matches!(self, Self::Paused | Self::Failed | Self::Canceled)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Starting => "starting",
            Self::Downloading => "downloading",
            Self::Waiting => "waiting",
            Self::Verifying => "verifying",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Canceled => "canceled",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "queued" => Self::Queued,
            "starting" => Self::Starting,
            "downloading" => Self::Downloading,
            "waiting" => Self::Waiting,
            "verifying" => Self::Verifying,
            "paused" => Self::Paused,
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            "canceled" => Self::Canceled,
            _ => return None,
        })
    }

    pub(crate) fn from_phase(phase: Phase) -> Self {
        match phase {
            Phase::Starting => Self::Starting,
            Phase::Downloading | Phase::Finalizing | Phase::Done => Self::Downloading,
            Phase::Waiting => Self::Waiting,
            Phase::Verifying => Self::Verifying,
        }
    }
}

/// What is persisted in SQLite for each download.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskRecord {
    pub id: TaskId,
    /// Credentials are stripped before this is stored.
    pub url: String,
    pub dir: PathBuf,
    pub file_name: Option<String>,
    pub path: Option<PathBuf>,
    pub state: TaskState,
    pub total: Option<u64>,
    pub downloaded: u64,
    pub error: Option<String>,
    pub error_kind: Option<String>,
    pub checksum: Option<String>,
    pub max_connections: Option<u32>,
    /// Category the file was sorted into, if any.
    #[serde(default)]
    pub category: Option<String>,
    /// The folder was picked by Downpipe rather than the user, so it may move
    /// the finished file to the folder of its real type.
    #[serde(default)]
    pub auto_dir: bool,
    /// Queue id; see `Settings::queues`.
    #[serde(default = "main_queue")]
    pub queue: String,
    /// Order inside the queue: lower starts first.
    #[serde(default)]
    pub position: i64,
    /// Set when the URL is a stream manifest (HLS, DASH) rather than a file.
    #[serde(default)]
    pub media: Option<MediaInfo>,
    /// Set when the URL is a video page a site provider downloads (YouTube).
    #[serde(default)]
    pub site: Option<SiteInfo>,
    /// Pattern of the site rule that applied when the download was added
    /// or last started; `None` when no rule matched.
    #[serde(default)]
    pub site_rule: Option<String>,
    /// This download's own limit in bytes per second, under the global one.
    /// `None`: only the global limit applies.
    #[serde(default)]
    pub speed_limit: Option<u64>,
    /// "Pause until": the paused download is queued again at this time
    /// (Unix seconds). Cleared when it is resumed or paused by hand.
    #[serde(default)]
    pub resume_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub completed_at: Option<i64>,
}

/// What a stream download remembers between sessions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaInfo {
    pub kind: MediaKind,
    /// The tracks to download. `None` takes the best quality when it starts,
    /// which is then stored here so a resume takes the same.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<MediaSelection>,
    /// "1080p" and the like, once known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
}

impl MediaInfo {
    pub fn new(kind: MediaKind) -> Self {
        Self {
            kind,
            selection: None,
            quality: None,
        }
    }
}

/// What a site download remembers between sessions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteInfo {
    /// The site, as people name it ("YouTube").
    pub site: String,
    /// The formats to download, in the stream chooser's shape (the "video"
    /// of an Audio only choice is the sound format). `None` takes the best,
    /// which is then stored here so a resume takes the same.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<MediaSelection>,
    /// "1080p", "Audio only" and the like, once known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
}

impl SiteInfo {
    pub fn new(site: impl Into<String>) -> Self {
        Self {
            site: site.into(),
            selection: None,
            quality: None,
        }
    }
}

/// What a video page offers, for the quality chooser.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SiteChoices {
    pub site: String,
    /// The page's title, which the dialog offers as the file name.
    pub title: Option<String>,
    pub options: SiteOptions,
    /// The choice made so far, if any.
    pub selection: Option<MediaSelection>,
    /// FFmpeg is available to join separate video and sound.
    pub ffmpeg: bool,
}

/// The qualities of a page, in the same shape as a stream's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SiteOptions {
    /// Video qualities best first, then "Audio only" entries.
    pub videos: Vec<VideoOption>,
    pub audios: Vec<AudioOption>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// Fragments of a stream download.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Fragments {
    pub done: usize,
    pub total: usize,
}

/// What a stream offers, for the quality chooser.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MediaChoices {
    pub options: MediaOptions,
    /// The choice made so far, if any.
    pub selection: Option<MediaSelection>,
    /// FFmpeg is available to merge separate video and audio into one file.
    pub ffmpeg: bool,
}

/// Live numbers that only exist while a download runs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Live {
    pub speed: u64,
    pub connections: usize,
    pub target_connections: usize,
    pub resumable: bool,
    pub segments: Vec<SegmentView>,
    pub notices: Vec<String>,
    /// Why it waits, while its state is `Waiting`.
    pub waiting: Option<WaitReason>,
    /// Waiting for the user to confirm its name, folder and queue (and a
    /// stream's or video page's quality) before it starts: sent by the
    /// browser, or a video page added in the app.
    pub awaiting_confirmation: bool,
    /// Added in the app's own window (Add dialog, paste, drag, clipboard)
    /// rather than sent by the browser. Read while the download awaits
    /// confirmation: the app's main window asks then, where the user already
    /// is, and the confirmation window asks for everything else. Like the
    /// other live fields it is not stored, and it resets once the download
    /// runs.
    pub from_app: bool,
    /// Stream downloads: fragments done and in all.
    pub fragments: Option<Fragments>,
    /// Held because its link is already in the list (that download's id):
    /// the confirmation asks what to do, as IDM does.
    pub duplicate_of: Option<TaskId>,
}

impl Live {
    pub(crate) fn from_progress(p: &Progress) -> Self {
        Self {
            speed: p.speed,
            connections: p.connections,
            target_connections: p.target_connections,
            resumable: p.resumable,
            segments: p.segments.clone(),
            notices: p.notices.clone(),
            waiting: p.waiting,
            awaiting_confirmation: false,
            from_app: false,
            duplicate_of: None,
            // Site downloads report whole formats, not fragments.
            fragments: p
                .media
                .as_ref()
                .filter(|m| m.fragments > 0)
                .map(|m| Fragments {
                    done: m.fragments_done,
                    total: m.fragments,
                }),
        }
    }
}

/// Everything a UI needs to draw one row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TaskSnapshot {
    #[serde(flatten)]
    pub record: TaskRecord,
    #[serde(flatten)]
    pub live: Live,
    pub eta_secs: Option<u64>,
}

impl TaskSnapshot {
    pub(crate) fn new(record: &TaskRecord, live: &Live) -> Self {
        let eta_secs = match (record.total, live.speed) {
            (Some(total), speed) if speed > 0 && record.state.is_running() => {
                Some(total.saturating_sub(record.downloaded) / speed)
            }
            _ => None,
        };
        Self {
            record: record.clone(),
            live: live.clone(),
            eta_secs,
        }
    }
}

/// Options for [`crate::Manager::add`].
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct AddOptions {
    pub url: String,
    /// Defaults to the configured download folder.
    pub dir: Option<PathBuf>,
    pub file_name: Option<String>,
    /// `sha256:<hex>` or `sha512:<hex>`.
    pub checksum: Option<String>,
    pub max_connections: Option<u32>,
    /// Kept in memory only; never written to disk.
    pub headers: Vec<(String, String)>,
    /// Add without starting.
    pub paused: bool,
    /// Queue id; the main queue when missing or unknown.
    pub queue: Option<String>,
    /// Hold the download until the user confirms it (see
    /// [`crate::Manager::confirm`]).
    pub confirm: bool,
    /// Added in the app's own window, which then also asks for the
    /// confirmation (see [`Live::from_app`]). The browser bridge leaves it
    /// unset.
    pub from_app: bool,
    /// The URL is a stream manifest. Without it, `.m3u8` and `.mpd` links
    /// are recognized by their extension.
    pub media: Option<MediaKind>,
    /// What to do when the link is already in the list.
    pub on_duplicate: OnDuplicate,
}

/// What [`crate::Manager::add`] does with a link that is already in the list,
/// finished or not.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnDuplicate {
    /// Refuse with `CoreError::Duplicate`, so the caller can ask the user.
    #[default]
    Refuse,
    /// Add another download; its file gets a free name next to the first.
    AddCopy,
    /// Add it held for confirmation and marked with `duplicate_of`, so the
    /// confirmation asks: for downloads the browser sends, which cannot
    /// wait for an answer.
    Ask,
}

/// What the user chose in the confirmation dialog.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ConfirmOptions {
    pub file_name: Option<String>,
    /// A folder picked by the user; `None` keeps the automatic one.
    pub dir: Option<PathBuf>,
    pub queue: Option<String>,
    /// Start now; otherwise it stays paused ("Download later").
    pub start: bool,
    /// Stream downloads: the quality and audio to download.
    pub media: Option<MediaSelection>,
}

/// Where selected downloads move inside their queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueMove {
    Up,
    Down,
    Top,
    Bottom,
}

fn main_queue() -> String {
    crate::settings::MAIN_QUEUE.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_strings_round_trip() {
        for s in [
            TaskState::Queued,
            TaskState::Starting,
            TaskState::Downloading,
            TaskState::Waiting,
            TaskState::Verifying,
            TaskState::Paused,
            TaskState::Completed,
            TaskState::Failed,
            TaskState::Canceled,
        ] {
            assert_eq!(TaskState::parse(s.as_str()), Some(s));
            assert_eq!(
                serde_json::to_string(&s).unwrap(),
                format!("\"{}\"", s.as_str())
            );
        }
        assert_eq!(TaskState::parse("nope"), None);
    }
}
