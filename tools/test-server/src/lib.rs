//! A deliberately misbehaving HTTP server for Downpipe's tests and benchmarks.
//!
//! Content is a deterministic function of `(offset, version)`, so any byte can
//! be produced (and checked) without storing the file. Query parameters switch
//! on the failure modes real servers exhibit:
//!
//! | parameter                    | effect                                                 |
//! |------------------------------|--------------------------------------------------------|
//! | `ranges=0`                   | ignore `Range`, always answer `200`                    |
//! | `validators=0`               | no `ETag` / `Last-Modified`                            |
//! | `weak=1`                     | weak `ETag` (`W/"..."`)                                |
//! | `rate=N`                     | at most N bytes/s per connection                       |
//! | `slow_from=O&slow_rate=N`    | bytes past offset O trickle at N bytes/s               |
//! | `drop_after=N&drop_times=K`  | the first K responses die after N bytes                |
//! | `fail=S&fail_times=K`        | the first K requests get status S                      |
//! | `retry_after=SECS`           | `Retry-After` on injected failures                     |
//! | `name=NAME`                  | `Content-Disposition: attachment` with that name       |
//! | `type=MIME`                  | `Content-Type`                                         |
//! | `chunked=1`                  | no `Content-Length` (chunked transfer)                 |
//! | `auth=user:pass`             | require HTTP Basic auth                                |
//!
//! `/redirect?to=/file/100` answers `302` to the given path.

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use bytes::Bytes;
use parking_lot::Mutex;
use serde::Deserialize;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

const CHUNK: usize = 16 * 1024;

/// Byte `offset` of the file for content `version`.
pub fn byte_at(offset: u64, version: u64) -> u8 {
    let mut z = offset.wrapping_add(version.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    (z ^ (z >> 31)) as u8
}

/// The complete expected file, for comparisons in tests.
pub fn content(size: u64, version: u64) -> Vec<u8> {
    (0..size).map(|i| byte_at(i, version)).collect()
}

#[derive(Debug, Clone, Default)]
pub struct RequestLog {
    pub path: String,
    pub range: Option<String>,
    pub if_range: Option<String>,
    pub referer: Option<String>,
    pub cookie: Option<String>,
    pub status: u16,
}

#[derive(Debug, Default)]
pub struct ServerState {
    /// Bumping this changes the content, ETag and Last-Modified of every file.
    pub version: AtomicU64,
    pub bytes_served: AtomicU64,
    active: AtomicU64,
    max_active: AtomicU64,
    drops: AtomicU64,
    failures: AtomicU64,
    log: Mutex<Vec<RequestLog>>,
}

impl ServerState {
    pub fn set_version(&self, v: u64) {
        self.version.store(v, Ordering::SeqCst);
    }

    pub fn requests(&self) -> Vec<RequestLog> {
        self.log.lock().clone()
    }

    pub fn max_concurrent(&self) -> u64 {
        self.max_active.load(Ordering::SeqCst)
    }

    pub fn bytes_served(&self) -> u64 {
        self.bytes_served.load(Ordering::SeqCst)
    }

    pub fn reset_stats(&self) {
        self.log.lock().clear();
        self.bytes_served.store(0, Ordering::SeqCst);
        self.max_active.store(0, Ordering::SeqCst);
        self.drops.store(0, Ordering::SeqCst);
        self.failures.store(0, Ordering::SeqCst);
    }

    fn record(&self, path: &str, headers: &HeaderMap, status: StatusCode) {
        let get = |name: header::HeaderName| {
            headers
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned)
        };
        self.log.lock().push(RequestLog {
            path: path.to_string(),
            range: get(header::RANGE),
            if_range: get(header::IF_RANGE),
            referer: get(header::REFERER),
            cookie: get(header::COOKIE),
            status: status.as_u16(),
        });
    }
}

#[derive(Debug)]
pub struct TestServer {
    pub addr: SocketAddr,
    pub state: Arc<ServerState>,
    task: JoinHandle<()>,
}

impl TestServer {
    /// Bind to an ephemeral port on loopback.
    pub async fn start() -> anyhow::Result<Self> {
        Self::bind(SocketAddr::from(([127, 0, 0, 1], 0))).await
    }

    pub async fn bind(addr: SocketAddr) -> anyhow::Result<Self> {
        let state = Arc::new(ServerState::default());
        let listener = TcpListener::bind(addr).await?;
        let addr = listener.local_addr()?;
        let app = router(state.clone());
        let task = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        Ok(Self { addr, state, task })
    }

    pub fn url(&self, path_and_query: &str) -> String {
        format!("http://{}{}", self.addr, path_and_query)
    }

    /// Serve until the task ends (used by the binary).
    pub async fn wait(mut self) {
        let _ = (&mut self.task).await;
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Ready-made URLs for each failure mode: printed by the binary and linked
/// from `/`, so the browser extension can be tried against them.
pub const SAMPLES: &[(&str, &str)] = &[
    (
        "/file/16777216?rate=2097152&name=browser-bridge.bin",
        "16 MiB at 2 MiB/s per connection, named browser-bridge.bin",
    ),
    ("/file/104857600", "100 MiB, ranges + ETag"),
    (
        "/file/52428800?ranges=0",
        "no range support (single connection)",
    ),
    (
        "/file/209715200?rate=4194304",
        "4 MiB/s per connection, so more connections help",
    ),
    (
        "/file/52428800?drop_after=3000000&drop_times=4",
        "first four responses die mid-stream",
    ),
    (
        "/file/20971520?fail=503&fail_times=2&retry_after=1",
        "503 with Retry-After twice, then OK",
    ),
    (
        "/file/10485760?name=r%C3%A9sum%C3%A9.pdf&type=application/pdf",
        "Content-Disposition with a UTF-8 name",
    ),
];

pub fn router(state: Arc<ServerState>) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/file/{size}", get(file))
        .route("/redirect", get(redirect))
        .with_state(state)
}

#[derive(Debug, Deserialize, Default)]
struct FileQuery {
    ranges: Option<u8>,
    validators: Option<u8>,
    weak: Option<u8>,
    rate: Option<u64>,
    slow_from: Option<u64>,
    slow_rate: Option<u64>,
    drop_after: Option<u64>,
    drop_times: Option<u64>,
    fail: Option<u16>,
    fail_times: Option<u64>,
    retry_after: Option<u64>,
    name: Option<String>,
    #[serde(rename = "type")]
    mime: Option<String>,
    chunked: Option<u8>,
    auth: Option<String>,
}

/// A page of links to the samples, for right-clicking in a browser.
async fn index() -> Response {
    let mut items = String::new();
    for (path, what) in SAMPLES {
        let href = path.replace('&', "&amp;");
        items.push_str(&format!(
            "<li><a href=\"{href}\">{href}</a><br><small>{what}</small></li>
"
        ));
    }
    let html = format!(
        "<!doctype html>
<html lang=\"en\"><head><meta charset=\"utf-8\">         <title>Downpipe test server</title></head>         <body style=\"font-family: system-ui, sans-serif; max-width: 720px; margin: 32px auto; padding: 0 16px\">         <h1>Downpipe test server</h1>         <p>Right-click a link and choose Download with Downpipe.</p>         <ul style=\"line-height: 1.6\">
{items}</ul></body></html>
"
    );
    let mut resp = html.into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    resp
}

#[derive(Debug, Deserialize)]
struct RedirectQuery {
    to: String,
}

async fn redirect(Query(q): Query<RedirectQuery>) -> Response {
    let mut resp = StatusCode::FOUND.into_response();
    if let Ok(v) = HeaderValue::from_str(&q.to) {
        resp.headers_mut().insert(header::LOCATION, v);
    }
    resp
}

async fn file(
    State(state): State<Arc<ServerState>>,
    Path(size): Path<u64>,
    Query(q): Query<FileQuery>,
    headers: HeaderMap,
) -> Response {
    let path = format!("/file/{size}");
    let resp = build_file_response(&state, size, &q, &headers);
    state.record(&path, &headers, resp.status());
    resp
}

fn build_file_response(
    state: &Arc<ServerState>,
    size: u64,
    q: &FileQuery,
    headers: &HeaderMap,
) -> Response {
    if let Some(expected) = &q.auth {
        let ok = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Basic "))
            .is_some_and(|b64| b64 == basic(expected));
        if !ok {
            let mut resp = StatusCode::UNAUTHORIZED.into_response();
            resp.headers_mut().insert(
                header::WWW_AUTHENTICATE,
                HeaderValue::from_static("Basic realm=\"test\""),
            );
            return resp;
        }
    }

    if let Some(status) = q.fail {
        let times = q.fail_times.unwrap_or(u64::MAX);
        if state.failures.fetch_add(1, Ordering::SeqCst) < times {
            let mut resp = StatusCode::from_u16(status)
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
                .into_response();
            if let Some(secs) = q.retry_after {
                resp.headers_mut()
                    .insert(header::RETRY_AFTER, HeaderValue::from(secs));
            }
            return resp;
        }
    }

    let version = state.version.load(Ordering::SeqCst);
    let etag = if q.weak == Some(1) {
        format!("W/\"v{version}-{size}\"")
    } else {
        format!("\"v{version}-{size}\"")
    };
    let last_modified = httpdate(1_790_000_000 + version);
    let validators = q.validators != Some(0);
    let ranges = q.ranges != Some(0);

    let mut range = None;
    if ranges && let Some(raw) = headers.get(header::RANGE).and_then(|v| v.to_str().ok()) {
        let fresh = match headers.get(header::IF_RANGE).and_then(|v| v.to_str().ok()) {
            None => true,
            Some(v) => validators && (v == etag || v == last_modified),
        };
        if fresh {
            match parse_range(raw, size) {
                Some(Ok(r)) => range = Some(r),
                Some(Err(())) => {
                    let mut resp = StatusCode::RANGE_NOT_SATISFIABLE.into_response();
                    if let Ok(v) = HeaderValue::from_str(&format!("bytes */{size}")) {
                        resp.headers_mut().insert(header::CONTENT_RANGE, v);
                    }
                    return resp;
                }
                None => {}
            }
        }
    }

    let (status, start, end) = match range {
        Some((s, e)) => (StatusCode::PARTIAL_CONTENT, s, e),
        None => (StatusCode::OK, 0, size),
    };

    let drop_at = q.drop_after.filter(|_| {
        let times = q.drop_times.unwrap_or(u64::MAX);
        state.drops.fetch_add(1, Ordering::SeqCst) < times
    });
    let body = body_stream(
        state.clone(),
        BodyPlan {
            start,
            end,
            version,
            rate: q.rate,
            slow_from: q.slow_from,
            slow_rate: q.slow_rate,
            drop_after: drop_at,
        },
    );

    let mut resp = Response::new(body);
    *resp.status_mut() = status;
    let h = resp.headers_mut();
    let mime = q.mime.as_deref().unwrap_or("application/octet-stream");
    if let Ok(v) = HeaderValue::from_str(mime) {
        h.insert(header::CONTENT_TYPE, v);
    }
    if ranges {
        h.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    }
    if validators {
        if let Ok(v) = HeaderValue::from_str(&etag) {
            h.insert(header::ETAG, v);
        }
        if let Ok(v) = HeaderValue::from_str(&last_modified) {
            h.insert(header::LAST_MODIFIED, v);
        }
    }
    if q.chunked != Some(1) {
        h.insert(header::CONTENT_LENGTH, HeaderValue::from(end - start));
    }
    if status == StatusCode::PARTIAL_CONTENT
        && let Ok(v) = HeaderValue::from_str(&format!("bytes {start}-{}/{size}", end - 1))
    {
        h.insert(header::CONTENT_RANGE, v);
    }
    if let Some(name) = &q.name {
        let encoded: String = name
            .bytes()
            .map(|b| {
                if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                    (b as char).to_string()
                } else {
                    format!("%{b:02X}")
                }
            })
            .collect();
        let ascii: String = name
            .chars()
            .map(|c| {
                if c.is_ascii() && c != '"' && c != '\\' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let value = format!("attachment; filename=\"{ascii}\"; filename*=UTF-8''{encoded}");
        if let Ok(v) = HeaderValue::from_str(&value) {
            h.insert(header::CONTENT_DISPOSITION, v);
        }
    }
    resp
}

/// `Some(Ok((start, end_exclusive)))`, `Some(Err(()))` if unsatisfiable,
/// `None` if the header is not a single byte range we understand.
fn parse_range(raw: &str, size: u64) -> Option<Result<(u64, u64), ()>> {
    let spec = raw.trim().strip_prefix("bytes=")?;
    if spec.contains(',') {
        return None;
    }
    let (a, b) = spec.split_once('-')?;
    let (a, b) = (a.trim(), b.trim());
    if a.is_empty() {
        let suffix: u64 = b.parse().ok()?;
        if suffix == 0 || size == 0 {
            return Some(Err(()));
        }
        return Some(Ok((size.saturating_sub(suffix), size)));
    }
    let start: u64 = a.parse().ok()?;
    if start >= size {
        return Some(Err(()));
    }
    let end = if b.is_empty() {
        size
    } else {
        let last: u64 = b.parse().ok()?;
        if last < start {
            return None;
        }
        (last + 1).min(size)
    };
    Some(Ok((start, end)))
}

#[derive(Debug, Clone, Copy)]
struct BodyPlan {
    start: u64,
    end: u64,
    version: u64,
    rate: Option<u64>,
    slow_from: Option<u64>,
    slow_rate: Option<u64>,
    drop_after: Option<u64>,
}

struct ActiveGuard(Arc<ServerState>);

impl ActiveGuard {
    fn new(state: Arc<ServerState>) -> Self {
        let now = state.active.fetch_add(1, Ordering::SeqCst) + 1;
        state.max_active.fetch_max(now, Ordering::SeqCst);
        Self(state)
    }
}

impl Drop for ActiveGuard {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::SeqCst);
    }
}

fn body_stream(state: Arc<ServerState>, plan: BodyPlan) -> Body {
    struct Cursor {
        guard: ActiveGuard,
        pos: u64,
        sent: u64,
        /// `(started, bytes sent when started, rate)` of the current pacing
        /// phase. Sleeping until a deadline instead of for a fixed time keeps
        /// the rate accurate even with Windows' ~15 ms timer granularity.
        pace: Option<(tokio::time::Instant, u64, u64)>,
    }
    let cursor = Cursor {
        guard: ActiveGuard::new(state),
        pos: plan.start,
        sent: 0,
        pace: None,
    };
    let stream = futures_util::stream::unfold(cursor, move |mut c| async move {
        if c.pos >= plan.end {
            return None;
        }
        if plan.drop_after.is_some_and(|limit| c.sent >= limit) {
            let err = std::io::Error::new(std::io::ErrorKind::ConnectionReset, "injected drop");
            return Some((Err(err), Cursor { pos: plan.end, ..c }));
        }
        let mut len = (CHUNK as u64).min(plan.end - c.pos);
        if let Some(limit) = plan.drop_after {
            len = len.min(limit - c.sent).max(1);
        }
        let rate = match (plan.slow_from, plan.slow_rate) {
            (Some(from), Some(rate)) if c.pos >= from => Some(rate),
            (Some(from), Some(_)) if c.pos + len > from => {
                len = from - c.pos;
                plan.rate
            }
            _ => plan.rate,
        };
        match rate.filter(|r| *r > 0) {
            Some(rate) => {
                let (started, base, _) = match c.pace {
                    Some(p) if p.2 == rate => p,
                    _ => {
                        let p = (tokio::time::Instant::now(), c.sent, rate);
                        c.pace = Some(p);
                        p
                    }
                };
                let due = (c.sent + len - base) as f64 / rate as f64;
                tokio::time::sleep_until(started + Duration::from_secs_f64(due)).await;
            }
            None => c.pace = None,
        }
        let chunk: Vec<u8> = (c.pos..c.pos + len)
            .map(|i| byte_at(i, plan.version))
            .collect();
        c.pos += len;
        c.sent += len;
        c.guard.0.bytes_served.fetch_add(len, Ordering::SeqCst);
        Some((Ok::<_, std::io::Error>(Bytes::from(chunk)), c))
    });
    Body::from_stream(stream)
}

fn basic(user_pass: &str) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = user_pass.as_bytes();
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(TABLE[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn httpdate(unix: u64) -> String {
    httpdate::fmt_http_date(std::time::UNIX_EPOCH + Duration::from_secs(unix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges() {
        assert_eq!(parse_range("bytes=0-", 10), Some(Ok((0, 10))));
        assert_eq!(parse_range("bytes=2-4", 10), Some(Ok((2, 5))));
        assert_eq!(parse_range("bytes=5-100", 10), Some(Ok((5, 10))));
        assert_eq!(parse_range("bytes=-3", 10), Some(Ok((7, 10))));
        assert_eq!(parse_range("bytes=10-", 10), Some(Err(())));
        assert_eq!(parse_range("bytes=0-1,3-4", 10), None);
        assert_eq!(parse_range("items=0-1", 10), None);
    }

    #[test]
    fn basic_auth_encoding() {
        assert_eq!(basic("user:pass"), "dXNlcjpwYXNz");
        assert_eq!(basic("a:b"), "YTpi");
        assert_eq!(basic("ab:c"), "YWI6Yw==");
    }

    #[test]
    fn content_is_deterministic() {
        assert_eq!(content(64, 1), content(64, 1));
        assert_ne!(content(64, 1), content(64, 2));
    }
}
