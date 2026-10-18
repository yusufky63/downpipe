//! yt-dlp's command lines, built as argument lists.
//!
//! Every run starts with `--ignore-config` and never passes cookies, so a
//! yt-dlp configuration file on the machine cannot make Downpipe send a
//! browser's session data, change where files go or download a whole
//! playlist. The page URL always comes last, after `--`, so it can never be
//! read as an option.

use std::ffi::OsString;
use std::path::Path;

use downpipe_http::media::MediaSelection;
use serde::{Deserialize, Serialize};

use crate::tools::Tools;

/// Marks the lines Downpipe asks yt-dlp to print, so nothing else on its
/// output is mistaken for them.
pub const START: &str = "DOWNPIPE:START";
pub const PROGRESS: &str = "DOWNPIPE:PROGRESS";
pub const DONE: &str = "DOWNPIPE:DONE";

/// Fields of the chosen formats printed before the download starts.
const FORMAT_FIELDS: &str = "format_id,ext,height,fps,vcodec,acodec,filesize,filesize_approx";

/// A format id as it may appear in a `-f` selector: a plain token, so a
/// value from the UI cannot inject selector syntax (`+`, `/`, `[`).
pub fn valid_format_id(id: &str) -> bool {
    (1..=64).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

/// A subtitle language code as it may appear in `--sub-langs`, which
/// yt-dlp reads as a comma-separated list of regular expressions: letters,
/// digits and dashes only (`en`, `pt-BR`, `zh-Hans`, `en-orig`).
pub fn valid_sub_lang(lang: &str) -> bool {
    (1..=32).contains(&lang.len())
        && lang
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

/// One quality for every video of a playlist, whose formats are not known
/// when they are chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Preset {
    Best,
    #[serde(rename = "1080p")]
    P1080,
    #[serde(rename = "720p")]
    P720,
    /// The best sound alone.
    Audio,
}

impl Preset {
    /// The quality label the list shows before yt-dlp names the formats.
    pub fn label(self) -> &'static str {
        match self {
            Self::Best => "Best",
            Self::P1080 => "1080p",
            Self::P720 => "720p",
            Self::Audio => "Audio only",
        }
    }

    fn height(self) -> Option<u32> {
        match self {
            Self::P1080 => Some(1080),
            Self::P720 => Some(720),
            Self::Best | Self::Audio => None,
        }
    }
}

/// A subtitle to save with the video: the uploader's (`auto: false`) or
/// the site's automatic captions in that language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubtitleRef {
    pub language: String,
    #[serde(default)]
    pub auto: bool,
}

/// Options every run gets.
fn common(tools: &Tools, proxy: Option<&str>) -> Vec<OsString> {
    let mut args = base(tools, proxy);
    args.insert(3, "--no-playlist".into());
    args
}

/// [`common`] without `--no-playlist`, for reading a playlist.
fn base(tools: &Tools, proxy: Option<&str>) -> Vec<OsString> {
    let mut args: Vec<OsString> = [
        "--ignore-config",
        "--no-cookies",
        "--no-cookies-from-browser",
        "--color",
        "never",
    ]
    .into_iter()
    .map(OsString::from)
    .collect();
    if let Some(js) = &tools.js {
        args.push("--js-runtimes".into());
        let mut value = OsString::from(format!("{}:", js.kind.as_str()));
        value.push(&js.path);
        args.push(value);
    }
    // An empty proxy is yt-dlp's "connect directly", which a site rule
    // without a proxy asks for even when the system has one.
    if let Some(proxy) = proxy {
        args.push("--proxy".into());
        args.push(proxy.into());
    }
    args
}

/// `yt-dlp -J` for one page.
pub fn inspect(url: &str, tools: &Tools, proxy: Option<&str>) -> Vec<OsString> {
    let mut args = common(tools, proxy);
    args.push("-J".into());
    args.push("--".into());
    args.push(url.into());
    args
}

/// The most videos read from one playlist or channel.
pub const MAX_LIST_ENTRIES: usize = 5000;

/// `yt-dlp --flat-playlist -J` for a playlist or channel: its videos' ids
/// and titles from the list pages alone, without opening each video.
pub fn list(url: &str, tools: &Tools, proxy: Option<&str>) -> Vec<OsString> {
    let mut args = base(tools, proxy);
    args.extend(
        [
            "--yes-playlist",
            "--flat-playlist",
            "--playlist-end",
            &MAX_LIST_ENTRIES.to_string(),
            "-J",
            "--",
            url,
        ]
        .map(OsString::from),
    );
    args
}

/// Everything a download needs besides the tools.
#[derive(Debug, Clone)]
pub struct DownloadArgs<'a> {
    pub url: &'a str,
    pub dir: &'a Path,
    /// The file name without extension; yt-dlp adds the container's.
    pub stem: &'a str,
    pub selection: Option<&'a MediaSelection>,
    /// The quality when there is no `selection` (a video from a playlist).
    pub preset: Option<Preset>,
    pub subtitles: &'a [SubtitleRef],
    pub proxy: Option<&'a str>,
    /// Bytes per second, 0 for no limit.
    pub rate_limit: u64,
}

/// `yt-dlp` downloading one page into `dir/stem.<ext>`, printing the lines
/// [`crate::progress`] reads.
pub fn download(d: &DownloadArgs<'_>, tools: &Tools) -> Vec<OsString> {
    let mut args = common(tools, d.proxy);
    let ffmpeg = tools.ffmpeg.is_some();
    args.extend(
        [
            "--newline",
            "--progress",
            "--progress-delta",
            "0.5",
            "--continue",
            "--no-mtime",
            // A live stream never ends; the inspection refuses them, and
            // this keeps a page that went live since from starting one.
            "--match-filters",
            "!is_live & live_status != is_upcoming",
            "--max-downloads",
            "1",
            // Best quality first; at equal quality, codecs that join into
            // an MP4 that plays everywhere.
            "-S",
            "res,fps,vcodec:h264,acodec:aac",
            "--merge-output-format",
            "mp4/mkv",
            "-f",
        ]
        .map(OsString::from),
    );
    args.push(
        match (d.selection, d.preset) {
            (None, Some(preset)) => preset_selector(preset, ffmpeg),
            (selection, _) => format_selector(selection, ffmpeg),
        }
        .into(),
    );
    args.extend(subtitle_args(d, ffmpeg));
    if let Some(path) = &tools.ffmpeg {
        args.push("--ffmpeg-location".into());
        args.push(path.into());
    }
    if d.rate_limit > 0 {
        args.push("--limit-rate".into());
        args.push(d.rate_limit.to_string().into());
    }
    args.push("-P".into());
    args.push(d.dir.into());
    args.push("-o".into());
    // Literal text in an output template only needs `%` doubled.
    args.push(format!("{}.%(ext)s", d.stem.replace('%', "%%")).into());
    args.push("--progress-template".into());
    args.push(format!("download:{PROGRESS} %(info.format_id)j %(progress.{{status,downloaded_bytes,total_bytes,total_bytes_estimate,speed,fragment_index,fragment_count}})j").into());
    args.push("--print".into());
    args.push(
        format!("before_dl:{START} %(.{{{FORMAT_FIELDS}}})j %(requested_formats.:.{{{FORMAT_FIELDS}}}|null)j")
            .into(),
    );
    args.push("--print".into());
    args.push(format!("after_move:{DONE} %(filepath)j").into());
    args.push("--".into());
    args.push(d.url.into());
    args
}

/// The `-f` value for a choice from the quality list. The chosen formats
/// come first; if the site no longer offers them, the closest by height.
pub fn format_selector(selection: Option<&MediaSelection>, ffmpeg: bool) -> String {
    let video = selection
        .and_then(|s| s.video.as_ref())
        .filter(|t| valid_format_id(&t.id));
    let audio = selection
        .and_then(|s| s.audio.as_ref())
        .filter(|t| valid_format_id(&t.id));
    match (video, audio) {
        (Some(v), Some(a)) => {
            let fallback = match v.height {
                Some(h) => format!("bv*[height<={h}]+ba/b[height<={h}]/b"),
                None => "bv*+ba/b".to_string(),
            };
            format!("{}+{}/{fallback}", v.id, a.id)
        }
        // A video with its own sound, or an "Audio only" entry (no height).
        (Some(v), None) => match v.height {
            Some(h) if ffmpeg => format!("{}/bv*[height<={h}]+ba/b[height<={h}]/b", v.id),
            Some(h) => format!("{}/b[height<={h}]/b", v.id),
            None => format!("{}/ba/b", v.id),
        },
        (None, Some(a)) => format!("{}/ba/b", a.id),
        // Without FFmpeg only a format that has both video and sound in
        // one file can be saved as one file.
        (None, None) if ffmpeg => "bv*+ba/b".to_string(),
        (None, None) => "b".to_string(),
    }
}

/// The `-f` value for a playlist's quality: the best formats up to that
/// height, joined when FFmpeg is there, else the best single file.
pub fn preset_selector(preset: Preset, ffmpeg: bool) -> String {
    match (preset, preset.height()) {
        (Preset::Audio, _) => "ba/b".into(),
        (_, Some(h)) if ffmpeg => format!("bv*[height<={h}]+ba/b[height<={h}]/b"),
        (_, Some(h)) => format!("b[height<={h}]/b"),
        (_, None) if ffmpeg => "bv*+ba/b".into(),
        (_, None) => "b".into(),
    }
}

/// Whether a choice is the sound alone.
pub fn is_audio_only(selection: Option<&MediaSelection>, preset: Option<Preset>) -> bool {
    match selection {
        Some(s) => s.audio.is_none() && s.video.as_ref().is_some_and(|v| v.height.is_none()),
        None => preset == Some(Preset::Audio),
    }
}

/// Subtitles: embedded in the video when FFmpeg is there, else written
/// next to it; next to an audio file too, which cannot carry them.
fn subtitle_args(d: &DownloadArgs<'_>, ffmpeg: bool) -> Vec<OsString> {
    let chosen: Vec<&SubtitleRef> = d
        .subtitles
        .iter()
        .filter(|s| valid_sub_lang(&s.language))
        .collect();
    if chosen.is_empty() {
        return Vec::new();
    }
    let mut args: Vec<OsString> = Vec::new();
    // With both, yt-dlp takes the uploader's subtitles and falls back to
    // the automatic captions for a language that has none.
    if chosen.iter().any(|s| !s.auto) {
        args.push("--write-subs".into());
    }
    if chosen.iter().any(|s| s.auto) {
        args.push("--write-auto-subs".into());
    }
    let mut langs: Vec<&str> = Vec::new();
    for s in &chosen {
        if !langs.contains(&s.language.as_str()) {
            langs.push(&s.language);
        }
    }
    args.push("--sub-langs".into());
    args.push(langs.join(",").into());
    if ffmpeg && !is_audio_only(d.selection, d.preset) {
        args.push("--embed-subs".into());
    } else if ffmpeg {
        // Players read SRT next to an audio file more often than WebVTT.
        args.push("--convert-subs".into());
        args.push("srt".into());
    } else {
        args.push("--sub-format".into());
        args.push("srt/vtt/best".into());
    }
    args
}

/// Whether a choice downloads separate video and sound, which only FFmpeg
/// can join.
pub fn needs_merge(selection: Option<&MediaSelection>) -> bool {
    selection.is_some_and(|s| s.video.is_some() && s.audio.is_some())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use downpipe_http::media::TrackRef;

    use super::*;
    use crate::tools::{JsKind, JsRuntime};

    fn track(id: &str, height: Option<u32>) -> TrackRef {
        TrackRef {
            id: id.into(),
            height,
            ..TrackRef::default()
        }
    }

    fn text(args: &[OsString]) -> Vec<String> {
        args.iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn selectors() {
        let both = MediaSelection {
            video: Some(track("137", Some(1080))),
            audio: Some(track("140", None)),
        };
        assert_eq!(
            format_selector(Some(&both), true),
            "137+140/bv*[height<=1080]+ba/b[height<=1080]/b"
        );
        assert!(needs_merge(Some(&both)));
        let audio_only = MediaSelection {
            video: Some(track("251", None)),
            audio: None,
        };
        assert_eq!(format_selector(Some(&audio_only), false), "251/ba/b");
        assert!(!needs_merge(Some(&audio_only)));
        let progressive = MediaSelection {
            video: Some(track("18", Some(360))),
            audio: None,
        };
        assert_eq!(
            format_selector(Some(&progressive), false),
            "18/b[height<=360]/b"
        );
        assert_eq!(format_selector(None, true), "bv*+ba/b");
        assert_eq!(format_selector(None, false), "b");
        // Ids from the UI that are not plain tokens are ignored.
        let injected = MediaSelection {
            video: Some(track("137/bestvideo[filesize>1G]", Some(1080))),
            audio: None,
        };
        assert_eq!(format_selector(Some(&injected), true), "bv*+ba/b");
    }

    #[test]
    fn download_arguments() {
        let tools = Tools {
            yt_dlp: Some(PathBuf::from("yt-dlp")),
            js: Some(JsRuntime {
                kind: JsKind::Deno,
                path: PathBuf::from("C:/tools/deno.exe"),
            }),
            ffmpeg: Some(PathBuf::from("C:/ffmpeg/ffmpeg.exe")),
        };
        let args = text(&download(
            &DownloadArgs {
                url: "https://www.youtube.com/watch?v=jNQXAC9IVRw",
                dir: Path::new("D:/Video"),
                stem: "100% zoo",
                selection: None,
                preset: None,
                subtitles: &[],
                proxy: Some("socks5://127.0.0.1:9050"),
                rate_limit: 1_000_000,
            },
            &tools,
        ));
        let at = |flag: &str| {
            args.iter()
                .position(|a| a == flag)
                .map(|i| args[i + 1].as_str())
        };
        assert_eq!(args[0], "--ignore-config");
        assert!(args.contains(&"--no-cookies".to_string()));
        assert!(args.contains(&"--no-playlist".to_string()));
        assert_eq!(at("--js-runtimes"), Some("deno:C:/tools/deno.exe"));
        assert_eq!(at("--ffmpeg-location"), Some("C:/ffmpeg/ffmpeg.exe"));
        assert_eq!(at("--proxy"), Some("socks5://127.0.0.1:9050"));
        assert_eq!(at("--limit-rate"), Some("1000000"));
        let direct = text(&inspect("https://youtu.be/x", &tools, Some("")));
        let i = direct.iter().position(|a| a == "--proxy").unwrap();
        assert_eq!(direct[i + 1], "", "an empty proxy connects directly");
        assert!(
            !text(&inspect("https://youtu.be/x", &tools, None)).contains(&"--proxy".to_string())
        );
        assert_eq!(at("-P"), Some("D:/Video"));
        assert_eq!(at("-o"), Some("100%% zoo.%(ext)s"));
        assert_eq!(at("-f"), Some("bv*+ba/b"));
        assert!(
            at("--progress-template")
                .unwrap()
                .starts_with("download:DOWNPIPE:PROGRESS ")
        );
        // The URL is last, behind `--`.
        assert_eq!(
            &args[args.len() - 2..],
            ["--", "https://www.youtube.com/watch?v=jNQXAC9IVRw"]
        );
        assert!(
            !args
                .iter()
                .any(|a| a.contains("cookies-from-browser") && a != "--no-cookies-from-browser")
        );
    }

    #[test]
    fn presets_and_subtitles() {
        assert_eq!(preset_selector(Preset::Best, true), "bv*+ba/b");
        assert_eq!(preset_selector(Preset::Best, false), "b");
        assert_eq!(
            preset_selector(Preset::P720, true),
            "bv*[height<=720]+ba/b[height<=720]/b"
        );
        assert_eq!(preset_selector(Preset::P1080, false), "b[height<=1080]/b");
        assert_eq!(preset_selector(Preset::Audio, true), "ba/b");
        assert!(is_audio_only(None, Some(Preset::Audio)));
        assert!(!is_audio_only(None, Some(Preset::P720)));

        let mut tools = Tools {
            yt_dlp: Some(PathBuf::from("yt-dlp")),
            js: None,
            ffmpeg: Some(PathBuf::from("ffmpeg")),
        };
        let subs = [
            SubtitleRef {
                language: "en".into(),
                auto: false,
            },
            SubtitleRef {
                language: "en".into(),
                auto: true,
            },
            SubtitleRef {
                language: "tr;rm".into(),
                auto: true,
            },
        ];
        let run = |tools: &Tools, preset| {
            text(&download(
                &DownloadArgs {
                    url: "https://www.youtube.com/watch?v=jNQXAC9IVRw",
                    dir: Path::new("D:/Video"),
                    stem: "Clip",
                    selection: None,
                    preset: Some(preset),
                    subtitles: &subs,
                    proxy: None,
                    rate_limit: 0,
                },
                tools,
            ))
        };
        let at = |args: &[String], flag: &str| {
            args.iter()
                .position(|a| a == flag)
                .map(|i| args[i + 1].clone())
        };
        let args = run(&tools, Preset::P720);
        assert_eq!(
            at(&args, "-f").as_deref(),
            Some("bv*[height<=720]+ba/b[height<=720]/b")
        );
        assert!(args.contains(&"--write-subs".to_string()));
        assert!(args.contains(&"--write-auto-subs".to_string()));
        // The unsafe code is dropped, the repeated one listed once.
        assert_eq!(at(&args, "--sub-langs").as_deref(), Some("en"));
        assert!(args.contains(&"--embed-subs".to_string()));
        // An audio file cannot carry them: SRT next to it.
        let args = run(&tools, Preset::Audio);
        assert!(!args.contains(&"--embed-subs".to_string()));
        assert_eq!(at(&args, "--convert-subs").as_deref(), Some("srt"));
        // Without FFmpeg: next to the video, as the site has them.
        tools.ffmpeg = None;
        let args = run(&tools, Preset::Best);
        assert!(!args.contains(&"--embed-subs".to_string()));
        assert_eq!(at(&args, "--sub-format").as_deref(), Some("srt/vtt/best"));
        assert_eq!(at(&args, "-f").as_deref(), Some("b"));
    }

    #[test]
    fn list_arguments() {
        let tools = Tools {
            yt_dlp: Some(PathBuf::from("yt-dlp")),
            js: None,
            ffmpeg: None,
        };
        let args = text(&list(
            "https://www.youtube.com/playlist?list=PLx",
            &tools,
            None,
        ));
        assert_eq!(args[0], "--ignore-config");
        assert!(args.contains(&"--no-cookies".to_string()));
        assert!(!args.contains(&"--no-playlist".to_string()));
        assert!(args.contains(&"--flat-playlist".to_string()));
        assert_eq!(
            &args[args.len() - 3..],
            ["-J", "--", "https://www.youtube.com/playlist?list=PLx"]
        );
        // A video still never takes its playlist along.
        let args = text(&inspect("https://www.youtube.com/watch?v=x", &tools, None));
        assert!(args.contains(&"--no-playlist".to_string()));
    }

    #[test]
    fn format_ids() {
        assert!(valid_format_id("140-drc"));
        assert!(valid_format_id("hls-1080p"));
        assert!(!valid_format_id(""));
        assert!(!valid_format_id("137+140"));
        assert!(!valid_format_id("b[height<=720]"));
        assert!(!valid_format_id(&"x".repeat(65)));
    }
}
