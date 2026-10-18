//! Routes, the guards in front of them, and what they send. The API only
//! ever reads the list and settings and calls `Manager::add`, `pause`,
//! `resume` and `remove`; it has no way to browse files or run anything.

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Instant;

use axum::extract::rejection::JsonRejection;
use axum::extract::{ConnectInfo, DefaultBodyLimit, Path, Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use downpipe_core::{
    AddOptions, CoreError, MAIN_QUEUE, Manager, OnDuplicate, TaskSnapshot, TaskState,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

use crate::assets::{self, Assets, PAGE};
use crate::limiter::FailureLimiter;
use crate::{folder, peer, token};

/// Requests carry a link and a few names; nothing legitimate comes close.
pub(crate) const MAX_BODY: usize = 16 * 1024;
const MAX_URL: usize = 8 * 1024;
/// Requests handled at once; more get `503` instead of piling up.
pub(crate) const MAX_IN_FLIGHT: usize = 32;

/// The page's own scripts, styles and fonts; nothing from elsewhere, and no
/// other site may frame it.
const PAGE_CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; \
     img-src 'self' data:; font-src 'self' data:; connect-src 'self'; object-src 'none'; \
     base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

#[derive(Debug)]
pub(crate) struct AppState {
    pub(crate) manager: Manager,
    pub(crate) token_hash: [u8; 32],
    pub(crate) limiter: Mutex<FailureLimiter>,
    pub(crate) assets: Arc<dyn Assets>,
    /// Listening on a network address: refuse peers outside local networks.
    pub(crate) network: bool,
    pub(crate) in_flight: Semaphore,
}

pub(crate) fn router(state: Arc<AppState>) -> Router {
    let api = Router::new()
        .route("/downloads", get(list).post(add))
        .route("/downloads/{id}", delete(remove))
        .route("/downloads/{id}/pause", post(pause))
        .route("/downloads/{id}/resume", post(resume))
        .route("/settings", get(settings))
        .fallback(|| async { error(StatusCode::NOT_FOUND, "not_found") })
        .route_layer(middleware::from_fn_with_state(state.clone(), authorize));
    Router::new()
        .nest("/api/v1", api)
        .route("/", get(page))
        .route("/icon.svg", get(icon))
        .route("/assets/{name}", get(asset))
        .fallback(|| async { error(StatusCode::NOT_FOUND, "not_found") })
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .layer(middleware::from_fn_with_state(state.clone(), guard))
        .with_state(state)
}

/// In front of everything, the page included: who may connect, from which
/// page, how many at once, and the headers every answer carries.
async fn guard(State(st): State<Arc<AppState>>, req: Request, next: Next) -> Response {
    let Some(ip) = peer_ip(&req) else {
        return error(StatusCode::FORBIDDEN, "forbidden");
    };
    if st.network && !peer::is_local_network(ip) {
        tracing::warn!(%ip, "remote control: refused a peer outside the local network");
        return error(StatusCode::FORBIDDEN, "forbidden");
    }
    if !same_origin(req.headers()) {
        return error(StatusCode::FORBIDDEN, "cross_origin");
    }
    if req.method() == Method::OPTIONS {
        // A CORS preflight: no other site's page may call the API, so there
        // is nothing to allow.
        return error(StatusCode::FORBIDDEN, "cross_origin");
    }
    let Ok(_permit) = st.in_flight.try_acquire() else {
        return error(StatusCode::SERVICE_UNAVAILABLE, "busy");
    };
    let mut res = next.run(req).await;
    let headers = res.headers_mut();
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert(
        "cross-origin-resource-policy",
        HeaderValue::from_static("same-origin"),
    );
    res
}

/// Every API request needs `Authorization: Bearer <token>`. A peer that
/// failed too often is refused before its token is looked at, so further
/// guesses tell it nothing.
async fn authorize(State(st): State<Arc<AppState>>, req: Request, next: Next) -> Response {
    let Some(ip) = peer_ip(&req) else {
        return error(StatusCode::FORBIDDEN, "forbidden");
    };
    if let Some(wait) = st.limiter.lock().blocked(ip, Instant::now()) {
        let mut res = error(StatusCode::TOO_MANY_REQUESTS, "too_many_attempts");
        if let Ok(v) = HeaderValue::from_str(&wait.as_secs().to_string()) {
            res.headers_mut().insert(header::RETRY_AFTER, v);
        }
        return res;
    }
    let ok = bearer(req.headers()).is_some_and(|t| token::matches(t, &st.token_hash));
    if !ok {
        st.limiter.lock().failed(ip, Instant::now());
        tracing::info!(%ip, "remote control: wrong or missing token");
        let mut res = error(StatusCode::UNAUTHORIZED, "unauthorized");
        res.headers_mut().insert(
            header::WWW_AUTHENTICATE,
            HeaderValue::from_static("Bearer realm=\"Downpipe\""),
        );
        return res;
    }
    st.limiter.lock().succeeded(ip);
    let mut res = next.run(req).await;
    res.headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    res
}

fn peer_ip(req: &Request) -> Option<IpAddr> {
    req.extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.ip().to_canonical())
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    scheme.eq_ignore_ascii_case("bearer").then(|| token.trim())
}

/// A browser names the page that made a request in `Origin` (always for
/// cross-site requests and for every POST and DELETE). It must be this
/// server, as the browser reached it. Requests without `Origin` come from
/// the page's own GETs or from programs, which still need the token.
fn same_origin(headers: &HeaderMap) -> bool {
    if headers
        .get("sec-fetch-site")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|site| site == "cross-site")
    {
        return false;
    }
    let Some(origin) = headers.get(header::ORIGIN) else {
        return true;
    };
    let (Ok(origin), Some(Ok(host))) = (
        origin.to_str(),
        headers.get(header::HOST).map(HeaderValue::to_str),
    ) else {
        return false;
    };
    origin.eq_ignore_ascii_case(&format!("http://{host}"))
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
}

fn error(status: StatusCode, code: &'static str) -> Response {
    (
        status,
        Json(ErrorBody {
            error: code,
            id: None,
        }),
    )
        .into_response()
}

fn core_error(err: CoreError) -> Response {
    let (status, id) = match &err {
        CoreError::NotFound(_) => (StatusCode::NOT_FOUND, None),
        CoreError::Duplicate(id) => (StatusCode::CONFLICT, Some(id.clone())),
        CoreError::InvalidState { .. } => (StatusCode::CONFLICT, None),
        CoreError::InvalidUrl(_) | CoreError::InvalidSetting(_) => (StatusCode::BAD_REQUEST, None),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, None),
    };
    if status == StatusCode::INTERNAL_SERVER_ERROR {
        tracing::warn!(error = %err, "remote control: request failed");
    }
    (
        status,
        Json(ErrorBody {
            error: err.code(),
            id,
        }),
    )
        .into_response()
}

fn json_error(rejection: JsonRejection) -> Response {
    let status = rejection.status();
    let code = if status == StatusCode::PAYLOAD_TOO_LARGE {
        "too_large"
    } else {
        "bad_request"
    };
    error(status, code)
}

/// One row of the list: what the phone shows, nothing more. No folders or
/// paths, and no full link, whose query string often carries a signature.
#[derive(Debug, Serialize, PartialEq)]
pub(crate) struct DownloadView {
    id: String,
    name: String,
    host: Option<String>,
    state: TaskState,
    total: Option<u64>,
    downloaded: u64,
    speed: u64,
    eta_secs: Option<u64>,
    error_kind: Option<String>,
    waiting: Option<downpipe_core::http::WaitReason>,
    queue: String,
    category: Option<String>,
    awaiting_confirmation: bool,
    /// "1080p", "Audio only" and the like, for streams and video pages.
    quality: Option<String>,
    created_at: i64,
    completed_at: Option<i64>,
}

impl DownloadView {
    pub(crate) fn new(task: &TaskSnapshot) -> Self {
        let r = &task.record;
        let parsed = url::Url::parse(&r.url).ok();
        let host = parsed
            .as_ref()
            .and_then(|u| u.host_str())
            .map(str::to_string);
        let name = r
            .file_name
            .clone()
            .or_else(|| parsed.as_ref().and_then(last_segment))
            .or_else(|| host.clone())
            .unwrap_or_default();
        let quality = r
            .site
            .as_ref()
            .and_then(|s| s.quality.clone())
            .or_else(|| r.media.as_ref().and_then(|m| m.quality.clone()));
        Self {
            id: r.id.clone(),
            name,
            host,
            state: r.state,
            total: r.total,
            downloaded: r.downloaded,
            speed: task.live.speed,
            eta_secs: task.eta_secs,
            error_kind: r.error_kind.clone(),
            waiting: task.live.waiting,
            queue: r.queue.clone(),
            category: r.category.clone(),
            awaiting_confirmation: task.live.awaiting_confirmation,
            quality,
            created_at: r.created_at,
            completed_at: r.completed_at,
        }
    }
}

/// The link's file name until the server names the file.
fn last_segment(url: &url::Url) -> Option<String> {
    let segment = url.path_segments()?.rev().find(|s| !s.is_empty())?;
    let decoded = percent_encoding::percent_decode_str(segment).decode_utf8_lossy();
    let name: String = decoded
        .chars()
        .filter(|c| !c.is_control())
        .take(200)
        .collect();
    (!name.is_empty()).then_some(name)
}

#[derive(Debug, Serialize)]
struct ListBody {
    downloads: Vec<DownloadView>,
}

async fn list(State(st): State<Arc<AppState>>) -> Json<ListBody> {
    let mut tasks = st.manager.list();
    // Newest first, the order a phone screen wants.
    tasks.sort_by_key(|t| std::cmp::Reverse((t.record.created_at, t.record.position)));
    Json(ListBody {
        downloads: tasks.iter().map(DownloadView::new).collect(),
    })
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AddRequest {
    url: String,
    /// A folder inside the download folder; empty or missing: automatic.
    #[serde(default)]
    folder: Option<String>,
    #[serde(default)]
    queue: Option<String>,
    /// Add without starting.
    #[serde(default)]
    paused: bool,
    /// The link is already in the list and the user wants it again.
    #[serde(default)]
    add_copy: bool,
}

#[derive(Debug, Serialize)]
struct DownloadBody {
    download: DownloadView,
}

async fn add(
    State(st): State<Arc<AppState>>,
    body: Result<Json<AddRequest>, JsonRejection>,
) -> Response {
    let Json(req) = match body {
        Ok(body) => body,
        Err(rejection) => return json_error(rejection),
    };
    let url = req.url.trim();
    if url.is_empty() || url.len() > MAX_URL {
        return error(StatusCode::BAD_REQUEST, "invalid_url");
    }
    let settings = st.manager.settings();
    let dir = match folder::resolve(&settings.download_dir, req.folder.as_deref().unwrap_or("")) {
        Ok(dir) => dir,
        Err(()) => return error(StatusCode::BAD_REQUEST, "invalid_folder"),
    };
    let queue = match req
        .queue
        .as_deref()
        .map(str::trim)
        .filter(|q| !q.is_empty())
    {
        None => None,
        Some(q) if settings.queue(q).is_some() => Some(q.to_string()),
        Some(_) => return error(StatusCode::BAD_REQUEST, "unknown_queue"),
    };
    let result = st.manager.add(AddOptions {
        url: url.to_string(),
        dir,
        queue,
        paused: req.paused,
        on_duplicate: if req.add_copy {
            OnDuplicate::AddCopy
        } else {
            OnDuplicate::Refuse
        },
        ..AddOptions::default()
    });
    match result {
        Ok(task) => {
            tracing::info!(id = %task.record.id, "remote control: added a download");
            (
                StatusCode::CREATED,
                Json(DownloadBody {
                    download: DownloadView::new(&task),
                }),
            )
                .into_response()
        }
        Err(err) => core_error(err),
    }
}

async fn pause(State(st): State<Arc<AppState>>, Path(id): Path<String>) -> Response {
    done(st.manager.pause(&id))
}

async fn resume(State(st): State<Arc<AppState>>, Path(id): Path<String>) -> Response {
    done(st.manager.resume(&id))
}

/// Takes the download off the list. What an unfinished download wrote is
/// deleted with it (it could never be resumed); a finished file is never
/// touched from here.
async fn remove(State(st): State<Arc<AppState>>, Path(id): Path<String>) -> Response {
    let Some(task) = st.manager.get(&id) else {
        return error(StatusCode::NOT_FOUND, "not_found");
    };
    let partial = task.record.state != TaskState::Completed;
    done(st.manager.remove(&id, partial))
}

fn done(result: Result<(), CoreError>) -> Response {
    match result {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => core_error(err),
    }
}

#[derive(Debug, Serialize)]
struct QueueView {
    id: String,
    name: String,
}

/// Settings the page may show. No folders, proxy, tool paths or anything
/// about the remote control itself.
#[derive(Debug, Serialize)]
struct SettingsView {
    version: &'static str,
    language: String,
    main_queue: &'static str,
    queues: Vec<QueueView>,
    max_concurrent_downloads: u32,
    max_connections_per_download: u32,
    speed_limit: u64,
    use_categories: bool,
}

async fn settings(State(st): State<Arc<AppState>>) -> Json<SettingsView> {
    let s = st.manager.settings();
    Json(SettingsView {
        version: env!("CARGO_PKG_VERSION"),
        language: s.language,
        main_queue: MAIN_QUEUE,
        queues: s
            .queues
            .into_iter()
            .map(|q| QueueView {
                id: q.id,
                name: q.name,
            })
            .collect(),
        max_concurrent_downloads: s.max_concurrent_downloads,
        max_connections_per_download: s.max_connections_per_download,
        speed_limit: s.speed_limit,
        use_categories: s.use_categories,
    })
}

async fn page(State(st): State<Arc<AppState>>) -> Response {
    match st.assets.get(PAGE) {
        Some(asset) => {
            let mut res = serve(asset, "no-cache");
            res.headers_mut().insert(
                header::CONTENT_SECURITY_POLICY,
                HeaderValue::from_static(PAGE_CSP),
            );
            res
        }
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            "This build of Downpipe has no remote control page; the API under /api/v1 works.",
        )
            .into_response(),
    }
}

async fn icon(State(st): State<Arc<AppState>>) -> Response {
    file(&st, "icon.svg", "public, max-age=86400")
}

async fn asset(State(st): State<Arc<AppState>>, Path(name): Path<String>) -> Response {
    // Built files carry a hash of their content in the name.
    file(
        &st,
        &format!("assets/{name}"),
        "public, max-age=31536000, immutable",
    )
}

fn file(st: &AppState, path: &str, cache: &'static str) -> Response {
    if !assets::is_servable(path) {
        return error(StatusCode::NOT_FOUND, "not_found");
    }
    match st.assets.get(path) {
        Some(asset) => serve(asset, cache),
        None => error(StatusCode::NOT_FOUND, "not_found"),
    }
}

fn serve(asset: crate::Asset, cache: &'static str) -> Response {
    let mime = HeaderValue::from_str(&asset.mime)
        .unwrap_or(HeaderValue::from_static("application/octet-stream"));
    (
        [
            (header::CONTENT_TYPE, mime),
            (header::CACHE_CONTROL, HeaderValue::from_static(cache)),
        ],
        asset.bytes,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (k, v) in pairs {
            map.insert(*k, HeaderValue::from_str(v).unwrap());
        }
        map
    }

    #[test]
    fn origin_must_be_this_server() {
        assert!(same_origin(&headers(&[("host", "192.168.1.5:9420")])));
        assert!(same_origin(&headers(&[
            ("host", "192.168.1.5:9420"),
            ("origin", "http://192.168.1.5:9420"),
        ])));
        assert!(!same_origin(&headers(&[
            ("host", "192.168.1.5:9420"),
            ("origin", "https://evil.example"),
        ])));
        assert!(!same_origin(&headers(&[(
            "origin",
            "http://192.168.1.5:9420"
        )])));
        assert!(!same_origin(&headers(&[
            ("host", "127.0.0.1:9420"),
            ("sec-fetch-site", "cross-site"),
        ])));
    }

    #[test]
    fn reads_bearer_tokens() {
        assert_eq!(
            bearer(&headers(&[("authorization", "Bearer abc ")])),
            Some("abc")
        );
        assert_eq!(
            bearer(&headers(&[("authorization", "bearer abc")])),
            Some("abc")
        );
        assert_eq!(bearer(&headers(&[("authorization", "Basic abc")])), None);
        assert_eq!(bearer(&headers(&[])), None);
    }

    #[test]
    fn names_come_from_the_file_or_the_link() {
        let url = url::Url::parse("https://cdn.example/files/My%20Movie.mkv?sig=secret").unwrap();
        assert_eq!(last_segment(&url).as_deref(), Some("My Movie.mkv"));
        let root = url::Url::parse("https://example.com/").unwrap();
        assert_eq!(last_segment(&root), None);
    }
}
