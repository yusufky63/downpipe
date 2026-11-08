//! Downpipe's own update check and installer download, against a stand-in
//! for GitHub's release pages.

use std::path::Path;

use downpipe_http::DownloadError;
use downpipe_providers::update::{self, REPO, Update};
use downpipe_test_server::TestServer;
use sha2::Digest as _;

fn write(path: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(sha2::Sha256::digest(bytes))
}

#[tokio::test]
async fn a_newer_release_is_found_and_its_installer_checked() {
    let server = TestServer::start().await.unwrap();
    let files = tempfile::tempdir().unwrap();
    server.serve_files(files.path());
    let base = url::Url::parse(&server.url("/releases/")).unwrap();
    let client = reqwest::Client::new();
    let repo = files.path().join(REPO);
    write(&repo.join("latest"), b"v0.9.0\n");

    assert_eq!(
        update::check(&client, &base, "0.9.0").await.unwrap(),
        None,
        "up to date"
    );
    let found = update::check(&client, &base, "0.5.5")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.version, "0.9.0");
    assert!(
        found
            .page
            .ends_with("/yusufky63/downpipe/releases/tag/v0.9.0"),
        "{}",
        found.page
    );

    let name = "Downpipe_0.9.0_x64-setup.exe";
    let setup = b"installer".repeat(2000);
    write(&repo.join("v0.9.0").join(name), &setup);
    write(
        &repo.join("v0.9.0/SHA256SUMS.txt"),
        format!("{} *{name}\n{} *other.zip\n", sha256(&setup), sha256(b"x")).as_bytes(),
    );
    let windows = Update {
        installer: Some(name.into()),
        ..found.clone()
    };
    let dir = files.path().join("updates");
    let path = update::download_installer(&client, &base, &windows, &dir)
        .await
        .unwrap();
    assert_eq!(path, dir.join(name));
    assert_eq!(std::fs::read(&path).unwrap(), setup);

    // A file that is not what the release lists is refused and removed.
    std::fs::remove_file(&path).unwrap();
    write(&repo.join("v0.9.0").join(name), b"tampered");
    let err = update::download_installer(&client, &base, &windows, &dir)
        .await
        .unwrap_err();
    assert!(
        matches!(err, DownloadError::ChecksumMismatch { .. }),
        "{err:?}"
    );
    assert_eq!(
        std::fs::read_dir(&dir).unwrap().count(),
        0,
        "nothing left behind"
    );

    // Without an installer for this system there is nothing to download.
    let page_only = Update {
        installer: None,
        ..found
    };
    assert!(
        update::download_installer(&client, &base, &page_only, &dir)
            .await
            .is_err()
    );
}
