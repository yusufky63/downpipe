//! yt-dlp's error output turned into errors the UI can explain.

use std::path::Path;

use downpipe_http::DownloadError;

/// Characters of yt-dlp's message kept for the user.
const MAX_MESSAGE: usize = 300;

pub(crate) fn no_yt_dlp() -> DownloadError {
    DownloadError::ToolMissing(
        "yt-dlp is not set up. Settings, Media can download it, or set the path to yt-dlp there."
            .into(),
    )
}

pub(crate) fn no_ffmpeg() -> DownloadError {
    DownloadError::ToolMissing(
        "This quality comes as separate video and sound, which only FFmpeg can join. Set up FFmpeg in Settings, Media, or choose Audio only."
            .into(),
    )
}

pub(crate) fn spawn_failed(program: &Path, err: &std::io::Error) -> DownloadError {
    DownloadError::ToolMissing(format!(
        "{} could not be started ({err}). Check it in Settings, Media.",
        program
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "yt-dlp".into())
    ))
}

/// The message of yt-dlp's last `ERROR:` line, without its prefixes:
/// `ERROR: [youtube] jNQXAC9IVRw: Private video` becomes `Private video`.
pub(crate) fn last_error(lines: &[String]) -> Option<String> {
    let line = lines
        .iter()
        .rev()
        .find_map(|l| l.trim().strip_prefix("ERROR:"))?;
    let mut message = line.trim();
    if let Some(rest) = message.strip_prefix('[')
        && let Some((_, after)) = rest.split_once(']')
    {
        message = after.trim();
        // "<video id>: " follows the extractor name.
        if let Some((id, after)) = message.split_once(": ")
            && !id.contains(' ')
        {
            message = after.trim();
        }
    }
    let clean: String = message
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_MESSAGE)
        .collect();
    (!clean.is_empty()).then_some(clean)
}

/// Decide what went wrong from yt-dlp's error output and exit code.
pub(crate) fn classify(lines: &[String], code: Option<i32>, ffmpeg: bool) -> DownloadError {
    let Some(message) = last_error(lines) else {
        return DownloadError::Extractor(match code {
            Some(code) => format!("yt-dlp stopped with exit code {code}"),
            None => "yt-dlp was stopped".into(),
        });
    };
    let lower = message.to_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|n| lower.contains(n));
    if has(&["drm"]) {
        return DownloadError::Protected;
    }
    if has(&[
        "live event will begin",
        "premieres in",
        "this live event",
        "is live now",
    ]) {
        return DownloadError::LiveStream;
    }
    if has(&["ffmpeg"]) && has(&["not found", "not installed", "could not find"]) {
        return no_ffmpeg();
    }
    if has(&[
        "requested format is not available",
        "requested format not available",
    ]) {
        return if ffmpeg {
            DownloadError::Extractor(
                "The chosen quality is no longer offered. Choose another one.".into(),
            )
        } else {
            DownloadError::ToolMissing(
                "This video is only offered as separate video and sound, which only FFmpeg can join. Set up FFmpeg in Settings, Media."
                    .into(),
            )
        };
    }
    if has(&["not a bot"]) {
        return DownloadError::Unavailable(format!(
            "YouTube wants a signed-in browser for this download ({message}). Downpipe does not use browser cookies; try again later."
        ));
    }
    if has(&[
        "private video",
        "video unavailable",
        "this video is unavailable",
        "has been removed",
        "no longer available",
        "not available in your country",
        "blocked it in your country",
        "members-only",
        "join this channel",
        "confirm your age",
        "age-restricted",
        "inappropriate for some users",
        "copyright claim",
    ]) {
        return DownloadError::Unavailable(message);
    }
    if has(&["unsupported url"]) {
        return DownloadError::UnsupportedMedia("yt-dlp does not support this page".into());
    }
    if has(&["http error 403"]) {
        return DownloadError::Extractor(format!(
            "The site refused the download ({message}). A newer yt-dlp usually fixes this; Settings, Media can download it again."
        ));
    }
    if has(&[
        "timed out",
        "getaddrinfo",
        "connection",
        "network is unreachable",
        "temporary failure in name resolution",
        "unable to download",
        "http error 5",
        "ssl",
    ]) {
        return DownloadError::Network(message);
    }
    DownloadError::Extractor(format!("yt-dlp: {message}"))
}

/// A warning from yt-dlp worth showing next to the download.
pub(crate) fn notice(line: &str) -> Option<String> {
    let line = line.trim().strip_prefix("WARNING:")?.to_lowercase();
    if line.contains("javascript runtime") {
        return Some(
            "No JavaScript runtime was found, so some qualities may be missing. Settings, Media can download Deno."
                .into(),
        );
    }
    if line.contains("ffmpeg") && line.contains("not") {
        return Some("FFmpeg was not found, so video and sound were not joined.".into());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_string).collect()
    }

    #[test]
    fn strips_prefixes() {
        assert_eq!(
            last_error(&lines("WARNING: x\nERROR: [youtube] jNQXAC9IVRw: Private video. Sign in if you've been granted access")).as_deref(),
            Some("Private video. Sign in if you've been granted access")
        );
        assert_eq!(
            last_error(&lines("ERROR: Unsupported URL: https://example.org/")).as_deref(),
            Some("Unsupported URL: https://example.org/")
        );
        assert_eq!(last_error(&lines("all fine")), None);
    }

    #[test]
    fn kinds() {
        let kind = |text: &str, ffmpeg: bool| classify(&lines(text), Some(1), ffmpeg).kind();
        assert_eq!(
            kind("ERROR: [youtube] x: This video is DRM protected", true),
            "drm_protected"
        );
        assert_eq!(
            kind(
                "ERROR: [youtube] x: This live event will begin in 3 hours.",
                true
            ),
            "live_stream"
        );
        assert_eq!(
            kind(
                "ERROR: [youtube] xxxxxxxxxxx: This video is unavailable",
                true
            ),
            "unavailable"
        );
        assert_eq!(
            kind("ERROR: [youtube] x: Sign in to confirm your age.", true),
            "unavailable"
        );
        assert_eq!(
            kind(
                "ERROR: [youtube] x: Sign in to confirm you're not a bot.",
                true
            ),
            "unavailable"
        );
        assert_eq!(
            kind(
                "ERROR: [youtube] x: Requested format is not available.",
                false
            ),
            "tool_missing"
        );
        assert_eq!(
            kind(
                "ERROR: [youtube] x: Requested format is not available.",
                true
            ),
            "extractor"
        );
        assert_eq!(
            kind("ERROR: Unsupported URL: https://example.org/", true),
            "unsupported_media"
        );
        assert_eq!(
            kind(
                "ERROR: unable to download video data: HTTP Error 403: Forbidden",
                true
            ),
            "extractor"
        );
        assert_eq!(
            kind(
                "ERROR: [youtube] x: Unable to download API page: <urlopen error timed out>",
                true
            ),
            "network"
        );
        assert_eq!(kind("ERROR: something new", true), "extractor");
        assert_eq!(
            classify(&[], Some(2), true).to_string(),
            "yt-dlp stopped with exit code 2"
        );
    }

    #[test]
    fn notices() {
        assert!(
            notice("WARNING: [youtube] No supported JavaScript runtime could be found.").is_some()
        );
        assert!(notice("WARNING: [youtube] something harmless").is_none());
        assert!(notice("ERROR: x").is_none());
    }
}
