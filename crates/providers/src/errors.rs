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
    // yt-dlp warns about the cause (no JavaScript runtime, a missing PO
    // token) before the error it leads to, which only says a format or a
    // request failed.
    let all = lines.join("\n").to_lowercase();
    let warned = |needles: &[&str]| needles.iter().any(|n| all.contains(n));
    let site = |kind: &'static str, text: &str| DownloadError::Site {
        kind,
        message: format!("{text} (yt-dlp: {message})"),
    };
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
    let js_runtime = [
        "javascript runtime",
        "js runtime",
        "challenge solving failed",
        "n challenge",
        "nsig extraction failed",
        "signature solving failed",
    ];
    if has(&js_runtime) {
        return site(
            "site_js_runtime",
            "yt-dlp could not answer YouTube's JavaScript challenges. Download Deno in Settings, Media, then try again.",
        );
    }
    let po_token = ["po token", "po_token", "pot provider", "proof of origin"];
    if has(&po_token) || (has(&["http error 403"]) && warned(&po_token)) {
        return site(
            "site_po_token",
            "YouTube asked for a proof-of-origin (PO) token for this video. Update yt-dlp in Settings, Media, or choose another quality.",
        );
    }
    if has(&[
        "requested format is not available",
        "requested format not available",
    ]) {
        // Without a runtime YouTube offers only a few formats, and often
        // not the one chosen.
        if warned(&js_runtime) {
            return site(
                "site_js_runtime",
                "yt-dlp could not answer YouTube's JavaScript challenges, so this quality is missing. Download Deno in Settings, Media, then try again.",
            );
        }
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
    // Before the sign-in checks: these messages also say "Sign in".
    if has(&[
        "confirm your age",
        "age-restricted",
        "age restricted",
        "inappropriate for some users",
    ]) {
        return site(
            "site_age_restricted",
            "YouTube shows this video only to signed-in adults. Downpipe does not use accounts or browser cookies, so it cannot download it.",
        );
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
        "copyright claim",
        "playlist does not exist",
        "this channel does not exist",
    ]) {
        return DownloadError::Unavailable(message);
    }
    if has(&[
        "not a bot",
        "sign in to confirm",
        "sign in to view",
        "login required",
    ]) {
        return site(
            "site_sign_in",
            "YouTube wants a signed-in browser for this download. Downpipe does not use browser cookies; wait a while or use another network, then try again.",
        );
    }
    if has(&["unsupported url"]) {
        return DownloadError::UnsupportedMedia("yt-dlp does not support this page".into());
    }
    if has(&["http error 403"]) {
        return site(
            "site_forbidden",
            "YouTube refused the download. A newer yt-dlp usually fixes this: update it in Settings, Media, then try again.",
        );
    }
    if has(&[
        "unable to extract",
        "please report this issue",
        "unable to parse",
        "failed to parse json",
    ]) {
        return site(
            "site_outdated",
            "yt-dlp no longer understands this page; YouTube probably changed it. Update yt-dlp in Settings, Media, then try again.",
        );
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
            "site_age_restricted"
        );
        assert_eq!(
            kind(
                "ERROR: [youtube] x: Private video. Sign in if you've been granted access to this video",
                true
            ),
            "unavailable"
        );
        assert_eq!(
            kind(
                "ERROR: [youtube] x: Sign in to confirm you're not a bot. Use --cookies-from-browser",
                true
            ),
            "site_sign_in"
        );
        assert_eq!(
            kind(
                "WARNING: [youtube] x: No supported JavaScript runtime could be found.\nERROR: [youtube] x: Requested format is not available.",
                true
            ),
            "site_js_runtime"
        );
        assert_eq!(
            kind("ERROR: [youtube] x: n challenge solving failed", true),
            "site_js_runtime"
        );
        assert_eq!(
            kind(
                "WARNING: [youtube] x: Some web formats require a GVS PO Token which was not provided.\nERROR: unable to download video data: HTTP Error 403: Forbidden",
                true
            ),
            "site_po_token"
        );
        assert_eq!(
            kind(
                "ERROR: [youtube] x: Unable to extract yt initial data; please report this issue",
                true
            ),
            "site_outdated"
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
            "site_forbidden"
        );
        // The UI's own text for each, with yt-dlp's words kept.
        let err = classify(
            &lines("ERROR: unable to download video data: HTTP Error 403: Forbidden"),
            Some(1),
            true,
        );
        assert!(err.to_string().contains("HTTP Error 403"));
        assert!(downpipe_http::SITE_KINDS.contains(&err.kind()));
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
