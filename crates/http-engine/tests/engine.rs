//! End-to-end tests against the fault-injection server.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use downpipe_http::{
    Checksum, DownloadError, DownloadHandle, DownloadRequest, Engine, EngineConfig, KIB, MIB,
    Outcome, Progress, RateLimiter, meta,
};
use downpipe_test_server::{TestServer, content};

fn config() -> EngineConfig {
    EngineConfig {
        max_connections: 4,
        initial_connections: 4,
        min_split_size: 64 * KIB,
        buffer_size: 32 * KIB as usize,
        stall_timeout: Duration::from_secs(5),
        checkpoint_interval: Duration::from_millis(100),
        ramp_window: Duration::from_millis(300),
        retry_base_delay: Duration::from_millis(50),
        retry_max_delay: Duration::from_millis(300),
        ..EngineConfig::default()
    }
}

fn engine(cfg: EngineConfig) -> Engine {
    Engine::new(cfg).expect("engine")
}

async fn wait_for(handle: &DownloadHandle, cond: impl Fn(&Progress) -> bool) {
    let mut rx = handle.progress();
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            if cond(&rx.borrow_and_update()) {
                return;
            }
            if rx.changed().await.is_err() {
                return;
            }
        }
    })
    .await
    .expect("condition not reached in time");
}

fn completed_path(outcome: Outcome) -> std::path::PathBuf {
    match outcome {
        Outcome::Completed { path, .. } => path,
        other => panic!("expected completion, got {other:?}"),
    }
}

fn assert_only_file(dir: &Path, name: &str) {
    let names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec![name.to_string()], "leftover files: {names:?}");
}

#[tokio::test]
async fn segmented_download_matches_source() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let size = 8 * MIB;

    let handle = engine(config()).start(DownloadRequest::new(
        server.url(&format!("/file/{size}")),
        dir.path(),
    ));
    let path = completed_path(handle.wait().await.unwrap());

    assert_eq!(std::fs::read(&path).unwrap(), content(size, 0));
    assert_only_file(dir.path(), &size.to_string());
    assert!(
        server.state.max_concurrent() > 1,
        "expected parallel connections"
    );
    let first = &server.state.requests()[0];
    assert_eq!(first.range.as_deref(), Some("bytes=0-"));
    assert_eq!(first.status, 206);
}

#[tokio::test]
async fn server_without_ranges_uses_one_connection() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let size = 3 * MIB;

    let handle = engine(config()).start(DownloadRequest::new(
        server.url(&format!("/file/{size}?ranges=0")),
        dir.path(),
    ));
    let rx = handle.progress();
    let path = completed_path(handle.wait().await.unwrap());

    assert_eq!(std::fs::read(&path).unwrap(), content(size, 0));
    assert_eq!(server.state.max_concurrent(), 1);
    assert!(
        rx.borrow()
            .notices
            .iter()
            .any(|n| n.contains("byte ranges")),
        "{:?}",
        rx.borrow().notices
    );
}

#[tokio::test]
async fn pause_then_resume_in_a_new_engine_continues() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let size = 8 * MIB;
    // 2 MiB/s per connection keeps the download running long enough to pause.
    let url = server.url(&format!("/file/{size}?rate=2097152"));
    let cfg = EngineConfig {
        max_connections: 2,
        initial_connections: 2,
        ..config()
    };

    let handle = engine(cfg.clone()).start(DownloadRequest::new(&url, dir.path()));
    wait_for(&handle, |p| p.downloaded >= 2 * MIB).await;
    handle.pause();
    let paused = handle.wait().await.unwrap();
    let Outcome::Paused {
        path: Some(final_path),
        downloaded,
        resumable: true,
        ..
    } = paused
    else {
        panic!("expected a resumable pause, got {paused:?}");
    };
    assert!(downloaded >= 2 * MIB && downloaded < size);

    // What a restarted program finds on disk.
    let saved = meta::load(&meta::meta_path(&final_path)).unwrap();
    assert!(meta::part_path(&final_path).exists());
    assert!(saved.completed_bytes() > 0);
    assert_eq!(saved.total_size, size);
    assert!(saved.etag.is_some());

    server.state.reset_stats();
    let handle = engine(cfg).start(DownloadRequest::new(&url, dir.path()));
    let path = completed_path(handle.wait().await.unwrap());

    assert_eq!(path, final_path);
    assert_eq!(std::fs::read(&path).unwrap(), content(size, 0));
    assert_only_file(dir.path(), &size.to_string());

    let first = &server.state.requests()[0];
    assert_eq!(first.if_range.as_deref(), saved.etag.as_deref());
    assert_ne!(first.range.as_deref(), Some("bytes=0-"));
    let served = server.state.bytes_served();
    assert!(
        served <= size - saved.completed_bytes() + 64 * KIB,
        "re-downloaded too much: {served} of {size}"
    );
}

#[tokio::test]
async fn resume_detects_a_changed_file_and_starts_over() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let size = 8 * MIB;
    let url = server.url(&format!("/file/{size}?rate=1048576"));

    let handle = engine(config()).start(DownloadRequest::new(&url, dir.path()));
    wait_for(&handle, |p| p.downloaded >= MIB).await;
    handle.pause();
    assert!(matches!(
        handle.wait().await.unwrap(),
        Outcome::Paused {
            resumable: true,
            ..
        }
    ));

    server.state.set_version(7);
    let handle = engine(config()).start(DownloadRequest::new(&url, dir.path()));
    let rx = handle.progress();
    let path = completed_path(handle.wait().await.unwrap());

    assert_eq!(std::fs::read(&path).unwrap(), content(size, 7));
    assert!(
        rx.borrow()
            .notices
            .iter()
            .any(|n| n.contains("starting over")),
        "{:?}",
        rx.borrow().notices
    );
    assert_only_file(dir.path(), &size.to_string());
}

#[tokio::test]
async fn dropped_connections_are_retried() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let size = 4 * MIB;

    let handle = engine(config()).start(DownloadRequest::new(
        server.url(&format!("/file/{size}?drop_after=300000&drop_times=6")),
        dir.path(),
    ));
    let path = completed_path(handle.wait().await.unwrap());
    assert_eq!(std::fs::read(&path).unwrap(), content(size, 0));
}

#[tokio::test]
async fn retry_after_is_honoured() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let size = 2 * MIB;

    let started = Instant::now();
    let handle = engine(config()).start(DownloadRequest::new(
        server.url(&format!("/file/{size}?fail=503&fail_times=1&retry_after=1")),
        dir.path(),
    ));
    let path = completed_path(handle.wait().await.unwrap());
    assert!(started.elapsed() >= Duration::from_millis(900));
    assert_eq!(std::fs::read(&path).unwrap(), content(size, 0));
    assert_eq!(server.state.requests()[0].status, 503);
}

#[tokio::test]
async fn hard_errors_fail_fast() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();

    let err = engine(config())
        .start(DownloadRequest::new(
            server.url("/file/100?fail=404"),
            dir.path(),
        ))
        .wait()
        .await
        .unwrap_err();
    assert!(
        matches!(err, DownloadError::NotFound { status: 404 }),
        "{err:?}"
    );

    let err = engine(config())
        .start(DownloadRequest::new(
            server.url("/file/100?fail=403"),
            dir.path(),
        ))
        .wait()
        .await
        .unwrap_err();
    assert!(matches!(err, DownloadError::Forbidden), "{err:?}");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn basic_auth_from_url() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = "/file/1000?auth=user:pass";

    let err = engine(config())
        .start(DownloadRequest::new(server.url(path), dir.path()))
        .wait()
        .await
        .unwrap_err();
    assert!(matches!(err, DownloadError::AuthRequired { status: 401 }));

    let url = format!("http://user:pass@{}{path}", server.addr);
    let done = engine(config())
        .start(DownloadRequest::new(url, dir.path()))
        .wait()
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(completed_path(done)).unwrap(),
        content(1000, 0)
    );
}

#[tokio::test]
async fn content_disposition_names_are_sanitized() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();

    let handle = engine(config()).start(DownloadRequest::new(
        server.url("/file/5000?name=..%2F..%2Fr%C3%A9sum%C3%A9%3F.pdf"),
        dir.path(),
    ));
    let path = completed_path(handle.wait().await.unwrap());
    assert_eq!(path, dir.path().join("résumé_.pdf"));
}

#[tokio::test]
async fn existing_files_are_not_overwritten() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("1000"), b"keep me").unwrap();

    let handle = engine(config()).start(DownloadRequest::new(server.url("/file/1000"), dir.path()));
    let path = completed_path(handle.wait().await.unwrap());
    assert_eq!(path, dir.path().join("1000 (1)"));
    assert_eq!(std::fs::read(dir.path().join("1000")).unwrap(), b"keep me");
}

#[tokio::test]
async fn redirects_are_followed() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let size = 2 * MIB;

    let handle = engine(config()).start(DownloadRequest::new(
        server.url(&format!("/redirect?to=/file/{size}")),
        dir.path(),
    ));
    let path = completed_path(handle.wait().await.unwrap());
    assert_eq!(std::fs::read(&path).unwrap(), content(size, 0));
    // Follow-up range requests go straight to the redirect target.
    assert!(
        server
            .state
            .requests()
            .iter()
            .skip(1)
            .all(|r| r.path.starts_with("/file/"))
    );
}

#[tokio::test]
async fn unknown_length_streams_to_the_end() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let size = MIB + 123;

    let handle = engine(config()).start(DownloadRequest::new(
        server.url(&format!("/file/{size}?chunked=1&ranges=0")),
        dir.path(),
    ));
    let path = completed_path(handle.wait().await.unwrap());
    assert_eq!(std::fs::read(&path).unwrap(), content(size, 0));
}

#[tokio::test]
async fn empty_files_complete() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let handle = engine(config()).start(DownloadRequest::new(server.url("/file/0"), dir.path()));
    let path = completed_path(handle.wait().await.unwrap());
    assert_eq!(std::fs::metadata(path).unwrap().len(), 0);
}

#[tokio::test]
async fn checksums_are_verified() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let size = 3 * MIB;
    let url = server.url(&format!("/file/{size}"));

    let reference = dir.path().join("reference.bin");
    std::fs::write(&reference, content(size, 0)).unwrap();
    let hex = downpipe_http::hash_file(&reference, downpipe_http::HashAlgo::Sha256).unwrap();
    std::fs::remove_file(&reference).unwrap();

    let mut req = DownloadRequest::new(&url, dir.path());
    req.checksum = Some(format!("sha256:{hex}").parse::<Checksum>().unwrap());
    let outcome = engine(config()).start(req).wait().await.unwrap();
    assert!(matches!(
        outcome,
        Outcome::Completed {
            verified: Some(_),
            ..
        }
    ));

    let wrong = dir.path().join("wrong");
    std::fs::create_dir(&wrong).unwrap();
    let mut req = DownloadRequest::new(&url, &wrong);
    req.checksum = Some(format!("sha256:{}", "0".repeat(64)).parse().unwrap());
    let err = engine(config()).start(req).wait().await.unwrap_err();
    assert!(
        matches!(err, DownloadError::ChecksumMismatch { .. }),
        "{err:?}"
    );
    // The data is kept for inspection, never published under the real name.
    assert!(!wrong.join(size.to_string()).exists());
    assert!(meta::part_path(&wrong.join(size.to_string())).exists());
}

#[tokio::test]
async fn idle_connections_steal_from_a_slow_segment() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let size = 8 * MIB;
    // The last quarter trickles at 1 MiB/s; everything before it is instant.
    let url = server.url(&format!(
        "/file/{size}?slow_from={}&slow_rate=1048576",
        6 * MIB
    ));

    let handle = engine(config()).start(DownloadRequest::new(url, dir.path()));
    let path = completed_path(handle.wait().await.unwrap());
    assert_eq!(std::fs::read(&path).unwrap(), content(size, 0));
    let requests = server.state.requests().len();
    assert!(
        requests > 4,
        "expected splits beyond the initial 4 segments, saw {requests} requests"
    );
}

#[tokio::test]
async fn connections_ramp_up_when_it_helps() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let size = 8 * MIB;
    // Per-connection cap: two connections are twice as fast as one.
    let url = server.url(&format!("/file/{size}?rate=1048576"));
    let cfg = EngineConfig {
        initial_connections: 1,
        max_connections: 4,
        ..config()
    };

    let handle = engine(cfg).start(DownloadRequest::new(url, dir.path()));
    wait_for(&handle, |p| p.connections >= 4).await;
    // Let the newest connections reach the server before stopping.
    tokio::time::sleep(Duration::from_millis(300)).await;
    handle.pause();
    handle.wait().await.unwrap();
    assert!(
        server.state.max_concurrent() >= 3,
        "{}",
        server.state.max_concurrent()
    );
}

#[tokio::test]
async fn cancel_removes_partial_data() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let url = server.url(&format!("/file/{}?rate=1048576", 8 * MIB));

    let handle = engine(config()).start(DownloadRequest::new(url, dir.path()));
    wait_for(&handle, |p| p.downloaded > 256 * KIB).await;
    handle.cancel();
    assert_eq!(handle.wait().await.unwrap(), Outcome::Canceled);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn pausing_a_non_resumable_download_discards_it() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let url = server.url(&format!("/file/{}?rate=1048576&ranges=0", 4 * MIB));

    let handle = engine(config()).start(DownloadRequest::new(url, dir.path()));
    wait_for(&handle, |p| p.downloaded > 128 * KIB).await;
    handle.pause();
    let outcome = handle.wait().await.unwrap();
    assert!(
        matches!(
            outcome,
            Outcome::Paused {
                resumable: false,
                ..
            }
        ),
        "{outcome:?}"
    );
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn global_speed_limit_applies() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let limiter = Arc::new(RateLimiter::new(2 * MIB));
    let engine = Engine::with_limiter(config(), limiter).unwrap();

    let started = Instant::now();
    let handle = engine.start(DownloadRequest::new(
        server.url(&format!("/file/{}", 2 * MIB)),
        dir.path(),
    ));
    completed_path(handle.wait().await.unwrap());
    let elapsed = started.elapsed();
    assert!(
        elapsed >= Duration::from_millis(800),
        "finished too fast: {elapsed:?}"
    );
}

#[tokio::test]
async fn probe_reports_server_capabilities() {
    let server = TestServer::start().await.unwrap();
    let engine = engine(config());

    let info = engine
        .probe(&server.url("/file/12345?name=a.zip"), &[])
        .await
        .unwrap();
    assert_eq!(info.total_size, Some(12345));
    assert!(info.accepts_ranges && info.is_resumable());
    assert_eq!(info.file_name, "a.zip");
    assert!(info.etag.is_some());

    let info = engine
        .probe(&server.url("/file/777?ranges=0"), &[])
        .await
        .unwrap();
    assert_eq!(info.total_size, Some(777));
    assert!(!info.accepts_ranges);
}
