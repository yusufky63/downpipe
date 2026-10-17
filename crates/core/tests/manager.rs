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
    .unwrap_or_else(|_| {
        let task = manager.get(id);
        panic!(
            "condition not reached in time; last: {:?}",
            task.map(|t| (
                t.record.state,
                t.record.downloaded,
                t.record.error,
                t.live.notices
            ))
        )
    })
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

async fn open_into(data: &Path, downloads: &Path) -> Manager {
    let manager = open(data).await;
    manager
        .update_settings(Settings {
            download_dir: downloads.to_path_buf(),
            ..manager.settings()
        })
        .unwrap();
    manager
}

fn add_auto(manager: &Manager, url: String) -> TaskSnapshot {
    manager
        .add(AddOptions {
            url,
            ..AddOptions::default()
        })
        .expect("add")
}

#[tokio::test]
async fn sorts_downloads_into_category_folders() {
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let manager = open_into(data.path(), out.path()).await;

    // The URL names the type: the folder is chosen up front.
    let zip = add_auto(&manager, server.url("/file/2000/archive.zip"));
    assert_eq!(zip.record.dir, out.path().join("Compressed"));
    assert_eq!(zip.record.category.as_deref(), Some("Compressed"));

    // Only the server's answer names the type: the file moves when done.
    let iso = add_auto(&manager, server.url("/file/3000?name=disk.iso"));
    assert_eq!(iso.record.dir, out.path());
    assert_eq!(iso.record.category, None);

    // A folder the user picked is never second-guessed.
    let picked = tempfile::tempdir().unwrap();
    let pdf = add(
        &manager,
        server.url("/file/1000?name=manual.pdf"),
        picked.path(),
    );

    let zip = wait_for(&manager, &zip.record.id, |t| {
        t.record.state == TaskState::Completed
    })
    .await;
    let iso = wait_for(&manager, &iso.record.id, |t| {
        t.record.state == TaskState::Completed
    })
    .await;
    let pdf = wait_for(&manager, &pdf.record.id, |t| {
        t.record.state == TaskState::Completed
    })
    .await;

    assert_eq!(
        zip.record.path.unwrap().parent().unwrap(),
        out.path().join("Compressed")
    );
    let iso_path = iso.record.path.unwrap();
    assert_eq!(iso_path, out.path().join("Disk images").join("disk.iso"));
    assert_eq!(std::fs::read(&iso_path).unwrap(), content(3000, 0));
    assert_eq!(iso.record.category.as_deref(), Some("Disk images"));
    assert!(!out.path().join("disk.iso").exists(), "moved, not copied");
    assert_eq!(pdf.record.path.unwrap(), picked.path().join("manual.pdf"));
    assert_eq!(pdf.record.category.as_deref(), Some("Documents"));
}

#[tokio::test]
async fn categories_can_be_switched_off() {
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let manager = open_into(data.path(), out.path()).await;
    manager
        .update_settings(Settings {
            use_categories: false,
            ..manager.settings()
        })
        .unwrap();
    let task = add_auto(&manager, server.url("/file/1000/x.zip"));
    let done = wait_for(&manager, &task.record.id, |t| {
        t.record.state == TaskState::Completed
    })
    .await;
    assert_eq!(done.record.path.unwrap(), out.path().join("x.zip"));
}

#[tokio::test]
async fn changing_the_link_resumes_or_refuses() {
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let manager = open(data.path()).await;
    let size = 8 * MIB;
    let task = add(
        &manager,
        server.url(&format!("/file/{size}?rate=1048576&mirror=a")),
        out.path(),
    );
    let id = task.record.id;

    wait_for(&manager, &id, |t| t.record.downloaded > 512 * KIB).await;
    manager.pause(&id).unwrap();
    let paused = wait_for(&manager, &id, |t| t.record.state == TaskState::Paused).await;
    let path = paused.record.path.clone().unwrap();

    // Running downloads must be paused first.
    manager.resume(&id).unwrap();
    wait_for(&manager, &id, |t| t.record.state.is_running()).await;
    assert!(matches!(
        manager.change_url(&id, &server.url("/file/1?x")),
        Err(CoreError::InvalidState { .. })
    ));
    manager.pause(&id).unwrap();
    wait_for(&manager, &id, |t| t.record.state == TaskState::Paused).await;

    // A different file behind the new link: refused, data kept.
    server.state.set_version(9);
    manager
        .change_url(&id, &server.url(&format!("/file/{size}?mirror=b")))
        .unwrap();
    let failed = wait_for(&manager, &id, |t| t.record.state == TaskState::Failed).await;
    assert_eq!(failed.record.error_kind.as_deref(), Some("different_file"));
    assert!(meta::part_path(&path).exists());

    // The same file again: resumes into the same path.
    server.state.set_version(0);
    manager
        .change_url(&id, &server.url(&format!("/file/{size}?mirror=c")))
        .unwrap();
    let done = wait_for(&manager, &id, |t| t.record.state == TaskState::Completed).await;
    assert_eq!(done.record.path.unwrap(), path);
    assert_eq!(std::fs::read(&path).unwrap(), content(size, 0));
    assert!(done.record.url.contains("mirror=c"));

    assert!(matches!(
        manager.change_url(&id, "ftp://example.org/x"),
        Err(CoreError::InvalidUrl(_))
    ));
}

#[tokio::test]
async fn proxy_and_user_agent_settings() {
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let manager = open(data.path()).await;

    for bad in ["nonsense", "ftp://proxy:21", "http://"] {
        let err = manager
            .update_settings(Settings {
                proxy: bad.into(),
                ..manager.settings()
            })
            .unwrap_err();
        assert!(
            matches!(err, CoreError::InvalidSetting(_)),
            "{bad}: {err:?}"
        );
    }
    assert_eq!(
        manager.settings().proxy,
        "",
        "rejected values are not saved"
    );

    manager
        .update_settings(Settings {
            user_agent: "Mozilla/5.0 Downpipe-test".into(),
            ..manager.settings()
        })
        .unwrap();
    let task = add(&manager, server.url("/file/1000"), out.path());
    wait_for(&manager, &task.record.id, |t| {
        t.record.state == TaskState::Completed
    })
    .await;
    assert_eq!(
        server.state.requests()[0].user_agent.as_deref(),
        Some("Mozilla/5.0 Downpipe-test")
    );

    // A proxy nobody listens on: the setting is accepted, downloads fail.
    manager
        .update_settings(Settings {
            proxy: "http://127.0.0.1:9".into(),
            ..manager.settings()
        })
        .unwrap();
    let task = add(&manager, server.url("/file/1000?again"), out.path());
    let failed = wait_for(&manager, &task.record.id, |t| {
        t.record.state == TaskState::Failed
    })
    .await;
    assert_eq!(failed.record.error_kind.as_deref(), Some("network"));
}
