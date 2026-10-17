//! A stand-in for GitHub's release downloads, for the tool installer's tests.
//!
//! Under the folder given to [`crate::TestServer::serve_files`]:
//!
//! ```text
//! <owner>/<repo>/latest            the tag, as text
//! <owner>/<repo>/<tag>/<file>      the release's files
//! ```
//!
//! are served the way github.com lays them out:
//!
//! ```text
//! /releases/<owner>/<repo>/releases/latest          302 to .../releases/tag/<tag>
//! /releases/<owner>/<repo>/releases/tag/<tag>       200
//! /releases/<owner>/<repo>/releases/download/<tag>/<file>
//! ```

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};

use crate::ServerState;

pub(crate) async fn release(
    State(state): State<Arc<ServerState>>,
    Path(path): Path<String>,
    headers: HeaderMap,
) -> Response {
    let Some(root) = state.files.lock().clone() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let parts: Vec<&str> = path.split('/').collect();
    if parts
        .iter()
        .any(|p| p.is_empty() || *p == ".." || p.contains(['\\', ':']))
    {
        return StatusCode::NOT_FOUND.into_response();
    }
    let resp = match parts.as_slice() {
        [owner, repo, "releases", "latest"] => {
            match tokio::fs::read_to_string(root.join(owner).join(repo).join("latest")).await {
                Ok(tag) => {
                    let mut resp = StatusCode::FOUND.into_response();
                    let to = format!("/releases/{owner}/{repo}/releases/tag/{}", tag.trim());
                    if let Ok(v) = HeaderValue::from_str(&to) {
                        resp.headers_mut().insert(header::LOCATION, v);
                    }
                    resp
                }
                Err(_) => StatusCode::NOT_FOUND.into_response(),
            }
        }
        [_, _, "releases", "tag", _] => "release".into_response(),
        [owner, repo, "releases", "download", tag, file] => {
            match tokio::fs::read(root.join(owner).join(repo).join(tag).join(file)).await {
                Ok(bytes) => {
                    let mut resp = bytes.into_response();
                    resp.headers_mut().insert(
                        header::CONTENT_TYPE,
                        HeaderValue::from_static("application/octet-stream"),
                    );
                    resp
                }
                Err(_) => StatusCode::NOT_FOUND.into_response(),
            }
        }
        _ => StatusCode::NOT_FOUND.into_response(),
    };
    state.record(&format!("/releases/{path}"), &headers, resp.status());
    resp
}
