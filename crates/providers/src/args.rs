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

/// Options every run gets.
fn common(tools: &Tools, proxy: Option<&str>) -> Vec<OsString> {
    let mut args: Vec<OsString> = [
        "--ignore-config",
        "--no-cookies",
        "--no-cookies-from-browser",
        "--no-playlist",
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
    if let Some(proxy) = proxy.filter(|p| !p.is_empty()) {
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

/// Everything a download needs besides the tools.
#[derive(Debug, Clone)]
pub struct DownloadArgs<'a> {
    pub url: &'a str,
    pub dir: &'a Path,
    /// The file name without extension; yt-dlp adds the container's.
    pub stem: &'a str,
    pub selection: Option<&'a MediaSelection>,
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
    args.push(format_selector(d.selection, ffmpeg).into());
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
    fn format_ids() {
        assert!(valid_format_id("140-drc"));
        assert!(valid_format_id("hls-1080p"));
        assert!(!valid_format_id(""));
        assert!(!valid_format_id("137+140"));
        assert!(!valid_format_id("b[height<=720]"));
        assert!(!valid_format_id(&"x".repeat(65)));
    }
}
