use std::time::Duration;

use crate::progress::WaitReason;

/// The `kind` codes of [`DownloadError::Site`], each with its own text in
/// the apps:
///
/// * `site_forbidden`: the site refused the media (HTTP 403); a newer
///   yt-dlp usually fixes it.
/// * `site_sign_in`: the site wants a signed-in browser ("confirm you're
///   not a bot"); Downpipe never sends cookies, so waiting or another
///   network is what helps.
/// * `site_age_restricted`: only signed-in adults may watch it.
/// * `site_js_runtime`: yt-dlp could not answer the site's JavaScript
///   challenges, because no JavaScript runtime (Deno) is set up.
/// * `site_po_token`: the site wants a proof-of-origin token yt-dlp could
///   not make for this video.
/// * `site_outdated`: yt-dlp no longer understands the site's pages.
pub const SITE_KINDS: &[&str] = &[
    "site_forbidden",
    "site_sign_in",
    "site_age_restricted",
    "site_js_runtime",
    "site_po_token",
    "site_outdated",
];

/// Everything that can stop a download, grouped so callers can tell the user
/// *why* it failed instead of showing a bare status code.
#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("unsupported URL scheme `{0}` (only http and https are supported)")]
    UnsupportedScheme(String),

    #[error("invalid header: {0}")]
    InvalidHeader(String),

    #[error("the server requires authentication (HTTP {status})")]
    AuthRequired { status: u16 },

    #[error(
        "the server refused the request (HTTP 403); the link may have expired or need a Referer, cookies or a login"
    )]
    Forbidden,

    #[error("the file was not found on the server (HTTP {status})")]
    NotFound { status: u16 },

    #[error("the server is rate limiting this download (HTTP {status})")]
    Throttled {
        status: u16,
        retry_after: Option<Duration>,
    },

    #[error("unexpected HTTP status {status}")]
    HttpStatus { status: u16 },

    #[error("the remote file changed while downloading ({0})")]
    ResourceChanged(String),

    #[error("the new link points to a different file ({0}); the partial download was kept")]
    DifferentFile(String),

    /// A web page came back where the file was expected: what file hosts
    /// send once a link's token or session ran out.
    #[error(
        "the server sent a web page instead of the file; the link has probably expired (the partial download was kept)"
    )]
    ExpiredLink,

    /// Like [`ExpiredLink`](Self::ExpiredLink), but the page is a sign-in
    /// page.
    #[error(
        "the server sent a sign-in page instead of the file; sign in in the browser, then use a new link (the partial download was kept)"
    )]
    LoginRequired,

    /// A file host's file that only signed-in users with access may fetch.
    /// The message names the host and what to do.
    #[error("{0}")]
    PrivateFile(String),

    #[error("the server stopped honouring byte ranges")]
    RangeNotHonoured,

    #[error("the connection closed before the expected data arrived")]
    PrematureEnd,

    #[error("no data received for {0:?}")]
    Stalled(Duration),

    #[error("network error: {0}")]
    Network(String),

    #[error("checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("invalid checksum `{0}` (expected sha256:<hex> or sha512:<hex>)")]
    InvalidChecksum(String),

    #[error("disk error: {0}")]
    Io(#[from] std::io::Error),

    #[error("resume data is unreadable: {0}")]
    CorruptMeta(String),

    #[error("this stream is protected by DRM; Downpipe does not download protected media")]
    Protected,

    #[error("this is a live stream; only finished (on-demand) streams can be downloaded")]
    LiveStream,

    #[error("the stream manifest is not valid: {0}")]
    InvalidManifest(String),

    #[error("this stream is not supported: {0}")]
    UnsupportedMedia(String),

    /// A program a site provider needs (yt-dlp, FFmpeg) is not set up. The
    /// message says which and where to set it up.
    #[error("{0}")]
    ToolMissing(String),

    /// The site will not serve this video to anyone without an account:
    /// private, removed, age or region restricted.
    #[error("{0}")]
    Unavailable(String),

    /// An external extractor (yt-dlp) failed for a reason of its own.
    #[error("{0}")]
    Extractor(String),

    /// A site provider's failure whose cause is known well enough for the
    /// UI to explain it, and what to do, in the user's language. `kind` is
    /// a stable `site_*` code (see [`SITE_KINDS`]); `message` is the
    /// English explanation with the tool's own words.
    #[error("{message}")]
    Site { kind: &'static str, message: String },

    #[error("gave up after {attempts} attempts: {last}")]
    RetriesExhausted {
        attempts: u32,
        last: Box<DownloadError>,
    },
}

impl DownloadError {
    /// Whether a fresh request for the same bytes has a reasonable chance of
    /// succeeding. Auth, 404 and checksum failures will not fix themselves.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Throttled { .. }
                | Self::PrematureEnd
                | Self::Stalled(_)
                | Self::Network(_)
                | Self::RangeNotHonoured
        ) || matches!(self, Self::HttpStatus { status } if *status == 408 || *status >= 500)
    }

    /// The network between us and the server failed rather than the server:
    /// offline, DNS, refused, reset, or silence. These are retried for as
    /// long as [`EngineConfig::offline_grace`](crate::EngineConfig) allows,
    /// not a fixed number of times, so a Wi-Fi switch or a VPN reconnect
    /// does not fail a download.
    pub fn is_connectivity(&self) -> bool {
        matches!(
            self,
            Self::Network(_) | Self::Stalled(_) | Self::PrematureEnd
        )
    }

    /// What a download waiting to retry after this error tells the user.
    pub fn wait_reason(&self) -> WaitReason {
        match self {
            Self::Stalled(_) => WaitReason::NoData,
            Self::Network(_) | Self::PrematureEnd => WaitReason::Network,
            Self::Throttled { .. } => WaitReason::Throttled,
            Self::RetriesExhausted { last, .. } => last.wait_reason(),
            _ => WaitReason::Server,
        }
    }

    /// Server-provided delay, if the error carried one (`Retry-After`).
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::Throttled { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// Stable machine-readable category for UIs and diagnostics.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::InvalidUrl(_) | Self::UnsupportedScheme(_) => "invalid_url",
            Self::InvalidHeader(_) => "invalid_header",
            Self::AuthRequired { .. } => "auth_required",
            Self::Forbidden => "forbidden",
            Self::NotFound { .. } => "not_found",
            Self::Throttled { .. } => "throttled",
            Self::HttpStatus { .. } => "http_status",
            Self::ResourceChanged(_) => "resource_changed",
            Self::DifferentFile(_) => "different_file",
            Self::ExpiredLink => "expired_link",
            Self::LoginRequired => "login_required",
            Self::PrivateFile(_) => "private_file",
            Self::RangeNotHonoured => "range_not_honoured",
            Self::PrematureEnd | Self::Stalled(_) | Self::Network(_) => "network",
            Self::ChecksumMismatch { .. } | Self::InvalidChecksum(_) => "checksum",
            Self::Io(_) => "io",
            Self::CorruptMeta(_) => "corrupt_meta",
            Self::Protected => "drm_protected",
            Self::LiveStream => "live_stream",
            Self::InvalidManifest(_) => "invalid_manifest",
            Self::UnsupportedMedia(_) => "unsupported_media",
            Self::ToolMissing(_) => "tool_missing",
            Self::Unavailable(_) => "unavailable",
            Self::Extractor(_) => "extractor",
            Self::Site { kind, .. } => kind,
            Self::RetriesExhausted { last, .. } => last.kind(),
        }
    }

    pub(crate) fn from_reqwest(err: &reqwest::Error) -> Self {
        // reqwest's Display includes the URL, which may carry tokens in the
        // query string; keep only the error class and its source.
        let class = if err.is_timeout() {
            "timed out"
        } else if err.is_connect() {
            "could not connect"
        } else if err.is_body() || err.is_decode() {
            "body read failed"
        } else if err.is_redirect() {
            "too many redirects"
        } else {
            "request failed"
        };
        let source = std::error::Error::source(err)
            .map(|s| format!(": {s}"))
            .unwrap_or_default();
        Self::Network(format!("{class}{source}"))
    }
}

/// Map a non-success status to the error the user should see.
pub(crate) fn status_error(status: u16, retry_after: Option<Duration>) -> DownloadError {
    match status {
        401 | 407 => DownloadError::AuthRequired { status },
        403 => DownloadError::Forbidden,
        404 | 410 => DownloadError::NotFound { status },
        429 | 503 => DownloadError::Throttled {
            status,
            retry_after,
        },
        _ => DownloadError::HttpStatus { status },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_statuses() {
        assert!(matches!(
            status_error(401, None),
            DownloadError::AuthRequired { .. }
        ));
        assert!(matches!(status_error(403, None), DownloadError::Forbidden));
        assert!(matches!(
            status_error(410, None),
            DownloadError::NotFound { .. }
        ));
        assert!(status_error(429, None).is_retryable());
        assert!(status_error(503, None).is_retryable());
        assert!(status_error(502, None).is_retryable());
        assert!(status_error(408, None).is_retryable());
        assert!(!status_error(404, None).is_retryable());
        assert!(!status_error(400, None).is_retryable());
    }

    #[test]
    fn exhausted_keeps_inner_kind() {
        let err = DownloadError::RetriesExhausted {
            attempts: 3,
            last: Box::new(DownloadError::PrematureEnd),
        };
        assert_eq!(err.kind(), "network");
        assert!(!err.is_retryable());
    }
}
