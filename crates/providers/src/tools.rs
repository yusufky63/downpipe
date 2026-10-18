//! The programs a site download runs, found on this machine.
//!
//! * yt-dlp extracts the video and downloads it.
//! * A JavaScript runtime solves YouTube's player challenges for yt-dlp:
//!   Deno 2.3.0 or newer (yt-dlp's default), or else Node 22 or newer.
//! * FFmpeg joins the separate video and sound YouTube serves above 360p.
//!
//! Each is looked up in the path set in the settings (a wrong path counts
//! as missing, so a typo is not papered over by another copy), then in
//! Downpipe's own tools folder (filled only by [`crate::install`] after the
//! user asks), then on `PATH`. Node is only taken from `PATH`.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::install::{self, Records, Tool};
use crate::process;

/// Oldest Deno yt-dlp accepts.
pub const MIN_DENO: (u32, u32, u32) = (2, 3, 0);
/// Oldest Node yt-dlp accepts.
pub const MIN_NODE: (u32, u32, u32) = (22, 0, 0);

/// Where the user said the tools are. Empty paths mean "find it".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolConfig {
    pub yt_dlp: PathBuf,
    pub deno: PathBuf,
    pub ffmpeg: PathBuf,
    /// Downpipe's tools folder, where [`crate::install`] puts its downloads.
    pub managed_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JsKind {
    Deno,
    Node,
}

impl JsKind {
    /// The name yt-dlp's `--js-runtimes` takes.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Deno => "deno",
            Self::Node => "node",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsRuntime {
    pub kind: JsKind,
    pub path: PathBuf,
}

/// The tools a download would use right now.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tools {
    pub yt_dlp: Option<PathBuf>,
    pub js: Option<JsRuntime>,
    pub ffmpeg: Option<PathBuf>,
}

/// Where a tool was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolSource {
    /// The path in the settings.
    Settings,
    /// Downpipe's tools folder.
    Managed,
    Path,
}

/// File name of a program on this system.
pub fn exe_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

/// Look a tool up without starting anything. Cheap enough to call each
/// time a download starts.
pub fn find(config: &ToolConfig) -> Tools {
    let yt_dlp = locate(&config.yt_dlp, config.managed_dir.as_deref(), "yt-dlp").map(|(p, _)| p);
    let js = locate(&config.deno, config.managed_dir.as_deref(), "deno")
        .map(|(path, _)| JsRuntime {
            kind: JsKind::Deno,
            path,
        })
        .or_else(|| {
            // Node is a fallback only when no Deno is set or found.
            config.deno.as_os_str().is_empty().then_some(())?;
            on_path("node").map(|path| JsRuntime {
                kind: JsKind::Node,
                path,
            })
        });
    let ffmpeg = find_ffmpeg(config);
    Tools { yt_dlp, js, ffmpeg }
}

/// FFmpeg alone, for stream downloads, which need nothing else.
pub fn find_ffmpeg(config: &ToolConfig) -> Option<PathBuf> {
    locate(&config.ffmpeg, config.managed_dir.as_deref(), "ffmpeg").map(|(p, _)| p)
}

/// Settings path, else the tools folder, else `PATH`.
fn locate(configured: &Path, managed: Option<&Path>, name: &str) -> Option<(PathBuf, ToolSource)> {
    if !configured.as_os_str().is_empty() {
        return configured
            .is_file()
            .then(|| (configured.to_path_buf(), ToolSource::Settings));
    }
    if let Some(dir) = managed {
        let path = dir.join(exe_name(name));
        if path.is_file() {
            return Some((path, ToolSource::Managed));
        }
    }
    on_path(name).map(|p| (p, ToolSource::Path))
}

fn on_path(name: &str) -> Option<PathBuf> {
    let exe = exe_name(name);
    std::env::split_paths(&std::env::var_os("PATH")?)
        .filter(|dir| dir.is_absolute())
        .map(|dir| dir.join(&exe))
        .find(|candidate| candidate.is_file())
}

/// One tool, for the settings page.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ToolStatus {
    pub path: Option<PathBuf>,
    pub source: Option<ToolSource>,
    /// What the program says its version is.
    pub version: Option<String>,
    /// Why it cannot be used, when it was found but is not usable.
    pub problem: Option<String>,
}

/// All tools, for the settings page.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ToolsStatus {
    pub yt_dlp: ToolStatus,
    /// Deno, or Node when no Deno is found.
    pub js: ToolStatus,
    pub js_kind: Option<JsKind>,
    pub ffmpeg: ToolStatus,
    /// Where the download buttons put the tools.
    pub managed_dir: Option<PathBuf>,
    /// About how much each download is; `None` where Downpipe has no build
    /// to download for this system (FFmpeg on macOS).
    pub sizes: Sizes,
    /// When Downpipe last looked for a newer yt-dlp, in seconds since the
    /// epoch; `None` before the first look.
    pub yt_dlp_checked: Option<u64>,
}

/// Bytes, approximately.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Sizes {
    pub yt_dlp: Option<u64>,
    pub deno: Option<u64>,
    pub ffmpeg: Option<u64>,
}

impl Sizes {
    pub fn here() -> Self {
        let size =
            |tool| install::download_size(tool, std::env::consts::OS, std::env::consts::ARCH);
        Self {
            yt_dlp: size(Tool::YtDlp),
            deno: size(Tool::Deno),
            ffmpeg: size(Tool::Ffmpeg),
        }
    }
}

/// Find every tool and ask each for its version. Starts each program once.
pub async fn status(config: &ToolConfig) -> ToolsStatus {
    let managed = config.managed_dir.as_deref();

    let yt_dlp = match locate(&config.yt_dlp, managed, "yt-dlp") {
        Some((path, source)) => {
            let version = process::first_line(&path, &["--version"]).await;
            let problem = version
                .is_none()
                .then(|| "It did not start. Download it again, or check the path.".to_string());
            ToolStatus {
                path: Some(path),
                source: Some(source),
                version,
                problem,
            }
        }
        None => missing(&config.yt_dlp, "yt-dlp"),
    };

    let (js, js_kind) = match locate(&config.deno, managed, "deno") {
        Some((path, source)) => (
            runtime_status(path, source, JsKind::Deno).await,
            Some(JsKind::Deno),
        ),
        None if config.deno.as_os_str().is_empty() => match on_path("node") {
            Some(path) => (
                runtime_status(path, ToolSource::Path, JsKind::Node).await,
                Some(JsKind::Node),
            ),
            None => (ToolStatus::default(), None),
        },
        None => (missing(&config.deno, "deno"), None),
    };

    let ffmpeg = match locate(&config.ffmpeg, managed, "ffmpeg") {
        Some((path, source)) => {
            let line = process::first_line(&path, &["-version"]).await;
            let version = line.as_deref().and_then(ffmpeg_version);
            let problem = line
                .is_none()
                .then(|| "It did not start. Download it again, or check the path.".to_string());
            ToolStatus {
                path: Some(path),
                source: Some(source),
                version,
                problem,
            }
        }
        None => missing(&config.ffmpeg, "ffmpeg"),
    };

    ToolsStatus {
        yt_dlp,
        js,
        js_kind,
        ffmpeg,
        managed_dir: config.managed_dir.clone(),
        sizes: Sizes::here(),
        yt_dlp_checked: managed.and_then(|dir| Records::read(dir).yt_dlp_checked),
    }
}

fn missing(configured: &Path, name: &str) -> ToolStatus {
    ToolStatus {
        problem: (!configured.as_os_str().is_empty())
            .then(|| format!("There is no {name} at this path.")),
        ..ToolStatus::default()
    }
}

async fn runtime_status(path: PathBuf, source: ToolSource, kind: JsKind) -> ToolStatus {
    let line = process::first_line(&path, &["--version"]).await;
    let version = line.as_deref().and_then(|l| runtime_version(kind, l));
    let (min, name) = match kind {
        JsKind::Deno => (MIN_DENO, "Deno"),
        JsKind::Node => (MIN_NODE, "Node"),
    };
    let problem = match version.as_deref().and_then(parse_version) {
        None => Some(format!("{name} did not start. Check the path.")),
        Some(v) if v < min => Some(format!(
            "yt-dlp needs {name} {}.{}.{} or newer.",
            min.0, min.1, min.2
        )),
        Some(_) => None,
    };
    ToolStatus {
        path: Some(path),
        source: Some(source),
        version,
        problem,
    }
}

/// "2.9.7" from Deno's "deno 2.9.7 (stable, ...)", "24.19.0" from Node's "v24.19.0".
pub fn runtime_version(kind: JsKind, line: &str) -> Option<String> {
    let token = match kind {
        JsKind::Deno => line.strip_prefix("deno ")?.split_whitespace().next()?,
        JsKind::Node => line.trim().strip_prefix('v')?,
    };
    parse_version(token).map(|_| token.to_string())
}

/// "7.1" from "ffmpeg version 7.1-full_build-www.gyan.dev Copyright ...".
fn ffmpeg_version(line: &str) -> Option<String> {
    let rest = line.strip_prefix("ffmpeg version ")?;
    rest.split_whitespace()
        .next()
        .map(|v| v.chars().take(40).collect())
}

/// `major.minor.patch`; missing parts count as 0.
pub fn parse_version(text: &str) -> Option<(u32, u32, u32)> {
    let mut parts = text.split('.').map(|p| {
        let digits: String = p.chars().take_while(char::is_ascii_digit).collect();
        digits.parse::<u32>().ok()
    });
    let major = parts.next()??;
    let minor = parts.next().flatten().unwrap_or(0);
    let patch = parts.next().flatten().unwrap_or(0);
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert_eq!(
            runtime_version(
                JsKind::Deno,
                "deno 2.9.7 (stable, release, x86_64-pc-windows-msvc)"
            ),
            Some("2.9.7".into())
        );
        assert_eq!(
            runtime_version(JsKind::Node, "v24.19.0"),
            Some("24.19.0".into())
        );
        assert_eq!(runtime_version(JsKind::Deno, "v24.19.0"), None);
        assert_eq!(parse_version("2.3.0"), Some((2, 3, 0)));
        assert_eq!(parse_version("22"), Some((22, 0, 0)));
        assert_eq!(parse_version("x"), None);
        assert!(parse_version("2.2.15").unwrap() < MIN_DENO);
        assert!(parse_version("2.10.0").unwrap() > MIN_DENO);
        assert_eq!(
            ffmpeg_version("ffmpeg version 7.1-full_build-www.gyan.dev Copyright (c) 2000-2024"),
            Some("7.1-full_build-www.gyan.dev".into())
        );
    }

    #[test]
    fn settings_path_then_managed_folder() {
        let dir = tempfile::tempdir().unwrap();
        let managed = dir.path().join("tools");
        std::fs::create_dir_all(&managed).unwrap();
        let config = ToolConfig {
            managed_dir: Some(managed.clone()),
            ..ToolConfig::default()
        };
        // Nothing in the folder: whatever PATH has, if anything.
        let before = find(&config).yt_dlp;
        assert!(before.as_ref().is_none_or(|p| !p.starts_with(&managed)));

        let managed_exe = managed.join(exe_name("yt-dlp"));
        std::fs::write(&managed_exe, b"").unwrap();
        assert_eq!(find(&config).yt_dlp, Some(managed_exe));

        // A path in the settings wins, and a wrong one is not replaced.
        let own = dir.path().join(exe_name("my-yt-dlp"));
        std::fs::write(&own, b"").unwrap();
        let mut config = config;
        config.yt_dlp = own.clone();
        assert_eq!(find(&config).yt_dlp, Some(own));
        config.yt_dlp = dir.path().join("missing.exe");
        assert_eq!(find(&config).yt_dlp, None);
    }

    #[test]
    fn a_configured_deno_that_is_missing_does_not_fall_back_to_node() {
        let config = ToolConfig {
            deno: PathBuf::from("Z:/nowhere/deno.exe"),
            ..ToolConfig::default()
        };
        assert_eq!(find(&config).js, None);
    }
}
