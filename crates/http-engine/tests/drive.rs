//! Google Drive's answers, mimicked on loopback: the virus-scan warning page
//! and its form, a public file, a private one and the quota page.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use axum::Router;
use axum::extract::{Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use downpipe_http::hosts::drive;
use downpipe_http::{DownloadError, DownloadRequest, Engine, EngineConfig, Outcome};
use reqwest::header::HeaderMap;
use url::Url;

const SIZE: usize = 300_000;

#[derive(Default)]
struct Drive {
    port: u16,
    confirmed: AtomicU32,
}

fn bytes() -> Vec<u8> {
    (0..SIZE).map(|i| (i % 251) as u8).collect()
}

async fn download(
    State(d): State<Arc<Drive>>,
    Query(q): Query<Vec<(String, String)>>,
    headers: axum::http::HeaderMap,
) -> Response {
    let get = |k: &str| q.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
    let html = |status: StatusCode, body: String| {
        (
            status,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            body,
        )
            .into_response()
    };
    match get("id") {
        Some("big") if get("confirm") == Some("t") && get("uuid") == Some("u-123") => {
            d.confirmed.fetch_add(1, Ordering::SeqCst);
            let data = bytes();
            // Honour ranges like Drive does, so the engine can segment.
            if let Some(range) = headers.get(header::RANGE).and_then(|v| v.to_str().ok()) {
                let spec = range.trim_start_matches("bytes=");
                let (a, b) = spec.split_once('-').unwrap();
                let a: usize = a.parse().unwrap();
                let b: usize = if b.is_empty() { SIZE - 1 } else { b.parse::<usize>().unwrap().min(SIZE - 1) };
                return (
                    StatusCode::PARTIAL_CONTENT,
                    [
                        (header::CONTENT_TYPE, "application/octet-stream".to_string()),
                        (header::CONTENT_RANGE, format!("bytes {a}-{b}/{SIZE}")),
                        (header::CONTENT_DISPOSITION, "attachment; filename=\"big video.mp4\"".to_string()),
                        (header::ACCEPT_RANGES, "bytes".to_string()),
                    ],
                    data[a..=b].to_vec(),
                )
                    .into_response();
            }
            ([(header::CONTENT_TYPE, "application/octet-stream")], data).into_response()
        }
        Some("big") => html(
            StatusCode::OK,
            format!(
                r#"<html><body><p>Google Drive can't scan this file for viruses.</p>
                <form id="download-form" action="http://127.0.0.1:{}/download" method="get">
                <input type="submit" value="Download anyway"/>
                <input type="hidden" name="id" value="big"><input type="hidden" name="export" value="download">
                <input type="hidden" name="confirm" value="t"><input type="hidden" name="uuid" value="u-123"></form></body></html>"#,
                d.port
            ),
        ),
        Some("small") => ([(header::CONTENT_TYPE, "application/pdf")], b"%PDF-1.7".to_vec()).into_response(),
        Some("private") => Redirect::temporary("/ServiceLogin?continue=x").into_response(),
        Some("access") => html(StatusCode::FORBIDDEN, "<html><body>You need access. Request access</body></html>".into()),
        Some("quota") => html(
            StatusCode::OK,
            "<html><body>Sorry, you can't view or download this file at this time. Too many users have viewed or downloaded this file recently.</body></html>".into(),
        ),
        _ => html(StatusCode::NOT_FOUND, "<html><body>Not found</body></html>".into()),
    }
}

async fn sign_in() -> Response {
    (
        [(header::CONTENT_TYPE, "text/html")],
        "<html><body>Sign in</body></html>",
    )
        .into_response()
}

async fn serve() -> (Arc<Drive>, String) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let state = Arc::new(Drive {
        port,
        ..Drive::default()
    });
    let app = Router::new()
        .route("/download", axum::routing::get(download))
        .route("/ServiceLogin", axum::routing::get(sign_in))
        .with_state(state.clone());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (state, format!("http://127.0.0.1:{port}"))
}

fn client() -> reqwest::Client {
    reqwest::Client::builder().no_proxy().build().unwrap()
}

fn page(base: &str, id: &str) -> Url {
    Url::parse(&format!("{base}/download?id={id}&export=download")).unwrap()
}

#[tokio::test]
async fn follows_the_virus_scan_warning_and_downloads_the_file() {
    let (state, base) = serve().await;
    let confirmed = drive::follow(&client(), &page(&base, "big"), &HeaderMap::new())
        .await
        .unwrap();
    assert_eq!(
        confirmed.as_str(),
        format!("{base}/download?id=big&export=download&confirm=t&uuid=u-123")
    );

    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::new(EngineConfig {
        proxy: None,
        ..EngineConfig::default()
    })
    .unwrap();
    let outcome = engine
        .start(DownloadRequest::new(confirmed.as_str(), dir.path()))
        .wait()
        .await
        .unwrap();
    let Outcome::Completed { path, .. } = outcome else {
        panic!("{outcome:?}")
    };
    assert_eq!(path.file_name().unwrap(), "big video.mp4");
    assert_eq!(std::fs::read(&path).unwrap(), bytes());
    assert!(state.confirmed.load(Ordering::SeqCst) >= 1);
}

#[tokio::test]
async fn a_file_answer_is_kept_as_is() {
    let (_state, base) = serve().await;
    let start = page(&base, "small");
    assert_eq!(
        drive::follow(&client(), &start, &HeaderMap::new())
            .await
            .unwrap(),
        start
    );
}

#[tokio::test]
async fn private_files_and_quotas_say_why() {
    let (_state, base) = serve().await;
    let follow = |id: &'static str| {
        let start = page(&base, id);
        async move { drive::follow(&client(), &start, &HeaderMap::new()).await }
    };
    let err = follow("private").await.unwrap_err();
    assert_eq!(err.kind(), "private_file");
    assert!(err.to_string().contains("not shared publicly"), "{err}");
    assert_eq!(follow("access").await.unwrap_err().kind(), "private_file");
    let quota = follow("quota").await.unwrap_err();
    assert!(matches!(quota, DownloadError::Unavailable(_)));
    assert!(quota.to_string().contains("quota"));
    assert_eq!(follow("gone").await.unwrap_err().kind(), "not_found");
}
