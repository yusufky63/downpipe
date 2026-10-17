//! FFmpeg, found on the user's machine rather than shipped.
//!
//! Downpipe does not bundle FFmpeg: builds differ in license (LGPL or GPL
//! depending on the enabled codecs) and size, and the project reviews
//! redistribution terms before shipping third-party binaries. A user-installed
//! FFmpeg (on `PATH`, or the path set in the app) merges separate video and
//! audio and turns transport streams into MP4. Nothing is re-encoded
//! (`-c copy`), so this takes seconds. Without FFmpeg, tracks are saved as
//! they came.
//!
//! FFmpeg runs with an argument list, never through a shell, and only ever
//! reads files Downpipe wrote and writes one file in Downpipe's work folder.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::io::AsyncReadExt;

use super::Container;

/// Characters of FFmpeg's error output kept for the notice.
const MAX_ERROR_TEXT: usize = 400;

/// `configured` when it names an existing file, otherwise `ffmpeg` on `PATH`.
pub fn find_ffmpeg(configured: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = configured.filter(|p| !p.as_os_str().is_empty()) {
        return path.is_file().then(|| path.to_path_buf());
    }
    let exe = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    std::env::split_paths(&std::env::var_os("PATH")?)
        .filter(|dir| dir.is_absolute())
        .map(|dir| dir.join(exe))
        .find(|candidate| candidate.is_file())
}

/// The container separate tracks are merged into: WebM when both are WebM,
/// MP4 when MP4 can hold both, Matroska otherwise.
pub fn merged_container(video: Container, audio: Container) -> &'static str {
    match (video, audio) {
        (Container::WebM, Container::WebM) => "webm",
        (Container::WebM, _) | (_, Container::WebM) => "mkv",
        _ => "mp4",
    }
}

/// Arguments that copy the given tracks into `out` without re-encoding:
/// all streams of a single input, or the first video stream of the first
/// input with the first audio stream of the second.
pub(crate) fn args(inputs: &[(PathBuf, Container)], out: &Path) -> Vec<OsString> {
    let mut args: Vec<OsString> = ["-hide_banner", "-nostdin", "-loglevel", "error", "-y"]
        .into_iter()
        .map(OsString::from)
        .collect();
    for (path, _) in inputs {
        args.push("-i".into());
        args.push(path.into());
    }
    if inputs.len() > 1 {
        args.extend(["-map", "0:v:0", "-map", "1:a:0"].map(OsString::from));
    } else {
        args.extend(["-map", "0"].map(OsString::from));
    }
    args.extend(["-c", "copy"].map(OsString::from));
    // AAC from transport streams carries ADTS headers that MP4 does not
    // allow; this rewrites them, still without re-encoding.
    let mp4 = out
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("mp4"));
    if mp4
        && inputs
            .iter()
            .any(|(_, c)| matches!(c, Container::Ts | Container::Aac))
    {
        args.extend(["-bsf:a", "aac_adtstoasc"].map(OsString::from));
    }
    args.push(out.into());
    args
}

/// Run FFmpeg to completion. The error is FFmpeg's own message, trimmed.
pub(crate) async fn run(
    ffmpeg: &Path,
    inputs: &[(PathBuf, Container)],
    out: &Path,
) -> Result<(), String> {
    let mut command = tokio::process::Command::new(ffmpeg);
    command
        .args(args(inputs, out))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        // Dropping the future (a pause) stops FFmpeg instead of leaving it
        // writing into a folder that is about to be reused.
        .kill_on_drop(true);
    #[cfg(windows)]
    {
        // No console window flashing up from a GUI app.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("could not start FFmpeg: {e}"))?;
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stderr.take() {
        let _ = pipe.read_to_string(&mut stderr).await;
    }
    let status = child
        .wait()
        .await
        .map_err(|e| format!("FFmpeg did not finish: {e}"))?;
    if status.success() {
        return Ok(());
    }
    let text: String = stderr
        .trim()
        .chars()
        .rev()
        .take(MAX_ERROR_TEXT)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    Err(if text.is_empty() {
        format!("FFmpeg failed ({status})")
    } else {
        format!("FFmpeg failed: {text}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(args: &[OsString]) -> Vec<String> {
        args.iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn merges_two_tracks_by_copying() {
        let args = args(
            &[
                (PathBuf::from("v.mp4"), Container::Mp4),
                (PathBuf::from("a.m4a"), Container::M4a),
            ],
            Path::new("out.mp4"),
        );
        assert_eq!(
            text(&args),
            [
                "-hide_banner",
                "-nostdin",
                "-loglevel",
                "error",
                "-y",
                "-i",
                "v.mp4",
                "-i",
                "a.m4a",
                "-map",
                "0:v:0",
                "-map",
                "1:a:0",
                "-c",
                "copy",
                "out.mp4"
            ]
        );
    }

    #[test]
    fn rewrites_adts_audio_for_mp4_only() {
        let ts = args(
            &[(PathBuf::from("v.ts"), Container::Ts)],
            Path::new("out.mp4"),
        );
        assert!(text(&ts).ends_with(&["-bsf:a".into(), "aac_adtstoasc".into(), "out.mp4".into()]));
        let mkv = args(
            &[
                (PathBuf::from("v.webm"), Container::WebM),
                (PathBuf::from("a.aac"), Container::Aac),
            ],
            Path::new("out.mkv"),
        );
        assert!(!text(&mkv).contains(&"aac_adtstoasc".to_string()));
    }

    #[test]
    fn picks_a_container_both_tracks_fit() {
        assert_eq!(merged_container(Container::Mp4, Container::M4a), "mp4");
        assert_eq!(merged_container(Container::Ts, Container::Aac), "mp4");
        assert_eq!(merged_container(Container::WebM, Container::WebM), "webm");
        assert_eq!(merged_container(Container::WebM, Container::M4a), "mkv");
    }

    #[test]
    fn a_configured_path_must_exist() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("ffmpeg.exe");
        assert_eq!(find_ffmpeg(Some(&missing)), None);
        std::fs::write(&missing, b"").unwrap();
        assert_eq!(find_ffmpeg(Some(&missing)), Some(missing));
    }
}
