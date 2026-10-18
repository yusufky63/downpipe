//! Stand-ins for yt-dlp and Deno, for tests that start them as processes.
//!
//! A test copies its own executable under a tool's name with [`install`]
//! and writes a [`Scenario`] next to it. The test binary's `main` (these
//! tests use `harness = false`) calls [`run_if_fake`] first: started under a
//! tool's name, the binary plays that tool and exits instead of running the
//! tests. No interpreter, network or real yt-dlp is involved.
//!
//! The fake yt-dlp understands what Downpipe passes: `--version`, `-J`
//! (`--flat-playlist -J` prints the scenario's playlist), and a download
//! with `-P`, `-o` and the `DOWNPIPE:` print templates, writing the asked
//! subtitles next to the file unless they are to be embedded. It writes
//! `.part` files in steps, resumes them, "merges" parts by concatenating
//! them, and logs each invocation's arguments as a JSON line in
//! `<name>.calls.jsonl` next to itself.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// One format the fake download writes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FakePart {
    pub format_id: String,
    pub ext: String,
    pub size: u64,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub vcodec: Option<String>,
    #[serde(default)]
    pub acodec: Option<String>,
}

/// How the fake tool behaves.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Scenario {
    /// `--version` output.
    pub version: String,
    /// Printed for `-J`.
    pub info: Value,
    /// Printed for `--flat-playlist -J`.
    pub playlist: Value,
    /// Printed as `ERROR: <error>` (exit code 1) instead of doing the work.
    pub error: Option<String>,
    /// Fail like this once this many bytes of the download are written.
    pub error_after: Option<u64>,
    /// Printed as `WARNING: <warning>` first.
    pub warning: Option<String>,
    /// The formats a download writes, in order.
    pub parts: Vec<FakePart>,
    /// Extension of the merged file when there are two parts.
    pub merged_ext: String,
    /// Bytes per progress line.
    pub step: u64,
    /// Pause after each step.
    pub delay_ms: u64,
}

/// The executable's name for a tool on this system.
pub fn exe_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

/// Copy the running test binary to `dir` as `name` (`yt-dlp`, `deno`) and
/// give it `scenario`. Returns the executable's path.
pub fn install(dir: &Path, name: &str, scenario: &Scenario) -> PathBuf {
    std::fs::create_dir_all(dir).expect("create the fake tool's folder");
    let exe = dir.join(exe_name(name));
    let me = std::env::current_exe().expect("the test binary's path");
    std::fs::copy(me, &exe).expect("copy the test binary");
    set_scenario(&exe, scenario);
    exe
}

/// Change what an installed fake does next time it runs.
pub fn set_scenario(exe: &Path, scenario: &Scenario) {
    std::fs::write(
        sidecar(exe, "scenario.json"),
        serde_json::to_vec_pretty(scenario).expect("scenario as JSON"),
    )
    .expect("write the scenario");
}

/// The arguments of every run of an installed fake so far.
pub fn calls(exe: &Path) -> Vec<Vec<String>> {
    std::fs::read_to_string(sidecar(exe, "calls.jsonl"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

fn sidecar(exe: &Path, suffix: &str) -> PathBuf {
    let stem = exe
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    exe.with_file_name(format!("{stem}.{suffix}"))
}

/// Call first in a `harness = false` test's `main`: when this binary was
/// started as a fake tool, play it and exit.
pub fn run_if_fake() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let role = exe
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    if !matches!(role.as_str(), "yt-dlp" | "deno" | "node") {
        return;
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Ok(mut log) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(sidecar(&exe, "calls.jsonl"))
    {
        let _ = writeln!(log, "{}", json!(args));
    }
    let scenario: Scenario = std::fs::read(sidecar(&exe, "scenario.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let code = match role.as_str() {
        "yt-dlp" => yt_dlp(&args, &scenario),
        _ => {
            if args.first().map(String::as_str) == Some("--version") {
                println!("{}", scenario.version);
                0
            } else {
                1
            }
        }
    };
    std::process::exit(code);
}

fn value_after<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    let at = args.iter().position(|a| a == flag)?;
    args.get(at + 1).map(String::as_str)
}

fn fail(message: &str) -> i32 {
    eprintln!("ERROR: {message}");
    1
}

fn yt_dlp(args: &[String], s: &Scenario) -> i32 {
    if args.first().map(String::as_str) == Some("--version") {
        println!("{}", s.version);
        return 0;
    }
    if let Some(warning) = &s.warning {
        eprintln!("WARNING: {warning}");
    }
    if let Some(error) = &s.error {
        return fail(error);
    }
    if args.iter().any(|a| a == "-J") {
        if args.iter().any(|a| a == "--flat-playlist") {
            println!("{}", s.playlist);
        } else {
            println!("{}", s.info);
        }
        return 0;
    }
    let (Some(dir), Some(template)) = (value_after(args, "-P"), value_after(args, "-o")) else {
        return fail("the fake yt-dlp needs -P and -o");
    };
    let dir = PathBuf::from(dir);
    let stem = template
        .strip_suffix(".%(ext)s")
        .unwrap_or(template)
        .replace("%%", "%");
    let _ = std::fs::create_dir_all(&dir);

    let part_json = |p: &FakePart| {
        json!({"format_id": p.format_id, "ext": p.ext, "height": p.height, "vcodec": p.vcodec,
               "acodec": p.acodec, "filesize": p.size})
    };
    let merged = s.parts.len() > 1;
    let final_ext = if merged {
        s.merged_ext.clone()
    } else {
        s.parts.first().map(|p| p.ext.clone()).unwrap_or_default()
    };
    let final_path = dir.join(format!("{stem}.{final_ext}"));
    let done = |path: &Path| {
        println!(
            "DOWNPIPE:DONE {}",
            json!(path.to_string_lossy().into_owned())
        );
    };
    if final_path.exists() {
        done(&final_path);
        return 0;
    }
    let ids: Vec<&str> = s.parts.iter().map(|p| p.format_id.as_str()).collect();
    let single = match s.parts.first() {
        Some(first) if !merged => part_json(first),
        _ => json!({"format_id": ids.join("+"), "ext": final_ext}),
    };
    let list = if merged {
        Value::Array(s.parts.iter().map(part_json).collect())
    } else {
        Value::Null
    };
    println!("DOWNPIPE:START {single} {list}");

    // Subtitles asked for and not embedded land next to the video, as
    // `<stem>.<lang>.vtt`, before the media (as yt-dlp does it).
    let wants_subs = args
        .iter()
        .any(|a| a == "--write-subs" || a == "--write-auto-subs");
    if wants_subs && !args.iter().any(|a| a == "--embed-subs") {
        for lang in value_after(args, "--sub-langs").unwrap_or("").split(',') {
            if !lang.is_empty() {
                let _ = std::fs::write(
                    dir.join(format!("{stem}.{lang}.vtt")),
                    "WEBVTT\n\n00:00.000 --> 00:01.000\nHello\n",
                );
            }
        }
    }

    let step = s.step.max(1);
    let mut written_total: u64 = 0;
    let mut files = Vec::new();
    for part in &s.parts {
        let name = if merged {
            format!("{stem}.f{}.{}", part.format_id, part.ext)
        } else {
            format!("{stem}.{}", part.ext)
        };
        let path = dir.join(&name);
        let progress = |status: &str, downloaded: u64| {
            println!(
                "DOWNPIPE:PROGRESS {} {}",
                json!(part.format_id),
                json!({"status": status, "downloaded_bytes": downloaded,
                       "total_bytes": part.size, "speed": 1_048_576.0})
            );
        };
        if path.exists() {
            progress("finished", part.size);
            files.push(path);
            continue;
        }
        let part_path = dir.join(format!("{name}.part"));
        let mut have = std::fs::metadata(&part_path).map(|m| m.len()).unwrap_or(0);
        let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&part_path)
        else {
            return fail("could not open the part file");
        };
        while have < part.size {
            let n = step.min(part.size - have);
            let chunk: Vec<u8> = (have..have + n).map(|i| (i % 251) as u8).collect();
            if file.write_all(&chunk).and_then(|()| file.flush()).is_err() {
                return fail("could not write");
            }
            have += n;
            written_total += n;
            progress("downloading", have);
            if let Some(limit) = s.error_after
                && written_total >= limit
            {
                return fail(
                    "[youtube] jNQXAC9IVRw: Unable to download video data: HTTP Error 503",
                );
            }
            if s.delay_ms > 0 {
                std::thread::sleep(Duration::from_millis(s.delay_ms));
            }
        }
        drop(file);
        if std::fs::rename(&part_path, &path).is_err() {
            return fail("could not finish the part");
        }
        progress("finished", part.size);
        files.push(path);
    }
    if merged {
        let temp = dir.join(format!("{stem}.temp.{final_ext}"));
        let mut out = Vec::new();
        for f in &files {
            out.extend(std::fs::read(f).unwrap_or_default());
        }
        if std::fs::write(&temp, out).is_err() || std::fs::rename(&temp, &final_path).is_err() {
            return fail("could not merge");
        }
        for f in &files {
            let _ = std::fs::remove_file(f);
        }
    }
    done(&final_path);
    0
}

/// What the fake writes for a part of `size` bytes, to check a resumed file.
pub fn part_content(size: u64) -> Vec<u8> {
    (0..size).map(|i| (i % 251) as u8).collect()
}
