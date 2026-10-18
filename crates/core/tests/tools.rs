//! Downpipe's own tools through the manager: installing on request and the
//! daily yt-dlp update, against releases on a local server (no network).

use std::path::Path;

use downpipe_core::Manager;
use downpipe_core::providers::install::{self, Tool};
use downpipe_test_server::TestServer;
use sha2::Digest as _;

fn write(path: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

/// Publish `bytes` as yt-dlp's latest release `tag`.
fn publish(files: &Path, tag: &str, bytes: &[u8]) {
    let asset =
        install::asset_name(Tool::YtDlp, std::env::consts::OS, std::env::consts::ARCH).unwrap();
    let repo = files.join("yt-dlp/yt-dlp");
    write(&repo.join("latest"), tag.as_bytes());
    write(&repo.join(tag).join(&asset), bytes);
    let hash = hex::encode(sha2::Sha256::digest(bytes));
    write(
        &repo.join(tag).join("SHA2-256SUMS"),
        format!("{hash}  {asset}\n").as_bytes(),
    );
}

#[tokio::test]
async fn the_daily_check_updates_only_downpipes_own_yt_dlp() {
    let server = TestServer::start().await.unwrap();
    let files = tempfile::tempdir().unwrap();
    server.serve_files(files.path());
    let dir = tempfile::tempdir().unwrap();
    let manager = Manager::open(&dir.path().join("data")).await.unwrap();
    manager.set_tool_releases(url::Url::parse(&server.url("/releases/")).unwrap());
    let exe = dir.path().join("data/tools").join(if cfg!(windows) {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    });

    publish(files.path(), "2026.08.19", b"first build");
    let seen = std::sync::Mutex::new(0);
    manager
        .install_site_tools(&[Tool::YtDlp], false, |_| *seen.lock().unwrap() += 1)
        .await
        .unwrap();
    assert_eq!(std::fs::read(&exe).unwrap(), b"first build");
    assert!(*seen.lock().unwrap() > 0, "progress was reported");

    // Nothing new: nothing downloaded, the look recorded.
    manager.check_yt_dlp_now().await;
    assert_eq!(std::fs::read(&exe).unwrap(), b"first build");
    let checked = manager.site_tools().await.yt_dlp_checked;
    assert!(checked.is_some());

    // A newer release replaces Downpipe's copy, checked like an install.
    publish(files.path(), "2026.09.01", b"second build");
    manager.check_yt_dlp_now().await;
    assert_eq!(std::fs::read(&exe).unwrap(), b"second build");

    // A release whose file does not match its hash leaves the copy alone.
    publish(files.path(), "2026.09.15", b"third build");
    write(
        &files.path().join("yt-dlp/yt-dlp/2026.09.15").join(
            install::asset_name(Tool::YtDlp, std::env::consts::OS, std::env::consts::ARCH).unwrap(),
        ),
        b"tampered",
    );
    manager.check_yt_dlp_now().await;
    assert_eq!(std::fs::read(&exe).unwrap(), b"second build");

    // Switched off: not touched.
    let settings = manager.settings();
    manager
        .update_settings(downpipe_core::Settings {
            yt_dlp_auto_update: false,
            ..settings.clone()
        })
        .unwrap();
    publish(files.path(), "2026.10.01", b"fourth build");
    manager.check_yt_dlp_now().await;
    assert_eq!(std::fs::read(&exe).unwrap(), b"second build");
    manager.update_settings(settings).unwrap();

    // A copy the user put in its place is theirs: never replaced.
    std::fs::write(&exe, b"the user's own").unwrap();
    manager.check_yt_dlp_now().await;
    assert_eq!(std::fs::read(&exe).unwrap(), b"the user's own");
}

#[tokio::test]
async fn only_missing_skips_tools_found_meanwhile() {
    let server = TestServer::start().await.unwrap();
    let files = tempfile::tempdir().unwrap();
    server.serve_files(files.path());
    let dir = tempfile::tempdir().unwrap();
    let manager = Manager::open(&dir.path().join("data")).await.unwrap();
    manager.set_tool_releases(url::Url::parse(&server.url("/releases/")).unwrap());
    // A yt-dlp set in the settings counts as found.
    let own = dir.path().join("my-yt-dlp");
    std::fs::write(&own, b"mine").unwrap();
    manager
        .update_settings(downpipe_core::Settings {
            yt_dlp_path: own.to_string_lossy().into_owned(),
            ..manager.settings()
        })
        .unwrap();
    // Nothing is published: a download would fail.
    manager
        .install_site_tools(&[Tool::YtDlp], true, |_| {})
        .await
        .unwrap();
    assert!(!dir.path().join("data/tools").exists());
}
