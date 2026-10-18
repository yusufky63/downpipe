//! BitTorrent downloads through the manager, from a seeder on loopback (no
//! DHT, no trackers, no internet).
//!
//! `harness = false`: `main` sets `DOWNPIPE_TORRENT_BIND` before any thread
//! exists (so the manager's session listens on loopback only, never on the
//! network), then runs the cases.

use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::time::Duration;

use downpipe_core::{
    AddOptions, ConfirmOptions, CoreError, Manager, Settings, TaskSnapshot, TaskState,
    TorrentSettings,
};
use downpipe_torrent::testing::{LocalSeeder, content};

type Case = fn() -> Pin<Box<dyn Future<Output = ()>>>;

fn main() {
    // SAFETY: no other thread runs yet; the runtime is built below.
    unsafe { std::env::set_var("DOWNPIPE_TORRENT_BIND", "127.0.0.1") };
    let cases: &[(&str, Case)] = &[
        ("magnet_lists_files_then_downloads_the_chosen_ones", || {
            Box::pin(magnet_lists_files_then_downloads_the_chosen_ones())
        }),
        (
            "torrent_files_are_kept_and_duplicates_found_by_hash",
            || Box::pin(torrent_files_are_kept_and_duplicates_found_by_hash()),
        ),
        ("pause_resume_and_remove_with_files", || {
            Box::pin(pause_resume_and_remove_with_files())
        }),
        ("no_seeding_when_switched_off", || {
            Box::pin(no_seeding_when_switched_off())
        }),
        ("a_magnet_nobody_answers_fails_clearly", || {
            Box::pin(a_magnet_nobody_answers_fails_clearly())
        }),
    ];
    let filter: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .collect();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let mut failed = Vec::new();
    let mut ran = 0;
    for (name, case) in cases {
        if !filter.is_empty() && !filter.iter().any(|f| name.contains(f.as_str())) {
            continue;
        }
        ran += 1;
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| runtime.block_on(case()))) {
            Ok(()) => println!("test {name} ... ok"),
            Err(_) => {
                println!("test {name} ... FAILED");
                failed.push(*name);
            }
        }
    }
    println!(
        "\ntest result: {}. {} passed; {} failed",
        if failed.is_empty() { "ok" } else { "FAILED" },
        ran - failed.len(),
        failed.len()
    );
    if !failed.is_empty() {
        std::process::exit(1);
    }
}

/// A manager whose torrents find peers only through the links' `x.pe`.
async fn open(data: &Path, torrent: TorrentSettings) -> Manager {
    let manager = Manager::open(data).await.expect("open manager");
    manager
        .update_settings(Settings {
            torrent: TorrentSettings {
                dht: false,
                lsd: false,
                upnp: false,
                ..torrent
            },
            ..manager.settings()
        })
        .unwrap();
    manager
}

fn files() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("movie.mkv", content(400_000, 1)),
        ("extras/notes.txt", content(30_000, 2)),
        ("extras/cover.jpg", content(55_555, 3)),
    ]
}

async fn wait_for(
    manager: &Manager,
    id: &str,
    cond: impl Fn(&TaskSnapshot) -> bool,
) -> TaskSnapshot {
    tokio::time::timeout(Duration::from_secs(60), async {
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
            task.map(|t| (t.record.state, t.record.downloaded, t.record.error))
        )
    })
}

async fn magnet_lists_files_then_downloads_the_chosen_ones() {
    let all = files();
    let seeder = LocalSeeder::start("Holiday", &all, 0).await;
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let manager = open(data.path(), TorrentSettings::default()).await;

    let task = manager
        .add(AddOptions {
            url: seeder.magnet(),
            dir: Some(out.path().to_path_buf()),
            confirm: true,
            from_app: true,
            ..AddOptions::default()
        })
        .unwrap();
    assert_eq!(task.record.state, TaskState::Paused);
    assert!(task.live.awaiting_confirmation);
    let info = task.record.torrent.clone().expect("a torrent download");
    assert_eq!(info.info_hash, seeder.info_hash);
    assert_eq!(info.name.as_deref(), Some("test"), "the link's dn");
    assert!(task.record.url.starts_with("magnet:?xt=urn:btih:"));

    // The same torrent again, by another link: found by its hash.
    let again = manager.add(AddOptions {
        url: format!("magnet:?xt=urn:btih:{}", seeder.info_hash.to_uppercase()),
        ..AddOptions::default()
    });
    assert!(matches!(again, Err(CoreError::Duplicate(id)) if id == task.record.id));

    let choices = manager.inspect_torrent(&task.record.id).await.unwrap();
    assert_eq!(choices.name, "Holiday");
    assert!(choices.folder);
    assert_eq!(choices.files.len(), 3);
    assert_eq!(choices.total, 400_000 + 30_000 + 55_555);
    let learned = manager.get(&task.record.id).unwrap();
    let info = learned.record.torrent.clone().unwrap();
    assert_eq!(info.name.as_deref(), Some("Holiday"));
    assert_eq!(info.files, 3);
    assert_eq!(learned.record.file_name.as_deref(), Some("Holiday"));

    let index = |p: &str| choices.files.iter().find(|f| f.path == p).unwrap().index;
    let chosen = vec![index("movie.mkv"), index("extras/cover.jpg")];
    // Nothing chosen is refused.
    let empty = manager.confirm(
        &task.record.id,
        ConfirmOptions {
            files: Some(Vec::new()),
            start: true,
            ..ConfirmOptions::default()
        },
    );
    assert!(matches!(empty, Err(CoreError::InvalidSetting(_))));
    manager
        .confirm(
            &task.record.id,
            ConfirmOptions {
                files: Some(chosen.clone()),
                start: true,
                ..ConfirmOptions::default()
            },
        )
        .unwrap();

    let done = wait_for(&manager, &task.record.id, |t| {
        t.record.state == TaskState::Completed
    })
    .await;
    let folder = out.path().join("Holiday");
    assert_eq!(done.record.path.as_deref(), Some(folder.as_path()));
    assert_eq!(done.record.total, Some(400_000 + 55_555));
    let mut chosen_sorted = chosen;
    chosen_sorted.sort_unstable();
    assert_eq!(
        done.record.torrent.as_ref().unwrap().only_files,
        Some(chosen_sorted)
    );
    assert_eq!(std::fs::read(folder.join("movie.mkv")).unwrap(), all[0].1);
    assert_eq!(
        std::fs::read(folder.join("extras").join("cover.jpg")).unwrap(),
        all[2].1
    );

    // Seeding by default, shown in the list, until stopped.
    let seeding = wait_for(&manager, &task.record.id, |t| {
        t.live.swarm.as_ref().is_some_and(|s| s.seeding)
    })
    .await;
    assert_eq!(seeding.record.state, TaskState::Completed);
    manager.stop_seeding(&task.record.id).unwrap();
    wait_for(&manager, &task.record.id, |t| t.live.swarm.is_none()).await;
    assert!(matches!(
        manager.stop_seeding(&task.record.id),
        Err(CoreError::InvalidState { .. })
    ));

    // The record survives a restart with what it knew.
    manager.shutdown().await;
    drop(manager);
    let manager = open(data.path(), TorrentSettings::default()).await;
    let back = manager.get(&task.record.id).unwrap();
    assert_eq!(back.record.state, TaskState::Completed);
    assert_eq!(
        back.record.torrent,
        done.record.torrent.map(|mut t| {
            t.uploaded = back.record.torrent.as_ref().unwrap().uploaded;
            t
        })
    );
    manager.shutdown().await;
    seeder.stop().await;
}

async fn torrent_files_are_kept_and_duplicates_found_by_hash() {
    let seeder = LocalSeeder::start("Holiday", &files(), 0).await;
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let manager = open(data.path(), TorrentSettings::default()).await;

    let task = manager
        .add_torrent_file(
            &seeder.torrent,
            AddOptions {
                dir: Some(out.path().to_path_buf()),
                paused: true,
                ..AddOptions::default()
            },
        )
        .unwrap();
    let info = task.record.torrent.clone().unwrap();
    assert_eq!(info.info_hash, seeder.info_hash);
    assert_eq!(info.files, 3);
    assert_eq!(task.record.total, Some(400_000 + 30_000 + 55_555));
    assert_eq!(task.record.file_name.as_deref(), Some("Holiday"));

    // Its file list needs no peer: the `.torrent` was kept.
    seeder.stop().await;
    let choices = manager.inspect_torrent(&task.record.id).await.unwrap();
    assert_eq!(choices.files.len(), 3);

    let magnet = manager.add(AddOptions {
        url: format!("magnet:?xt=urn:btih:{}&dn=other", info.info_hash),
        ..AddOptions::default()
    });
    assert!(matches!(magnet, Err(CoreError::Duplicate(_))));
    let copy = manager.add(AddOptions {
        url: format!("magnet:?xt=urn:btih:{}", info.info_hash),
        on_duplicate: downpipe_core::OnDuplicate::AddCopy,
        ..AddOptions::default()
    });
    assert!(matches!(copy, Err(CoreError::InvalidUrl(_))));

    // Not a torrent at all.
    let bad = manager.add_torrent_file(b"<html>", AddOptions::default());
    assert!(matches!(bad, Err(CoreError::Download(e)) if e.kind() == "torrent"));

    let kept = data
        .path()
        .join("torrents")
        .join(format!("{}.torrent", task.record.id));
    assert!(kept.exists());
    manager.remove(&task.record.id, false).unwrap();
    assert!(!kept.exists(), "the .torrent goes with the download");
    manager.shutdown().await;
}

async fn pause_resume_and_remove_with_files() {
    let data_bytes = content(1_500_000, 9);
    let seeder = LocalSeeder::start(
        "Pack",
        &[
            ("big.bin", data_bytes.clone()),
            ("small.txt", content(1_000, 4)),
        ],
        300_000,
    )
    .await;
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    std::fs::write(out.path().join("mine.txt"), b"not the torrent's").unwrap();
    let manager = open(
        data.path(),
        TorrentSettings {
            seed: false,
            ..TorrentSettings::default()
        },
    )
    .await;
    let task = manager
        .add(AddOptions {
            url: seeder.magnet(),
            dir: Some(out.path().to_path_buf()),
            ..AddOptions::default()
        })
        .unwrap();
    let id = task.record.id.clone();
    let running = wait_for(&manager, &id, |t| t.record.downloaded > 100_000).await;
    assert_eq!(running.record.state, TaskState::Downloading);
    assert!(running.live.swarm.as_ref().is_some_and(|s| s.peers >= 1));
    manager.pause(&id).unwrap();
    let paused = wait_for(&manager, &id, |t| t.record.state == TaskState::Paused).await;
    assert!(paused.record.downloaded > 0);

    manager.resume(&id).unwrap();
    let done = wait_for(&manager, &id, |t| t.record.state == TaskState::Completed).await;
    let folder = out.path().join("Pack");
    assert_eq!(std::fs::read(folder.join("big.bin")).unwrap(), data_bytes);
    assert!(done.live.swarm.is_none(), "seeding is off");

    manager.remove(&id, true).unwrap();
    assert!(manager.get(&id).is_none());
    assert!(!folder.exists(), "the torrent's files and folder are gone");
    assert!(out.path().join("mine.txt").exists(), "other files stay");
    manager.shutdown().await;
    seeder.stop().await;
}

async fn no_seeding_when_switched_off() {
    let seeder = LocalSeeder::start("one.iso", &[("one.iso", content(200_000, 5))], 0).await;
    let data = tempfile::tempdir().unwrap();
    let manager = open(
        data.path(),
        TorrentSettings {
            seed: false,
            ..TorrentSettings::default()
        },
    )
    .await;
    // No folder given: the single file's type picks the category folder.
    let task = manager
        .add(AddOptions {
            url: seeder.magnet(),
            confirm: true,
            ..AddOptions::default()
        })
        .unwrap();
    manager.inspect_torrent(&task.record.id).await.unwrap();
    let learned = manager.get(&task.record.id).unwrap();
    assert_eq!(learned.record.category.as_deref(), Some("Disk images"));
    let dir = data.path().join("downloads");
    manager
        .confirm(
            &task.record.id,
            ConfirmOptions {
                dir: Some(dir.clone()),
                start: true,
                ..ConfirmOptions::default()
            },
        )
        .unwrap();
    let done = wait_for(&manager, &task.record.id, |t| {
        t.record.state == TaskState::Completed
    })
    .await;
    assert_eq!(done.record.path, Some(dir.join("one.iso")));
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(manager.get(&task.record.id).unwrap().live.swarm.is_none());
    manager.shutdown().await;
    seeder.stop().await;
}

async fn a_magnet_nobody_answers_fails_clearly() {
    let data = tempfile::tempdir().unwrap();
    let manager = open(data.path(), TorrentSettings::default()).await;
    let bad = manager.add(AddOptions {
        url: "magnet:?dn=no-hash".into(),
        ..AddOptions::default()
    });
    assert!(matches!(bad, Err(CoreError::Download(_))));
    let task = manager
        .add(AddOptions {
            url: "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567".into(),
            ..AddOptions::default()
        })
        .unwrap();
    let failed = wait_for(&manager, &task.record.id, |t| {
        t.record.state == TaskState::Failed
    })
    .await;
    assert_eq!(failed.record.error_kind.as_deref(), Some("torrent"));
    assert!(failed.record.error.unwrap().contains("DHT is off"));
    manager.shutdown().await;
}
