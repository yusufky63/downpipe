//! Backup and restore through a real `Manager`, and IDM entries added the
//! same way.

use std::path::Path;
use std::time::Duration;

use downpipe_core::backup::{Backup, ImportChoice};
use downpipe_core::http::{EngineConfig, KIB};
use downpipe_core::idm;
use downpipe_core::{AddOptions, Manager, Queue, Settings, TaskSnapshot, TaskState};
use downpipe_test_server::TestServer;

fn config() -> EngineConfig {
    EngineConfig {
        max_connections: 2,
        initial_connections: 2,
        min_split_size: 64 * KIB,
        checkpoint_interval: Duration::from_millis(100),
        ..EngineConfig::default()
    }
}

async fn open(data: &Path, downloads: &Path) -> Manager {
    let manager = Manager::open_with(data, config()).await.unwrap();
    manager
        .update_settings(Settings {
            download_dir: downloads.to_path_buf(),
            ..manager.settings()
        })
        .unwrap();
    manager
}

async fn wait_for(manager: &Manager, id: &str, cond: impl Fn(&TaskSnapshot) -> bool) {
    tokio::time::timeout(Duration::from_secs(20), async {
        while !manager.get(id).is_some_and(|t| cond(&t)) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("condition not reached in time");
}

#[tokio::test]
async fn a_backup_restores_settings_and_the_list_without_secrets() {
    let server = TestServer::start().await.unwrap();
    let (data, out) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let source = open(data.path(), out.path()).await;
    source
        .update_settings(Settings {
            max_concurrent_downloads: 5,
            queues: vec![
                Queue {
                    id: "main".into(),
                    ..Queue::default()
                },
                Queue {
                    id: String::new(),
                    name: "Night".into(),
                    max_concurrent: 2,
                },
            ],
            ..source.settings()
        })
        .unwrap();

    let done = source
        .add(AddOptions {
            url: server.url("/file/4096"),
            dir: Some(out.path().to_path_buf()),
            file_name: Some("done.bin".into()),
            headers: vec![("Authorization".into(), "Bearer very-secret".into())],
            ..AddOptions::default()
        })
        .unwrap();
    wait_for(&source, &done.record.id, |t| {
        t.record.state == TaskState::Completed
    })
    .await;
    let waiting = source
        .add(AddOptions {
            url: format!("{}?token=1", server.url("/file/8192"))
                .replace("http://", "http://me:hunter2@"),
            dir: Some(out.path().to_path_buf()),
            paused: true,
            queue: Some("night".into()),
            ..AddOptions::default()
        })
        .unwrap();

    let json = source.backup("9.9.9").to_json();
    assert!(
        !json.contains("very-secret") && !json.contains("Authorization"),
        "no headers"
    );
    assert!(!json.contains("hunter2"), "no credentials");
    assert!(!json.to_lowercase().contains("cookie"));

    // Restore into a fresh installation.
    let (data2, out2) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let target = open(data2.path(), out2.path()).await;
    let backup = Backup::parse(json.as_bytes()).unwrap();
    let summary = target.backup_summary(&backup);
    assert_eq!(
        (summary.downloads, summary.new_downloads, summary.queues),
        (2, 2, 1)
    );
    assert_eq!(summary.app_version, "9.9.9");

    let report = target
        .import_backup(
            &backup,
            ImportChoice {
                settings: true,
                downloads: true,
            },
        )
        .unwrap();
    assert!(report.settings);
    assert_eq!((report.added, report.duplicates, report.invalid), (2, 0, 0));
    let settings = target.settings();
    assert_eq!(settings.max_concurrent_downloads, 5);
    assert!(settings.queue("night").is_some());

    let list = target.list();
    let restored_done = list
        .iter()
        .find(|t| t.record.state == TaskState::Completed)
        .unwrap();
    assert_eq!(
        restored_done.record.path.as_deref(),
        Some(out.path().join("done.bin").as_path())
    );
    let restored_waiting = list
        .iter()
        .find(|t| t.record.url == waiting.record.url)
        .unwrap();
    assert_eq!(restored_waiting.record.state, TaskState::Paused);
    assert_eq!(restored_waiting.record.queue, "night");
    assert_ne!(restored_waiting.record.id, waiting.record.id);

    // A second import adds nothing.
    let again = target
        .import_backup(
            &backup,
            ImportChoice {
                settings: false,
                downloads: true,
            },
        )
        .unwrap();
    assert_eq!((again.added, again.duplicates), (0, 2));
    assert_eq!(target.backup_summary(&backup).new_downloads, 0);
}

#[tokio::test]
async fn importing_never_touches_existing_files_or_starts_downloads() {
    let server = TestServer::start().await.unwrap();
    let (data, out) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let manager = open(data.path(), out.path()).await;
    std::fs::write(out.path().join("taken.bin"), b"keep me").unwrap();
    let text = serde_json::json!({
        "format": "downpipe-backup",
        "version": 1,
        "settings": {},
        "downloads": [
            {"url": server.url("/file/2048"), "dir": out.path(), "file_name": "taken.bin", "state": "downloading"},
            {"url": server.url("/file/2048"), "dir": out.path(), "file_name": "twice.bin", "state": "queued"},
            {"url": "javascript:alert(1)", "dir": out.path(), "state": "paused"},
        ],
    })
    .to_string();
    let backup = Backup::parse(text.as_bytes()).unwrap();
    assert_eq!(backup.invalid, 1);
    let report = manager
        .import_backup(
            &backup,
            ImportChoice {
                settings: false,
                downloads: true,
            },
        )
        .unwrap();
    assert_eq!((report.added, report.duplicates, report.invalid), (1, 1, 1));
    tokio::time::sleep(Duration::from_millis(300)).await;
    let list = manager.list();
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].record.state,
        TaskState::Paused,
        "nothing starts by itself"
    );
    assert_eq!(
        std::fs::read(out.path().join("taken.bin")).unwrap(),
        b"keep me"
    );

    // Resumed, it saves next to the existing file instead of over it.
    manager.resume(&list[0].record.id).unwrap();
    wait_for(&manager, &list[0].record.id, |t| {
        t.record.state == TaskState::Completed
    })
    .await;
    assert_eq!(
        std::fs::read(out.path().join("taken.bin")).unwrap(),
        b"keep me"
    );
    let path = manager
        .get(&list[0].record.id)
        .unwrap()
        .record
        .path
        .unwrap();
    assert_ne!(path, out.path().join("taken.bin"));
}

#[tokio::test]
async fn idm_entries_are_added_paused_and_once() {
    let (data, out) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let manager = open(data.path(), out.path()).await;
    let entries = idm::parse_list(
        "https://example.org/a.zip\n<\nhttps://example.org/b.pdf\nfilename: report.pdf\ncookie: sid=1\n>\n",
    );
    assert_eq!(entries.len(), 2);
    assert_eq!(
        manager.listed(entries.iter().map(|e| e.url.as_str())),
        [false, false]
    );
    let report = manager
        .import_downloads(entries.iter().map(idm::IdmEntry::to_download).collect())
        .unwrap();
    assert_eq!(report.added, 2);
    assert_eq!(
        manager.listed(entries.iter().map(|e| e.url.as_str())),
        [true, true]
    );
    let list = manager.list();
    assert!(list.iter().all(|t| t.record.state == TaskState::Paused));
    let pdf = list
        .iter()
        .find(|t| t.record.url.ends_with("b.pdf"))
        .unwrap();
    assert_eq!(pdf.record.file_name.as_deref(), Some("report.pdf"));
    // No folder in the export: the category's folder in the download folder.
    assert!(
        pdf.record.dir.starts_with(out.path()),
        "{:?}",
        pdf.record.dir
    );

    let again = manager
        .import_downloads(entries.iter().map(idm::IdmEntry::to_download).collect())
        .unwrap();
    assert_eq!((again.added, again.duplicates), (0, 2));
}
