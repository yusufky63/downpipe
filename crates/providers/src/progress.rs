//! Reading yt-dlp's output while it downloads.
//!
//! [`crate::args::download`] makes yt-dlp print three kinds of lines, each
//! a marker followed by JSON values:
//!
//! ```text
//! DOWNPIPE:START {"format_id": "137+140", ...} [{"format_id": "137", "filesize": 1234, ...}, ...]
//! DOWNPIPE:PROGRESS "137" {"status": "downloading", "downloaded_bytes": 1024, ...}
//! DOWNPIPE:DONE "C:\\Users\\me\\Videos\\Clip.mp4"
//! ```
//!
//! yt-dlp writes JSON with `ensure_ascii`, so every line is ASCII whatever
//! the title. A merged download is two parts downloaded one after the
//! other; [`Tracker`] adds them up into one size and one byte count.

use std::path::PathBuf;

use downpipe_http::media::{MediaSelection, TrackRef, quality_label};
use serde::Deserialize;
use serde_json::Value;

use crate::args::{DONE, PROGRESS, START};

/// One format yt-dlp is about to download.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Part {
    #[serde(default)]
    pub format_id: String,
    #[serde(default)]
    pub ext: Option<String>,
    #[serde(default)]
    pub height: Option<f64>,
    #[serde(default)]
    pub fps: Option<f64>,
    #[serde(default)]
    pub vcodec: Option<String>,
    #[serde(default)]
    pub acodec: Option<String>,
    #[serde(default)]
    pub filesize: Option<f64>,
    #[serde(default)]
    pub filesize_approx: Option<f64>,
}

impl Part {
    fn size(&self) -> Option<u64> {
        bytes(self.filesize.or(self.filesize_approx))
    }

    fn has_video(&self) -> bool {
        self.vcodec.as_deref().is_some_and(|c| c != "none")
    }
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
struct RawProgress {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    downloaded_bytes: Option<f64>,
    #[serde(default)]
    total_bytes: Option<f64>,
    #[serde(default)]
    total_bytes_estimate: Option<f64>,
    #[serde(default)]
    speed: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// The formats chosen, before any byte is downloaded.
    Start { parts: Vec<Part> },
    Progress {
        format_id: String,
        finished: bool,
        downloaded: u64,
        total: Option<u64>,
        speed: Option<u64>,
    },
    /// The finished file, merged and moved into place.
    Done { path: PathBuf },
}

fn bytes(v: Option<f64>) -> Option<u64> {
    v.filter(|v| v.is_finite() && *v >= 0.0).map(|v| v as u64)
}

/// Two JSON values separated by whitespace.
fn values(rest: &str) -> Option<(Value, Option<Value>)> {
    let mut stream = serde_json::Deserializer::from_str(rest).into_iter::<Value>();
    let first = stream.next()?.ok()?;
    let second = stream.next().and_then(Result::ok);
    Some((first, second))
}

/// Read one line of yt-dlp's output; `None` for anything else it prints.
pub fn parse_line(line: &str) -> Option<Event> {
    let line = line.trim();
    if let Some(rest) = line.strip_prefix(START) {
        let (first, second) = values(rest)?;
        let single: Part = serde_json::from_value(first).ok()?;
        let parts = second
            .and_then(|v| serde_json::from_value::<Vec<Part>>(v).ok())
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| vec![single]);
        return Some(Event::Start { parts });
    }
    if let Some(rest) = line.strip_prefix(PROGRESS) {
        let (first, second) = values(rest)?;
        let format_id = match first {
            Value::String(s) => s,
            Value::Number(n) => n.to_string(),
            _ => String::new(),
        };
        let p: RawProgress = serde_json::from_value(second?).ok()?;
        return Some(Event::Progress {
            format_id,
            finished: p.status.as_deref() == Some("finished"),
            downloaded: bytes(p.downloaded_bytes).unwrap_or(0),
            total: bytes(p.total_bytes).or(bytes(p.total_bytes_estimate)),
            speed: bytes(p.speed),
        });
    }
    if let Some(rest) = line.strip_prefix(DONE) {
        let (first, _) = values(rest)?;
        return match first {
            Value::String(path) if !path.is_empty() => Some(Event::Done {
                path: PathBuf::from(path),
            }),
            _ => None,
        };
    }
    None
}

#[derive(Debug, Clone, Default, PartialEq)]
struct PartState {
    part: Part,
    total: Option<u64>,
    downloaded: u64,
    finished: bool,
}

/// Where a running download stands, from the lines read so far.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Tracker {
    parts: Vec<PartState>,
    started: bool,
    pub speed: u64,
    pub done: Option<PathBuf>,
}

impl Tracker {
    pub fn apply(&mut self, event: Event) {
        match event {
            Event::Start { parts } => {
                self.parts = parts
                    .into_iter()
                    .map(|part| PartState {
                        total: part.size(),
                        part,
                        ..PartState::default()
                    })
                    .collect();
            }
            Event::Progress {
                format_id,
                finished,
                downloaded,
                total,
                speed,
            } => {
                self.started = true;
                let at = match self
                    .parts
                    .iter()
                    .position(|p| p.part.format_id == format_id)
                {
                    Some(at) => at,
                    None => {
                        self.parts.push(PartState {
                            part: Part {
                                format_id,
                                ..Part::default()
                            },
                            ..PartState::default()
                        });
                        self.parts.len() - 1
                    }
                };
                let state = &mut self.parts[at];
                state.total = total.or(state.total);
                state.downloaded = downloaded;
                if finished {
                    state.finished = true;
                    let size = state.total.unwrap_or(downloaded).max(downloaded);
                    state.downloaded = size;
                    state.total = Some(size);
                    self.speed = 0;
                } else {
                    self.speed = speed.unwrap_or(self.speed);
                }
            }
            Event::Done { path } => self.done = Some(path),
        }
    }

    /// The first progress line arrived: bytes are moving.
    pub fn started(&self) -> bool {
        self.started
    }

    pub fn downloaded(&self) -> u64 {
        self.parts.iter().map(|p| p.downloaded).sum()
    }

    /// Known only when every part's size is.
    pub fn total(&self) -> Option<u64> {
        if self.parts.is_empty() {
            return None;
        }
        self.parts.iter().map(|p| p.total).sum()
    }

    /// Every part is downloaded; yt-dlp is merging or moving the result.
    pub fn finishing(&self) -> bool {
        !self.parts.is_empty() && self.parts.iter().all(|p| p.finished) && self.done.is_none()
    }

    /// "1080p", "720p 60" or "Audio only", once the formats are known.
    pub fn quality(&self) -> Option<String> {
        let video = self.parts.iter().find(|p| p.part.has_video());
        match video {
            Some(v) => {
                let height = v.part.height.filter(|h| *h > 0.0).map(|h| h as u32);
                let fps = v.part.fps.map(|f| f.to_string());
                Some(quality_label(height, fps.as_deref(), None))
            }
            None if !self.parts.is_empty() => Some("Audio only".to_string()),
            None => None,
        }
    }

    /// The formats being downloaded, in the shape the quality chooser
    /// uses, so a resume asks for the same ones.
    pub fn selection(&self) -> Option<MediaSelection> {
        let track = |p: &PartState| TrackRef {
            id: p.part.format_id.clone(),
            height: p.part.height.filter(|h| *h > 0.0).map(|h| h as u32),
            ..TrackRef::default()
        };
        let first = self.parts.first()?;
        Some(MediaSelection {
            video: Some(track(first)),
            audio: self.parts.get(1).map(track),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const START_LINE: &str = r#"DOWNPIPE:START {"format_id": "134+140", "ext": "mp4", "height": 240, "fps": 15, "vcodec": "avc1.4D400C", "acodec": "mp4a.40.2"} [{"format_id": "134", "filesize": 322379, "filesize_approx": 322376, "ext": "mp4", "height": 240, "fps": 15, "vcodec": "avc1.4D400C", "acodec": "none"}, {"format_id": "140", "filesize": 309288, "ext": "m4a", "vcodec": "none", "acodec": "mp4a.40.2"}]"#;

    #[test]
    fn reads_a_merged_download() {
        let mut t = Tracker::default();
        t.apply(parse_line(START_LINE).unwrap());
        assert_eq!(t.total(), Some(631_667));
        assert_eq!(t.downloaded(), 0);
        assert!(!t.started());
        assert_eq!(t.quality().as_deref(), Some("240p"));

        t.apply(
            parse_line(r#"DOWNPIPE:PROGRESS "134" {"status": "downloading", "downloaded_bytes": 1024, "total_bytes": 322379, "speed": 219455.68}"#)
                .unwrap(),
        );
        assert!(t.started());
        assert_eq!(t.downloaded(), 1024);
        assert_eq!(t.speed, 219_455);
        t.apply(
            parse_line(r#"DOWNPIPE:PROGRESS "134" {"downloaded_bytes": 322379, "total_bytes": 322379, "status": "finished", "speed": 612569.6}"#)
                .unwrap(),
        );
        assert_eq!(t.downloaded(), 322_379);
        assert!(!t.finishing());
        // The audio part's real size replaces the estimate.
        t.apply(
            parse_line(r#"DOWNPIPE:PROGRESS "140" {"status": "downloading", "downloaded_bytes": 100, "total_bytes": 309300, "speed": null}"#)
                .unwrap(),
        );
        assert_eq!(t.total(), Some(322_379 + 309_300));
        assert_eq!(t.downloaded(), 322_479);
        t.apply(
            parse_line(r#"DOWNPIPE:PROGRESS "140" {"status": "finished", "downloaded_bytes": 309300, "total_bytes": 309300}"#)
                .unwrap(),
        );
        assert!(t.finishing());
        t.apply(parse_line(r#"DOWNPIPE:DONE "C:\\Videos\\Zoo test.mp4""#).unwrap());
        assert!(!t.finishing());
        assert_eq!(t.done, Some(PathBuf::from(r"C:\Videos\Zoo test.mp4")));
        let selection = t.selection().unwrap();
        assert_eq!(selection.video.unwrap().id, "134");
        assert_eq!(selection.audio.unwrap().id, "140");
    }

    #[test]
    fn reads_a_single_format() {
        let mut t = Tracker::default();
        t.apply(
            parse_line(r#"DOWNPIPE:START {"format_id": "140", "ext": "m4a", "vcodec": "none", "acodec": "mp4a.40.2", "filesize_approx": 5000} null"#)
                .unwrap(),
        );
        assert_eq!(t.total(), Some(5000));
        assert_eq!(t.quality().as_deref(), Some("Audio only"));
        let selection = t.selection().unwrap();
        assert_eq!(selection.video.unwrap().id, "140");
        assert!(selection.audio.is_none());
        // An empty list (yt-dlp's answer for one format) counts as none.
        let empty = parse_line(r#"DOWNPIPE:START {"format_id": "18", "height": 360, "fps": 30, "vcodec": "avc1", "acodec": "mp4a"} []"#).unwrap();
        assert!(matches!(empty, Event::Start { parts } if parts.len() == 1));
    }

    #[test]
    fn sizes_stay_unknown_until_every_part_has_one() {
        let mut t = Tracker::default();
        t.apply(
            parse_line(r#"DOWNPIPE:START {"format_id": "a+b"} [{"format_id": "a", "filesize": 10, "vcodec": "vp9", "height": 1080, "fps": 60}, {"format_id": "b", "acodec": "opus"}]"#)
                .unwrap(),
        );
        assert_eq!(t.total(), None);
        assert_eq!(t.quality().as_deref(), Some("1080p 60"));
        t.apply(
            parse_line(r#"DOWNPIPE:PROGRESS "b" {"status": "downloading", "downloaded_bytes": 1, "total_bytes_estimate": 20}"#)
                .unwrap(),
        );
        assert_eq!(t.total(), Some(30));
    }

    #[test]
    fn ignores_everything_else() {
        assert_eq!(parse_line("[youtube] Extracting URL: https://..."), None);
        assert_eq!(parse_line("DOWNPIPE:PROGRESS"), None);
        assert_eq!(parse_line("DOWNPIPE:PROGRESS \"1\" not json"), None);
        assert_eq!(parse_line("DOWNPIPE:DONE NA"), None);
        assert_eq!(parse_line("DOWNPIPE:DONE \"\""), None);
    }
}
