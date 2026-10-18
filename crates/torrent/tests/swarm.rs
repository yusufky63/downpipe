//! Downloads from a seeder on loopback: no DHT, no trackers, no internet.

use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;
use std::time::Duration;

use downpipe_http::{Outcome, Phase};
use downpipe_torrent::testing::{LocalSeeder, content};
use downpipe_torrent::{Limits, NetConfig, SeedPolicy, Source, TorrentRequest, Torrents};

const LIMIT: Duration = Duration::from_secs(60);

fn torrents(state: &Path) -> Torrents {
    Torrents::new(
        NetConfig {
            bind: IpAddr::V4(Ipv4Addr::LOCALHOST),
            dht: false,
            lsd: false,
            state_dir: state.to_path_buf(),
            ..NetConfig::default()
        },
        Limits::default(),
    )
}

fn source(seeder: &LocalSeeder, state: &Path) -> Source {
    Source {
        link: seeder.magnet(),
        headers: Vec::new(),
        client: None,
        metainfo: state.join("t.torrent"),
    }
}

fn files() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("a.bin", content(300_000, 1)),
        ("docs/b.txt", content(70_000, 2)),
        ("docs/c.bin", content(45_001, 3)),
    ]
}

#[tokio::test(flavor = "multi_thread")]
async fn magnet_metadata_comes_from_the_peer_and_is_kept() {
    let seeder = LocalSeeder::start("Pack", &files(), 0).await;
    let state = tempfile::tempdir().unwrap();
    let t = torrents(state.path());
    let meta = tokio::time::timeout(LIMIT, t.metadata(&source(&seeder, state.path())))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(meta.info_hash, seeder.info_hash);
    assert_eq!(meta.name, "Pack");
    let mut paths: Vec<_> = meta.files.iter().map(|f| f.path.as_str()).collect();
    paths.sort_unstable();
    assert_eq!(paths, ["a.bin", "docs/b.txt", "docs/c.bin"]);
    assert_eq!(meta.total, 300_000 + 70_000 + 45_001);
    assert!(meta.is_folder());
    // Kept, so the next look needs no peer.
    assert!(state.path().join("t.torrent").exists());
    seeder.stop().await;
    let again = t
        .metadata(&source_without_peers(state.path()))
        .await
        .unwrap();
    assert_eq!(again, meta);
    t.shutdown().await;
}

fn source_without_peers(state: &Path) -> Source {
    Source {
        link: "magnet:?xt=urn:btih:0000000000000000000000000000000000000000".into(),
        headers: Vec::new(),
        client: None,
        metainfo: state.join("t.torrent"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn downloads_chosen_files_then_seeds_until_the_limit() {
    let all = files();
    let seeder = LocalSeeder::start("Pack", &all, 0).await;
    let state = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let t = torrents(state.path());
    let src = source(&seeder, state.path());
    let meta = t.metadata(&src).await.unwrap();
    let index = |p: &str| meta.files.iter().position(|f| f.path == p).unwrap();
    let chosen = vec![index("a.bin"), index("docs/c.bin")];

    let handle = t.start(TorrentRequest {
        source: src,
        dir: out.path().to_path_buf(),
        only_files: Some(chosen),
        seed: SeedPolicy {
            enabled: true,
            ratio: None,
            time: Some(Duration::from_secs(2)),
        },
        uploaded: 0,
    });
    let progress = handle.progress();
    let outcome = tokio::time::timeout(LIMIT, handle.wait())
        .await
        .unwrap()
        .unwrap();
    let Outcome::Completed { path, bytes, .. } = outcome else {
        panic!("expected completed, got {outcome:?}");
    };
    assert_eq!(path, out.path().join("Pack"));
    assert_eq!(bytes, 300_000 + 45_001);
    let last = progress.borrow().clone();
    assert_eq!(last.phase, Phase::Done);
    assert!(last.torrent.is_some());
    assert_eq!(std::fs::read(path.join("a.bin")).unwrap(), all[0].1);
    assert_eq!(
        std::fs::read(path.join("docs").join("c.bin")).unwrap(),
        all[2].1
    );

    // Seeding for two seconds, then it stops by itself.
    let mut seeding = t
        .seeding(&seeder.info_hash)
        .expect("seeding after finishing");
    tokio::time::timeout(LIMIT, seeding.wait_for(|s| !s.seeding))
        .await
        .unwrap()
        .unwrap();
    assert!(t.seeding(&seeder.info_hash).is_none());
    t.shutdown().await;
    seeder.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn pause_keeps_the_data_and_resume_finishes() {
    let data = content(2_000_000, 7);
    // 400 KB/s: five seconds in all, time enough to pause half way.
    let seeder = LocalSeeder::start("big.bin", &[("big.bin", data.clone())], 400_000).await;
    let state = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let t = torrents(state.path());
    let request = TorrentRequest {
        source: source(&seeder, state.path()),
        dir: out.path().to_path_buf(),
        only_files: None,
        seed: SeedPolicy::NONE,
        uploaded: 0,
    };

    let handle = t.start(request.clone());
    let mut progress = handle.progress();
    let control = handle.control();
    tokio::time::timeout(LIMIT, progress.wait_for(|p| p.downloaded > 200_000))
        .await
        .unwrap()
        .unwrap();
    control.pause();
    let paused = handle.wait().await.unwrap();
    let Outcome::Paused { downloaded, .. } = paused else {
        panic!("expected paused, got {paused:?}");
    };
    assert!(downloaded > 0 && downloaded < 2_000_000, "{downloaded}");

    let handle = t.start(request);
    let outcome = tokio::time::timeout(LIMIT, handle.wait())
        .await
        .unwrap()
        .unwrap();
    let Outcome::Completed { path, bytes, .. } = outcome else {
        panic!("expected completed, got {outcome:?}");
    };
    // One file: no folder of its own.
    assert_eq!(path, out.path().join("big.bin"));
    assert_eq!(bytes, 2_000_000);
    assert_eq!(std::fs::read(&path).unwrap(), data);
    // No seeding was asked for.
    assert!(t.seeding(&seeder.info_hash).is_none());
    t.shutdown().await;
    seeder.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn cancel_deletes_what_it_wrote() {
    let seeder = LocalSeeder::start("Pack", &files(), 100_000).await;
    let state = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    std::fs::write(out.path().join("unrelated.txt"), b"keep me").unwrap();
    let t = torrents(state.path());
    let handle = t.start(TorrentRequest {
        source: source(&seeder, state.path()),
        dir: out.path().to_path_buf(),
        only_files: None,
        seed: SeedPolicy::NONE,
        uploaded: 0,
    });
    let mut progress = handle.progress();
    let control = handle.control();
    tokio::time::timeout(LIMIT, progress.wait_for(|p| p.downloaded > 0))
        .await
        .unwrap()
        .unwrap();
    control.cancel();
    assert!(matches!(handle.wait().await.unwrap(), Outcome::Canceled));
    assert!(!out.path().join("Pack").exists());
    assert!(out.path().join("unrelated.txt").exists());
    t.shutdown().await;
    seeder.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_magnet_without_any_way_to_find_peers_fails_clearly() {
    let state = tempfile::tempdir().unwrap();
    let t = torrents(state.path());
    let err = t
        .metadata(&source_without_peers(state.path()))
        .await
        .unwrap_err();
    assert_eq!(err.kind(), "torrent");
    assert!(err.to_string().contains("DHT is off"), "{err}");
    t.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_torrent_link_on_a_web_server_is_fetched() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let seeder = LocalSeeder::start("Pack", &files(), 0).await;
    let body = seeder.torrent.clone();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = vec![0u8; 4096];
        let n = stream.read(&mut request).await.unwrap();
        let request = String::from_utf8_lossy(&request[..n]).to_string();
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-bittorrent\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(head.as_bytes()).await.unwrap();
        stream.write_all(&body).await.unwrap();
        request
    });
    let state = tempfile::tempdir().unwrap();
    let t = torrents(state.path());
    let meta = t
        .metadata(&Source {
            link: format!("http://{addr}/files/pack.torrent"),
            headers: vec![("Referer".into(), "https://example.org/".into())],
            client: Some(reqwest::Client::new()),
            metainfo: state.path().join("web.torrent"),
        })
        .await
        .unwrap();
    assert_eq!(meta.info_hash, seeder.info_hash);
    let request = server.await.unwrap();
    assert!(request.starts_with("GET /files/pack.torrent "), "{request}");
    assert!(
        request
            .to_ascii_lowercase()
            .contains("referer: https://example.org/")
    );
    assert!(state.path().join("web.torrent").exists());
    t.shutdown().await;
    seeder.stop().await;
}
