//! The remote control against a real `Manager` and the test server, over
//! real HTTP.

use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use downpipe_core::http::{EngineConfig, KIB};
use downpipe_core::{Manager, Queue, RemoteSettings, Settings, TaskState};
use downpipe_remote::{Config, DirAssets, NoAssets, RemoteServer, token};
use downpipe_test_server::TestServer;
use reqwest::StatusCode;
use serde_json::{Value, json};

struct Remote {
    server: RemoteServer,
    token: String,
    http: reqwest::Client,
}

impl Remote {
    fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.server.local_addr())
    }

    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.http.get(self.url(path)).bearer_auth(&self.token)
    }

    fn post(&self, path: &str) -> reqwest::RequestBuilder {
        self.http.post(self.url(path)).bearer_auth(&self.token)
    }

    async fn list(&self) -> Vec<Value> {
        let body: Value = self
            .get("/api/v1/downloads")
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        body["downloads"].as_array().unwrap().clone()
    }
}

async fn open(data: &Path, downloads: &Path) -> Manager {
    let config = EngineConfig {
        min_split_size: 64 * KIB,
        checkpoint_interval: Duration::from_millis(100),
        ..EngineConfig::default()
    };
    let manager = Manager::open_with(data, config).await.unwrap();
    let settings = manager.settings();
    manager
        .update_settings(Settings {
            download_dir: downloads.to_path_buf(),
            proxy: String::new(),
            queues: vec![
                settings.queues[0].clone(),
                Queue {
                    id: "night".into(),
                    name: "Night".into(),
                    max_concurrent: 1,
                    command: String::new(),
                },
            ],
            ..settings
        })
        .unwrap();
    manager
}

async fn start(manager: &Manager, assets: Arc<dyn downpipe_remote::Assets>) -> Remote {
    let token = token::generate();
    let config = Config {
        bind: IpAddr::V4(Ipv4Addr::LOCALHOST),
        port: 0,
        token_hash: token::parse_hash(&token::hash(&token)).unwrap(),
    };
    let server = RemoteServer::start(manager.clone(), config, assets)
        .await
        .unwrap();
    Remote {
        server,
        token,
        http: reqwest::Client::builder().no_proxy().build().unwrap(),
    }
}

async fn wait_for_state(manager: &Manager, id: &str, state: TaskState) {
    tokio::time::timeout(Duration::from_secs(20), async {
        while manager.get(id).map(|t| t.record.state) != Some(state) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("{id} never became {state:?}"));
}

#[tokio::test]
async fn every_api_request_needs_the_token() {
    let data = tempfile::tempdir().unwrap();
    let downloads = tempfile::tempdir().unwrap();
    let manager = open(data.path(), downloads.path()).await;
    let remote = start(&manager, Arc::new(NoAssets)).await;

    let none = remote
        .http
        .get(remote.url("/api/v1/downloads"))
        .send()
        .await
        .unwrap();
    assert_eq!(none.status(), StatusCode::UNAUTHORIZED);
    assert!(none.headers().contains_key("www-authenticate"));

    let wrong = remote
        .http
        .post(remote.url("/api/v1/downloads"))
        .bearer_auth("not-the-token")
        .json(&json!({ "url": "https://example.com/a.zip" }))
        .send()
        .await
        .unwrap();
    assert_eq!(wrong.status(), StatusCode::UNAUTHORIZED);
    assert!(manager.list().is_empty(), "nothing was added");

    let ok = remote.get("/api/v1/downloads").send().await.unwrap();
    assert_eq!(ok.status(), StatusCode::OK);
    assert_eq!(ok.headers()["cache-control"], "no-store");
    assert_eq!(ok.headers()["x-content-type-options"], "nosniff");
    let grouped = remote
        .http
        .get(remote.url("/api/v1/settings"))
        .bearer_auth(token::grouped(&remote.token).to_uppercase())
        .send()
        .await
        .unwrap();
    assert_eq!(grouped.status(), StatusCode::OK, "typed with dashes");
    remote.server.stop().await;
}

#[tokio::test]
async fn too_many_wrong_tokens_lock_the_peer_out() {
    let data = tempfile::tempdir().unwrap();
    let downloads = tempfile::tempdir().unwrap();
    let manager = open(data.path(), downloads.path()).await;
    let remote = start(&manager, Arc::new(NoAssets)).await;

    for _ in 0..10 {
        let res = remote
            .http
            .get(remote.url("/api/v1/downloads"))
            .bearer_auth("guess")
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
    let locked = remote.get("/api/v1/downloads").send().await.unwrap();
    assert_eq!(
        locked.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "even the right token waits, so guesses learn nothing"
    );
    let wait: u64 = locked.headers()["retry-after"]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!(wait > 0 && wait <= 600);
    let body: Value = locked.json().await.unwrap();
    assert_eq!(body["error"], "too_many_attempts");
    remote.server.stop().await;
}

#[tokio::test]
async fn adds_pauses_resumes_and_removes() {
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let downloads = tempfile::tempdir().unwrap();
    let manager = open(data.path(), downloads.path()).await;
    let remote = start(&manager, Arc::new(NoAssets)).await;

    let url = server.url(&format!(
        "/file/{}/movie.bin?rate=262144&sig=secret",
        4 * 1024 * 1024
    ));
    let res = remote
        .post("/api/v1/downloads")
        .json(&json!({ "url": url, "folder": "Phone/Movies", "queue": "night" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let body: Value = res.json().await.unwrap();
    let view = &body["download"];
    let id = view["id"].as_str().unwrap().to_string();
    assert_eq!(view["name"], "movie.bin");
    assert_eq!(view["queue"], "night");
    let text = view.to_string();
    assert!(!text.contains("secret"), "no full link: {text}");
    assert!(
        !text.contains(&downloads.path().display().to_string().replace('\\', "\\\\")),
        "no paths: {text}"
    );
    let record = manager.get(&id).unwrap().record;
    assert_eq!(record.dir, downloads.path().join("Phone").join("Movies"));

    let again = remote
        .post("/api/v1/downloads")
        .json(&json!({ "url": url }))
        .send()
        .await
        .unwrap();
    assert_eq!(again.status(), StatusCode::CONFLICT);
    let body: Value = again.json().await.unwrap();
    assert_eq!(body["error"], "duplicate");
    assert_eq!(body["id"], id.as_str());

    wait_for_state(&manager, &id, TaskState::Downloading).await;
    let res = remote
        .post(&format!("/api/v1/downloads/{id}/pause"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    wait_for_state(&manager, &id, TaskState::Paused).await;
    let listed = remote.list().await;
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["state"], "paused");
    assert_eq!(listed[0]["total"], 4 * 1024 * 1024);
    assert_eq!(listed[0]["host"], "127.0.0.1");

    let res = remote
        .post(&format!("/api/v1/downloads/{id}/resume"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    assert_ne!(manager.get(&id).unwrap().record.state, TaskState::Paused);

    let res = remote
        .http
        .delete(remote.url(&format!("/api/v1/downloads/{id}")))
        .bearer_auth(&remote.token)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    tokio::time::timeout(Duration::from_secs(10), async {
        while manager.get(&id).is_some() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("removed");

    let missing = remote
        .post("/api/v1/downloads/nope/pause")
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    manager.shutdown().await;
    remote.server.stop().await;
}

#[tokio::test]
async fn removing_a_finished_download_keeps_its_file() {
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let downloads = tempfile::tempdir().unwrap();
    let manager = open(data.path(), downloads.path()).await;
    let remote = start(&manager, Arc::new(NoAssets)).await;

    let res = remote
        .post("/api/v1/downloads")
        .json(&json!({ "url": server.url("/file/1000/small.bin"), "folder": "Kept" }))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    let id = body["download"]["id"].as_str().unwrap().to_string();
    wait_for_state(&manager, &id, TaskState::Completed).await;
    let path = manager.get(&id).unwrap().record.path.unwrap();
    assert!(path.starts_with(downloads.path().join("Kept")));

    let res = remote
        .http
        .delete(remote.url(&format!("/api/v1/downloads/{id}")))
        .bearer_auth(&remote.token)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    assert!(manager.get(&id).is_none());
    assert!(path.exists(), "the finished file stays");
    remote.server.stop().await;
}

#[tokio::test]
async fn refuses_folders_outside_the_download_folder_and_bad_requests() {
    let data = tempfile::tempdir().unwrap();
    let downloads = tempfile::tempdir().unwrap();
    let manager = open(data.path(), downloads.path()).await;
    let remote = start(&manager, Arc::new(NoAssets)).await;

    for (body, code) in [
        (
            json!({ "url": "https://example.com/a.zip", "folder": "../outside" }),
            "invalid_folder",
        ),
        (
            json!({ "url": "https://example.com/a.zip", "folder": "C:/Windows" }),
            "invalid_folder",
        ),
        (
            json!({ "url": "https://example.com/a.zip", "queue": "nope" }),
            "unknown_queue",
        ),
        (json!({ "url": "file:///etc/passwd" }), "invalid_url"),
        (
            json!({ "url": "https://example.com/a.zip", "dir": "C:/" }),
            "bad_request",
        ),
        (json!({ "url": "" }), "invalid_url"),
    ] {
        let res = remote
            .post("/api/v1/downloads")
            .json(&body)
            .send()
            .await
            .unwrap();
        assert!(res.status().is_client_error(), "{body}");
        let answer: Value = res.json().await.unwrap();
        assert_eq!(answer["error"], code, "{body}");
    }

    let big = "x".repeat(32 * 1024);
    let res = remote
        .post("/api/v1/downloads")
        .header("content-type", "application/json")
        .body(format!(r#"{{"url":"https://example.com/{big}"}}"#))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert!(manager.list().is_empty());
    remote.server.stop().await;
}

#[tokio::test]
async fn other_sites_cannot_call_it() {
    let data = tempfile::tempdir().unwrap();
    let downloads = tempfile::tempdir().unwrap();
    let manager = open(data.path(), downloads.path()).await;
    let remote = start(&manager, Arc::new(NoAssets)).await;

    let res = remote
        .post("/api/v1/downloads")
        .header("origin", "https://evil.example")
        .json(&json!({ "url": "https://example.com/a.zip" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    assert!(!res.headers().contains_key("access-control-allow-origin"));

    let preflight = remote
        .http
        .request(reqwest::Method::OPTIONS, remote.url("/api/v1/downloads"))
        .header("origin", "https://evil.example")
        .header("access-control-request-method", "POST")
        .send()
        .await
        .unwrap();
    assert!(!preflight.status().is_success());
    assert!(
        !preflight
            .headers()
            .contains_key("access-control-allow-origin")
    );

    let own = remote
        .post("/api/v1/downloads")
        .header("origin", remote.url(""))
        .json(&json!({ "url": "https://example.com/a.zip", "paused": true }))
        .send()
        .await
        .unwrap();
    assert_eq!(own.status(), StatusCode::CREATED, "the page itself may");
    remote.server.stop().await;
}

#[tokio::test]
async fn settings_leave_out_paths_and_secrets() {
    let data = tempfile::tempdir().unwrap();
    let downloads = tempfile::tempdir().unwrap();
    let manager = open(data.path(), downloads.path()).await;
    let settings = manager.settings();
    manager
        .update_settings(Settings {
            proxy: "http://user:pass@127.0.0.1:3128".into(),
            yt_dlp_path: "C:/Tools/yt-dlp.exe".into(),
            remote: RemoteSettings {
                enabled: true,
                token_hash: token::hash("stored"),
                ..RemoteSettings::default()
            },
            ..settings
        })
        .unwrap();
    let remote = start(&manager, Arc::new(NoAssets)).await;

    let body: Value = remote
        .get("/api/v1/settings")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let text = body.to_string();
    for secret in ["pass", "3128", "yt-dlp", "token", "Tools", "download_dir"] {
        assert!(!text.contains(secret), "{secret} in {text}");
    }
    assert_eq!(body["queues"][1]["id"], "night");
    assert_eq!(body["main_queue"], "main");
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
    remote.server.stop().await;
}

#[tokio::test]
async fn serves_only_the_page_and_its_assets() {
    let data = tempfile::tempdir().unwrap();
    let downloads = tempfile::tempdir().unwrap();
    let dist = tempfile::tempdir().unwrap();
    std::fs::create_dir(dist.path().join("assets")).unwrap();
    std::fs::write(
        dist.path().join("remote.html"),
        "<!doctype html><title>Remote</title>",
    )
    .unwrap();
    std::fs::write(dist.path().join("index.html"), "desktop").unwrap();
    std::fs::write(dist.path().join("assets/remote-abc.js"), "console.log(1)").unwrap();
    let manager = open(data.path(), downloads.path()).await;
    let remote = start(&manager, Arc::new(DirAssets::new(dist.path()))).await;

    let page = remote.http.get(remote.url("/")).send().await.unwrap();
    assert_eq!(page.status(), StatusCode::OK, "the page needs no token");
    let csp = page.headers()["content-security-policy"]
        .to_str()
        .unwrap()
        .to_string();
    assert!(csp.contains("frame-ancestors 'none'") && csp.contains("script-src 'self'"));
    assert!(
        page.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/html")
    );
    assert!(page.text().await.unwrap().contains("Remote"));

    let script = remote
        .http
        .get(remote.url("/assets/remote-abc.js"))
        .send()
        .await
        .unwrap();
    assert_eq!(script.status(), StatusCode::OK);
    assert!(
        script.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/javascript")
    );

    for path in [
        "/index.html",
        "/assets/..%2findex.html",
        "/assets/%2e%2e%2fremote.html",
        "/../Cargo.toml",
        "/api/v1/../../index.html",
    ] {
        let res = remote.http.get(remote.url(path)).send().await.unwrap();
        assert!(
            res.status() == StatusCode::NOT_FOUND || res.status() == StatusCode::UNAUTHORIZED,
            "{path}: {}",
            res.status()
        );
        assert_ne!(res.text().await.unwrap(), "desktop", "{path}");
    }
    remote.server.stop().await;
}

#[tokio::test]
async fn config_needs_switch_and_token() {
    let mut remote = RemoteSettings {
        enabled: true,
        token_hash: token::hash("abc"),
        ..RemoteSettings::default()
    };
    let config = Config::from_settings(&remote).unwrap();
    assert!(config.bind.is_loopback());
    remote.enabled = false;
    assert!(Config::from_settings(&remote).is_none());
    remote.enabled = true;
    remote.token_hash.clear();
    assert!(Config::from_settings(&remote).is_none());
}

#[tokio::test]
async fn a_taken_port_is_reported() {
    let data = tempfile::tempdir().unwrap();
    let downloads = tempfile::tempdir().unwrap();
    let manager = open(data.path(), downloads.path()).await;
    let first = start(&manager, Arc::new(NoAssets)).await;
    let config = Config {
        bind: IpAddr::V4(Ipv4Addr::LOCALHOST),
        port: first.server.local_addr().port(),
        token_hash: [0; 32],
    };
    let err = RemoteServer::start(manager.clone(), config, Arc::new(NoAssets))
        .await
        .unwrap_err();
    assert_eq!(err.code(), "port_in_use");
    first.server.stop().await;
}
