//! HTTP plumbing: client construction, range requests and response parsing.

use std::time::Duration;

use reqwest::header::{
    ACCEPT_ENCODING, CONTENT_DISPOSITION, CONTENT_RANGE, CONTENT_TYPE, ETAG, HeaderMap, HeaderName,
    HeaderValue, IF_RANGE, LAST_MODIFIED, RANGE, RETRY_AFTER,
};
use reqwest::{Client, RequestBuilder, Response};
use serde::Serialize;
use url::Url;

use crate::config::EngineConfig;
use crate::{DownloadError, filename, retry};

/// What we learned about the remote file from the first response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResourceInfo {
    /// URL after redirects, credentials stripped.
    pub final_url: String,
    pub total_size: Option<u64>,
    /// The server answered a range request with `206 Partial Content`.
    pub accepts_ranges: bool,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub content_type: Option<String>,
    /// Sanitized name derived from `Content-Disposition` or the URL.
    pub file_name: String,
}

impl ResourceInfo {
    /// Segmented, resumable download is possible.
    pub fn is_resumable(&self) -> bool {
        self.accepts_ranges && self.total_size.is_some_and(|t| t > 0)
    }

    pub(crate) fn from_response(resp: &Response, accepts_ranges: bool, total: Option<u64>) -> Self {
        let headers = resp.headers();
        let content_type = header_str(headers, &CONTENT_TYPE);
        let mut name = header_str(headers, &CONTENT_DISPOSITION)
            .and_then(|v| filename::from_content_disposition(&v))
            .or_else(|| filename::from_url(resp.url()))
            .unwrap_or_else(|| "download".to_string());
        if !name.contains('.')
            && let Some(ext) = content_type
                .as_deref()
                .and_then(filename::extension_for_mime)
        {
            name = format!("{name}.{ext}");
        }
        Self {
            final_url: redact_url(resp.url()),
            total_size: total,
            accepts_ranges,
            etag: header_str(headers, &ETAG),
            last_modified: header_str(headers, &LAST_MODIFIED),
            content_type,
            file_name: filename::sanitize(&name),
        }
    }
}

/// Parsed `Content-Range: bytes first-last/complete`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentRange {
    pub first: u64,
    pub last: u64,
    pub complete: Option<u64>,
}

pub fn parse_content_range(value: &str) -> Option<ContentRange> {
    let rest = value.trim().strip_prefix("bytes")?.trim_start();
    let (range, complete) = rest.split_once('/')?;
    let (first, last) = range.trim().split_once('-')?;
    let first: u64 = first.trim().parse().ok()?;
    let last: u64 = last.trim().parse().ok()?;
    let complete = match complete.trim() {
        "*" => None,
        n => Some(n.parse().ok()?),
    };
    if last < first || complete.is_some_and(|c| last >= c) {
        return None;
    }
    Some(ContentRange {
        first,
        last,
        complete,
    })
}

pub(crate) fn build_client(config: &EngineConfig) -> Result<Client, DownloadError> {
    let mut builder = Client::builder()
        .user_agent(config.user_agent.clone())
        .connect_timeout(config.connect_timeout)
        // Acceleration comes from several TCP connections each with its own
        // congestion window. HTTP/2 would multiplex every segment onto one
        // connection and erase the benefit, so stay on HTTP/1.1.
        .http1_only()
        .pool_max_idle_per_host(config.max_connections.max(1))
        .pool_idle_timeout(Duration::from_secs(30))
        .tcp_nodelay(true)
        .redirect(reqwest::redirect::Policy::limited(10));
    if let Some(proxy) = &config.proxy {
        let proxy = reqwest::Proxy::all(proxy)
            .map_err(|_| DownloadError::InvalidUrl("proxy URL is not valid".into()))?;
        builder = builder.proxy(proxy);
    }
    builder.build().map_err(|e| DownloadError::from_reqwest(&e))
}

/// `GET` for `[start, end)`; `end = None` means "to the end of the file".
pub(crate) fn range_get(
    client: &Client,
    url: &Url,
    headers: &HeaderMap,
    start: u64,
    end: Option<u64>,
    if_range: Option<&str>,
) -> RequestBuilder {
    let range = match end {
        Some(end) if end > start => format!("bytes={start}-{}", end - 1),
        _ => format!("bytes={start}-"),
    };
    let mut req = client
        .get(url.clone())
        .headers(headers.clone())
        .header(RANGE, range)
        // Byte offsets must refer to the stored representation, not a
        // gzip-on-the-fly variant whose length differs per request.
        .header(ACCEPT_ENCODING, "identity");
    if let Some(v) = if_range.and_then(|v| HeaderValue::from_str(v).ok()) {
        req = req.header(IF_RANGE, v);
    }
    req
}

pub(crate) fn plain_get(client: &Client, url: &Url, headers: &HeaderMap) -> RequestBuilder {
    client
        .get(url.clone())
        .headers(headers.clone())
        .header(ACCEPT_ENCODING, "identity")
}

const FORBIDDEN_HEADERS: &[&str] = &[
    "range",
    "if-range",
    "accept-encoding",
    "host",
    "content-length",
    "transfer-encoding",
    "connection",
    "keep-alive",
    "proxy-connection",
    "te",
    "trailer",
    "upgrade",
];

/// Validate user-supplied headers.
pub fn build_headers(pairs: &[(String, String)]) -> Result<HeaderMap, DownloadError> {
    let mut map = HeaderMap::new();
    for (name, value) in pairs {
        let lower = name.trim().to_ascii_lowercase();
        if FORBIDDEN_HEADERS.contains(&lower.as_str()) {
            return Err(DownloadError::InvalidHeader(format!(
                "`{}` is managed by the engine",
                name.trim()
            )));
        }
        let name = HeaderName::from_bytes(lower.as_bytes())
            .map_err(|_| DownloadError::InvalidHeader(format!("`{name}` is not a valid name")))?;
        let value = HeaderValue::from_str(value.trim())
            .map_err(|_| DownloadError::InvalidHeader(format!("value of `{name}` is not valid")))?;
        map.append(name, value);
    }
    Ok(map)
}

pub(crate) fn header_str(headers: &HeaderMap, name: &HeaderName) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub(crate) fn content_range(resp: &Response) -> Option<ContentRange> {
    header_str(resp.headers(), &CONTENT_RANGE).and_then(|v| parse_content_range(&v))
}

pub(crate) fn retry_after(resp: &Response) -> Option<Duration> {
    header_str(resp.headers(), &RETRY_AFTER).and_then(|v| retry::parse_retry_after(&v))
}

pub(crate) fn parse_url(input: &str) -> Result<Url, DownloadError> {
    let url = Url::parse(input.trim()).map_err(|e| DownloadError::InvalidUrl(e.to_string()))?;
    match url.scheme() {
        "http" | "https" => Ok(url),
        other => Err(DownloadError::UnsupportedScheme(other.to_string())),
    }
}

/// URL without `user:password@`. Used for anything persisted.
pub fn redact_url(url: &Url) -> String {
    let mut url = url.clone();
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.to_string()
}

/// URL without credentials, query or fragment. Used for logs.
pub fn display_url(url: &Url) -> String {
    let mut url = url.clone();
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_query(None);
    url.set_fragment(None);
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_range_parsing() {
        assert_eq!(
            parse_content_range("bytes 0-99/1000"),
            Some(ContentRange {
                first: 0,
                last: 99,
                complete: Some(1000)
            })
        );
        assert_eq!(
            parse_content_range("bytes 5-9/*"),
            Some(ContentRange {
                first: 5,
                last: 9,
                complete: None
            })
        );
        assert_eq!(parse_content_range("bytes */1000"), None);
        assert_eq!(parse_content_range("bytes 9-5/10"), None);
        assert_eq!(parse_content_range("bytes 0-10/10"), None);
        assert_eq!(parse_content_range("items 0-1/2"), None);
    }

    #[test]
    fn rejects_engine_headers() {
        let ok = build_headers(&[("Referer".into(), "https://example.org/".into())]).unwrap();
        assert_eq!(ok.get("referer").unwrap(), "https://example.org/");
        assert!(build_headers(&[("Range".into(), "bytes=0-".into())]).is_err());
        assert!(build_headers(&[("Bad Name".into(), "x".into())]).is_err());
        assert!(build_headers(&[("X-A".into(), "line\nbreak".into())]).is_err());
    }

    #[test]
    fn urls() {
        let url = parse_url("https://user:secret@example.org/a.iso?token=abc#x").unwrap();
        assert_eq!(redact_url(&url), "https://example.org/a.iso?token=abc#x");
        assert_eq!(display_url(&url), "https://example.org/a.iso");
        assert!(matches!(
            parse_url("ftp://example.org/a"),
            Err(DownloadError::UnsupportedScheme(_))
        ));
        assert!(matches!(
            parse_url("not a url"),
            Err(DownloadError::InvalidUrl(_))
        ));
    }
}
