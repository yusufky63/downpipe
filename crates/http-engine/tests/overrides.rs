//! Per-download overrides a site rule sets: retries, a site speed limit, and engines derived for another proxy that keep what the first one
//! learned about hosts.

use std::sync::Arc;
use std::time::{Duration, Instant};

use downpipe_http::{
    DownloadError, DownloadRequest, Engine, EngineConfig, KIB, MIB, Outcome, RateLimiter,
};
use downpipe_test_server::{TestServer, content};

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
        retry_base_delay: Duration::from_millis(20),
        retry_max_delay: Duration::from_millis(100),
        ..EngineConfig::default()
    }
}

#[tokio::test]
async fn a_download_can_retry_less_than_the_engine() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::new(config()).unwrap();

    let url = server.url("/file/65536?fail=503&fail_times=2");
    let mut req = DownloadRequest::new(&url, dir.path());
    req.max_retries = Some(0);
    let err = engine.start(req).wait().await.unwrap_err();
    assert!(
        matches!(
            err,
            DownloadError::RetriesExhausted { attempts: 1, .. } | DownloadError::HttpStatus { .. }
        ),
        "{err:?}"
    );

    // The engine's own count rides out the same two failures.
    let server = TestServer::start().await.unwrap();
    let url = server.url("/file/65536?fail=503&fail_times=2");
    let outcome = engine
        .start(DownloadRequest::new(&url, dir.path()))
        .wait()
        .await
        .unwrap();
    assert!(matches!(outcome, Outcome::Completed { .. }), "{outcome:?}");
}

#[tokio::test]
async fn a_site_limiter_holds_it_below_the_engine() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::new(config()).unwrap();
    let size = 512 * KIB;

    let mut req = DownloadRequest::new(server.url(&format!("/file/{size}")), dir.path());
    req.site_limiter = Some(Arc::new(RateLimiter::new(MIB)));
    let start = Instant::now();
    let outcome = engine.start(req).wait().await.unwrap();
    let elapsed = start.elapsed();
    let Outcome::Completed { path, .. } = outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(std::fs::read(path).unwrap(), content(size, 0));
    // Half a MiB at 1 MiB/s, minus the quarter second of burst.
    assert!(elapsed >= Duration::from_millis(200), "{elapsed:?}");
}

#[tokio::test]
async fn derived_engines_share_what_hosts_refused() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let size = 4 * MIB;
    let url = server.url(&format!(
        "/file/{size}?max_conns=2&limit_status=503&rate=4194304"
    ));
    let engine = Engine::new(config()).unwrap();
    let direct = engine
        .derive(EngineConfig {
            no_proxy: true,
            ..config()
        })
        .unwrap();
    assert!(direct.connection_limits().is_empty());

    let outcome = engine
        .start(DownloadRequest::new(&url, dir.path()))
        .wait()
        .await
        .unwrap();
    assert!(matches!(outcome, Outcome::Completed { .. }), "{outcome:?}");

    assert_eq!(direct.connection_limit(&url), Some(2));
    let port = url::Url::parse(&url).unwrap().port().unwrap();
    assert_eq!(
        direct.connection_limits(),
        [("127.0.0.1".to_string(), port, 2)]
    );
}
