//! Streamed media: HLS playlists and DASH manifests turned into plain files.
//!
//! A web player fetches a manifest, picks a quality and then requests the
//! stream in small fragments. This module does the same without a player:
//!
//! * [`hls`] and [`dash`] parse manifests into fragment lists. Protected
//!   (DRM) and live streams are refused while parsing, before any media byte
//!   is requested.
//! * [`resolve`] fetches the manifests of one download and lists the
//!   qualities a user can choose from, or turns a choice into a [`Plan`].
//! * The job (`job.rs`) downloads the fragments of a plan with several
//!   connections, keeps each finished fragment as its own file so a pause or
//!   crash loses at most the fragments in flight, and joins them at the end.
//! * [`mux`] finds a user-installed FFmpeg and merges separate video and
//!   audio with it (`-c copy`, no re-encoding). Without FFmpeg the two
//!   tracks are saved as two files.

use serde::{Deserialize, Serialize};
use url::Url;

mod crypto;
pub mod dash;
pub mod hls;
pub(crate) mod job;
pub mod mux;
pub mod resolve;
mod state;

pub use resolve::{Plan, Track, TrackRole};
pub use state::{remove_work_dir, work_dir};

/// How the stream is described.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    /// HTTP Live Streaming: an `.m3u8` playlist.
    Hls,
    /// MPEG-DASH: an `.mpd` manifest.
    Dash,
}

impl MediaKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hls => "hls",
            Self::Dash => "dash",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "hls" => Some(Self::Hls),
            "dash" => Some(Self::Dash),
            _ => None,
        }
    }

    /// [`MediaKind::from_url`] for a link that may not parse.
    pub fn from_link(raw: &str) -> Option<Self> {
        Url::parse(raw.trim())
            .ok()
            .as_ref()
            .and_then(Self::from_url)
    }

    /// Guess from a URL's path: `.m3u8` is HLS, `.mpd` is DASH.
    pub fn from_url(url: &Url) -> Option<Self> {
        let path = url.path().to_ascii_lowercase();
        if path.ends_with(".m3u8") || path.ends_with(".m3u") {
            Some(Self::Hls)
        } else if path.ends_with(".mpd") {
            Some(Self::Dash)
        } else {
            None
        }
    }
}

/// Half-open byte span inside a resource: `length` bytes from `offset`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ByteSpan {
    pub offset: u64,
    pub length: u64,
}

impl ByteSpan {
    /// Inclusive last byte, as a `Range` header wants it.
    pub fn last(&self) -> u64 {
        self.offset + self.length.saturating_sub(1)
    }
}

/// AES-128-CBC key of an HLS segment (`#EXT-X-KEY:METHOD=AES-128`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyRef {
    /// Where the 16-byte key is served.
    pub uri: Url,
    pub iv: [u8; 16],
}

/// One request's worth of a stream.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Segment {
    pub url: Url,
    /// Only these bytes of `url` (`#EXT-X-BYTERANGE`, DASH `mediaRange`).
    pub range: Option<ByteSpan>,
    /// Set when the segment is encrypted with AES-128.
    pub key: Option<KeyRef>,
    /// Play time, for estimates. 0 when unknown.
    pub duration_ms: u64,
}

impl Segment {
    pub fn new(url: Url) -> Self {
        Self {
            url,
            range: None,
            key: None,
            duration_ms: 0,
        }
    }
}

/// What the fragments of a track add up to once joined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Container {
    /// MPEG transport stream (`.ts`), the classic HLS format.
    Ts,
    /// Fragmented MP4 (an init section followed by `moof`/`mdat` fragments).
    Mp4,
    /// Audio-only fragmented MP4.
    M4a,
    WebM,
    /// Raw AAC with ADTS headers (HLS "packed audio").
    Aac,
    Mp3,
}

impl Container {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Ts => "ts",
            Self::Mp4 => "mp4",
            Self::M4a => "m4a",
            Self::WebM => "webm",
            Self::Aac => "aac",
            Self::Mp3 => "mp3",
        }
    }

    /// Guess from a segment URL's extension; `None` when it says nothing.
    pub fn from_path(path: &str) -> Option<Self> {
        let ext = path.rsplit_once('.')?.1.to_ascii_lowercase();
        Some(match ext.as_str() {
            "ts" | "m2ts" | "mts" => Self::Ts,
            "mp4" | "m4s" | "m4v" | "cmfv" | "fmp4" => Self::Mp4,
            "m4a" | "cmfa" => Self::M4a,
            "webm" => Self::WebM,
            "aac" => Self::Aac,
            "mp3" => Self::Mp3,
            _ => return None,
        })
    }
}

/// One quality a user can pick.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoOption {
    /// Stable within one manifest; see [`TrackRef`].
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// Bits per second, as the manifest announces it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bandwidth: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codecs: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_rate: Option<String>,
    /// The video carries its own sound, so no audio track is needed.
    pub has_audio: bool,
    /// HLS: the audio renditions that go with this quality
    /// ([`AudioOption::group`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_group: Option<String>,
    /// A quality that is sound only (some HLS masters offer one).
    #[serde(default)]
    pub audio_only: bool,
    /// Size estimated from bandwidth and duration, when both are known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_size: Option<u64>,
}

/// One separate audio track a user can pick.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioOption {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bandwidth: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codecs: Option<String>,
    /// The manifest marks it as the one to play by default.
    pub default: bool,
    /// HLS: the group of qualities it belongs to ([`VideoOption::audio_group`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}

/// What a manifest offers, best quality first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaOptions {
    pub kind: MediaKind,
    pub videos: Vec<VideoOption>,
    /// Separate audio tracks. Empty when the sound is inside the video.
    pub audios: Vec<AudioOption>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// A track picked by a user, with enough detail to find it again when the
/// manifest is fetched anew (ids of HLS variants are their position, which
/// a server may change).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackRef {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bandwidth: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

/// The video and audio to download. `None` picks the best one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaSelection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video: Option<TrackRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<TrackRef>,
}

/// Attached to a [`crate::DownloadRequest`] to download a stream instead of
/// a single file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaRequest {
    pub kind: MediaKind,
    pub selection: MediaSelection,
    /// FFmpeg to merge separate video and audio with. Without it they are
    /// saved as two files.
    pub ffmpeg: Option<std::path::PathBuf>,
}

/// "1080p", "720p 60", "1280×720" or "2.5 Mbit/s": how a quality is named
/// in lists.
pub fn quality_label(
    height: Option<u32>,
    frame_rate: Option<&str>,
    bandwidth: Option<u64>,
) -> String {
    let fps = frame_rate
        .and_then(|f| f.parse::<f64>().ok())
        .filter(|f| *f > 30.5)
        .map(|f| format!(" {}", f.round() as u64))
        .unwrap_or_default();
    match (height, bandwidth) {
        (Some(h), _) => format!("{h}p{fps}"),
        (None, Some(b)) => format!("{:.1} Mbit/s", b as f64 / 1_000_000.0),
        (None, None) => "Default".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_from_urls() {
        let u = |s: &str| Url::parse(s).unwrap();
        assert_eq!(
            MediaKind::from_url(&u("https://a.org/x/master.M3U8?t=1")),
            Some(MediaKind::Hls)
        );
        assert_eq!(
            MediaKind::from_url(&u("https://a.org/manifest.mpd")),
            Some(MediaKind::Dash)
        );
        assert_eq!(MediaKind::from_url(&u("https://a.org/video.mp4")), None);
        assert_eq!(MediaKind::parse("dash"), Some(MediaKind::Dash));
        assert_eq!(MediaKind::parse("mp4"), None);
    }

    #[test]
    fn containers_from_paths() {
        assert_eq!(Container::from_path("/a/seg-1.ts"), Some(Container::Ts));
        assert_eq!(Container::from_path("/a/seg-1.M4S"), Some(Container::Mp4));
        assert_eq!(Container::from_path("/a/seg"), None);
    }

    #[test]
    fn labels() {
        assert_eq!(quality_label(Some(1080), Some("29.97"), Some(1)), "1080p");
        assert_eq!(quality_label(Some(720), Some("60"), None), "720p 60");
        assert_eq!(quality_label(None, None, Some(2_500_000)), "2.5 Mbit/s");
        assert_eq!(quality_label(None, None, None), "Default");
    }
}
