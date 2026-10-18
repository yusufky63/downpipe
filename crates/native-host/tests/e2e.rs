//! End to end: framed messages go into the real `downpipe-native-host`
//! binary, through the local pipe into a real `Manager`, and the file comes
//! from the fault-injection test server.

use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use downpipe_core::http::{EngineConfig, KIB};
use downpipe_core::{ConfirmOptions, Manager, Settings, TaskSnapshot, TaskState};
use downpipe_native_host::frame;
use downpipe_native_host::ipc::{self, AppRequest, AppResponse, ENDPOINT_ENV, Endpoint};
use downpipe_native_host::protocol::DownloadRequest;
use downpipe_native_host::server::BridgeServer;
use downpipe_test_server::{TestServer, content};
use serde_json::{Value, json};

const HOST: &str = env!("CARGO_BIN_EXE_downpipe-native-host");
/// What Chrome passes on the command line.
const ORIGIN: &str = "chrome-extension://efgicfboaldflckpncfdojkeoaamflpf/";

fn config() -> EngineConfig {
    EngineConfig {
        max_connections: 2,
        initial_connections: 2,
        min_split_size: 64 * KIB,
        checkpoint_interval: Duration::from_millis(100),
        retry_base_delay: Duration::from_millis(50),
        retry_max_delay: Duration::from_millis(200),
        ..EngineConfig::default()
    }
}

/// Never the user's real endpoint, so a running Downpipe is not disturbed.
fn test_endpoint(dir: &Path) -> Endpoint {
    if cfg!(windows) {
        Endpoint::new(format!(r"\\.\pipe\downpipe-e2e-{}", uuid::Uuid::new_v4()))
    } else {
        Endpoint::new(dir.join("bridge").join("bridge.sock"))
    }
}

/// A desktop app without the window: a manager plus the bridge server.
struct App {
    manager: Manager,
    endpoint: Endpoint,
    serving: tokio::task::JoinHandle<()>,
    _data: tempfile::TempDir,
    downloads: tempfile::TempDir,
}

impl App {
    /// Downloads from the browser start at once.
    async fn start() -> Self {
        Self::start_with(false).await
    }

    async fn start_with(confirm_browser_downloads: bool) -> Self {
        let data = tempfile::tempdir().unwrap();
        let downloads = tempfile::tempdir().unwrap();
        let manager = Manager::open_with(data.path(), config()).await.unwrap();
        manager
            .update_settings(Settings {
                download_dir: downloads.path().to_path_buf(),
                confirm_browser_downloads,
                // Streams are saved the same with or without an FFmpeg on
                // the test machine.
                ffmpeg_path: data.path().join("no-ffmpeg").to_string_lossy().into_owned(),
                ..manager.settings()
            })
            .unwrap();
        let endpoint = test_endpoint(data.path());
        let server = BridgeServer::bind(endpoint.clone()).unwrap();
        let serving = tokio::spawn(server.serve(manager.clone(), "9.8.7-test"));
        Self {
            manager,
            endpoint,
            serving,
            _data: data,
            downloads,
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.serving.abort();
    }
}

/// The host process, driven the way a browser drives it.
struct Host {
    child: Child,
}

impl Host {
    fn spawn(endpoint: &Endpoint) -> Self {
        let child = Command::new(HOST)
            .arg(ORIGIN)
            .arg("--parent-window=0")
            .env(ENDPOINT_ENV, endpoint.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("start downpipe-native-host");
        Self { child }
    }

    fn send_raw(&mut self, bytes: &[u8]) {
        use std::io::Write;
        let stdin = self.child.stdin.as_mut().unwrap();
        stdin.write_all(bytes).unwrap();
        stdin.flush().unwrap();
    }

    fn ask(&mut self, message: &Value) -> Value {
        frame::write_frame(
            self.child.stdin.as_mut().unwrap(),
            message.to_string().as_bytes(),
        )
        .unwrap();
        self.recv()
    }

    fn recv(&mut self) -> Value {
        let bytes = frame::read_frame(self.child.stdout.as_mut().unwrap())
            .unwrap()
            .expect("the host answered");
        serde_json::from_slice(&bytes).unwrap()
    }

    /// Close stdin like a browser closing the port, then wait for the exit.
    fn finish(mut self) -> ExitStatus {
        drop(self.child.stdin.take());
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status;
            }
            assert!(Instant::now() < deadline, "the host did not exit");
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

async fn wait_for(
    manager: &Manager,
    id: &str,
    cond: impl Fn(&TaskSnapshot) -> bool,
) -> TaskSnapshot {
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            if let Some(task) = manager.get(id)
                && cond(&task)
            {
                return task;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("condition not reached in time")
}

// The host blocks on its pipes, so the bridge server needs worker threads of
// its own while the test thread talks to the host.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn link_from_the_browser_downloads_in_the_manager() {
    let web = TestServer::start().await.unwrap();
    let app = App::start().await;
    let mut host = Host::spawn(&app.endpoint);

    let pong = host.ask(&json!({"protocol_version": 1, "type": "ping", "id": "p1"}));
    assert_eq!(pong["type"], "pong", "{pong}");
    assert_eq!(pong["id"], "p1");
    assert_eq!(pong["app_running"], true);
    assert_eq!(pong["app_version"], "9.8.7-test");
    assert_eq!(pong["host_version"], env!("CARGO_PKG_VERSION"));

    // Throttled so it is still running when the same link arrives again.
    let size = 512 * KIB;
    let url = web.url(&format!("/file/{size}?rate=262144&name=from-browser.bin"));
    let page = web.url("/downloads.html");
    let download = json!({
        "protocol_version": 1,
        "type": "download",
        "id": "d1",
        "resource_url": url,
        "page_url": page,
        "referrer": page,
        "mime_type": "application/octet-stream",
    });
    let accepted = host.ask(&download);
    assert_eq!(accepted["type"], "accepted", "{accepted}");
    assert_eq!(accepted["id"], "d1");
    assert!(
        accepted.get("awaiting_confirmation").is_none(),
        "starts at once: {accepted}"
    );
    let task_id = accepted["task_id"].as_str().unwrap().to_string();

    // The same link again is asked about in the app, as IDM does; the
    // browser is answered at once and told so.
    let again = host.ask(&download);
    assert_eq!(again["type"], "accepted", "{again}");
    assert_eq!(again["awaiting_confirmation"], true, "{again}");
    let asked = again["task_id"].as_str().unwrap().to_string();
    assert_ne!(asked, task_id);
    // A third time, while that question waits, adds nothing new.
    let third = host.ask(&download);
    assert_eq!(third["type"], "error", "{third}");
    assert_eq!(third["code"], "duplicate");
    assert_eq!(third["task_id"], asked.as_str());

    assert!(host.finish().success());

    let done = wait_for(&app.manager, &task_id, |t| {
        t.record.state == TaskState::Completed
    })
    .await;
    let path = done.record.path.unwrap();
    assert_eq!(path.parent().unwrap(), app.downloads.path());
    assert_eq!(path.file_name().unwrap(), "from-browser.bin");
    assert_eq!(std::fs::read(&path).unwrap(), content(size, 0));
    let question = app.manager.get(&asked).unwrap();
    assert!(question.live.awaiting_confirmation);
    assert_eq!(
        question.live.duplicate_of.as_deref(),
        Some(task_id.as_str())
    );
    assert_eq!(app.manager.list().len(), 2);

    // The referrer reached the web server as `Referer`; no cookie ever did.
    let requests = web.state.requests();
    assert!(!requests.is_empty());
    for request in &requests {
        assert_eq!(request.referer.as_deref(), Some(page.as_str()));
        assert_eq!(request.cookie, None);
    }
}

/// The browser is answered at once and told the download waits in the app
/// until the user picks its name and folder.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_downloads_wait_for_the_user() {
    let web = TestServer::start().await.unwrap();
    let app = App::start_with(true).await;
    let mut host = Host::spawn(&app.endpoint);

    let size = 64 * KIB;
    let accepted = host.ask(&json!({
        "protocol_version": 1,
        "type": "download",
        "id": "d1",
        "resource_url": web.url(&format!("/file/{size}?name=asked.bin")),
    }));
    assert_eq!(accepted["type"], "accepted", "{accepted}");
    assert_eq!(accepted["awaiting_confirmation"], true, "{accepted}");
    let id = accepted["task_id"].as_str().unwrap().to_string();

    // What the popup asks while the dialog is open.
    let tasks_query = json!({"protocol_version": 1, "type": "tasks", "task_ids": [id]});
    let answer = host.ask(&tasks_query);
    let held = &answer["tasks"][0];
    assert_eq!(held["state"], "paused", "{answer}");
    assert_eq!(held["awaiting_confirmation"], true, "{answer}");
    assert!(host.finish().success());

    let waiting = app.manager.get(&id).unwrap();
    assert_eq!(waiting.record.state, TaskState::Paused);
    assert!(waiting.live.awaiting_confirmation);
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(
        web.state.requests().is_empty(),
        "nothing is fetched before the answer"
    );

    let chosen = tempfile::tempdir().unwrap();
    app.manager
        .confirm(
            &id,
            ConfirmOptions {
                file_name: Some("renamed.bin".into()),
                dir: Some(chosen.path().to_path_buf()),
                queue: None,
                start: true,
                media: None,
                post: None,
            },
        )
        .unwrap();
    let done = wait_for(&app.manager, &id, |t| {
        t.record.state == TaskState::Completed
    })
    .await;
    let path = done.record.path.unwrap();
    assert_eq!(path, chosen.path().join("renamed.bin"));
    assert_eq!(std::fs::read(&path).unwrap(), content(size, 0));

    // Answered: the flag is gone from what the popup sees.
    let mut host = Host::spawn(&app.endpoint);
    let answer = host.ask(&tasks_query);
    let finished = &answer["tasks"][0];
    assert_eq!(finished["state"], "completed", "{answer}");
    assert_eq!(finished["file_name"], "renamed.bin", "{answer}");
    assert!(finished.get("awaiting_confirmation").is_none(), "{answer}");
    assert!(host.finish().success());
}

/// What the popup does: ask for the downloads it sent until they finish.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn progress_of_sent_downloads_is_reported() {
    let web = TestServer::start().await.unwrap();
    let app = App::start().await;
    let mut host = Host::spawn(&app.endpoint);

    let size = 768 * KIB;
    let accepted = host.ask(&json!({
        "protocol_version": 1,
        "type": "download",
        "resource_url": web.url(&format!("/file/{size}?rate=262144&name=progress.bin")),
    }));
    let task_id = accepted["task_id"].as_str().unwrap().to_string();

    let deadline = Instant::now() + Duration::from_secs(30);
    let mut states = Vec::new();
    let last = loop {
        let answer = host.ask(&json!({
            "protocol_version": 1,
            "type": "tasks",
            "id": "q",
            "task_ids": [task_id, "not-a-download"],
        }));
        assert_eq!(answer["type"], "tasks", "{answer}");
        assert_eq!(answer["id"], "q");
        // Unknown ids are simply left out.
        let tasks = answer["tasks"].as_array().unwrap();
        assert_eq!(tasks.len(), 1, "{answer}");
        let task = tasks[0].clone();
        assert_eq!(task["task_id"], task_id.as_str());
        let state = task["state"].as_str().unwrap().to_string();
        if states.last() != Some(&state) {
            states.push(state.clone());
        }
        if state == "completed" {
            break task;
        }
        assert!(Instant::now() < deadline, "still {state}");
        std::thread::sleep(Duration::from_millis(100));
    };
    assert!(
        states.iter().any(|s| s == "downloading"),
        "never seen running: {states:?}"
    );
    assert_eq!(last["file_name"], "progress.bin");
    assert_eq!(last["downloaded"], size);
    assert_eq!(last["total"], size);
    assert!(host.finish().success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn app_not_running_is_reported_to_the_extension() {
    let dir = tempfile::tempdir().unwrap();
    let mut host = Host::spawn(&test_endpoint(dir.path()));

    let pong = host.ask(&json!({"protocol_version": 1, "type": "ping"}));
    assert_eq!(pong["type"], "pong", "{pong}");
    assert_eq!(pong["app_running"], false);
    assert!(pong.get("app_version").is_none());

    let answer = host.ask(&json!({
        "protocol_version": 1,
        "type": "download",
        "resource_url": "https://example.org/file.zip",
    }));
    assert_eq!(answer["type"], "error");
    assert_eq!(answer["code"], "app_not_running");
    assert!(
        answer["message"].as_str().unwrap().contains("not running"),
        "{answer}"
    );
    assert!(host.finish().success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bad_messages_never_reach_the_manager() {
    let app = App::start().await;
    let mut host = Host::spawn(&app.endpoint);

    let cookies = host.ask(&json!({
        "protocol_version": 1,
        "type": "download",
        "resource_url": "https://example.org/file.zip",
        "cookies": "session=secret",
    }));
    assert_eq!(cookies["code"], "cookies_not_accepted", "{cookies}");

    let script = host.ask(&json!({
        "protocol_version": 1,
        "type": "download",
        "resource_url": "javascript:alert(document.cookie)",
    }));
    assert_eq!(script["code"], "invalid_url", "{script}");

    let newer = host.ask(&json!({"protocol_version": 2, "type": "ping"}));
    assert_eq!(newer["code"], "unsupported_version", "{newer}");

    let garbage = {
        frame::write_frame(host.child.stdin.as_mut().unwrap(), b"{not json").unwrap();
        host.recv()
    };
    assert_eq!(garbage["code"], "invalid_message", "{garbage}");

    // A length prefix over the limit: answered, then the session ends.
    host.send_raw(&(frame::MAX_MESSAGE_LEN as u32 + 1).to_ne_bytes());
    let too_large = host.recv();
    assert_eq!(too_large["code"], "message_too_large", "{too_large}");
    assert!(!host.finish().success());

    assert!(app.manager.list().is_empty());
}

/// Anything running as the user can open the pipe, so the app validates
/// requests itself instead of trusting the host.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn app_validates_requests_that_skip_the_host() {
    let app = App::start().await;
    let endpoint = app.endpoint.clone();
    let answer = tokio::task::spawn_blocking(move || {
        let request = AppRequest::download(DownloadRequest::new("file:///etc/passwd"), None);
        ipc::call(&endpoint, &request, Duration::from_secs(5))
    })
    .await
    .unwrap()
    .unwrap();
    match answer {
        AppResponse::Error { code, .. } => assert_eq!(code, "invalid_url"),
        other => panic!("{other:?}"),
    }
    assert!(app.manager.list().is_empty());

    let endpoint = app.endpoint.clone();
    let answer = tokio::task::spawn_blocking(move || {
        let flood = (0..1000).map(|i| format!("t{i}")).collect();
        ipc::call(&endpoint, &AppRequest::tasks(flood), Duration::from_secs(5))
    })
    .await
    .unwrap()
    .unwrap();
    match answer {
        AppResponse::Error { code, .. } => assert_eq!(code, "invalid_message"),
        other => panic!("{other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_second_server_cannot_take_over_the_endpoint() {
    let app = App::start().await;
    let err = BridgeServer::bind(app.endpoint.clone()).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::AddrInUse, "{err}");
}

/// A stream from the media list: the app reads its qualities for the
/// dialog, downloads the chosen one with the page's referrer, and the popup
/// follows it like any download.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn streams_from_the_browser_are_offered_with_their_qualities() {
    use downpipe_core::http::media::{MediaSelection, TrackRef};
    use downpipe_test_server::streams::joined;

    let web = TestServer::start().await.unwrap();
    let app = App::start_with(true).await;
    let mut host = Host::spawn(&app.endpoint);

    let pong = host.ask(&json!({"protocol_version": 1, "type": "ping"}));
    assert_eq!(pong["features"], json!(["media", "page"]), "{pong}");

    let q = "segments=3&size=4000&split=1";
    let page = web.url("/watch/clip");
    let accepted = host.ask(&json!({
        "protocol_version": 1,
        "type": "media",
        "id": "s1",
        "manifest_url": web.url(&format!("/hls/master.m3u8?{q}")),
        "kind": "hls",
        "page_url": page,
        "referrer": page,
        "origin": web.url(""),
        "title": "Clip: the week",
    }));
    assert_eq!(accepted["type"], "accepted", "{accepted}");
    assert_eq!(accepted["awaiting_confirmation"], true, "{accepted}");
    assert_eq!(accepted["file_name"], "Clip_ the week.mp4", "{accepted}");
    let id = accepted["task_id"].as_str().unwrap().to_string();
    assert!(host.finish().success());

    let choices = app.manager.inspect_media(&id).await.unwrap();
    assert_eq!(choices.options.videos.len(), 2);
    let lower = choices.options.videos.last().unwrap();
    app.manager
        .confirm(
            &id,
            ConfirmOptions {
                start: true,
                media: Some(MediaSelection {
                    video: Some(TrackRef {
                        id: lower.id.clone(),
                        height: lower.height,
                        ..TrackRef::default()
                    }),
                    audio: None,
                }),
                ..ConfirmOptions::default()
            },
        )
        .unwrap();
    let done = wait_for(&app.manager, &id, |t| {
        t.record.state == TaskState::Completed
    })
    .await;
    let path = done.record.path.unwrap();
    assert_eq!(path.file_name().unwrap(), "Clip_ the week.mp4");
    assert_eq!(std::fs::read(&path).unwrap(), joined("v0", q));
    assert_eq!(
        std::fs::read(path.with_file_name("Clip_ the week.audio.m4a")).unwrap(),
        joined("audio", q)
    );

    // Manifests, fragments and the key went out with the page as referrer
    // and without cookies.
    let requests = web.state.requests();
    assert!(requests.iter().any(|r| r.path.contains("/seg-")));
    for request in &requests {
        assert_eq!(
            request.referer.as_deref(),
            Some(page.as_str()),
            "{}",
            request.path
        );
        assert_eq!(request.origin.as_deref(), Some(web.url("").as_str()));
        assert_eq!(request.cookie, None);
    }

    let mut host = Host::spawn(&app.endpoint);
    let answer = host.ask(&json!({"protocol_version": 1, "type": "tasks", "task_ids": [id]}));
    assert_eq!(answer["tasks"][0]["state"], "completed", "{answer}");
    assert!(host.finish().success());
}

/// A video page: the app takes the page address and title only, holds it
/// for the quality dialog, and keeps one link per video.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn video_pages_go_to_the_site_provider() {
    let app = App::start_with(true).await;
    let mut host = Host::spawn(&app.endpoint);
    let fixture: Value = serde_json::from_slice(
        &std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/page-youtube.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let accepted = host.ask(&fixture);
    assert_eq!(accepted["type"], "accepted", "{accepted}");
    assert_eq!(accepted["id"], "y1");
    assert_eq!(accepted["awaiting_confirmation"], true, "{accepted}");
    assert_eq!(accepted["file_name"], "Me at the zoo.mp4", "{accepted}");
    let id = accepted["task_id"].as_str().unwrap().to_string();

    // The same video from a share link is a duplicate of it.
    let again = host.ask(&json!({
        "protocol_version": 1,
        "type": "page",
        "page_url": "https://youtu.be/jNQXAC9IVRw?si=share",
    }));
    assert_eq!(again["code"], "duplicate", "{again}");
    assert_eq!(again["task_id"], id.as_str());

    let other = host.ask(&json!({
        "protocol_version": 1,
        "type": "page",
        "page_url": "https://example.org/watch?v=jNQXAC9IVRw",
    }));
    assert_eq!(other["code"], "unsupported_page", "{other}");
    assert!(host.finish().success());

    let task = app.manager.get(&id).unwrap();
    assert_eq!(
        task.record.url,
        "https://www.youtube.com/watch?v=jNQXAC9IVRw"
    );
    assert_eq!(task.record.site.unwrap().site, "YouTube");
    assert_eq!(task.record.state, TaskState::Paused);
    assert_eq!(app.manager.list().len(), 1);
}
