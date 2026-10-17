use std::time::Duration;

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
            Self::RangeNotHonoured => "range_not_honoured",
            Self::PrematureEnd | Self::Stalled(_) | Self::Network(_) => "network",
            Self::ChecksumMismatch { .. } | Self::InvalidChecksum(_) => "checksum",
            Self::Io(_) => "io",
            Self::CorruptMeta(_) => "corrupt_meta",
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
