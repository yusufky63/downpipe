//! The provider against fake yt-dlp and Deno processes (no network).
//!
//! `harness = false`: this binary is also the fake tools. Copied under a
//! tool's name, `run_if_fake` makes it play that tool; otherwise `main`
//! runs the cases below one after the other.

use std::future::Future;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::Duration;

use downpipe_http::media::{MediaSelection, TrackRef};
use downpipe_http::{DownloadError, DownloadHandle, Outcome, Phase, Progress};
use downpipe_providers::install::{self, Tool};
use downpipe_providers::tools::{self, JsKind, JsRuntime, ToolConfig, Tools};
use downpipe_providers::{Preset, SiteRequest, SubtitleRef, info, job, playlist};
use downpipe_test_server::TestServer;
use downpipe_test_server::fake_tools::{self, FakePart, Scenario, calls, part_content};
use serde_json::json;
use sha2::Digest as _;

const URL: &str = "https://www.youtube.com/watch?v=jNQXAC9IVRw";

type Case = fn() -> Pin<Box<dyn Future<Output = ()>>>;

fn main() {
    fake_tools::run_if_fake();
    let cases: &[(&str, Case)] = &[
        ("inspect_reads_the_formats", || {
            Box::pin(inspect_reads_the_formats())
        }),
        ("inspect_explains_errors", || {
            Box::pin(inspect_explains_errors())
        }),
        ("downloads_and_merges_two_parts", || {
            Box::pin(downloads_and_merges_two_parts())
        }),
        ("pause_then_resume_continues", || {
            Box::pin(pause_then_resume_continues())
        }),
        ("cancel_deletes_work_files", || {
            Box::pin(cancel_deletes_work_files())
        }),
        ("names_from_the_title_and_avoids_taken_names", || {
            Box::pin(names_from_the_title_and_avoids_taken_names())
        }),
        ("failures_are_classified", || {
            Box::pin(failures_are_classified())
        }),
        ("lists_a_playlist", || Box::pin(lists_a_playlist())),
        ("playlist_videos_use_preset_template_and_subtitles", || {
            Box::pin(playlist_videos_use_preset_template_and_subtitles())
        }),
        ("readable_errors_for_common_failures", || {
            Box::pin(readable_errors_for_common_failures())
        }),
        ("missing_tools_fail_before_starting", || {
            Box::pin(missing_tools_fail_before_starting())
        }),
        ("tool_status_reads_versions", || {
            Box::pin(tool_status_reads_versions())
        }),
        ("installs_checked_releases", || {
            Box::pin(installs_checked_releases())
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
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| runtime.block_on(case())));
        match result {
            Ok(()) => println!("test {name} ... ok"),
            Err(_) => {
                println!("test {name} ... FAILED");
                failed.push(*name);
            }
        }
        let _ = std::io::stdout().flush();
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

fn info_fixture() -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(info::fixture_dir().join("youtube-info.json")).unwrap())
        .unwrap()
}

fn two_parts() -> Vec<FakePart> {
    vec![
        FakePart {
            format_id: "134".into(),
            ext: "mp4".into(),
            size: 300_000,
            height: Some(240),
            vcodec: Some("avc1.4D400C".into()),
            acodec: Some("none".into()),
        },
        FakePart {
            format_id: "140".into(),
            ext: "m4a".into(),
            size: 100_000,
            height: None,
            vcodec: Some("none".into()),
            acodec: Some("mp4a.40.2".into()),
        },
    ]
}

struct Setup {
    _dir: tempfile::TempDir,
    root: PathBuf,
    out: PathBuf,
    yt_dlp: PathBuf,
    tools: Tools,
}

fn setup(scenario: &Scenario) -> Setup {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let yt_dlp = fake_tools::install(&root.join("bin"), "yt-dlp", scenario);
    let deno = fake_tools::install(
        &root.join("bin"),
        "deno",
        &Scenario {
            version: "deno 2.9.7 (stable, release)".into(),
            ..Scenario::default()
        },
    );
    // Any file will do: FFmpeg only has to exist for the merge to be asked for.
    let ffmpeg = root.join("bin").join(fake_tools::exe_name("ffmpeg"));
    std::fs::write(&ffmpeg, b"").unwrap();
    let tools = Tools {
        yt_dlp: Some(yt_dlp.clone()),
        js: Some(JsRuntime {
            kind: JsKind::Deno,
            path: deno,
        }),
        ffmpeg: Some(ffmpeg),
    };
    let out = root.join("out");
    Setup {
        _dir: dir,
        root,
        out,
        yt_dlp,
        tools,
    }
}

fn request(s: &Setup, file_name: Option<&str>, selection: Option<MediaSelection>) -> SiteRequest {
    SiteRequest {
        url: URL.into(),
        dir: s.out.clone(),
        file_name: file_name.map(String::from),
        selection,
        preset: None,
        subtitles: Vec::new(),
        name_template: downpipe_providers::naming::DEFAULT_TEMPLATE.into(),
        index: None,
        tools: s.tools.clone(),
        proxy: None,
        rate_limit: 0,
    }
}

/// Run to the end, keeping every progress snapshot.
async fn finish(handle: DownloadHandle) -> (Result<Outcome, DownloadError>, Vec<Progress>) {
    let mut rx = handle.progress();
    let seen = tokio::spawn(async move {
        let mut seen = Vec::new();
        while rx.changed().await.is_ok() {
            seen.push(rx.borrow_and_update().clone());
        }
        seen
    });
    let outcome = tokio::time::timeout(Duration::from_secs(60), handle.wait())
        .await
        .expect("the download finishes");
    (outcome, seen.await.unwrap())
}

async fn wait_for_bytes(handle: &DownloadHandle, at_least: u64) {
    let mut rx = handle.progress();
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            if rx.borrow().downloaded >= at_least {
                return;
            }
            rx.changed().await.expect("still running");
        }
    })
    .await
    .expect("progress arrives");
}

async fn inspect_reads_the_formats() {
    let s = setup(&Scenario {
        info: info_fixture(),
        ..Scenario::default()
    });
    let page = info::inspect(&s.tools, URL, Some("http://127.0.0.1:3128"))
        .await
        .unwrap();
    assert_eq!(page.title.as_deref(), Some("Me at the zoo"));
    assert_eq!(page.videos[0].id, "133");
    let args = &calls(&s.yt_dlp)[0];
    assert_eq!(args[0], "--ignore-config");
    assert!(args.contains(&"-J".to_string()));
    assert!(args.contains(&"--no-cookies".to_string()));
    let deno = s
        .tools
        .js
        .as_ref()
        .unwrap()
        .path
        .to_string_lossy()
        .into_owned();
    assert!(args.contains(&format!("deno:{deno}")), "{args:?}");
    let proxy = args.iter().position(|a| a == "--proxy").unwrap();
    assert_eq!(args[proxy + 1], "http://127.0.0.1:3128");
    assert_eq!(&args[args.len() - 2..], ["--", URL]);
}

async fn inspect_explains_errors() {
    let s = setup(&Scenario {
        error: Some("[youtube] jNQXAC9IVRw: Private video. Sign in if you've been granted access to this video".into()),
        ..Scenario::default()
    });
    let err = info::inspect(&s.tools, URL, None).await.unwrap_err();
    assert_eq!(err.kind(), "unavailable");
    assert!(err.to_string().starts_with("Private video"), "{err}");

    fake_tools::set_scenario(
        &s.yt_dlp,
        &Scenario {
            info: json!({"_type": "video", "is_live": true, "formats": []}),
            ..Scenario::default()
        },
    );
    let err = info::inspect(&s.tools, URL, None).await.unwrap_err();
    assert_eq!(err.kind(), "live_stream");
}

async fn downloads_and_merges_two_parts() {
    let s = setup(&Scenario {
        parts: two_parts(),
        merged_ext: "mp4".into(),
        step: 50_000,
        ..Scenario::default()
    });
    let selection = MediaSelection {
        video: Some(TrackRef {
            id: "134".into(),
            height: Some(240),
            ..TrackRef::default()
        }),
        audio: Some(TrackRef {
            id: "140".into(),
            ..TrackRef::default()
        }),
    };
    let (outcome, seen) = finish(job::start(request(&s, Some("Zoo.mp4"), Some(selection)))).await;
    let Ok(Outcome::Completed { path, bytes, .. }) = outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(path, s.out.join("Zoo.mp4"));
    assert_eq!(bytes, 400_000);
    let mut expected = part_content(300_000);
    expected.extend(part_content(100_000));
    assert_eq!(std::fs::read(&path).unwrap(), expected);
    // Work files are gone after the merge.
    assert!(downpipe_providers::files::partials(&s.out, "Zoo").is_empty());

    assert_eq!(seen[0].file_name.as_deref(), Some("Zoo.mp4"));
    let downloading: Vec<&Progress> = seen
        .iter()
        .filter(|p| p.phase == Phase::Downloading)
        .collect();
    assert!(!downloading.is_empty());
    // One size across both parts from the first progress on.
    assert!(downloading.iter().all(|p| p.total == Some(400_000)));
    assert!(
        downloading
            .windows(2)
            .all(|w| w[0].downloaded <= w[1].downloaded)
    );
    let media = downloading.last().unwrap().media.clone().unwrap();
    assert_eq!(media.quality, "240p");
    assert_eq!(media.selection.audio.unwrap().id, "140");
    assert!(seen.iter().any(|p| p.phase == Phase::Finalizing));

    let args = &calls(&s.yt_dlp)[0];
    let at = |flag: &str| args[args.iter().position(|a| a == flag).unwrap() + 1].clone();
    assert_eq!(at("-f"), "134+140/bv*[height<=240]+ba/b[height<=240]/b");
    assert_eq!(at("-o"), "Zoo.%(ext)s");
    assert_eq!(PathBuf::from(at("-P")), s.out);
    assert!(args.contains(&"--continue".to_string()));
}

async fn pause_then_resume_continues() {
    let s = setup(&Scenario {
        parts: two_parts(),
        merged_ext: "mp4".into(),
        step: 10_000,
        delay_ms: 40,
        ..Scenario::default()
    });
    let handle = job::start(request(&s, Some("Zoo.mp4"), None));
    wait_for_bytes(&handle, 60_000).await;
    handle.pause();
    let (outcome, _) = finish(handle).await;
    let Ok(Outcome::Paused {
        downloaded,
        resumable,
        ..
    }) = outcome
    else {
        panic!("{outcome:?}");
    };
    assert!(resumable);
    assert!((60_000..400_000).contains(&downloaded), "{downloaded}");
    let part = s.out.join("Zoo.f134.mp4.part");
    let kept = std::fs::metadata(&part).unwrap().len();
    assert!(kept >= 60_000, "{kept}");

    // Faster now; the second run picks the part file up.
    let faster = Scenario {
        parts: two_parts(),
        merged_ext: "mp4".into(),
        step: 50_000,
        ..Scenario::default()
    };
    fake_tools::set_scenario(&s.yt_dlp, &faster);
    let (outcome, seen) = finish(job::start(request(&s, Some("Zoo.mp4"), None))).await;
    assert!(
        matches!(outcome, Ok(Outcome::Completed { bytes: 400_000, .. })),
        "{outcome:?}"
    );
    let first = seen.iter().find(|p| p.phase == Phase::Downloading).unwrap();
    assert!(
        first.downloaded > kept,
        "resumed at {} after {kept}",
        first.downloaded
    );
    let mut expected = part_content(300_000);
    expected.extend(part_content(100_000));
    assert_eq!(std::fs::read(s.out.join("Zoo.mp4")).unwrap(), expected);
    assert_eq!(calls(&s.yt_dlp).len(), 2);
}

async fn cancel_deletes_work_files() {
    let s = setup(&Scenario {
        parts: two_parts(),
        merged_ext: "mp4".into(),
        step: 10_000,
        delay_ms: 40,
        ..Scenario::default()
    });
    let handle = job::start(request(&s, Some("Zoo.mp4"), None));
    wait_for_bytes(&handle, 30_000).await;
    handle.cancel();
    let (outcome, _) = finish(handle).await;
    assert!(matches!(outcome, Ok(Outcome::Canceled)), "{outcome:?}");
    // Give Windows a moment to release the killed process's handles.
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(downpipe_providers::files::partials(&s.out, "Zoo").is_empty());
}

async fn names_from_the_title_and_avoids_taken_names() {
    let s = setup(&Scenario {
        info: info_fixture(),
        parts: vec![FakePart {
            format_id: "140".into(),
            ext: "m4a".into(),
            size: 1000,
            vcodec: Some("none".into()),
            acodec: Some("mp4a.40.2".into()),
            ..FakePart::default()
        }],
        step: 1000,
        ..Scenario::default()
    });
    std::fs::create_dir_all(&s.out).unwrap();
    // An earlier download of the same video.
    std::fs::write(s.out.join("Me at the zoo.m4a"), b"old").unwrap();
    let (outcome, seen) = finish(job::start(request(&s, None, None))).await;
    let Ok(Outcome::Completed { path, .. }) = outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(path, s.out.join("Me at the zoo (1).m4a"));
    assert_eq!(
        std::fs::read(s.out.join("Me at the zoo.m4a")).unwrap(),
        b"old"
    );
    assert_eq!(seen[0].file_name.as_deref(), Some("Me at the zoo (1).mp4"));
    // -J first for the title, then the download.
    let runs = calls(&s.yt_dlp);
    assert_eq!(runs.len(), 2);
    assert!(runs[0].contains(&"-J".to_string()));
    assert_eq!(
        runs[1][runs[1].iter().position(|a| a == "-f").unwrap() + 1],
        "bv*+ba/b"
    );
}

fn playlist_fixture() -> serde_json::Value {
    serde_json::from_slice(
        &std::fs::read(info::fixture_dir().join("youtube-playlist.json")).unwrap(),
    )
    .unwrap()
}

const LIST: &str = "https://www.youtube.com/playlist?list=PLbpi6ZahtOH6Blw3RGYpWkSByi_T7Rygb";

async fn lists_a_playlist() {
    let s = setup(&Scenario {
        playlist: playlist_fixture(),
        ..Scenario::default()
    });
    let list = playlist::list(&s.tools, LIST, None).await.unwrap();
    assert_eq!(list.title.as_deref(), Some("Early YouTube"));
    assert_eq!(list.entries.len(), 4);
    assert_eq!(list.entries[1].id, "aqz-KE-bpKQ");
    let args = &calls(&s.yt_dlp)[0];
    assert!(args.contains(&"--flat-playlist".to_string()));
    assert!(args.contains(&"--yes-playlist".to_string()));
    assert!(!args.contains(&"--no-playlist".to_string()));
    assert!(args.contains(&"--no-cookies".to_string()));
    assert_eq!(&args[args.len() - 2..], ["--", LIST]);

    // A list that is gone, explained like a video.
    fake_tools::set_scenario(
        &s.yt_dlp,
        &Scenario {
            error: Some("[youtube:tab] PLx: The playlist does not exist.".into()),
            ..Scenario::default()
        },
    );
    let err = playlist::list(&s.tools, LIST, None).await.unwrap_err();
    assert_eq!(err.kind(), "unavailable");
}

async fn playlist_videos_use_preset_template_and_subtitles() {
    let s = setup(&Scenario {
        info: info_fixture(),
        parts: vec![FakePart {
            format_id: "18".into(),
            ext: "mp4".into(),
            size: 2000,
            height: Some(360),
            vcodec: Some("avc1.42001E".into()),
            acodec: Some("mp4a.40.2".into()),
        }],
        step: 1000,
        ..Scenario::default()
    });
    let subs = vec![
        SubtitleRef {
            language: "en".into(),
            auto: false,
        },
        SubtitleRef {
            language: "tr".into(),
            auto: true,
        },
    ];
    let mut req = request(&s, None, None);
    req.preset = Some(Preset::P720);
    req.subtitles = subs.clone();
    req.name_template = "{index} - {channel} - {title} ({date})".into();
    req.index = Some((3, 12));
    // Without FFmpeg the subtitles are saved next to the video.
    req.tools.ffmpeg = None;
    let (outcome, _) = finish(job::start(req)).await;
    let Ok(Outcome::Completed { path, .. }) = outcome else {
        panic!("{outcome:?}");
    };
    let stem = "03 - jawed - Me at the zoo (2005-04-23)";
    assert_eq!(path, s.out.join(format!("{stem}.mp4")));
    assert!(s.out.join(format!("{stem}.en.vtt")).exists());
    assert!(s.out.join(format!("{stem}.tr.vtt")).exists());
    let runs = calls(&s.yt_dlp);
    let args = runs.last().unwrap();
    let at = |flag: &str| args[args.iter().position(|a| a == flag).unwrap() + 1].clone();
    assert_eq!(at("-f"), "b[height<=720]/b");
    assert_eq!(at("--sub-langs"), "en,tr");
    assert!(args.contains(&"--write-subs".to_string()));
    assert!(args.contains(&"--write-auto-subs".to_string()));

    // With FFmpeg they go into the video, and nothing lands next to it.
    let mut req = request(&s, Some("Embedded.mp4"), None);
    req.preset = Some(Preset::Best);
    req.subtitles = subs;
    let (outcome, _) = finish(job::start(req)).await;
    assert!(
        matches!(outcome, Ok(Outcome::Completed { .. })),
        "{outcome:?}"
    );
    let args = calls(&s.yt_dlp).last().unwrap().clone();
    assert!(args.contains(&"--embed-subs".to_string()));
    assert!(!s.out.join("Embedded.en.vtt").exists());
    let f = args[args.iter().position(|a| a == "-f").unwrap() + 1].clone();
    assert_eq!(f, "bv*+ba/b");
}

async fn readable_errors_for_common_failures() {
    let cases: &[(&str, Option<&str>, &str)] = &[
        (
            "unable to download video data: HTTP Error 403: Forbidden",
            None,
            "site_forbidden",
        ),
        (
            "[youtube] jNQXAC9IVRw: Sign in to confirm you're not a bot. Use --cookies-from-browser or --cookies for the authentication.",
            None,
            "site_sign_in",
        ),
        (
            "[youtube] jNQXAC9IVRw: Sign in to confirm your age. This video may be inappropriate for some users.",
            None,
            "site_age_restricted",
        ),
        (
            "[youtube] jNQXAC9IVRw: Video unavailable. This video has been removed by the uploader",
            None,
            "unavailable",
        ),
        (
            "[youtube] jNQXAC9IVRw: Requested format is not available. Use --list-formats for a list of available formats",
            Some(
                "[youtube] No supported JavaScript runtime could be found. YouTube extraction without a JS runtime has been deprecated",
            ),
            "site_js_runtime",
        ),
        (
            "unable to download video data: HTTP Error 403: Forbidden",
            Some(
                "[youtube] jNQXAC9IVRw: Some tv client https formats have been skipped as they are missing a url. YouTube may have enabled the SABR-only or Server-Side Ad Placement experiment for the current session. GVS PO Token required",
            ),
            "site_po_token",
        ),
    ];
    for (error, warning, kind) in cases {
        let s = setup(&Scenario {
            error: Some((*error).into()),
            warning: warning.map(String::from),
            ..Scenario::default()
        });
        let err = info::inspect(&s.tools, URL, None).await.unwrap_err();
        assert_eq!(err.kind(), *kind, "{error}");
    }
}

async fn failures_are_classified() {
    let s = setup(&Scenario {
        parts: two_parts(),
        merged_ext: "mp4".into(),
        step: 10_000,
        error_after: Some(20_000),
        warning: Some("[youtube] No supported JavaScript runtime could be found.".into()),
        ..Scenario::default()
    });
    let (outcome, seen) = finish(job::start(request(&s, Some("Zoo.mp4"), None))).await;
    let err = outcome.unwrap_err();
    assert_eq!(err.kind(), "network", "{err}");
    assert!(
        seen.iter()
            .any(|p| p.notices.iter().any(|n| n.contains("JavaScript runtime"))),
        "the warning becomes a notice"
    );
    // The part file is kept for a retry.
    assert!(s.out.join("Zoo.f134.mp4.part").exists());

    fake_tools::set_scenario(
        &s.yt_dlp,
        &Scenario {
            error: Some("[youtube] jNQXAC9IVRw: This video is DRM protected".into()),
            ..Scenario::default()
        },
    );
    let (outcome, _) = finish(job::start(request(&s, Some("Zoo.mp4"), None))).await;
    assert_eq!(outcome.unwrap_err().kind(), "drm_protected");
}

async fn missing_tools_fail_before_starting() {
    let s = setup(&Scenario::default());
    let mut req = request(&s, Some("Zoo.mp4"), None);
    req.tools.yt_dlp = None;
    let (outcome, _) = finish(job::start(req)).await;
    assert_eq!(outcome.unwrap_err().kind(), "tool_missing");

    let mut req = request(
        &s,
        Some("Zoo.mp4"),
        Some(MediaSelection {
            video: Some(TrackRef {
                id: "137".into(),
                height: Some(1080),
                ..TrackRef::default()
            }),
            audio: Some(TrackRef {
                id: "140".into(),
                ..TrackRef::default()
            }),
        }),
    );
    req.tools.ffmpeg = None;
    let (outcome, _) = finish(job::start(req)).await;
    let err = outcome.unwrap_err();
    assert_eq!(err.kind(), "tool_missing");
    assert!(err.to_string().contains("FFmpeg"));
    assert!(calls(&s.yt_dlp).is_empty(), "yt-dlp never ran");
}

async fn tool_status_reads_versions() {
    let s = setup(&Scenario {
        version: "2026.08.19".into(),
        ..Scenario::default()
    });
    let managed = s.root.join("bin");
    let status = tools::status(&ToolConfig {
        managed_dir: Some(managed.clone()),
        ffmpeg: s.root.join("missing-ffmpeg.exe"),
        ..ToolConfig::default()
    })
    .await;
    assert_eq!(status.yt_dlp.version.as_deref(), Some("2026.08.19"));
    assert_eq!(status.yt_dlp.source, Some(tools::ToolSource::Managed));
    assert_eq!(status.js_kind, Some(JsKind::Deno));
    assert_eq!(status.js.version.as_deref(), Some("2.9.7"));
    assert_eq!(status.js.problem, None);
    assert!(status.ffmpeg.path.is_none());
    assert!(status.ffmpeg.problem.is_some());

    // An old Deno is found but flagged.
    fake_tools::set_scenario(
        &managed.join(fake_tools::exe_name("deno")),
        &Scenario {
            version: "deno 2.2.15 (stable)".into(),
            ..Scenario::default()
        },
    );
    let status = tools::status(&ToolConfig {
        managed_dir: Some(managed),
        ..ToolConfig::default()
    })
    .await;
    assert!(status.js.problem.unwrap().contains("2.3.0"));
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(sha2::Sha256::digest(bytes))
}

fn deno_zip(exe: &[u8]) -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(&mut buf);
    zip.start_file(
        fake_tools::exe_name("deno"),
        zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated),
    )
    .unwrap();
    zip.write_all(exe).unwrap();
    zip.finish().unwrap();
    buf.into_inner()
}

fn write(path: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

async fn installs_checked_releases() {
    let server = TestServer::start().await.unwrap();
    let files = tempfile::tempdir().unwrap();
    server.serve_files(files.path());
    let base = url::Url::parse(&server.url("/releases/")).unwrap();
    let client = reqwest::Client::new();
    let tools_dir = files.path().join("tools");

    let yt_asset =
        install::asset_name(Tool::YtDlp, std::env::consts::OS, std::env::consts::ARCH).unwrap();
    let deno_asset =
        install::asset_name(Tool::Deno, std::env::consts::OS, std::env::consts::ARCH).unwrap();
    let yt_bytes = b"yt-dlp build".repeat(1000);
    let deno_bytes = b"deno build".repeat(5000);
    let zip = deno_zip(&deno_bytes);

    let yt = files.path().join("yt-dlp/yt-dlp");
    write(&yt.join("latest"), b"2026.08.19\n");
    write(&yt.join("2026.08.19").join(&yt_asset), &yt_bytes);
    write(
        &yt.join("2026.08.19/SHA2-256SUMS"),
        format!(
            "{}  other\n{}  {yt_asset}\n",
            sha256(b"x"),
            sha256(&yt_bytes)
        )
        .as_bytes(),
    );
    let deno = files.path().join("denoland/deno");
    write(&deno.join("latest"), b"v2.9.7");
    write(&deno.join("v2.9.7").join(&deno_asset), &zip);
    write(
        &deno.join("v2.9.7").join(format!("{deno_asset}.sha256sum")),
        format!(
            "Algorithm : SHA256\r\nHash      : {}\r\nPath      : C:\\a\\{deno_asset}\r\n",
            sha256(&zip).to_uppercase()
        )
        .as_bytes(),
    );

    let installed = install::install(&client, &base, &tools_dir, Tool::YtDlp)
        .await
        .unwrap();
    assert_eq!(installed.release, "2026.08.19");
    assert_eq!(
        installed.path,
        tools_dir.join(fake_tools::exe_name("yt-dlp"))
    );
    assert_eq!(std::fs::read(&installed.path).unwrap(), yt_bytes);
    assert_eq!(installed.sha256, sha256(&yt_bytes));

    let installed = install::install(&client, &base, &tools_dir, Tool::Deno)
        .await
        .unwrap();
    assert_eq!(installed.release, "v2.9.7");
    assert_eq!(std::fs::read(&installed.path).unwrap(), deno_bytes);
    // Only the program is left behind, not the archive.
    let mut left: Vec<String> = std::fs::read_dir(&tools_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    left.sort();
    assert_eq!(
        left,
        [fake_tools::exe_name("deno"), fake_tools::exe_name("yt-dlp")]
    );

    // A file that does not match its published hash replaces nothing.
    write(&yt.join("latest"), b"2026.09.01");
    write(&yt.join("2026.09.01").join(&yt_asset), b"tampered");
    write(
        &yt.join("2026.09.01/SHA2-256SUMS"),
        format!("{}  {yt_asset}\n", sha256(&yt_bytes)).as_bytes(),
    );
    let err = install::install(&client, &base, &tools_dir, Tool::YtDlp)
        .await
        .unwrap_err();
    assert_eq!(err.kind(), "checksum");
    assert_eq!(
        std::fs::read(tools_dir.join(fake_tools::exe_name("yt-dlp"))).unwrap(),
        yt_bytes
    );
    assert_eq!(
        std::fs::read_dir(&tools_dir).unwrap().count(),
        2,
        "no staged file left"
    );

    // A release without a hash for this system's file is refused.
    write(&yt.join("2026.09.01/SHA2-256SUMS"), b"");
    let err = install::install(&client, &base, &tools_dir, Tool::YtDlp)
        .await
        .unwrap_err();
    assert_eq!(err.kind(), "extractor");
}
