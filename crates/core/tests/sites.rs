//! Video pages through the manager, with a fake yt-dlp (no network).
//!
//! `harness = false`: this binary is also the fake yt-dlp and Deno (see
//! downpipe_test_server::fake_tools); `main` runs the cases otherwise.

use std::future::Future;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::Duration;

use downpipe_core::http::media::{MediaSelection, TrackRef};
use downpipe_core::providers::{Preset, SubtitleRef};
use downpipe_core::{
    AddListOptions, AddOptions, ConfirmOptions, CoreError, ListPick, Manager, ManagerEvent,
    Settings, TaskSnapshot, TaskState,
};
use downpipe_test_server::fake_tools::{self, FakePart, Scenario, calls, part_content};

type Case = fn() -> Pin<Box<dyn Future<Output = ()>>>;

fn main() {
    fake_tools::run_if_fake();
    let cases: &[(&str, Case)] = &[
        ("video_pages_become_site_downloads", || {
            Box::pin(video_pages_become_site_downloads())
        }),
        ("confirm_with_a_quality_then_download", || {
            Box::pin(confirm_with_a_quality_then_download())
        }),
        ("pause_resume_and_cancel", || {
            Box::pin(pause_resume_and_cancel())
        }),
        ("unnamed_downloads_take_the_title", || {
            Box::pin(unnamed_downloads_take_the_title())
        }),
        ("missing_yt_dlp_is_explained", || {
            Box::pin(missing_yt_dlp_is_explained())
        }),
        ("playlists_list_and_add_chosen_videos", || {
            Box::pin(playlists_list_and_add_chosen_videos())
        }),
        ("template_names_and_subtitles_in_the_dialog", || {
            Box::pin(template_names_and_subtitles_in_the_dialog())
        }),
        ("site_failures_keep_their_kind", || {
            Box::pin(site_failures_keep_their_kind())
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
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../providers/tests/fixtures/youtube-info.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn parts() -> Vec<FakePart> {
    vec![
        FakePart {
            format_id: "134".into(),
            ext: "mp4".into(),
            size: 200_000,
            height: Some(240),
            vcodec: Some("avc1.4D400C".into()),
            acodec: Some("none".into()),
        },
        FakePart {
            format_id: "140".into(),
            ext: "m4a".into(),
            size: 50_000,
            vcodec: Some("none".into()),
            acodec: Some("mp4a.40.2".into()),
            ..FakePart::default()
        },
    ]
}

struct Env {
    _dir: tempfile::TempDir,
    out: PathBuf,
    yt_dlp: PathBuf,
    manager: Manager,
}

async fn env(scenario: &Scenario) -> Env {
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("bin");
    let yt_dlp = fake_tools::install(&bin, "yt-dlp", scenario);
    let deno = fake_tools::install(
        &bin,
        "deno",
        &Scenario {
            version: "deno 2.9.7 (stable)".into(),
            ..Scenario::default()
        },
    );
    let ffmpeg = bin.join(fake_tools::exe_name("ffmpeg"));
    std::fs::write(&ffmpeg, b"").unwrap();
    let out = dir.path().join("out");
    let manager = Manager::open(&dir.path().join("data")).await.unwrap();
    manager
        .update_settings(Settings {
            download_dir: out.clone(),
            yt_dlp_path: yt_dlp.to_string_lossy().into_owned(),
            deno_path: deno.to_string_lossy().into_owned(),
            ffmpeg_path: ffmpeg.to_string_lossy().into_owned(),
            use_categories: false,
            ..manager.settings()
        })
        .unwrap();
    Env {
        _dir: dir,
        out,
        yt_dlp,
        manager,
    }
}

fn add(manager: &Manager, url: &str, confirm: bool) -> TaskSnapshot {
    manager
        .add(AddOptions {
            url: url.into(),
            confirm,
            ..AddOptions::default()
        })
        .expect("add")
}

async fn wait_for(
    manager: &Manager,
    id: &str,
    cond: impl Fn(&TaskSnapshot) -> bool,
) -> TaskSnapshot {
    tokio::time::timeout(Duration::from_secs(30), async {
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
        let task = manager
            .get(id)
            .map(|t| (t.record.state, t.record.downloaded, t.record.error));
        panic!("condition not reached in time; last: {task:?}")
    })
}

async fn video_pages_become_site_downloads() {
    let e = env(&Scenario::default()).await;
    let task = add(&e.manager, "https://youtu.be/jNQXAC9IVRw?t=3&si=abc", true);
    let r = &task.record;
    assert_eq!(r.url, "https://www.youtube.com/watch?v=jNQXAC9IVRw");
    assert_eq!(r.site.as_ref().unwrap().site, "YouTube");
    assert!(r.media.is_none());
    assert_eq!(r.state, TaskState::Paused);
    assert!(task.live.awaiting_confirmation);
    // The same video under another link is a duplicate.
    let again = e.manager.add(AddOptions {
        url: "https://www.youtube.com/watch?v=jNQXAC9IVRw&list=PL1&index=2".into(),
        ..AddOptions::default()
    });
    assert!(matches!(again, Err(CoreError::Duplicate(id)) if id == r.id));
    // A page that is not a video stays a plain download.
    let plain = add(&e.manager, "https://www.youtube.com/feed/trending", true);
    assert!(plain.record.site.is_none());
    // A playlist is not one download: its videos are chosen first.
    let list = e.manager.add(AddOptions {
        url: "https://www.youtube.com/playlist?list=PL1".into(),
        ..AddOptions::default()
    });
    assert!(matches!(list, Err(CoreError::ListPage)));
    // Video pages are filed as videos even before their title is known.
    e.manager
        .update_settings(Settings {
            use_categories: true,
            ..e.manager.settings()
        })
        .unwrap();
    let shorts = add(
        &e.manager,
        "https://www.youtube.com/shorts/aqz-KE-bpKQ",
        true,
    );
    assert_eq!(shorts.record.category.as_deref(), Some("Video"));
}

async fn confirm_with_a_quality_then_download() {
    let e = env(&Scenario {
        info: info_fixture(),
        parts: parts(),
        merged_ext: "mp4".into(),
        step: 25_000,
        ..Scenario::default()
    })
    .await;
    let task = add(
        &e.manager,
        "https://www.youtube.com/watch?v=jNQXAC9IVRw",
        true,
    );
    let choices = e.manager.inspect_site(&task.record.id).await.unwrap();
    assert_eq!(choices.site, "YouTube");
    assert_eq!(choices.title.as_deref(), Some("Me at the zoo"));
    assert!(choices.ffmpeg);
    assert_eq!(choices.options.videos[0].id, "133");
    assert!(choices.options.audios[0].default);

    e.manager
        .confirm(
            &task.record.id,
            ConfirmOptions {
                file_name: Some("Zoo.mp4".into()),
                start: true,
                media: Some(MediaSelection {
                    video: Some(TrackRef {
                        id: "134".into(),
                        height: Some(240),
                        ..TrackRef::default()
                    }),
                    audio: Some(TrackRef {
                        id: "140".into(),
                        ..TrackRef::default()
                    }),
                }),
                ..ConfirmOptions::default()
            },
        )
        .unwrap();
    let done = wait_for(&e.manager, &task.record.id, |t| {
        t.record.state.is_finished() || t.record.state == TaskState::Failed
    })
    .await;
    assert_eq!(
        done.record.state,
        TaskState::Completed,
        "{:?}",
        done.record.error
    );
    assert_eq!(done.record.path, Some(e.out.join("Zoo.mp4")));
    assert_eq!(done.record.downloaded, 250_000);
    let site = done.record.site.unwrap();
    assert_eq!(site.quality.as_deref(), Some("240p"));
    assert_eq!(site.selection.unwrap().video.unwrap().id, "134");
    let mut expected = part_content(200_000);
    expected.extend(part_content(50_000));
    assert_eq!(std::fs::read(e.out.join("Zoo.mp4")).unwrap(), expected);

    let runs = calls(&e.yt_dlp);
    // -J for the dialog, then the download.
    assert_eq!(runs.len(), 2);
    let args = &runs[1];
    let at = |flag: &str| args[args.iter().position(|a| a == flag).unwrap() + 1].clone();
    assert_eq!(at("-f"), "134+140/bv*[height<=240]+ba/b[height<=240]/b");
    assert!(at("--js-runtimes").starts_with("deno:"));
    assert!(args.contains(&"--ffmpeg-location".to_string()));
}

async fn pause_resume_and_cancel() {
    let e = env(&Scenario {
        parts: parts(),
        merged_ext: "mp4".into(),
        step: 5_000,
        delay_ms: 30,
        ..Scenario::default()
    })
    .await;
    let task = add(
        &e.manager,
        "https://www.youtube.com/watch?v=jNQXAC9IVRw",
        true,
    );
    let id = task.record.id.clone();
    e.manager
        .confirm(
            &id,
            ConfirmOptions {
                file_name: Some("Zoo.mp4".into()),
                start: true,
                ..ConfirmOptions::default()
            },
        )
        .unwrap();
    wait_for(&e.manager, &id, |t| t.record.downloaded >= 20_000).await;
    e.manager.pause(&id).unwrap();
    let paused = wait_for(&e.manager, &id, |t| t.record.state == TaskState::Paused).await;
    assert!(paused.record.downloaded >= 20_000);
    assert!(e.out.join("Zoo.f134.mp4.part").exists());

    e.manager.resume(&id).unwrap();
    wait_for(&e.manager, &id, |t| {
        t.record.state == TaskState::Downloading && t.record.downloaded > paused.record.downloaded
    })
    .await;
    e.manager.cancel(&id).unwrap();
    let canceled = wait_for(&e.manager, &id, |t| t.record.state == TaskState::Canceled).await;
    assert_eq!(canceled.record.downloaded, 0);
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(downpipe_core::providers::files::partials(&e.out, "Zoo").is_empty());
    assert_eq!(calls(&e.yt_dlp).len(), 2);
}

async fn unnamed_downloads_take_the_title() {
    let e = env(&Scenario {
        info: info_fixture(),
        parts: vec![FakePart {
            format_id: "140".into(),
            ext: "m4a".into(),
            size: 4_000,
            vcodec: Some("none".into()),
            acodec: Some("mp4a.40.2".into()),
            ..FakePart::default()
        }],
        step: 4_000,
        ..Scenario::default()
    })
    .await;
    // Added by hand (no dialog): starts at once with the best quality.
    let task = add(
        &e.manager,
        "https://www.youtube.com/watch?v=jNQXAC9IVRw",
        false,
    );
    let done = wait_for(&e.manager, &task.record.id, |t| {
        t.record.state.is_finished() || t.record.state == TaskState::Failed
    })
    .await;
    assert_eq!(
        done.record.state,
        TaskState::Completed,
        "{:?}",
        done.record.error
    );
    assert_eq!(done.record.file_name.as_deref(), Some("Me at the zoo.m4a"));
    assert_eq!(
        done.record.site.unwrap().quality.as_deref(),
        Some("Audio only")
    );
}

async fn missing_yt_dlp_is_explained() {
    let e = env(&Scenario::default()).await;
    e.manager
        .update_settings(Settings {
            yt_dlp_path: e
                .out
                .join("nowhere")
                .join(fake_tools::exe_name("yt-dlp"))
                .to_string_lossy()
                .into_owned(),
            ..e.manager.settings()
        })
        .unwrap();
    let task = add(
        &e.manager,
        "https://www.youtube.com/watch?v=jNQXAC9IVRw",
        false,
    );
    let failed = wait_for(&e.manager, &task.record.id, |t| {
        t.record.state == TaskState::Failed
    })
    .await;
    assert_eq!(failed.record.error_kind.as_deref(), Some("tool_missing"));
    assert!(failed.record.error.unwrap().contains("Settings, Media"));
    let inspect = e.manager.inspect_site(&task.record.id).await.unwrap_err();
    assert_eq!(inspect.code(), "tool_missing");
    let status = e.manager.site_tools().await;
    assert!(status.yt_dlp.path.is_none());
    assert!(status.yt_dlp.problem.is_some());
    assert_eq!(status.js.version.as_deref(), Some("2.9.7"));
}

fn playlist_fixture() -> serde_json::Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../providers/tests/fixtures/youtube-playlist.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

const LIST: &str = "https://www.youtube.com/playlist?list=PLbpi6ZahtOH6Blw3RGYpWkSByi_T7Rygb";

async fn playlists_list_and_add_chosen_videos() {
    let e = env(&Scenario {
        info: info_fixture(),
        playlist: playlist_fixture(),
        parts: vec![FakePart {
            format_id: "18".into(),
            ext: "mp4".into(),
            size: 3_000,
            height: Some(360),
            vcodec: Some("avc1.42001E".into()),
            acodec: Some("mp4a.40.2".into()),
        }],
        step: 3_000,
        ..Scenario::default()
    })
    .await;
    e.manager
        .update_settings(Settings {
            media_name_template: "{index} - {title}".into(),
            ..e.manager.settings()
        })
        .unwrap();

    // A shared link of the list reads the list's own page.
    let choices = e
        .manager
        .inspect_list(&format!("{LIST}&si=abc"))
        .await
        .unwrap();
    assert_eq!(choices.kind, "playlist");
    assert_eq!(choices.url, LIST);
    assert_eq!(choices.title.as_deref(), Some("Early YouTube"));
    assert_eq!(choices.entries.len(), 4);
    assert_eq!(choices.dir, e.out);
    assert!(choices.ffmpeg);
    assert!(calls(&e.yt_dlp)[0].contains(&"--flat-playlist".to_string()));

    let picks: Vec<ListPick> = choices
        .entries
        .iter()
        .take(2)
        .map(|v| ListPick {
            id: v.id.clone(),
            title: v.title.clone(),
            index: v.index,
        })
        .collect();
    let options = AddListOptions {
        url: LIST.into(),
        title: Some("Early/YouTube".into()),
        entries: picks,
        count: 12,
        preset: Some(Preset::P720),
        subtitles: vec![
            SubtitleRef {
                language: "en".into(),
                auto: true,
            },
            SubtitleRef {
                language: "bad,lang".into(),
                auto: false,
            },
        ],
        paused: true,
        ..AddListOptions::default()
    };
    let result = e.manager.add_list(options.clone()).unwrap();
    assert_eq!(result.added.len(), 2);
    assert_eq!(result.skipped, 0);
    let folder = e.out.join("Early-YouTube");
    let first = &result.added[0].record;
    assert_eq!(first.dir, folder);
    assert_eq!(first.url, "https://www.youtube.com/watch?v=jNQXAC9IVRw");
    assert_eq!(first.state, TaskState::Paused);
    assert_eq!(result.added[1].record.position, first.position + 1);
    let site = first.site.as_ref().unwrap();
    assert_eq!(site.preset, Some(Preset::P720));
    assert_eq!(site.index, Some(1));
    assert_eq!(site.count, Some(12));
    assert_eq!(site.title.as_deref(), Some("Me at the zoo"));
    assert_eq!(
        site.subtitles,
        [SubtitleRef {
            language: "en".into(),
            auto: true
        }]
    );
    // Kept across a restart: the catalog has them.
    assert!(e.manager.get(&first.id).is_some());

    // The same videos again are skipped.
    let again = e.manager.add_list(options).unwrap();
    assert!(again.added.is_empty());
    assert_eq!(again.skipped, 2);

    // Started, the video is named by the template with its position.
    let id = first.id.clone();
    e.manager.resume(&id).unwrap();
    let done = wait_for(&e.manager, &id, |t| {
        t.record.state.is_finished() || t.record.state == TaskState::Failed
    })
    .await;
    assert_eq!(
        done.record.state,
        TaskState::Completed,
        "{:?}",
        done.record.error
    );
    assert_eq!(
        done.record.path,
        Some(folder.join("01 - Me at the zoo.mp4"))
    );
    let args = calls(&e.yt_dlp).last().unwrap().clone();
    // FFmpeg is set up: the subtitles go into the video.
    assert!(args.contains(&"--embed-subs".to_string()));
    assert!(!folder.join("01 - Me at the zoo.en.vtt").exists());
    let at = |flag: &str| args[args.iter().position(|a| a == flag).unwrap() + 1].clone();
    assert_eq!(at("-f"), "bv*[height<=720]+ba/b[height<=720]/b");
    assert_eq!(at("--sub-langs"), "en");
    assert!(args.contains(&"--write-auto-subs".to_string()));

    // From the browser: a window is asked to open the list.
    let mut events = e.manager.subscribe();
    let url = e
        .manager
        .offer_list("https://m.youtube.com/@jawed/featured")
        .unwrap();
    assert_eq!(url, "https://www.youtube.com/@jawed/videos");
    match events.recv().await.unwrap() {
        ManagerEvent::ListOffered { url } => {
            assert_eq!(url, "https://www.youtube.com/@jawed/videos")
        }
        other => panic!("{other:?}"),
    }
    assert!(
        e.manager
            .offer_list("https://www.youtube.com/watch?v=jNQXAC9IVRw")
            .is_err()
    );
}

async fn template_names_and_subtitles_in_the_dialog() {
    let e = env(&Scenario {
        info: info_fixture(),
        ..Scenario::default()
    })
    .await;
    e.manager
        .update_settings(Settings {
            media_name_template: "{channel} - {title} [{id}]".into(),
            ..e.manager.settings()
        })
        .unwrap();
    let task = add(
        &e.manager,
        "https://www.youtube.com/watch?v=jNQXAC9IVRw",
        true,
    );
    let choices = e.manager.inspect_site(&task.record.id).await.unwrap();
    assert_eq!(
        choices.name.as_deref(),
        Some("jawed - Me at the zoo [jNQXAC9IVRw].mp4")
    );
    let langs: Vec<(&str, bool)> = choices
        .subtitles
        .iter()
        .map(|s| (s.language.as_str(), s.auto))
        .collect();
    assert_eq!(langs[0], ("de", false));
    assert!(langs.contains(&("en-orig", true)));
    assert!(choices.chosen_subtitles.is_empty());

    e.manager
        .confirm(
            &task.record.id,
            ConfirmOptions {
                start: false,
                subtitles: Some(vec![
                    SubtitleRef {
                        language: "de".into(),
                        auto: false,
                    },
                    SubtitleRef {
                        language: "x;y".into(),
                        auto: false,
                    },
                ]),
                ..ConfirmOptions::default()
            },
        )
        .unwrap();
    let saved = e.manager.get(&task.record.id).unwrap();
    assert_eq!(
        saved.record.site.unwrap().subtitles,
        [SubtitleRef {
            language: "de".into(),
            auto: false
        }]
    );
    // An empty template is the title again.
    e.manager
        .update_settings(Settings {
            media_name_template: "  ".into(),
            ..e.manager.settings()
        })
        .unwrap();
    assert_eq!(e.manager.settings().media_name_template, "{title}");
}

async fn site_failures_keep_their_kind() {
    let e = env(&Scenario {
        error: Some("[youtube] jNQXAC9IVRw: Sign in to confirm you're not a bot.".into()),
        ..Scenario::default()
    })
    .await;
    let task = add(
        &e.manager,
        "https://www.youtube.com/watch?v=jNQXAC9IVRw",
        false,
    );
    let failed = wait_for(&e.manager, &task.record.id, |t| {
        t.record.state == TaskState::Failed
    })
    .await;
    assert_eq!(failed.record.error_kind.as_deref(), Some("site_sign_in"));
    let inspect = e.manager.inspect_site(&task.record.id).await.unwrap_err();
    assert_eq!(inspect.code(), "site_sign_in");
    let list = e.manager.inspect_list(LIST).await.unwrap_err();
    assert_eq!(list.code(), "site_sign_in");
}
