//! The videos of a playlist or channel, read with
//! `yt-dlp --flat-playlist -J`.
//!
//! Flat mode reads only the list pages: every video's id and title (and
//! usually its length and channel), not its formats, so a list of thousands
//! comes back in seconds. Each video is then added as a page of its own and
//! read in full when it starts. Only videos with a valid id are kept, and
//! their links are rebuilt from the id, so nothing in the list's data
//! decides where a download goes.

use downpipe_http::DownloadError;
use serde::{Deserialize, Serialize};

use crate::args::{self, MAX_LIST_ENTRIES};
use crate::errors;
use crate::process;
use crate::tools::Tools;

/// `-J` output larger than this is refused; a flat list of 5000 videos is
/// a few megabytes.
const MAX_JSON: usize = 64 * 1024 * 1024;
const MAX_TITLE: usize = 200;

/// A playlist or channel and its videos, in the list's order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListInfo {
    /// The playlist's title, or the channel's name and tab
    /// ("jawed - Videos"). It names the folder the videos go to.
    pub title: Option<String>,
    /// Who made the list.
    pub channel: Option<String>,
    pub entries: Vec<ListEntry>,
    /// The list has more videos than Downpipe reads
    /// ([`MAX_LIST_ENTRIES`]).
    pub truncated: bool,
}

/// One video of a list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListEntry {
    pub id: String,
    /// The video's page, made from the id.
    pub url: String,
    pub title: Option<String>,
    pub channel: Option<String>,
    pub duration_ms: Option<u64>,
    /// Position in the list, 1 first; names files with `{index}`.
    pub index: u32,
    /// `false` for videos the list shows but nobody can download without
    /// an account (private, deleted, members-only) and for live streams.
    pub available: bool,
}

#[derive(Debug, Default, Deserialize)]
struct RawList {
    #[serde(rename = "_type", default)]
    kind: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    channel: Option<String>,
    #[serde(default)]
    uploader: Option<String>,
    #[serde(default)]
    playlist_count: Option<f64>,
    #[serde(default)]
    entries: Vec<serde_json::Value>,
}

#[derive(Debug, Default, Deserialize)]
struct RawEntry {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    channel: Option<String>,
    #[serde(default)]
    uploader: Option<String>,
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    availability: Option<String>,
    #[serde(default)]
    live_status: Option<String>,
}

/// Ask yt-dlp for the videos of a playlist or channel page.
pub async fn list(
    tools: &Tools,
    url: &str,
    proxy: Option<&str>,
) -> Result<ListInfo, DownloadError> {
    let yt_dlp = tools.yt_dlp.as_ref().ok_or_else(errors::no_yt_dlp)?;
    let mut command = process::command(yt_dlp);
    command
        .args(args::list(url, tools, proxy))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let child = command
        .spawn()
        .map_err(|e| errors::spawn_failed(yt_dlp, &e))?;
    let output = child.wait_with_output().await.map_err(DownloadError::Io)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let lines: Vec<String> = stderr.lines().map(str::to_string).collect();
        return Err(errors::classify(
            &lines,
            output.status.code(),
            tools.ffmpeg.is_some(),
        ));
    }
    parse(&output.stdout)
}

/// Turn flat `-J` output into a [`ListInfo`].
pub fn parse(json: &[u8]) -> Result<ListInfo, DownloadError> {
    if json.len() > MAX_JSON {
        return Err(DownloadError::Extractor(
            "yt-dlp described this list with more data than Downpipe reads".into(),
        ));
    }
    let raw: RawList = serde_json::from_slice(json)
        .map_err(|e| DownloadError::Extractor(format!("yt-dlp's answer is not readable: {e}")))?;
    if raw.kind.as_deref() != Some("playlist") {
        return Err(DownloadError::UnsupportedMedia(
            "this is not a playlist or channel".into(),
        ));
    }
    let mut entries: Vec<ListEntry> = Vec::new();
    for value in raw.entries.iter().take(MAX_LIST_ENTRIES) {
        let Ok(entry) = RawEntry::deserialize(value) else {
            continue;
        };
        // A channel's tabs or a playlist of playlists: not videos.
        let Some(id) = entry.id.filter(|id| video_id(id)) else {
            continue;
        };
        if entries.iter().any(|e| e.id == id) {
            continue;
        }
        let title = entry.title.as_deref().and_then(clean);
        // yt-dlp keeps YouTube's placeholders for these in flat lists.
        let placeholder = title
            .as_deref()
            .is_some_and(|t| matches!(t, "[Private video]" | "[Deleted video]"));
        let restricted = matches!(
            entry.availability.as_deref(),
            Some("private" | "needs_auth" | "subscriber_only" | "premium_only")
        );
        let live = matches!(
            entry.live_status.as_deref(),
            Some("is_live" | "is_upcoming")
        );
        entries.push(ListEntry {
            url: format!("https://www.youtube.com/watch?v={id}"),
            id,
            title: title.filter(|_| !placeholder),
            channel: entry
                .channel
                .as_deref()
                .or(entry.uploader.as_deref())
                .and_then(clean),
            duration_ms: entry
                .duration
                .filter(|d| d.is_finite() && *d > 0.0)
                .map(|d| (d * 1000.0) as u64),
            index: u32::try_from(entries.len() + 1).unwrap_or(u32::MAX),
            available: !placeholder && !restricted && !live,
        });
    }
    let count = raw
        .playlist_count
        .filter(|c| c.is_finite() && *c >= 0.0)
        .map(|c| c as usize);
    Ok(ListInfo {
        title: raw.title.as_deref().and_then(clean),
        channel: raw
            .channel
            .as_deref()
            .or(raw.uploader.as_deref())
            .and_then(clean),
        truncated: raw.entries.len() > MAX_LIST_ENTRIES
            || count
                .is_some_and(|c| c > raw.entries.len() && raw.entries.len() >= MAX_LIST_ENTRIES),
        entries,
    })
}

fn video_id(id: &str) -> bool {
    id.len() == 11
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn clean(raw: &str) -> Option<String> {
    let text: String = raw
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_TITLE)
        .collect();
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_the_fixture() {
        let info = parse(
            &std::fs::read(crate::info::fixture_dir().join("youtube-playlist.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(info.title.as_deref(), Some("Early YouTube"));
        assert_eq!(info.channel.as_deref(), Some("Downpipe tests"));
        assert!(!info.truncated);
        let rows: Vec<(&str, u32, bool)> = info
            .entries
            .iter()
            .map(|e| (e.id.as_str(), e.index, e.available))
            .collect();
        // The channel tab and the bad id are left out, the repeat too; the
        // private and the upcoming video are listed but not available.
        assert_eq!(
            rows,
            [
                ("jNQXAC9IVRw", 1, true),
                ("aqz-KE-bpKQ", 2, true),
                ("xxxxxxxxxx1", 3, false),
                ("xxxxxxxxxx2", 4, false),
            ]
        );
        assert_eq!(info.entries[0].duration_ms, Some(19_000));
        assert_eq!(
            info.entries[0].url,
            "https://www.youtube.com/watch?v=jNQXAC9IVRw"
        );
        assert_eq!(info.entries[2].title, None);
    }

    #[test]
    fn refuses_what_is_not_a_list() {
        assert!(matches!(
            parse(
                json!({"_type": "video", "formats": []})
                    .to_string()
                    .as_bytes()
            ),
            Err(DownloadError::UnsupportedMedia(_))
        ));
        assert!(matches!(parse(b"{"), Err(DownloadError::Extractor(_))));
    }

    #[test]
    fn caps_long_lists() {
        let entries: Vec<serde_json::Value> = (0..MAX_LIST_ENTRIES + 2)
            .map(|i| json!({"id": format!("{i:011}"), "title": "x"}))
            .collect();
        let info = parse(
            json!({"_type": "playlist", "title": "Big", "entries": entries})
                .to_string()
                .as_bytes(),
        )
        .unwrap();
        assert_eq!(info.entries.len(), MAX_LIST_ENTRIES);
        assert!(info.truncated);
    }
}
