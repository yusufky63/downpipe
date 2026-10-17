use std::path::Path;
use std::time::Duration;

use downpipe_core::http::{EngineConfig, KIB, MIB, meta};
use downpipe_core::{
    AddOptions, CoreError, Manager, ManagerEvent, Settings, TaskSnapshot, TaskState,
};
use downpipe_test_server::{TestServer, content};

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

async fn open(data: &Path) -> Manager {
    Manager::open_with(data, config())
        .await
        .expect("open manager")
}

fn add(manager: &Manager, url: String, dir: &Path) -> TaskSnapshot {
    manager
        .add(AddOptions {
            url,
            dir: Some(dir.to_path_buf()),
            ..AddOptions::default()
        })
        .expect("add")
}

async fn wait_for(
    manager: &Manager,
    id: &str,
    cond: impl Fn(&TaskSnapshot) -> bool,
) -> TaskSnapshot {
    tokio::time::timeout(Duration::from_secs(20), async {
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

#[tokio::test]
async fn downloads_complete_and_emit_events() {
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let manager = open(data.path()).await;
    let mut events = manager.subscribe();

    let task = add(
        &manager,
        server.url(&format!("/file/{}", 3 * MIB)),
        out.path(),
    );
    assert_eq!(task.record.state, TaskState::Queued);
    let done = wait_for(&manager, &task.record.id, |t| {
        t.record.state == TaskState::Completed
    })
    .await;

    let path = done.record.path.clone().unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), content(3 * MIB, 0));
    assert_eq!(done.record.downloaded, 3 * MIB);
    assert!(done.record.completed_at.is_some());

    let mut saw_downloading = false;
    while let Ok(event) = events.try_recv() {
        if let ManagerEvent::Updated { task } = event {
            saw_downloading |= task.record.state == TaskState::Downloading;
        }
    }
    assert!(saw_downloading);
}

#[tokio::test]
async fn queue_respects_concurrency_limit() {
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let manager = open(data.path()).await;
    manager
        .update_settings(Settings {
            max_concurrent_downloads: 1,
            ..manager.settings()
        })
        .unwrap();

    let ids: Vec<String> = (0..3)
        .map(|i| {
            add(
                &manager,
                server.url(&format!("/file/{}?rate=4194304&name=f{i}.bin", MIB)),
                out.path(),
            )
            .record
            .id
        })
        .collect();

    let mut max_running = 0;
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let tasks = manager.list();
            max_running =
                max_running.max(tasks.iter().filter(|t| t.record.state.is_running()).count());
            if tasks.iter().all(|t| t.record.state == TaskState::Completed) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(max_running, 1);
    for id in ids {
        assert!(manager.get(&id).unwrap().record.path.unwrap().exists());
    }
}

#[tokio::test]
async fn restart_resumes_interrupted_downloads() {
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let size = 8 * MIB;
    let url = server.url(&format!("/file/{size}?rate=2097152"));

    let id = {
        let manager = open(data.path()).await;
        let id = add(&manager, url.clone(), out.path()).record.id;
        wait_for(&manager, &id, |t| t.record.downloaded >= 2 * MIB).await;
        manager.shutdown().await;
        let task = manager.get(&id).unwrap();
        assert_eq!(
            task.record.state,
            TaskState::Queued,
            "shutdown re-queues running work"
        );
        let path = task.record.path.unwrap();
        assert!(meta::meta_path(&path).exists());
        id
    };

    server.state.reset_stats();
    let manager = open(data.path()).await;
    let done = wait_for(&manager, &id, |t| t.record.state == TaskState::Completed).await;
    assert_eq!(
        std::fs::read(done.record.path.unwrap()).unwrap(),
        content(size, 0)
    );
    assert!(
        server.state.bytes_served() < size,
        "resume should not start from zero"
    );
}

#[tokio::test]
async fn pause_resume_and_cancel() {
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let manager = open(data.path()).await;
    let task = add(
        &manager,
        server.url(&format!("/file/{}?rate=2097152", 8 * MIB)),
        out.path(),
    );
    let id = task.record.id;

    wait_for(&manager, &id, |t| t.record.downloaded > 256 * KIB).await;
    manager.pause(&id).unwrap();
    let paused = wait_for(&manager, &id, |t| t.record.state == TaskState::Paused).await;
    assert!(paused.record.downloaded > 0);

    manager.resume(&id).unwrap();
    wait_for(&manager, &id, |t| t.record.state.is_running()).await;
    manager.cancel(&id).unwrap();
    let canceled = wait_for(&manager, &id, |t| t.record.state == TaskState::Canceled).await;
    assert_eq!(canceled.record.downloaded, 0);
    assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 0);

    manager.remove(&id, false).unwrap();
    assert!(manager.get(&id).is_none());
}

#[tokio::test]
async fn duplicates_and_bad_input_are_rejected() {
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let manager = open(data.path()).await;
    let url = server.url(&format!("/file/{}?rate=1048576", 4 * MIB));

    let first = add(&manager, url.clone(), out.path());
    let err = manager
        .add(AddOptions {
            url,
            ..AddOptions::default()
        })
        .unwrap_err();
    assert!(matches!(err, CoreError::Duplicate(ref id) if *id == first.record.id));

    let err = manager
        .add(AddOptions {
            url: "ftp://example.org/x".into(),
            ..AddOptions::default()
        })
        .unwrap_err();
    assert!(matches!(err, CoreError::InvalidUrl(_)));

    let err = manager
        .add(AddOptions {
            url: server.url("/file/10"),
            checksum: Some("md5:abc".into()),
            ..AddOptions::default()
        })
        .unwrap_err();
    assert!(matches!(err, CoreError::Download(_)));
    manager.cancel(&first.record.id).unwrap();
}

#[tokio::test]
async fn failures_are_reported_and_retryable() {
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let manager = open(data.path()).await;
    let task = add(
        &manager,
        server.url("/file/100?fail=404&fail_times=1"),
        out.path(),
    );
    let id = task.record.id;

    let failed = wait_for(&manager, &id, |t| t.record.state == TaskState::Failed).await;
    assert_eq!(failed.record.error_kind.as_deref(), Some("not_found"));
    assert!(failed.record.error.is_some());

    manager.resume(&id).unwrap();
    let done = wait_for(&manager, &id, |t| t.record.state == TaskState::Completed).await;
    assert!(done.record.error.is_none());
}

#[tokio::test]
async fn removing_a_completed_download_can_delete_the_file() {
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let manager = open(data.path()).await;
    let id = add(&manager, server.url("/file/2048"), out.path())
        .record
        .id;
    let done = wait_for(&manager, &id, |t| t.record.state == TaskState::Completed).await;
    let path = done.record.path.unwrap();
    assert!(path.exists());

    manager.remove(&id, true).unwrap();
    assert!(!path.exists());
    drop(manager);
    let reopened = open(data.path()).await;
    assert!(reopened.list().is_empty(), "removal is persisted");
}

#[tokio::test]
async fn settings_persist() {
    let data = tempfile::tempdir().unwrap();
    {
        let manager = open(data.path()).await;
        let mut s = manager.settings();
        s.speed_limit = 123_456;
        s.max_concurrent_downloads = 99;
        let saved = manager.update_settings(s).unwrap();
        assert_eq!(saved.max_concurrent_downloads, 16, "clamped");
    }
    let manager = open(data.path()).await;
    assert_eq!(manager.settings().speed_limit, 123_456);
}

#[tokio::test(flavor = "multi_thread")]
async fn can_be_driven_from_threads_outside_the_runtime() {
    // Tauri runs synchronous commands on the main thread, which has no Tokio
    // context; starting a download from there must still work.
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let manager = open(data.path()).await;

    let url = server.url("/file/4096");
    let dir = out.path().to_path_buf();
    let m = manager.clone();
    let id = std::thread::spawn(move || add(&m, url, &dir).record.id)
        .join()
        .expect("add from a plain thread");
    let done = wait_for(&manager, &id, |t| t.record.state == TaskState::Completed).await;
    assert!(done.record.path.unwrap().exists());
}
