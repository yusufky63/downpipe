//! Post-download actions through the manager: a command, a virus scan and
//! archive extraction, against the test server.
//!
//! `harness = false`: this binary is also the program and the scanner the
//! cases run. Copied under the name `fake-command` it logs its arguments and
//! prints a little; as `fake-scanner` it reports a threat in any file whose
//! name contains `infected`, ClamAV style.

use std::future::Future;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::Duration;

use downpipe_core::http::{EngineConfig, KIB};
use downpipe_core::{
    AddOptions, CommandResult, CoreError, ExtractResult, Manager, PostActions, Queue, ScanResult,
    Settings, TaskSnapshot, TaskState,
};
use downpipe_test_server::TestServer;

type Case = fn() -> Pin<Box<dyn Future<Output = ()>>>;

fn main() {
    run_if_fake();
    let cases: &[(&str, Case)] = &[
        ("command_gets_each_value_as_one_argument", || {
            Box::pin(command_gets_each_value_as_one_argument())
        }),
        ("failed_downloads_run_nothing", || {
            Box::pin(failed_downloads_run_nothing())
        }),
        ("the_download_then_the_queue_choose_the_command", || {
            Box::pin(the_download_then_the_queue_choose_the_command())
        }),
        ("slow_commands_are_stopped", || {
            Box::pin(slow_commands_are_stopped())
        }),
        ("a_threat_stops_the_rest_and_deletes_nothing", || {
            Box::pin(a_threat_stops_the_rest_and_deletes_nothing())
        }),
        ("clean_archives_extract_then_go", || {
            Box::pin(clean_archives_extract_then_go())
        }),
        ("command_lines_that_cannot_run_are_refused", || {
            Box::pin(command_lines_that_cannot_run_are_refused())
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

/// Play the command or the scanner when started under their names.
fn run_if_fake() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let role = exe
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    if !role.starts_with("fake-") {
        return;
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Ok(mut log) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(calls_file(&exe))
    {
        let _ = writeln!(log, "{}", serde_json::json!(args));
    }
    if role == "fake-scanner" {
        let file = args.last().cloned().unwrap_or_default();
        if file.contains("infected") {
            println!("{file}: Fake.Threat-1 FOUND");
            std::process::exit(1);
        }
        println!("{file}: OK");
        std::process::exit(0);
    }
    match args.first().map(String::as_str) {
        Some("sleep") => std::thread::sleep(Duration::from_secs(120)),
        Some("fail") => {
            eprintln!("something went wrong");
            std::process::exit(3);
        }
        _ => {}
    }
    println!("ran with {} arguments", args.len());
    eprintln!("a warning");
    std::process::exit(0);
}

fn calls_file(exe: &Path) -> PathBuf {
    exe.with_extension("calls.jsonl")
}

fn install_fake(dir: &Path, name: &str) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let exe = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    std::fs::copy(std::env::current_exe().unwrap(), &exe).unwrap();
    exe
}

fn calls(exe: &Path) -> Vec<Vec<String>> {
    std::fs::read_to_string(calls_file(exe))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

/// `exe` as the program of a command line, quoted for the spaces in
/// temporary folders.
fn quoted(exe: &Path) -> String {
    format!("\"{}\"", exe.display())
}

fn config() -> EngineConfig {
    EngineConfig {
        max_connections: 2,
        initial_connections: 2,
        min_split_size: 64 * KIB,
        checkpoint_interval: Duration::from_millis(100),
        retry_base_delay: Duration::from_millis(50),
        retry_max_delay: Duration::from_millis(200),
        max_retries: 1,
        ..EngineConfig::default()
    }
}

struct Env {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    downloads: PathBuf,
    server: TestServer,
    manager: Manager,
    command: PathBuf,
    scanner: PathBuf,
}

async fn env(settings: impl FnOnce(&mut Settings, &Path, &Path)) -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let downloads = root.join("my downloads");
    std::fs::create_dir_all(&downloads).unwrap();
    let tools = root.join("tool folder");
    let command = install_fake(&tools, "fake-command");
    let scanner = install_fake(&tools, "fake-scanner");
    let server = TestServer::start().await.unwrap();
    let manager = Manager::open_with(&root.join("data"), config())
        .await
        .unwrap();
    let mut s = manager.settings();
    s.download_dir = downloads.clone();
    settings(&mut s, &command, &scanner);
    manager.update_settings(s).expect("settings");
    Env {
        _tmp: tmp,
        root,
        downloads,
        server,
        manager,
        command,
        scanner,
    }
}

impl Env {
    fn add(&self, path: &str, name: &str, post: PostActions) -> TaskSnapshot {
        self.manager
            .add(AddOptions {
                url: self.server.url(path),
                dir: Some(self.downloads.clone()),
                file_name: Some(name.into()),
                post,
                ..AddOptions::default()
            })
            .expect("add")
    }

    /// The download once it completed and its post-download steps ended.
    async fn finished(&self, id: &str) -> TaskSnapshot {
        wait_for(&self.manager, id, |t| {
            matches!(t.record.state, TaskState::Completed | TaskState::Failed)
                && t.live.post_step.is_none()
        })
        .await
    }
}

async fn wait_for(
    manager: &Manager,
    id: &str,
    cond: impl Fn(&TaskSnapshot) -> bool,
) -> TaskSnapshot {
    tokio::time::timeout(Duration::from_secs(40), async {
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
    .unwrap_or_else(|_| panic!("not reached in time: {:?}", manager.get(id)))
}

async fn command_gets_each_value_as_one_argument() {
    let e = env(|s, command, _| {
        s.post.command = format!(
            "{} {{path}} {{name}} --in={{folder}} {{url}}",
            quoted(command)
        );
    })
    .await;
    // Spaces, quotes, `;`, `&` and `$( )` in a name would split or run
    // something in a shell; here they arrive as one argument.
    let name = "my file; echo pwned & $(whoami) 'x'.bin";
    let task = e.add("/file/2048?x=1&y=2", name, PostActions::default());
    let done = e.finished(&task.record.id).await;
    assert_eq!(done.record.state, TaskState::Completed);
    let path = done.record.path.clone().unwrap();
    let report = done.record.post.clone().unwrap().command.unwrap();
    assert_eq!(report.result, CommandResult::Ok, "{report:?}");
    assert_eq!(report.exit_code, Some(0));
    assert!(report.output.contains("ran with 4 arguments"), "{report:?}");
    assert!(report.output.contains("a warning"), "stderr is kept too");
    assert_eq!(
        calls(&e.command),
        vec![vec![
            path.to_string_lossy().into_owned(),
            name.to_string(),
            format!("--in={}", e.downloads.display()),
            e.server.url("/file/2048?x=1&y=2"),
        ]]
    );
    // The report survives a restart.
    let report = done.record.post.clone();
    e.manager.shutdown().await;
    let again = Manager::open_with(&e.root.join("data"), config())
        .await
        .unwrap();
    assert_eq!(again.get(&task.record.id).unwrap().record.post, report);
}

async fn failed_downloads_run_nothing() {
    let e = env(|s, command, scanner| {
        s.post.command = format!("{} {{path}}", quoted(command));
        s.post.scan = true;
        s.post.scanner = quoted(scanner);
    })
    .await;
    let task = e.add("/file/2048?fail=404", "missing.bin", PostActions::default());
    let done = e.finished(&task.record.id).await;
    assert_eq!(done.record.state, TaskState::Failed);
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(calls(&e.command).is_empty());
    assert!(calls(&e.scanner).is_empty());
    assert_eq!(e.manager.get(&task.record.id).unwrap().record.post, None);
}

async fn the_download_then_the_queue_choose_the_command() {
    let e = env(|s, command, _| {
        s.post.command = format!("{} global", quoted(command));
        s.queues.push(Queue {
            id: "night".into(),
            name: "Night".into(),
            command: format!("{} queue {{name}}", quoted(command)),
            ..Queue::default()
        });
    })
    .await;
    let in_queue = e
        .manager
        .add(AddOptions {
            url: e.server.url("/file/1024"),
            dir: Some(e.downloads.clone()),
            file_name: Some("q.bin".into()),
            queue: Some("night".into()),
            ..AddOptions::default()
        })
        .unwrap();
    e.finished(&in_queue.record.id).await;
    let own = e.add(
        "/file/1024?v=2",
        "own.bin",
        PostActions {
            command: format!("{} fail", quoted(&e.command)),
            extract: None,
        },
    );
    let own = e.finished(&own.record.id).await;
    let plain = e.add("/file/1024?v=3", "plain.bin", PostActions::default());
    e.finished(&plain.record.id).await;

    let calls = calls(&e.command);
    assert_eq!(calls[0], ["queue", "q.bin"]);
    assert_eq!(calls[1], ["fail"]);
    assert_eq!(calls[2], ["global"]);
    let report = own.record.post.unwrap().command.unwrap();
    assert_eq!(report.result, CommandResult::Failed);
    assert_eq!(report.exit_code, Some(3));
    assert!(report.output.contains("something went wrong"));
}

async fn slow_commands_are_stopped() {
    let e = env(|s, command, _| {
        s.post.command = format!("{} sleep", quoted(command));
        s.post.timeout_secs = 5;
    })
    .await;
    let task = e.add("/file/1024", "slow.bin", PostActions::default());
    let started = std::time::Instant::now();
    let done = e.finished(&task.record.id).await;
    let report = done.record.post.unwrap().command.unwrap();
    assert_eq!(report.result, CommandResult::TimedOut);
    assert!(started.elapsed() < Duration::from_secs(30));
}

/// A zip with one file, served by the test server.
fn serve_zip(e: &Env, name: &str) -> String {
    let dir = e.root.join("served");
    std::fs::create_dir_all(&dir).unwrap();
    let mut w = zip::ZipWriter::new(std::fs::File::create(dir.join(name)).unwrap());
    w.start_file(
        "inside/readme.txt",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    w.write_all(b"extracted by Downpipe").unwrap();
    w.finish().unwrap();
    e.server.serve_files(&dir);
    format!("/files/{name}")
}

async fn a_threat_stops_the_rest_and_deletes_nothing() {
    let e = env(|s, command, scanner| {
        s.post.command = format!("{} {{path}}", quoted(command));
        s.post.scan = true;
        s.post.scanner = format!("{} --no-summary", quoted(scanner));
        s.post.delete_archive = true;
        for c in &mut s.categories {
            c.extract = true;
        }
    })
    .await;
    let path = serve_zip(&e, "infected.zip");
    let task = e.add(&path, "infected.zip", PostActions::default());
    let done = e.finished(&task.record.id).await;
    let report = done.record.post.clone().unwrap();
    let scan = report.scan.clone().unwrap();
    assert_eq!(scan.result, ScanResult::Threat);
    assert_eq!(scan.detail.as_deref(), Some("Fake.Threat-1"));
    assert_eq!(
        scan.scanner,
        format!("fake-scanner{}", std::env::consts::EXE_SUFFIX)
    );
    assert!(report.stopped_by_threat && report.threat_found());
    assert!(report.extract.is_none() && report.command.is_none());
    assert!(calls(&e.command).is_empty(), "the command did not run");
    let file = done.record.path.unwrap();
    assert!(file.exists(), "the flagged file is left where it is");
    assert!(!e.downloads.join("infected").exists(), "nothing extracted");
    // The scanner got its own argument and the file, last.
    assert_eq!(
        calls(&e.scanner),
        vec![vec![
            "--no-summary".to_string(),
            file.to_string_lossy().into_owned()
        ]]
    );
}

async fn clean_archives_extract_then_go() {
    let e = env(|s, command, scanner| {
        s.post.command = format!("{} {{path}}", quoted(command));
        s.post.scan = true;
        s.post.scanner = format!("{} {{path}}", quoted(scanner));
        s.post.delete_archive = true;
        if let Some(c) = s.categories.iter_mut().find(|c| c.name == "Compressed") {
            c.extract = true;
        }
    })
    .await;
    let path = serve_zip(&e, "pack.zip");
    let task = e.add(&path, "pack.zip", PostActions::default());
    let done = e.finished(&task.record.id).await;
    let report = done.record.post.clone().unwrap();
    assert_eq!(report.scan.unwrap().result, ScanResult::Clean);
    let extract = report.extract.unwrap();
    assert_eq!(extract.result, ExtractResult::Done, "{extract:?}");
    assert_eq!((extract.files, extract.skipped), (1, 0));
    let folder = extract.folder.unwrap();
    assert_eq!(folder, e.downloads.join("pack"));
    assert_eq!(
        std::fs::read(folder.join("inside").join("readme.txt")).unwrap(),
        b"extracted by Downpipe"
    );
    assert!(extract.archive_deleted);
    let archive = done.record.path.unwrap();
    assert!(!archive.exists(), "deleted after the command ran");
    assert_eq!(
        calls(&e.command),
        vec![vec![archive.to_string_lossy().into_owned()]],
        "the command still saw the archive"
    );
    assert_eq!(report.command.unwrap().result, CommandResult::Ok);

    // A download's own choice beats its category.
    let path = serve_zip(&e, "keep.zip");
    let kept = e.add(
        &path,
        "keep.zip",
        PostActions {
            command: String::new(),
            extract: Some(false),
        },
    );
    let kept = e.finished(&kept.record.id).await;
    assert!(kept.record.post.unwrap().extract.is_none());
    assert!(kept.record.path.unwrap().exists());
}

async fn command_lines_that_cannot_run_are_refused() {
    let e = env(|_, _, _| {}).await;
    let mut s = e.manager.settings();
    s.post.command = "\"C:\\unclosed.exe {path}".into();
    assert!(matches!(
        e.manager.update_settings(s),
        Err(CoreError::InvalidSetting(_))
    ));
    let mut s = e.manager.settings();
    s.post.scanner = "{path}".into();
    assert!(matches!(
        e.manager.update_settings(s),
        Err(CoreError::InvalidSetting(_))
    ));
    let refused = e.manager.add(AddOptions {
        url: e.server.url("/file/10"),
        post: PostActions {
            command: "relative\\tool.exe {path}".into(),
            extract: None,
        },
        paused: true,
        ..AddOptions::default()
    });
    assert!(matches!(refused, Err(CoreError::InvalidSetting(_))));

    let task = e.add("/file/10?p=1", "a.bin", PostActions::default());
    e.manager.pause(&task.record.id).ok();
    let actions = PostActions {
        command: format!("  {} {{name}} ", quoted(&e.command)),
        extract: Some(true),
    };
    e.manager
        .set_post_actions(&task.record.id, actions)
        .unwrap();
    let stored = e.manager.get(&task.record.id).unwrap().record.post_actions;
    assert_eq!(stored.command, format!("{} {{name}}", quoted(&e.command)));
    assert_eq!(stored.extract, Some(true));
}
