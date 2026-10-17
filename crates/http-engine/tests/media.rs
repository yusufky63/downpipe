//! Stream downloads (HLS, DASH) against the test server's `/hls/` and
//! `/dash/` endpoints, whose fragments are predictable bytes.

use std::path::{Path, PathBuf};
use std::time::Duration;

use downpipe_http::media::{MediaKind, MediaRequest, MediaSelection, TrackRef, mux};
use downpipe_http::{
    DownloadError, DownloadHandle, DownloadRequest, Engine, EngineConfig, KIB, Outcome, Progress,
};
use downpipe_test_server::TestServer;
use downpipe_test_server::streams::joined;

fn config() -> EngineConfig {
    EngineConfig {
        max_connections: 4,
        buffer_size: 16 * KIB as usize,
        stall_timeout: Duration::from_secs(5),
        retry_base_delay: Duration::from_millis(20),
        retry_max_delay: Duration::from_millis(100),
        ..EngineConfig::default()
    }
}

fn engine() -> Engine {
    Engine::new(config()).expect("engine")
}

fn stream(url: String, dir: &Path, kind: MediaKind) -> DownloadRequest {
    let mut req = DownloadRequest::new(url, dir);
    req.media = Some(MediaRequest {
        kind,
        selection: MediaSelection::default(),
        ffmpeg: None,
    });
    req
}

fn files(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn completed(outcome: Outcome) -> PathBuf {
    match outcome {
        Outcome::Completed { path, .. } => path,
        other => panic!("expected completion, got {other:?}"),
    }
}

async fn wait_for(handle: &DownloadHandle, cond: impl Fn(&Progress) -> bool) {
    let mut rx = handle.progress();
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            if cond(&rx.borrow_and_update()) {
                return;
            }
            if rx.changed().await.is_err() {
                return;
            }
        }
    })
    .await
    .expect("condition not reached in time");
}

#[tokio::test]
async fn hls_takes_the_best_quality_and_joins_its_segments() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let q = "segments=5&size=30000";
    let mut req = stream(
        server.url(&format!("/hls/master.m3u8?{q}")),
        dir.path(),
        MediaKind::Hls,
    );
    req.file_name = Some("Clip of the week.mp4".into());
    let handle = engine().start(req);
    let rx = handle.progress();
    let path = completed(handle.wait().await.unwrap());

    // Transport stream segments without FFmpeg stay a transport stream.
    assert_eq!(path.file_name().unwrap(), "Clip of the week.ts");
    assert_eq!(std::fs::read(&path).unwrap(), joined("v1", q));
    assert_eq!(files(dir.path()), ["Clip of the week.ts"]);
    let media = rx.borrow().media.clone().unwrap();
    assert_eq!(media.quality, "720p");
    assert_eq!((media.fragments, media.fragments_done), (5, 5));
    assert_eq!(media.selection.video.unwrap().height, Some(720));
}

#[tokio::test]
async fn hls_playlists_beside_the_master_are_followed() {
    // Many CDNs keep the quality playlists in the master's own folder.
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let q = "segments=3&size=1000&flat=1";
    let path = completed(
        engine()
            .start(stream(
                server.url(&format!("/hls/master.m3u8?{q}")),
                dir.path(),
                MediaKind::Hls,
            ))
            .wait()
            .await
            .unwrap(),
    );
    assert_eq!(std::fs::read(&path).unwrap(), joined("v1", q));
    let requests = server.state.requests();
    assert!(requests.iter().any(|r| r.path == "/hls/v1.m3u8"));
    assert!(!requests.iter().any(|r| r.path.ends_with("index.m3u8")));
}

#[tokio::test]
async fn a_chosen_quality_is_downloaded() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let q = "segments=3&size=1000";
    let mut req = stream(
        server.url(&format!("/hls/master.m3u8?{q}")),
        dir.path(),
        MediaKind::Hls,
    );
    req.media.as_mut().unwrap().selection.video = Some(TrackRef {
        id: "v0".into(),
        height: Some(360),
        ..TrackRef::default()
    });
    let path = completed(engine().start(req).wait().await.unwrap());
    assert_eq!(std::fs::read(&path).unwrap(), joined("v0", q));
    assert!(
        !server
            .state
            .requests()
            .iter()
            .any(|r| r.path.starts_with("/hls/v1/"))
    );
}

#[tokio::test]
async fn aes_128_segments_are_decrypted() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    // Video has an explicit IV; the audio playlist uses sequence numbers.
    let q = "segments=4&size=5000&aes=1&split=1";
    let path = completed(
        engine()
            .start(stream(
                server.url(&format!("/hls/master.m3u8?{q}")),
                dir.path(),
                MediaKind::Hls,
            ))
            .wait()
            .await
            .unwrap(),
    );
    assert_eq!(std::fs::read(&path).unwrap(), joined("v1", q));
    let audio = path.with_file_name("master.audio.m4a");
    assert_eq!(std::fs::read(&audio).unwrap(), joined("audio", q));
    let key_requests = server
        .state
        .requests()
        .iter()
        .filter(|r| r.path == "/hls/key.bin")
        .count();
    assert_eq!(key_requests, 1, "the key is fetched once and reused");
}

#[tokio::test]
async fn separate_tracks_are_kept_apart_without_ffmpeg() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let q = "segments=3&size=2000&split=1";
    let handle = engine().start(stream(
        server.url(&format!("/dash/manifest.mpd?{q}")),
        dir.path(),
        MediaKind::Dash,
    ));
    let rx = handle.progress();
    let path = completed(handle.wait().await.unwrap());
    assert_eq!(files(dir.path()), ["manifest.audio.m4a", "manifest.mp4"]);
    assert_eq!(std::fs::read(&path).unwrap(), joined("v1", q));
    assert_eq!(
        std::fs::read(dir.path().join("manifest.audio.m4a")).unwrap(),
        joined("audio", q)
    );
    let notices = rx.borrow().notices.join("\n");
    assert!(notices.contains("FFmpeg was not found"), "{notices}");
    assert!(notices.contains("manifest.audio.m4a"), "{notices}");
}

#[tokio::test]
async fn dash_without_audio_is_one_file() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let q = "segments=4&size=3000";
    let path = completed(
        engine()
            .start(stream(
                server.url(&format!("/dash/manifest.mpd?{q}")),
                dir.path(),
                MediaKind::Dash,
            ))
            .wait()
            .await
            .unwrap(),
    );
    assert_eq!(files(dir.path()), ["manifest.mp4"]);
    // DASH templates here have an init section even without split.
    let mut expected = downpipe_test_server::streams::init("v1");
    expected.extend(joined("v1", q));
    assert_eq!(std::fs::read(&path).unwrap(), expected);
}

#[tokio::test]
async fn protected_and_live_streams_are_refused_before_any_media() {
    let server = TestServer::start().await.unwrap();
    for (path, kind, expected) in [
        ("/hls/master.m3u8?drm=1", MediaKind::Hls, "drm_protected"),
        ("/hls/v0/index.m3u8?drm=1", MediaKind::Hls, "drm_protected"),
        ("/dash/manifest.mpd?drm=1", MediaKind::Dash, "drm_protected"),
        ("/hls/master.m3u8?live=1", MediaKind::Hls, "live_stream"),
        ("/dash/manifest.mpd?live=1", MediaKind::Dash, "live_stream"),
    ] {
        server.state.reset_stats();
        let dir = tempfile::tempdir().unwrap();
        let err = engine()
            .start(stream(server.url(path), dir.path(), kind))
            .wait()
            .await
            .unwrap_err();
        assert_eq!(err.kind(), expected, "{path}: {err}");
        assert!(files(dir.path()).is_empty(), "{path} left files");
        assert!(
            !server
                .state
                .requests()
                .iter()
                .any(|r| r.path.contains("/seg-")),
            "{path}: no fragment may be requested"
        );
    }
}

#[tokio::test]
async fn failing_fragments_are_retried() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let q = "segments=4&size=4000&fail_seg=2&fail_times=2";
    let path = completed(
        engine()
            .start(stream(
                server.url(&format!("/hls/v1/index.m3u8?{q}")),
                dir.path(),
                MediaKind::Hls,
            ))
            .wait()
            .await
            .unwrap(),
    );
    assert_eq!(std::fs::read(&path).unwrap(), joined("v1", q));
    let tries = server
        .state
        .requests()
        .iter()
        .filter(|r| r.path == "/hls/v1/seg-2.ts")
        .count();
    assert_eq!(tries, 3);
}

#[tokio::test]
async fn a_fragment_that_keeps_failing_fails_the_download_and_keeps_the_rest() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let q = "segments=4&size=4000&fail_seg=1&fail_times=100";
    let err = engine()
        .start(stream(
            server.url(&format!("/hls/v1/index.m3u8?{q}")),
            dir.path(),
            MediaKind::Hls,
        ))
        .wait()
        .await
        .unwrap_err();
    assert!(
        matches!(err, DownloadError::RetriesExhausted { .. }),
        "{err}"
    );
    // The fragments that did arrive are kept for a retry.
    let work = dir.path().join("index.ts.part");
    assert!(work.join("t0-000000.frag").is_file());
}

#[tokio::test]
async fn the_referrer_goes_with_every_request() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let mut req = stream(
        server.url("/hls/master.m3u8?segments=2&size=100&aes=1"),
        dir.path(),
        MediaKind::Hls,
    );
    req.headers = vec![("Referer".into(), "https://example.org/".into())];
    completed(engine().start(req).wait().await.unwrap());
    let log = server.state.requests();
    assert!(log.len() >= 5);
    for r in log {
        assert_eq!(
            r.referer.as_deref(),
            Some("https://example.org/"),
            "{}",
            r.path
        );
        assert_eq!(r.cookie, None);
    }
}

#[tokio::test]
async fn pause_and_resume_skip_finished_fragments() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    // Two connections at about 0.4 s per fragment.
    let q = "segments=8&size=20000&rate=50000";
    let url = server.url(&format!("/hls/v1/index.m3u8?{q}"));
    let mut req = stream(url.clone(), dir.path(), MediaKind::Hls);
    req.max_connections = Some(2);
    let handle = engine().start(req);
    wait_for(&handle, |p| {
        p.media.as_ref().is_some_and(|m| m.fragments_done >= 3)
    })
    .await;
    let name = handle.progress().borrow().file_name.clone().unwrap();
    handle.pause();
    let paused = handle.wait().await.unwrap();
    let Outcome::Paused {
        downloaded,
        resumable,
        ..
    } = paused
    else {
        panic!("expected a pause, got {paused:?}");
    };
    assert!(resumable);
    assert!(downloaded >= 3 * 20_000, "{downloaded}");
    let work = dir.path().join(format!("{name}.part"));
    let kept: Vec<String> = files(&work)
        .into_iter()
        .filter(|n| n.ends_with(".frag"))
        .collect();
    assert!(kept.len() >= 3, "{kept:?}");

    server.state.reset_stats();
    // A new engine, as after a restart; the name tells it where to look.
    let mut again = stream(url, dir.path(), MediaKind::Hls);
    again.file_name = Some(name.clone());
    let path = completed(engine().start(again).wait().await.unwrap());
    assert_eq!(path.file_name().unwrap().to_string_lossy(), name);
    assert_eq!(std::fs::read(&path).unwrap(), joined("v1", q));
    let refetched: Vec<String> = server
        .state
        .requests()
        .into_iter()
        .filter(|r| r.path.contains("/seg-"))
        .map(|r| r.path)
        .collect();
    for frag in &kept {
        let index: u64 = frag[3..9].parse().unwrap();
        assert!(
            !refetched.contains(&format!("/hls/v1/seg-{index}.ts")),
            "fragment {index} was downloaded twice"
        );
    }
    assert_eq!(files(dir.path()), [name]);
}

#[tokio::test]
async fn cancel_removes_the_work_folder() {
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let q = "segments=8&size=20000&rate=40000";
    let handle = engine().start(stream(
        server.url(&format!("/hls/v0/index.m3u8?{q}")),
        dir.path(),
        MediaKind::Hls,
    ));
    wait_for(&handle, |p| {
        p.media.as_ref().is_some_and(|m| m.fragments_done >= 1)
    })
    .await;
    handle.cancel();
    assert_eq!(handle.wait().await.unwrap(), Outcome::Canceled);
    assert!(files(dir.path()).is_empty(), "{:?}", files(dir.path()));
}

#[tokio::test]
async fn options_list_qualities_and_audio_without_fetching_media() {
    let server = TestServer::start().await.unwrap();
    let hls = engine()
        .media_options(&server.url("/hls/master.m3u8?split=1"), MediaKind::Hls, &[])
        .await
        .unwrap();
    assert_eq!(
        hls.videos.iter().map(|v| v.height).collect::<Vec<_>>(),
        [Some(720), Some(360)]
    );
    assert!(!hls.videos[0].has_audio);
    assert_eq!(hls.audios.len(), 1);
    assert_eq!(hls.audios[0].language.as_deref(), Some("en"));
    let dash = engine()
        .media_options(
            &server.url("/dash/manifest.mpd?split=1&segments=10"),
            MediaKind::Dash,
            &[],
        )
        .await
        .unwrap();
    assert_eq!(dash.duration_ms, Some(40_000));
    assert_eq!(dash.videos[0].estimated_size, Some(2_500_000 / 8 * 40));
    assert!(
        server
            .state
            .requests()
            .iter()
            .all(|r| !r.path.contains("/seg-"))
    );
    let err = engine()
        .media_options(&server.url("/hls/master.m3u8?drm=1"), MediaKind::Hls, &[])
        .await
        .unwrap_err();
    assert_eq!(err.kind(), "drm_protected");
}

// ---------- With a real FFmpeg, when one is installed ----------

fn ffmpeg() -> Option<PathBuf> {
    let found = mux::find_ffmpeg(None);
    if found.is_none() {
        eprintln!("FFmpeg not found on PATH; skipping");
    }
    found
}

fn with_ffmpeg(req: &mut DownloadRequest, ffmpeg: &Path) {
    req.media.as_mut().unwrap().ffmpeg = Some(ffmpeg.to_path_buf());
}

/// Make real media with FFmpeg's own test sources: no encoder outside the
/// default build is needed.
fn generate(ffmpeg: &Path, dir: &Path, video_codec: &str, format_args: &[&str]) {
    let status = std::process::Command::new(ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-y"])
        .args([
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=4:size=160x120:rate=10",
        ])
        .args(["-f", "lavfi", "-i", "sine=frequency=440:duration=4"])
        .args([
            "-c:v",
            video_codec,
            "-g",
            "10",
            "-c:a",
            "aac",
            "-b:a",
            "64k",
        ])
        .args(format_args)
        .current_dir(dir)
        .status()
        .expect("run ffmpeg");
    assert!(status.success(), "ffmpeg could not make test media");
}

fn is_mp4(path: &Path) -> bool {
    let bytes = std::fs::read(path).unwrap();
    bytes.len() > 8 && &bytes[4..8] == b"ftyp"
}

/// HLS streams carry H.264; FFmpeg cannot move MPEG-4 Part 2 from a
/// transport stream into MP4, so this needs an H.264 encoder.
fn has_h264_encoder(ffmpeg: &Path) -> bool {
    let out = std::process::Command::new(ffmpeg)
        .args(["-hide_banner", "-encoders"])
        .output()
        .unwrap();
    let found = String::from_utf8_lossy(&out.stdout).contains("libx264");
    if !found {
        eprintln!("FFmpeg has no libx264; skipping");
    }
    found
}

#[tokio::test]
async fn ffmpeg_turns_a_transport_stream_into_mp4() {
    let Some(ffmpeg) = ffmpeg() else { return };
    if !has_h264_encoder(&ffmpeg) {
        return;
    }
    let media = tempfile::tempdir().unwrap();
    generate(
        &ffmpeg,
        media.path(),
        "libx264",
        &[
            "-f",
            "hls",
            "-hls_time",
            "1",
            "-hls_playlist_type",
            "vod",
            "clip.m3u8",
        ],
    );
    let server = TestServer::start().await.unwrap();
    server.serve_files(media.path());
    let dir = tempfile::tempdir().unwrap();
    let mut req = stream(server.url("/files/clip.m3u8"), dir.path(), MediaKind::Hls);
    with_ffmpeg(&mut req, &ffmpeg);
    let handle = engine().start(req);
    let rx = handle.progress();
    let path = completed(handle.wait().await.unwrap());
    assert_eq!(files(dir.path()), ["clip.mp4"], "{:?}", rx.borrow().notices);
    assert!(is_mp4(&path));
}

#[tokio::test]
async fn ffmpeg_merges_separate_video_and_audio() {
    let Some(ffmpeg) = ffmpeg() else { return };
    let media = tempfile::tempdir().unwrap();
    generate(
        &ffmpeg,
        media.path(),
        "mpeg4",
        &[
            "-f",
            "dash",
            "-seg_duration",
            "1",
            "-adaptation_sets",
            "id=0,streams=v id=1,streams=a",
            "clip.mpd",
        ],
    );
    let server = TestServer::start().await.unwrap();
    server.serve_files(media.path());
    let dir = tempfile::tempdir().unwrap();
    let mut req = stream(server.url("/files/clip.mpd"), dir.path(), MediaKind::Dash);
    with_ffmpeg(&mut req, &ffmpeg);
    let handle = engine().start(req);
    let rx = handle.progress();
    let path = completed(handle.wait().await.unwrap());
    assert_eq!(files(dir.path()), ["clip.mp4"], "{:?}", rx.borrow().notices);
    assert!(is_mp4(&path));
    // Both streams made it into the one file.
    if let Some(ffprobe) = ffmpeg.parent().map(|d| {
        d.join(if cfg!(windows) {
            "ffprobe.exe"
        } else {
            "ffprobe"
        })
    }) && ffprobe.is_file()
    {
        let out = std::process::Command::new(ffprobe)
            .args([
                "-v",
                "error",
                "-show_entries",
                "stream=codec_type",
                "-of",
                "csv=p=0",
            ])
            .arg(&path)
            .output()
            .unwrap();
        let types = String::from_utf8_lossy(&out.stdout);
        assert!(
            types.contains("video") && types.contains("audio"),
            "{types}"
        );
    }
}

#[tokio::test]
async fn a_failed_merge_keeps_both_tracks() {
    let Some(ffmpeg) = ffmpeg() else { return };
    let server = TestServer::start().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    // The test server's fragments are not real media, so FFmpeg refuses them.
    let mut req = stream(
        server.url("/dash/manifest.mpd?segments=2&size=500&split=1"),
        dir.path(),
        MediaKind::Dash,
    );
    with_ffmpeg(&mut req, &ffmpeg);
    let handle = engine().start(req);
    let rx = handle.progress();
    completed(handle.wait().await.unwrap());
    assert_eq!(files(dir.path()), ["manifest.audio.m4a", "manifest.mp4"]);
    let notices = rx.borrow().notices.join("\n");
    assert!(notices.contains("Could not merge with FFmpeg"), "{notices}");
}
