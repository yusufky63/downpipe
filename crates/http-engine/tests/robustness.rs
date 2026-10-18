//! What users of other download managers run into: hosts that cap
//! connections, silent connections, web pages where the file should be,
//! networks that drop for a minute, and awkward file names. Each test makes
//! the fault-injection server misbehave in one of these ways.

use std::path::Path;
use std::time::{Duration, Instant};

use downpipe_http::{
    DownloadError, DownloadHandle, DownloadRequest, Engine, EngineConfig, KIB, MIB, Outcome, Phase,
    Progress, WaitReason, meta,
};
use downpipe_test_server::{TestServer, content};
use tokio::task::JoinHandle;

fn config() -> EngineConfig {
    EngineConfig {
        max_connections: 4,
        initial_connections: 4,
        min_split_size: 64 * KIB,
        buffer_size: 32 * KIB as usize,
        stall_timeout: Duration::from_secs(5),
        response_timeout: Duration::from_secs(5),
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

/// Every distinct (phase, reason) the download goes through, in order.
fn record_phases(handle: &DownloadHandle) -> JoinHandle<Vec<(Phase, Option<WaitReason>)>> {
    let mut rx = handle.progress();
    tokio::spawn(async move {
        let mut seen: Vec<(Phase, Option<WaitReason>)> = Vec::new();
        loop {
            let now = {
                let p = rx.borrow_and_update();
                (p.phase, p.waiting)
            };
            if seen.last() != Some(&now) {
                seen.push(now);
            }
            if rx.changed().await.is_err() {
                return seen;
            }
        }
    })
}

fn completed_path(outcome: Outcome) -> std::path::PathBuf {
    match outcome {
        Outcome::Completed { path, .. } => path,
        other => panic!("expected completion, got {other:?}"),
    }
}

fn names_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

async fn pause_after(handle: DownloadHandle, bytes: u64) -> std::path::PathBuf {
    wait_for(&handle, |p| p.downloaded >= bytes).await;
    handle.pause();
    match handle.wait().await.unwrap() {
        Outcome::Paused {
            path: Some(path),
            resumable: true,
            ..
        } => path,
        other => panic!("expected a resumable pause, got {other:?}"),
    }
}

// ------------------------------------------------- hosts that cap connections

#[tokio::test]
async fn refused_extra_connections_drop_to_what_the_host_allows() {
    for status in [403, 429, 503] {
        let server = TestServer::start().await.unwrap();
        let dir = tempfile::tempdir().unwrap();
        let size = 6 * MIB;
        let url = server.url(&format!(
            "/file/{size}?max_conns=2&limit_status={status}&rate=4194304"
        ));
        let engine = engine(config());

        let handle = engine.start(DownloadRequest::new(&url, dir.path()));
        let rx = handle.progress();
        let path = completed_path(handle.wait().await.unwrap());
        assert_eq!(std::fs::read(&path).unwrap(), content(size, 0), "{status}");
        assert!(server.state.refused() > 0, "{status}: the limit was hit");
        assert!(
            rx.borrow()
                .notices
                .iter()
                .any(|n| n.contains("allows only 2")),
            "{status}: {:?}",
            rx.borrow().notices
        );
        assert_eq!(engine.connection_limit(&url), Some(2), "{status}");

        // A later download from the same host starts within the limit.
        server.state.reset_stats();
        let handle = engine.start(DownloadRequest::new(
            server.url(&format!(
                "/file/{size}?max_conns=2&limit_status={status}&rate=4194304&again=1"
            )),
            dir.path(),
        ));
        let path = completed_path(handle.wait().await.unwrap());
        assert_eq!(std::fs::read(&path).unwrap(), content(size, 0));
        assert_eq!(
            server.state.refused(),
            0,
            "{status}: no refusals the second time"
        );
        assert!(server.state.max_concurrent() <= 2);
    }
}

#[tokio::test]
async fn a_link_that_expires_mid_download_still_fails() {
    // Every request after the first is refused: not a connection limit the
    // download can live with once the first connection's part is done.
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cfg = EngineConfig {
        max_retries: 2,
        ..config()
    };
    let err = engine(cfg)
        .start(DownloadRequest::new(
            server.url(&format!(
                "/file/{}?fail=403&fail_from=1&rate=1048576",
                2 * MIB
            )),
            dir.path(),
        ))
        .wait()
        .await
        .unwrap_err();
    assert_eq!(err.kind(), "forbidden", "{err:?}");
}

// ---------------------------------------------------------------- stalls

#[tokio::test]
async fn a_silent_connection_shows_as_waiting_and_is_replaced() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let size = 2 * MIB;
    let cfg = EngineConfig {
        max_connections: 1,
        initial_connections: 1,
        stall_timeout: Duration::from_millis(1500),
        ..config()
    };

    let handle = engine(cfg).start(DownloadRequest::new(
        server.url(&format!(
            "/file/{size}?stall_at={MIB}&stall_times=1&rate=4194304"
        )),
        dir.path(),
    ));
    let phases = record_phases(&handle);
    let path = completed_path(handle.wait().await.unwrap());
    assert_eq!(std::fs::read(&path).unwrap(), content(size, 0));
    let phases = phases.await.unwrap();
    assert!(
        phases.contains(&(Phase::Waiting, Some(WaitReason::NoData))),
        "{phases:?}"
    );
    // Waiting always says why.
    assert!(
        phases
            .iter()
            .all(|(phase, why)| (*phase == Phase::Waiting) == why.is_some()),
        "{phases:?}"
    );
}

#[tokio::test]
async fn a_server_that_never_answers_is_asked_again() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let size = 2 * MIB;
    let cfg = EngineConfig {
        response_timeout: Duration::from_millis(700),
        ..config()
    };

    // The first request and two connections after it get no answer at all.
    let started = Instant::now();
    let handle = engine(cfg).start(DownloadRequest::new(
        server.url(&format!("/file/{size}?hang_times=3&rate=2097152")),
        dir.path(),
    ));
    let path = completed_path(handle.wait().await.unwrap());
    assert!(started.elapsed() >= Duration::from_millis(700));
    assert_eq!(std::fs::read(&path).unwrap(), content(size, 0));
}

// ------------------------------------------------ web pages instead of files

#[tokio::test]
async fn a_web_page_where_a_file_is_expected_is_refused() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();

    let err = engine(config())
        .start(DownloadRequest::new(
            server.url("/file/1000/archive.zip?html=1"),
            dir.path(),
        ))
        .wait()
        .await
        .unwrap_err();
    assert!(matches!(err, DownloadError::ExpiredLink), "{err:?}");
    assert_eq!(err.kind(), "expired_link");
    assert!(names_in(dir.path()).is_empty(), "nothing saved");

    // A sign-in page, for a download the browser named.
    let mut req = DownloadRequest::new(server.url("/redirect?to=/login"), dir.path());
    req.file_name = Some("report.pdf".into());
    let err = engine(config()).start(req).wait().await.unwrap_err();
    assert!(matches!(err, DownloadError::LoginRequired), "{err:?}");
    assert_eq!(err.kind(), "login_required");
    assert!(names_in(dir.path()).is_empty());
}

#[tokio::test]
async fn web_pages_can_still_be_downloaded_on_purpose() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let handle = engine(config()).start(DownloadRequest::new(
        server.url("/file/100/page.html?html=1"),
        dir.path(),
    ));
    let path = completed_path(handle.wait().await.unwrap());
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("Link expired"), "{text}");
}

#[tokio::test]
async fn a_resume_that_gets_a_web_page_keeps_the_partial_data() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let size = 8 * MIB;
    // One connection, so the first session makes exactly one request; every
    // request after it gets the "link expired" page.
    let cfg = EngineConfig {
        max_connections: 1,
        initial_connections: 1,
        ..config()
    };
    let url = server.url(&format!("/file/{size}?rate=1048576&html_after=1"));

    let path = pause_after(
        engine(cfg.clone()).start(DownloadRequest::new(&url, dir.path())),
        MIB,
    )
    .await;
    let kept = meta::load(&meta::meta_path(&path))
        .unwrap()
        .completed_bytes();
    assert!(kept > 0);

    let err = engine(cfg.clone())
        .start(DownloadRequest::new(&url, dir.path()))
        .wait()
        .await
        .unwrap_err();
    assert_eq!(err.kind(), "expired_link", "{err:?}");
    assert!(meta::part_path(&path).exists(), "partial file kept");
    assert_eq!(
        meta::load(&meta::meta_path(&path))
            .unwrap()
            .completed_bytes(),
        kept,
        "resume data untouched"
    );

    // A "new link" that leads to a sign-in page keeps it too.
    let mut req = DownloadRequest::new(server.url("/redirect?to=/login"), dir.path());
    req.file_name = Some(path.file_name().unwrap().to_string_lossy().into_owned());
    req.relink = true;
    let err = engine(cfg.clone()).start(req).wait().await.unwrap_err();
    assert_eq!(err.kind(), "login_required", "{err:?}");
    assert!(meta::part_path(&path).exists());

    // And a working link finishes the same file.
    let mut req = DownloadRequest::new(server.url(&format!("/file/{size}")), dir.path());
    req.file_name = Some(path.file_name().unwrap().to_string_lossy().into_owned());
    req.relink = true;
    let done = completed_path(engine(cfg).start(req).wait().await.unwrap());
    assert_eq!(done, path);
    assert_eq!(std::fs::read(&done).unwrap(), content(size, 0));
}

// --------------------------------------------------------- network drops

#[tokio::test]
async fn downloads_carry_on_after_the_network_comes_back() {
    let mut server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let size = 8 * MIB;
    // Two retries per segment: far fewer than the failures of a two-second
    // outage, which only the time-based budget survives.
    let cfg = EngineConfig {
        max_connections: 2,
        initial_connections: 2,
        max_retries: 2,
        offline_grace: Duration::from_secs(30),
        ..config()
    };

    let handle = engine(cfg).start(DownloadRequest::new(
        server.url(&format!("/file/{size}?rate=2097152")),
        dir.path(),
    ));
    let phases = record_phases(&handle);
    wait_for(&handle, |p| p.downloaded >= MIB).await;
    server.stop().await;
    wait_for(&handle, |p| p.waiting == Some(WaitReason::Network)).await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(!handle.is_finished(), "still waiting for the network");
    server.restart().await.unwrap();

    let path = completed_path(handle.wait().await.unwrap());
    assert_eq!(std::fs::read(&path).unwrap(), content(size, 0));
    let phases = phases.await.unwrap();
    let back = phases.iter().rposition(|p| p.0 == Phase::Waiting).unwrap();
    assert!(
        phases[back..].iter().any(|p| p.0 == Phase::Downloading),
        "downloading again after the outage: {phases:?}"
    );
}

#[tokio::test]
async fn a_resume_waits_for_the_network() {
    let mut server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let size = 4 * MIB;
    let cfg = EngineConfig {
        max_retries: 1,
        offline_grace: Duration::from_secs(30),
        ..config()
    };
    let url = server.url(&format!("/file/{size}?rate=1048576"));
    pause_after(
        engine(cfg.clone()).start(DownloadRequest::new(&url, dir.path())),
        MIB,
    )
    .await;

    server.stop().await;
    let handle = engine(cfg).start(DownloadRequest::new(&url, dir.path()));
    wait_for(&handle, |p| p.waiting == Some(WaitReason::Network)).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    server.restart().await.unwrap();
    let path = completed_path(handle.wait().await.unwrap());
    assert_eq!(std::fs::read(&path).unwrap(), content(size, 0));
}

#[tokio::test]
async fn an_outage_longer_than_the_grace_fails_as_network() {
    let mut server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cfg = EngineConfig {
        max_retries: 2,
        offline_grace: Duration::from_secs(1),
        ..config()
    };
    let handle = engine(cfg).start(DownloadRequest::new(
        server.url(&format!("/file/{}?rate=1048576", 8 * MIB)),
        dir.path(),
    ));
    wait_for(&handle, |p| p.downloaded >= MIB).await;
    server.stop().await;
    let err = tokio::time::timeout(Duration::from_secs(20), handle.wait())
        .await
        .expect("gives up once the grace is over")
        .unwrap_err();
    assert_eq!(err.kind(), "network", "{err:?}");
    // What arrived before the outage is kept for a later resume.
    let names = names_in(dir.path());
    assert!(names.iter().any(|n| n.ends_with(".part.meta")), "{names:?}");
}

// ---------------------------------------------------------------- names

#[tokio::test]
async fn raw_utf8_content_disposition_names_the_file() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    // `attachment; filename="özet: 2024?.pdf"` as raw UTF-8 bytes.
    let cd = "attachment%3B%20filename%3D%22%C3%B6zet%3A%202024%3F.pdf%22";
    let handle = engine(config()).start(DownloadRequest::new(
        server.url(&format!("/file/100?cd={cd}")),
        dir.path(),
    ));
    let path = completed_path(handle.wait().await.unwrap());
    assert_eq!(
        path.file_name().unwrap().to_string_lossy(),
        "özet_ 2024_.pdf"
    );
}

#[tokio::test]
async fn a_failed_single_connection_download_leaves_no_name_taken() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cfg = EngineConfig {
        max_retries: 1,
        ..config()
    };
    // No ranges, and every response dies early.
    let err = engine(cfg.clone())
        .start(DownloadRequest::new(
            server.url(&format!("/file/{MIB}/data.bin?ranges=0&drop_after=100000")),
            dir.path(),
        ))
        .wait()
        .await
        .unwrap_err();
    assert_eq!(err.kind(), "network", "{err:?}");
    assert!(
        names_in(dir.path()).is_empty(),
        "{:?}",
        names_in(dir.path())
    );

    // Retried under the name it had, it gets that name, not "data (1).bin".
    let mut req = DownloadRequest::new(
        server.url(&format!("/file/{MIB}/data.bin?ranges=0")),
        dir.path(),
    );
    req.file_name = Some("data.bin".into());
    let path = completed_path(engine(cfg).start(req).wait().await.unwrap());
    assert_eq!(names_in(dir.path()), vec!["data.bin".to_string()]);
    assert_eq!(std::fs::read(&path).unwrap(), content(MIB, 0));
}
