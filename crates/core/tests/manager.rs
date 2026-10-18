use std::path::Path;
use std::time::Duration;

use downpipe_core::http::{EngineConfig, KIB, MIB, meta};
use downpipe_core::{
    AddOptions, ConfirmOptions, CoreError, Manager, ManagerEvent, OnDuplicate, QueueMove, Settings,
    TaskSnapshot, TaskState,
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

/// A link already in the list, even a finished one, is refused unless the
/// caller asks for a copy or for the question to wait in the app.
#[tokio::test]
async fn duplicate_links_are_refused_copied_or_asked_about() {
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let manager = open(data.path()).await;
    let url = server.url(&format!("/file/{}/same.bin", 64 * KIB));
    let first = add(&manager, url.clone(), out.path());
    wait_for(&manager, &first.record.id, |t| {
        t.record.state == TaskState::Completed
    })
    .await;

    let again = |on_duplicate| {
        manager.add(AddOptions {
            url: url.clone(),
            dir: Some(out.path().to_path_buf()),
            on_duplicate,
            ..AddOptions::default()
        })
    };
    let err = again(OnDuplicate::Refuse).unwrap_err();
    assert!(matches!(err, CoreError::Duplicate(ref id) if *id == first.record.id));

    let asked = again(OnDuplicate::Ask).unwrap();
    assert_eq!(asked.record.state, TaskState::Paused);
    assert!(asked.live.awaiting_confirmation);
    assert_eq!(
        asked.live.duplicate_of.as_deref(),
        Some(first.record.id.as_str())
    );
    // While that question waits, the link is not added a third time.
    let err = again(OnDuplicate::AddCopy).unwrap_err();
    assert!(matches!(err, CoreError::Duplicate(ref id) if *id == asked.record.id));
    manager.remove(&asked.record.id, true).unwrap();

    let copy = again(OnDuplicate::AddCopy).unwrap();
    let done = wait_for(&manager, &copy.record.id, |t| {
        t.record.state == TaskState::Completed
    })
    .await;
    assert_eq!(done.record.path.unwrap(), out.path().join("same (1).bin"));
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

#[tokio::test]
async fn queues_run_side_by_side_with_their_own_limits() {
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let manager = open(data.path()).await;
    let mut s = manager.settings();
    s.max_concurrent_downloads = 1;
    s.queues.push(downpipe_core::Queue {
        id: String::new(),
        name: "Night".into(),
        max_concurrent: 1,
    });
    let s = manager.update_settings(s).unwrap();
    let night = s.queues[1].id.clone();

    let slow = |i: u32| server.url(&format!("/file/{}?rate=1048576&n={i}", 4 * MIB));
    let a = add(&manager, slow(1), out.path()).record.id;
    let b = add(&manager, slow(2), out.path()).record.id;
    let c = add(&manager, slow(3), out.path()).record.id;
    manager.set_queue(std::slice::from_ref(&c), &night).unwrap();

    // Main runs one at a time; Night runs its own download beside it.
    wait_for(&manager, &c, |t| t.record.state.is_running()).await;
    let state = |id: &str| manager.get(id).unwrap().record.state;
    assert!(state(&a).is_running());
    assert_eq!(state(&b), TaskState::Queued);
    assert_eq!(manager.get(&c).unwrap().record.queue, night);

    manager.stop_queue(&night);
    wait_for(&manager, &c, |t| t.record.state == TaskState::Paused).await;
    assert!(
        state(&a).is_running(),
        "stopping one queue leaves the others"
    );
    for id in [&a, &b] {
        manager.cancel(id).unwrap();
    }
}

#[tokio::test]
async fn reordering_decides_what_starts_next() {
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
    let ids: Vec<String> = (0..4)
        .map(|i| {
            add(
                &manager,
                server.url(&format!("/file/{}?rate=1048576&n={i}", 4 * MIB)),
                out.path(),
            )
            .record
            .id
        })
        .collect();
    let position = |id: &str| manager.get(id).unwrap().record.position;
    let order = |ids: &[String]| {
        let mut v: Vec<(i64, String)> = ids.iter().map(|id| (position(id), id.clone())).collect();
        v.sort();
        v.into_iter().map(|(_, id)| id).collect::<Vec<_>>()
    };
    assert_eq!(order(&ids), ids);

    manager
        .move_tasks(&[ids[3].clone()], QueueMove::Top)
        .unwrap();
    assert_eq!(
        order(&ids),
        [&ids[3], &ids[0], &ids[1], &ids[2]].map(String::clone)
    );
    manager
        .move_tasks(&[ids[1].clone(), ids[2].clone()], QueueMove::Up)
        .unwrap();
    assert_eq!(
        order(&ids),
        [&ids[3], &ids[1], &ids[2], &ids[0]].map(String::clone)
    );
    manager
        .move_tasks(&[ids[3].clone()], QueueMove::Bottom)
        .unwrap();
    manager
        .move_tasks(&[ids[1].clone()], QueueMove::Down)
        .unwrap();
    assert_eq!(
        order(&ids),
        [&ids[2], &ids[1], &ids[0], &ids[3]].map(String::clone)
    );

    // The first one is already running; when it is paused the next in queue
    // order (ids[2], moved to the front) starts, not the oldest.
    wait_for(&manager, &ids[0], |t| t.record.state.is_running()).await;
    manager.pause(&ids[0]).unwrap();
    wait_for(&manager, &ids[2], |t| t.record.state.is_running()).await;
    for id in &ids {
        let _ = manager.cancel(id);
    }
}

#[tokio::test]
async fn browser_downloads_wait_for_confirmation() {
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let picked = tempfile::tempdir().unwrap();
    let manager = open_into(data.path(), out.path()).await;

    let held = manager
        .add(AddOptions {
            url: server.url("/file/3000?name=report.pdf"),
            confirm: true,
            ..AddOptions::default()
        })
        .unwrap();
    assert_eq!(held.record.state, TaskState::Paused);
    assert!(held.live.awaiting_confirmation);
    assert!(!held.live.from_app, "the browser sent it");
    manager.resume_all();
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(
        manager.get(&held.record.id).unwrap().record.state,
        TaskState::Paused,
        "held until confirmed"
    );

    manager
        .confirm(
            &held.record.id,
            ConfirmOptions {
                file_name: Some("../renamed.pdf".into()),
                dir: Some(picked.path().to_path_buf()),
                queue: None,
                start: true,
                media: None,
            },
        )
        .unwrap();
    let done = wait_for(&manager, &held.record.id, |t| {
        t.record.state == TaskState::Completed
    })
    .await;
    assert_eq!(done.record.path.unwrap(), picked.path().join("renamed.pdf"));
    assert!(!done.live.awaiting_confirmation);
    assert!(matches!(
        manager.confirm(&held.record.id, ConfirmOptions::default()),
        Err(CoreError::InvalidState { .. })
    ));

    // "Download later": confirmed but left paused.
    let later = manager
        .add(AddOptions {
            url: server.url("/file/1000?later"),
            dir: Some(out.path().to_path_buf()),
            confirm: true,
            ..AddOptions::default()
        })
        .unwrap();
    manager
        .confirm(
            &later.record.id,
            ConfirmOptions {
                start: false,
                ..ConfirmOptions::default()
            },
        )
        .unwrap();
    let t = manager.get(&later.record.id).unwrap();
    assert_eq!(t.record.state, TaskState::Paused);
    assert!(!t.live.awaiting_confirmation);

    // Added in the app's own window, which asks itself: every window reads
    // that from the snapshot.
    let here = manager
        .add(AddOptions {
            url: server.url("/file/1000?here"),
            confirm: true,
            from_app: true,
            ..AddOptions::default()
        })
        .unwrap();
    assert!(here.live.awaiting_confirmation && here.live.from_app);
    let json = serde_json::to_value(manager.get(&here.record.id).unwrap()).unwrap();
    assert_eq!(json["from_app"], true);
    assert_eq!(json["awaiting_confirmation"], true);
}

#[tokio::test]
async fn finished_downloads_can_be_downloaded_again() {
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let manager = open(data.path()).await;
    let id = add(&manager, server.url("/file/5000?name=data.bin"), out.path())
        .record
        .id;
    let first = wait_for(&manager, &id, |t| t.record.state == TaskState::Completed).await;
    let path = first.record.path.unwrap();

    // Keep the old file: the new one gets a free name.
    manager.redownload(&id, false).unwrap();
    let again = wait_for(&manager, &id, |t| {
        t.record.state == TaskState::Completed && t.record.path.is_some()
    })
    .await;
    assert_eq!(again.record.path.unwrap(), out.path().join("data (1).bin"));
    assert!(path.exists());

    // Replace: the old file is deleted first and the name is reused.
    server.state.set_version(2);
    manager.redownload(&id, true).unwrap();
    let replaced = wait_for(&manager, &id, |t| {
        t.record.state == TaskState::Completed && t.record.path.is_some()
    })
    .await;
    let replaced_path = replaced.record.path.unwrap();
    assert_eq!(replaced_path, out.path().join("data (1).bin"));
    assert_eq!(std::fs::read(&replaced_path).unwrap(), content(5000, 2));

    let running = add(
        &manager,
        server.url(&format!("/file/{}?rate=1048576", 4 * MIB)),
        out.path(),
    )
    .record
    .id;
    assert!(matches!(
        manager.redownload(&running, false),
        Err(CoreError::InvalidState { .. })
    ));
    manager.cancel(&running).unwrap();
}

// ---------- Streams ----------

/// A manager whose stream downloads never find an FFmpeg, so the result
/// does not depend on what is installed on the test machine.
async fn open_without_ffmpeg(data: &Path, downloads: &Path) -> Manager {
    let manager = open(data).await;
    manager
        .update_settings(Settings {
            download_dir: downloads.to_path_buf(),
            ffmpeg_path: data.join("no-ffmpeg.exe").to_string_lossy().into_owned(),
            ..manager.settings()
        })
        .unwrap();
    manager
}

#[tokio::test]
async fn playlist_links_become_stream_downloads_in_the_video_folder() {
    use downpipe_test_server::streams::joined;
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let downloads = tempfile::tempdir().unwrap();
    let manager = open_without_ffmpeg(data.path(), downloads.path()).await;
    assert_eq!(manager.ffmpeg(), None);

    let q = "segments=4&size=3000";
    let task = add_auto(&manager, server.url(&format!("/hls/master.m3u8?{q}")));
    let info = task.record.media.clone().expect("recognized as a stream");
    assert_eq!(info.kind, downpipe_core::http::media::MediaKind::Hls);
    assert_eq!(task.record.category.as_deref(), Some("Video"));

    let done = wait_for(&manager, &task.record.id, |t| {
        t.record.state == TaskState::Completed
    })
    .await;
    let path = done.record.path.clone().unwrap();
    assert_eq!(path, downloads.path().join("Video").join("master.ts"));
    assert_eq!(std::fs::read(&path).unwrap(), joined("v1", q));
    let info = done.record.media.unwrap();
    assert_eq!(info.quality.as_deref(), Some("720p"));
    assert_eq!(info.selection.unwrap().video.unwrap().height, Some(720));
    assert_eq!(done.live.fragments.map(|f| (f.done, f.total)), Some((4, 4)));
}

#[tokio::test]
async fn the_quality_chosen_in_the_dialog_is_downloaded() {
    use downpipe_core::http::media::{MediaKind, MediaSelection, TrackRef};
    use downpipe_test_server::streams::joined;
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let downloads = tempfile::tempdir().unwrap();
    let manager = open_without_ffmpeg(data.path(), downloads.path()).await;

    let q = "segments=3&size=2000&split=1";
    let held = manager
        .add(AddOptions {
            url: server.url(&format!("/dash/manifest.mpd?{q}")),
            file_name: Some("Clip of the week.mp4".into()),
            media: Some(MediaKind::Dash),
            confirm: true,
            ..AddOptions::default()
        })
        .unwrap();
    assert!(held.live.awaiting_confirmation);

    let choices = manager.inspect_media(&held.record.id).await.unwrap();
    assert!(!choices.ffmpeg);
    assert_eq!(choices.selection, None);
    let heights: Vec<_> = choices.options.videos.iter().map(|v| v.height).collect();
    assert_eq!(heights, [Some(720), Some(360)]);
    assert_eq!(choices.options.audios.len(), 1);
    assert!(
        !server
            .state
            .requests()
            .iter()
            .any(|r| r.path.contains("/seg-")),
        "reading the qualities fetches no media"
    );

    let lower = &choices.options.videos[1];
    manager
        .confirm(
            &held.record.id,
            ConfirmOptions {
                start: true,
                media: Some(MediaSelection {
                    video: Some(TrackRef {
                        id: lower.id.clone(),
                        height: lower.height,
                        bandwidth: lower.bandwidth,
                        language: None,
                    }),
                    audio: None,
                }),
                ..ConfirmOptions::default()
            },
        )
        .unwrap();
    let done = wait_for(&manager, &held.record.id, |t| {
        t.record.state == TaskState::Completed
    })
    .await;
    let path = done.record.path.clone().unwrap();
    assert_eq!(path.file_name().unwrap(), "Clip of the week.mp4");
    assert_eq!(std::fs::read(&path).unwrap(), joined("v0", q));
    assert!(path.with_file_name("Clip of the week.audio.m4a").is_file());
    assert_eq!(done.record.media.unwrap().quality.as_deref(), Some("360p"));
    assert!(
        done.live
            .notices
            .iter()
            .any(|n| n.contains("FFmpeg was not found")),
        "{:?}",
        done.live.notices
    );

    let file = manager.add(AddOptions {
        url: server.url("/file/100"),
        dir: Some(downloads.path().to_path_buf()),
        ..AddOptions::default()
    });
    let plain = file.unwrap();
    assert!(matches!(
        manager.inspect_media(&plain.record.id).await,
        Err(CoreError::InvalidState { .. })
    ));
}

#[tokio::test]
async fn stream_downloads_resume_after_a_restart_with_the_same_quality() {
    use downpipe_test_server::streams::joined;
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let downloads = tempfile::tempdir().unwrap();
    let q = "segments=10&size=20000&rate=40000";
    let url = server.url(&format!("/hls/v1/index.m3u8?{q}"));

    let (id, before) = {
        let manager = open_without_ffmpeg(data.path(), downloads.path()).await;
        let id = add_auto(&manager, url).record.id;
        let running = wait_for(&manager, &id, |t| {
            t.live.fragments.is_some_and(|f| f.done >= 3)
        })
        .await;
        manager.shutdown().await;
        (id, running.live.fragments.unwrap().done)
    };

    server.state.reset_stats();
    let manager = open(data.path()).await;
    let done = wait_for(&manager, &id, |t| t.record.state == TaskState::Completed).await;
    assert_eq!(
        std::fs::read(done.record.path.unwrap()).unwrap(),
        joined("v1", q)
    );
    let fetched = server
        .state
        .requests()
        .iter()
        .filter(|r| r.path.contains("/seg-"))
        .count();
    assert!(
        fetched <= 10 - before,
        "{fetched} fragments fetched again after {before} were done"
    );
}

#[tokio::test]
async fn removing_a_stream_download_deletes_its_fragments() {
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let downloads = tempfile::tempdir().unwrap();
    let manager = open_without_ffmpeg(data.path(), downloads.path()).await;
    let task = add(
        &manager,
        server.url("/hls/v0/index.m3u8?segments=10&size=20000&rate=40000"),
        downloads.path(),
    );
    let id = task.record.id;
    wait_for(&manager, &id, |t| {
        t.live.fragments.is_some_and(|f| f.done >= 1)
    })
    .await;
    manager.pause(&id).unwrap();
    let paused = wait_for(&manager, &id, |t| t.record.state == TaskState::Paused).await;
    let work = meta::part_path(&paused.record.path.unwrap());
    assert!(work.is_dir());
    manager.remove(&id, true).unwrap();
    assert!(!work.exists());
    assert_eq!(
        std::fs::read_dir(downloads.path()).unwrap().count(),
        0,
        "nothing left behind"
    );
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[tokio::test]
async fn pause_until_resumes_at_its_time_and_survives_a_restart() {
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let url = server.url(&format!("/file/{}?rate=2097152", 8 * MIB));
    let at = unix_now() + 3600;

    let id = {
        let manager = open(data.path()).await;
        let id = add(&manager, url, out.path()).record.id;
        wait_for(&manager, &id, |t| t.record.downloaded > 256 * KIB).await;
        manager.pause_until(&id, at).unwrap();
        let paused = wait_for(&manager, &id, |t| t.record.state == TaskState::Paused).await;
        assert_eq!(paused.record.resume_at, Some(at));
        // Not yet: nothing happens before the time.
        manager.resume_due(at - 1);
        assert_eq!(manager.get(&id).unwrap().record.state, TaskState::Paused);
        manager.shutdown().await;
        id
    };

    // The time is stored with the download.
    let manager = open(data.path()).await;
    let task = manager.get(&id).unwrap();
    assert_eq!(task.record.state, TaskState::Paused);
    assert_eq!(task.record.resume_at, Some(at));
    manager.resume_due(at);
    let done = wait_for(&manager, &id, |t| t.record.state == TaskState::Completed).await;
    assert_eq!(done.record.resume_at, None);
}

#[tokio::test]
async fn pause_until_a_passed_time_resumes_by_itself() {
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let manager = open(data.path()).await;
    let id = manager
        .add(AddOptions {
            url: server.url(&format!("/file/{}", MIB)),
            dir: Some(out.path().to_path_buf()),
            paused: true,
            ..AddOptions::default()
        })
        .unwrap()
        .record
        .id;
    // The manager's own timer picks it up within a few seconds.
    manager.pause_until(&id, unix_now() - 1).unwrap();
    wait_for(&manager, &id, |t| t.record.state == TaskState::Completed).await;
}

#[tokio::test]
async fn pausing_or_resuming_by_hand_cancels_the_time() {
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let manager = open(data.path()).await;
    let id = manager
        .add(AddOptions {
            url: server.url(&format!("/file/{}", MIB)),
            dir: Some(out.path().to_path_buf()),
            paused: true,
            ..AddOptions::default()
        })
        .unwrap()
        .record
        .id;
    let at = unix_now() + 3600;
    manager.pause_until(&id, at).unwrap();
    manager.pause(&id).unwrap();
    assert_eq!(manager.get(&id).unwrap().record.resume_at, None);
    manager.resume_due(at);
    assert_eq!(manager.get(&id).unwrap().record.state, TaskState::Paused);

    manager.pause_until(&id, at).unwrap();
    manager.resume(&id).unwrap();
    let done = wait_for(&manager, &id, |t| t.record.state == TaskState::Completed).await;
    assert_eq!(done.record.resume_at, None);
    // A finished download has nothing to pause.
    assert!(matches!(
        manager.pause_until(&id, at),
        Err(CoreError::InvalidState { .. })
    ));
}

#[tokio::test]
async fn a_download_keeps_its_own_speed_limit() {
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let manager = open(data.path()).await;
    let id = manager
        .add(AddOptions {
            url: server.url(&format!("/file/{}", 4 * MIB)),
            dir: Some(out.path().to_path_buf()),
            paused: true,
            ..AddOptions::default()
        })
        .unwrap()
        .record
        .id;
    manager.set_speed_limit(&id, Some(256 * KIB)).unwrap();
    manager.resume(&id).unwrap();
    tokio::time::sleep(Duration::from_millis(600)).await;
    let early = manager.get(&id).unwrap();
    assert_eq!(early.record.speed_limit, Some(256 * KIB));
    assert!(
        early.record.downloaded < 2 * MIB,
        "limit not applied: {}",
        early.record.downloaded
    );
    // Lifted while it runs: the rest comes at full speed.
    manager.set_speed_limit(&id, Some(0)).unwrap();
    assert_eq!(manager.get(&id).unwrap().record.speed_limit, None);
    wait_for(&manager, &id, |t| t.record.state == TaskState::Completed).await;

    // Stored with the download.
    manager.set_speed_limit(&id, Some(MIB)).unwrap();
    drop(manager);
    let manager = open(data.path()).await;
    assert_eq!(manager.get(&id).unwrap().record.speed_limit, Some(MIB));
}
