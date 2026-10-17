use std::path::PathBuf;

use downpipe_http::{Phase, Progress, SegmentView};
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
    pub created_at: i64,
    pub updated_at: i64,
    pub completed_at: Option<i64>,
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
