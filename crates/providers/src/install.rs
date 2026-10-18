//! Downloading yt-dlp, Deno and FFmpeg into Downpipe's tools folder, only
//! after the user presses a button for it (in the settings, or in a dialog
//! that needs a missing tool), and keeping Downpipe's own yt-dlp current.
//!
//! Each comes from its project's latest GitHub release: the release is found
//! by following `releases/latest`, the file is the build for this system,
//! and it is checked against the SHA-256 the release publishes
//! (`SHA2-256SUMS` for yt-dlp, `<file>.sha256sum` for Deno,
//! `checksums.sha256` for FFmpeg) before it replaces anything. A failed check
//! leaves the old copy in place.
//!
//! FFmpeg is BtbN/FFmpeg-Builds' static `lgpl` build (LGPL-3.0, no GPL-only
//! libraries such as x264), from its newest release branch, for Windows and
//! Linux on x86_64 and arm64. Only `ffmpeg` and `ffprobe` are taken from the
//! archive. There is no macOS build that publishes a checksum, so there it
//! stays the user's to install (Homebrew).
//!
//! What was installed is written to [`RECORDS`] in the tools folder, so the
//! daily yt-dlp update only ever replaces a copy Downpipe put there itself.

use std::io::{Read, Write as _};
use std::path::{Path, PathBuf};

use downpipe_http::DownloadError;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;

use crate::tools::{exe_name, parse_version};

/// Where releases are looked up; tests point this at a local server.
pub const GITHUB: &str = "https://github.com/";
/// No official build of any of them is near this; anything larger is refused.
const MAX_DOWNLOAD: u64 = 400 * 1024 * 1024;
const MAX_CHECKSUMS: usize = 64 * 1024;
/// 256 MiB: what the dictionary of FFmpeg's `.tar.xz` may ask for, with room.
const XZ_MEMORY_KIB: u32 = 256 * 1024;
/// What an install wrote, next to the programs.
pub const RECORDS: &str = "installed.json";
const MB: u64 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tool {
    YtDlp,
    Deno,
    Ffmpeg,
}

impl Tool {
    pub const ALL: [Tool; 3] = [Tool::YtDlp, Tool::Deno, Tool::Ffmpeg];

    fn repo(self) -> &'static str {
        match self {
            Self::YtDlp => "yt-dlp/yt-dlp",
            Self::Deno => "denoland/deno",
            Self::Ffmpeg => "BtbN/FFmpeg-Builds",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::YtDlp => "yt-dlp",
            Self::Deno => "Deno",
            Self::Ffmpeg => "FFmpeg",
        }
    }

    /// The programs an install puts into the tools folder; the first is the
    /// one Downpipe runs.
    fn programs(self) -> &'static [&'static str] {
        match self {
            Self::YtDlp => &["yt-dlp"],
            Self::Deno => &["deno"],
            Self::Ffmpeg => &["ffmpeg", "ffprobe"],
        }
    }

    fn file(self) -> String {
        exe_name(self.programs()[0])
    }
}

/// What an install put where.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Installed {
    pub tool: Tool,
    pub path: PathBuf,
    /// The release ("2026.08.19", "v2.9.7"; for FFmpeg its branch, "n9.0").
    pub release: String,
    pub sha256: String,
}

/// How far a download has come, for a progress bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Progress {
    pub tool: Tool,
    pub done: u64,
    pub total: Option<u64>,
}

/// The release asset for this system, or `None` where the project ships
/// no build. FFmpeg's name changes with each release branch, so it is
/// picked from the release's checksum file instead ([`ffmpeg_asset`]).
pub fn asset_name(tool: Tool, os: &str, arch: &str) -> Option<String> {
    match tool {
        Tool::YtDlp => Some(
            match (os, arch) {
                ("windows", "x86_64") => "yt-dlp.exe",
                ("windows", "aarch64") => "yt-dlp_arm64.exe",
                ("windows", "x86") => "yt-dlp_x86.exe",
                ("linux", "x86_64") => "yt-dlp_linux",
                ("linux", "aarch64") => "yt-dlp_linux_aarch64",
                ("macos", _) => "yt-dlp_macos",
                _ => return None,
            }
            .to_string(),
        ),
        Tool::Deno => {
            let target = match (os, arch) {
                ("windows", "x86_64") => "x86_64-pc-windows-msvc",
                ("windows", "aarch64") => "aarch64-pc-windows-msvc",
                ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
                ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
                ("macos", "x86_64") => "x86_64-apple-darwin",
                ("macos", "aarch64") => "aarch64-apple-darwin",
                _ => return None,
            };
            Some(format!("deno-{target}.zip"))
        }
        Tool::Ffmpeg => None,
    }
}

/// BtbN's name for this system.
fn ffmpeg_target(os: &str, arch: &str) -> Option<&'static str> {
    match (os, arch) {
        ("windows", "x86_64") => Some("win64"),
        ("windows", "aarch64") => Some("winarm64"),
        ("linux", "x86_64") => Some("linux64"),
        ("linux", "aarch64") => Some("linuxarm64"),
        _ => None,
    }
}

/// Whether Downpipe can download `tool` for this system at all.
pub fn available(tool: Tool, os: &str, arch: &str) -> bool {
    match tool {
        Tool::Ffmpeg => ffmpeg_target(os, arch).is_some(),
        _ => asset_name(tool, os, arch).is_some(),
    }
}

/// About how much an install downloads, for the button that offers it;
/// `None` where there is no build. Taken from the releases of October 2026.
pub fn download_size(tool: Tool, os: &str, arch: &str) -> Option<u64> {
    if !available(tool, os, arch) {
        return None;
    }
    Some(match (tool, os, arch) {
        (Tool::YtDlp, "windows", _) => 18 * MB,
        (Tool::YtDlp, _, _) => 40 * MB,
        (Tool::Deno, _, _) => 42 * MB,
        (Tool::Ffmpeg, "windows", "x86_64") => 171 * MB,
        (Tool::Ffmpeg, "linux", "x86_64") => 138 * MB,
        (Tool::Ffmpeg, _, _) => 118 * MB,
    })
}

/// The newest release branch's static LGPL build for `target` in BtbN's
/// checksum file (`ffmpeg-n9.0-latest-win64-lgpl-9.0.zip`), else the
/// master branch's (`ffmpeg-master-latest-win64-lgpl.zip`).
pub fn ffmpeg_asset(sums: &str, target: &str) -> Option<String> {
    let ext = if target.starts_with("win") {
        "zip"
    } else {
        "tar.xz"
    };
    let names = sums.lines().filter_map(|l| l.split_whitespace().nth(1));
    let mut best: Option<((u32, u32, u32), &str)> = None;
    let mut master = None;
    for name in names {
        let name = name.trim_start_matches('*');
        if name == format!("ffmpeg-master-latest-{target}-lgpl.{ext}") {
            master = Some(name);
            continue;
        }
        let Some((version, rest)) = name
            .strip_prefix("ffmpeg-n")
            .and_then(|n| n.split_once("-latest-"))
        else {
            continue;
        };
        if rest != format!("{target}-lgpl-{version}.{ext}") {
            continue;
        }
        if let Some(v) = parse_version(version)
            && best.is_none_or(|(b, _)| v > b)
        {
            best = Some((v, name));
        }
    }
    best.map(|(_, n)| n).or(master).map(str::to_string)
}

/// "n9.0" from `ffmpeg-n9.0-latest-...`, "master" from `ffmpeg-master-...`.
fn ffmpeg_release(asset: &str) -> String {
    asset
        .strip_prefix("ffmpeg-")
        .and_then(|rest| rest.split_once("-latest-"))
        .map_or("latest", |(branch, _)| branch)
        .to_string()
}

fn no_build(tool: Tool) -> DownloadError {
    DownloadError::ToolMissing(match tool {
        Tool::Ffmpeg => "There is no FFmpeg build with a published checksum for this system; install it yourself (on macOS: brew install ffmpeg) and set its path.".to_string(),
        _ => format!(
            "{} has no official build for this system; install it yourself and set its path.",
            tool.name()
        ),
    })
}

/// The newest release of `tool`, without downloading anything.
pub async fn latest(
    client: &reqwest::Client,
    base: &Url,
    tool: Tool,
) -> Result<String, DownloadError> {
    latest_release(client, base, tool.repo()).await
}

/// Download the latest official build of `tool` into `dir`, telling
/// `progress` how far it has come.
pub async fn install(
    client: &reqwest::Client,
    base: &Url,
    dir: &Path,
    tool: Tool,
    progress: &(dyn Fn(Progress) + Send + Sync),
) -> Result<Installed, DownloadError> {
    let (os, arch) = (std::env::consts::OS, std::env::consts::ARCH);
    if !available(tool, os, arch) {
        return Err(no_build(tool));
    }
    let release = latest_release(client, base, tool.repo()).await?;
    let download = |file: &str| -> Result<Url, DownloadError> {
        base.join(&format!(
            "{}/releases/download/{release}/{file}",
            tool.repo()
        ))
        .map_err(|e| DownloadError::InvalidUrl(e.to_string()))
    };

    let known = asset_name(tool, os, arch);
    let sums_name = match (tool, &known) {
        (Tool::YtDlp, _) => "SHA2-256SUMS".to_string(),
        (Tool::Deno, Some(asset)) => format!("{asset}.sha256sum"),
        _ => "checksums.sha256".to_string(),
    };
    let sums = fetch_text(client, download(&sums_name)?).await?;
    let asset = match known {
        Some(asset) => asset,
        None => ffmpeg_target(os, arch)
            .and_then(|target| ffmpeg_asset(&sums, target))
            .ok_or_else(|| {
                DownloadError::Extractor(format!(
                    "the {} release has no build for this system",
                    tool.name()
                ))
            })?,
    };
    let expected = expected_hash(&sums, &asset).ok_or_else(|| {
        DownloadError::Extractor(format!(
            "the {} release does not list a SHA-256 for {asset}",
            tool.name()
        ))
    })?;

    tokio::fs::create_dir_all(dir).await?;
    for program in tool.programs() {
        let _ = tokio::fs::remove_file(aside(&dir.join(exe_name(program)))).await;
    }
    let staged = dir.join(format!(".{asset}.download"));
    let report = |done: u64, total: Option<u64>| progress(Progress { tool, done, total });
    let actual = match fetch_file(client, download(&asset)?, &staged, &report).await {
        Ok(hash) => hash,
        Err(e) => {
            let _ = tokio::fs::remove_file(&staged).await;
            return Err(e);
        }
    };
    if actual != expected {
        let _ = tokio::fs::remove_file(&staged).await;
        return Err(DownloadError::ChecksumMismatch { expected, actual });
    }

    let result = match tool {
        Tool::YtDlp => place(&staged, &dir.join(tool.file())).await,
        Tool::Deno | Tool::Ffmpeg => unpack_and_place(tool, &staged, &asset, dir).await,
    };
    let _ = tokio::fs::remove_file(&staged).await;
    result?;

    let release = match tool {
        Tool::Ffmpeg => ffmpeg_release(&asset),
        _ => release,
    };
    let program = dir.join(tool.file());
    let sha256 = tokio::task::spawn_blocking(move || file_hash(&program))
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let mut records = Records::read(dir);
    records.set(
        tool,
        Record {
            release: release.clone(),
            sha256,
            installed_at: unix_now(),
        },
    );
    if let Err(e) = records.write(dir) {
        tracing::warn!("could not write {RECORDS}: {e}");
    }
    Ok(Installed {
        tool,
        path: dir.join(tool.file()),
        release,
        sha256: expected,
    })
}

/// Take the tool's programs out of a checked archive and move them in.
async fn unpack_and_place(
    tool: Tool,
    archive: &Path,
    asset: &str,
    dir: &Path,
) -> Result<(), DownloadError> {
    let wanted: Vec<(String, PathBuf)> = tool
        .programs()
        .iter()
        .map(|p| {
            let file = exe_name(p);
            let out = dir.join(format!(".{file}.download"));
            (file, out)
        })
        .collect();
    let (zip, list, tar) = (
        archive.to_path_buf(),
        wanted.clone(),
        !asset.ends_with(".zip"),
    );
    let extracted = tokio::task::spawn_blocking(move || {
        if tar {
            extract_tar_xz(&zip, tool, &list)
        } else {
            extract_zip(&zip, tool, &list)
        }
    })
    .await
    .map_err(|e| DownloadError::Io(std::io::Error::other(e)))?;
    let mut result = extracted;
    // The program Downpipe runs goes in last, so it never finds a helper
    // from an older build next to a new one for longer than a rename.
    for (file, staged) in wanted.iter().rev() {
        if result.is_ok() {
            result = place(staged, &dir.join(file)).await;
        }
        let _ = tokio::fs::remove_file(staged).await;
    }
    result
}

/// Whether `path` in an archive is the program `file` of `tool`: Deno's
/// zip has it at the top, FFmpeg's archives in `<folder>/bin/`.
fn is_program(tool: Tool, path: &str, file: &str) -> bool {
    let parts: Vec<&str> = path.split(['/', '\\']).collect();
    match tool {
        Tool::Ffmpeg => matches!(parts.as_slice(), [_, "bin", name] if *name == file),
        _ => matches!(parts.as_slice(), [name] if *name == file),
    }
}

fn extract_zip(zip: &Path, tool: Tool, wanted: &[(String, PathBuf)]) -> Result<(), DownloadError> {
    let invalid = |e: zip::result::ZipError| {
        DownloadError::Extractor(format!("the archive is not readable: {e}"))
    };
    let mut archive = zip::ZipArchive::new(std::fs::File::open(zip)?).map_err(invalid)?;
    let mut found = vec![false; wanted.len()];
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(invalid)?;
        if !entry.is_file() {
            continue;
        }
        let name = String::from_utf8_lossy(entry.name_raw()).into_owned();
        if let Some(at) = wanted.iter().position(|(f, _)| is_program(tool, &name, f)) {
            if entry.size() > MAX_DOWNLOAD {
                return Err(too_large());
            }
            copy_out(&mut entry, &wanted[at].1)?;
            found[at] = true;
        }
    }
    missing_program(wanted, &found)
}

fn extract_tar_xz(
    archive: &Path,
    tool: Tool,
    wanted: &[(String, PathBuf)],
) -> Result<(), DownloadError> {
    let invalid =
        |e: std::io::Error| DownloadError::Extractor(format!("the archive is not readable: {e}"));
    let xz = lzma_rust2::XzReader::new_mem_limit(
        std::io::BufReader::new(std::fs::File::open(archive)?),
        true,
        XZ_MEMORY_KIB,
    );
    let mut tar = tar::Archive::new(xz);
    let mut found = vec![false; wanted.len()];
    for entry in tar.entries().map_err(invalid)? {
        let mut entry = entry.map_err(invalid)?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let name = String::from_utf8_lossy(&entry.path_bytes()).into_owned();
        if let Some(at) = wanted.iter().position(|(f, _)| is_program(tool, &name, f)) {
            if entry.header().size().unwrap_or(0) > MAX_DOWNLOAD {
                return Err(too_large());
            }
            copy_out(&mut entry, &wanted[at].1)?;
            found[at] = true;
        }
    }
    missing_program(wanted, &found)
}

fn copy_out(entry: &mut impl Read, out: &Path) -> Result<(), DownloadError> {
    let mut file = std::fs::File::create(out)?;
    // The declared size can lie; the copy stops one byte past the limit.
    let copied = std::io::copy(&mut entry.take(MAX_DOWNLOAD + 1), &mut file)?;
    if copied > MAX_DOWNLOAD {
        drop(file);
        let _ = std::fs::remove_file(out);
        return Err(too_large());
    }
    file.sync_all()?;
    Ok(())
}

fn missing_program(wanted: &[(String, PathBuf)], found: &[bool]) -> Result<(), DownloadError> {
    match wanted.iter().zip(found).find(|(_, f)| !**f) {
        Some(((file, _), _)) => Err(DownloadError::Extractor(format!(
            "the archive has no {file}"
        ))),
        None => Ok(()),
    }
}

fn too_large() -> DownloadError {
    DownloadError::Extractor("the archive's program is larger than expected".into())
}

/// The tag `releases/latest` redirects to.
async fn latest_release(
    client: &reqwest::Client,
    base: &Url,
    repo: &str,
) -> Result<String, DownloadError> {
    let url = base
        .join(&format!("{repo}/releases/latest"))
        .map_err(|e| DownloadError::InvalidUrl(e.to_string()))?;
    let resp = client.head(url).send().await.map_err(|e| network(&e))?;
    if !resp.status().is_success() {
        return Err(status(resp.status().as_u16()));
    }
    let path = resp.url().path().to_string();
    let tag = path
        .rsplit_once("/releases/tag/")
        .map(|(_, tag)| tag.trim_end_matches('/'))
        .filter(|tag| {
            !tag.is_empty()
                && tag.len() <= 64
                && tag
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
        })
        .ok_or_else(|| {
            DownloadError::Extractor(format!("could not find the latest release of {repo}"))
        })?;
    Ok(tag.to_string())
}

async fn fetch_text(client: &reqwest::Client, url: Url) -> Result<String, DownloadError> {
    let resp = client.get(url).send().await.map_err(|e| network(&e))?;
    if !resp.status().is_success() {
        return Err(status(resp.status().as_u16()));
    }
    let bytes = resp.bytes().await.map_err(|e| network(&e))?;
    if bytes.len() > MAX_CHECKSUMS {
        return Err(DownloadError::Extractor(
            "the checksum file is too large".into(),
        ));
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Stream `url` into `path`, returning the SHA-256 of what was written.
async fn fetch_file(
    client: &reqwest::Client,
    url: Url,
    path: &Path,
    progress: &(dyn Fn(u64, Option<u64>) + Send + Sync),
) -> Result<String, DownloadError> {
    const REPORT_EVERY: u64 = 1024 * 1024;
    let resp = client.get(url).send().await.map_err(|e| network(&e))?;
    if !resp.status().is_success() {
        return Err(status(resp.status().as_u16()));
    }
    let total = resp.content_length();
    if total.is_some_and(|n| n > MAX_DOWNLOAD) {
        return Err(DownloadError::Extractor(
            "the download is larger than expected".into(),
        ));
    }
    progress(0, total);
    let mut file = std::fs::File::create(path)?;
    let mut hasher = Sha256::new();
    let mut written: u64 = 0;
    let mut reported: u64 = 0;
    let mut body = resp.bytes_stream();
    while let Some(chunk) = body.next().await {
        let chunk = chunk.map_err(|e| network(&e))?;
        written += chunk.len() as u64;
        if written > MAX_DOWNLOAD {
            return Err(DownloadError::Extractor(
                "the download is larger than expected".into(),
            ));
        }
        hasher.update(&chunk);
        file.write_all(&chunk)?;
        if written - reported >= REPORT_EVERY {
            reported = written;
            progress(written, total);
        }
    }
    file.sync_all()?;
    progress(written, total);
    Ok(hex::encode(hasher.finalize()))
}

/// The hash listed for `file`: a `<hash>  <name>` line (sha256sum), or the
/// single hash of a one-file checksum (Deno's, which on Windows is
/// PowerShell's `Get-FileHash` output). Lower case.
pub fn expected_hash(sums: &str, file: &str) -> Option<String> {
    let is_hash = |t: &str| t.len() == 64 && t.bytes().all(|b| b.is_ascii_hexdigit());
    for line in sums.lines() {
        let mut tokens = line.split_whitespace();
        if let (Some(hash), Some(name)) = (tokens.next(), tokens.next())
            && is_hash(hash)
            && name.trim_start_matches('*') == file
        {
            return Some(hash.to_ascii_lowercase());
        }
    }
    // A one-file checksum: a single hash, with the file named next to it.
    let hashes: Vec<&str> = sums
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| is_hash(t))
        .collect();
    match hashes.as_slice() {
        [only] if sums.contains(file) => Some(only.to_ascii_lowercase()),
        _ => None,
    }
}

/// Where a running program is moved to so a new one can take its name.
fn aside(path: &Path) -> PathBuf {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    path.with_file_name(format!(".{name}.old"))
}

/// Move a checked file into place, executable.
async fn place(from: &Path, to: &Path) -> Result<(), DownloadError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(from, std::fs::Permissions::from_mode(0o755)).await?;
    }
    let in_use = || {
        DownloadError::Extractor(format!(
            "{} is in use; pause downloads from sites and try again",
            to.display()
        ))
    };
    match tokio::fs::rename(from, to).await {
        Ok(()) => Ok(()),
        // Windows will not replace a program that is running, but lets it
        // be renamed: the running copy goes on under the other name, which
        // the next install removes.
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            let old = aside(to);
            let _ = tokio::fs::remove_file(&old).await;
            if tokio::fs::rename(to, &old).await.is_err() {
                return Err(in_use());
            }
            match tokio::fs::rename(from, to).await {
                Ok(()) => Ok(()),
                Err(_) => {
                    let _ = tokio::fs::rename(&old, to).await;
                    Err(in_use())
                }
            }
        }
        Err(e) => Err(DownloadError::Io(e)),
    }
}

/// One tool as an install left it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Record {
    pub release: String,
    /// Of the program Downpipe runs, as it was put in place.
    pub sha256: String,
    /// Seconds since the epoch.
    pub installed_at: u64,
}

/// What Downpipe installed into its tools folder, and when it last looked
/// for a newer yt-dlp.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Records {
    pub yt_dlp: Option<Record>,
    pub deno: Option<Record>,
    pub ffmpeg: Option<Record>,
    /// Seconds since the epoch.
    pub yt_dlp_checked: Option<u64>,
}

impl Records {
    /// The folder's records; none when the file is missing or unreadable.
    pub fn read(dir: &Path) -> Self {
        std::fs::read(dir.join(RECORDS))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn write(&self, dir: &Path) -> std::io::Result<()> {
        let tmp = dir.join(format!(".{RECORDS}.tmp"));
        std::fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(&tmp, dir.join(RECORDS))
    }

    pub fn get(&self, tool: Tool) -> Option<&Record> {
        match tool {
            Tool::YtDlp => self.yt_dlp.as_ref(),
            Tool::Deno => self.deno.as_ref(),
            Tool::Ffmpeg => self.ffmpeg.as_ref(),
        }
    }

    fn set(&mut self, tool: Tool, record: Record) {
        let slot = match tool {
            Tool::YtDlp => &mut self.yt_dlp,
            Tool::Deno => &mut self.deno,
            Tool::Ffmpeg => &mut self.ffmpeg,
        };
        *slot = Some(record);
    }
}

/// Downpipe's own copy of `tool` in `dir`: its record, when the program
/// there is still the one Downpipe installed (same SHA-256). A copy the user
/// changed is not Downpipe's to replace. Reads the whole file.
pub fn own_copy(dir: &Path, tool: Tool) -> Option<Record> {
    let hash = file_hash(&dir.join(tool.file()))?;
    match Records::read(dir).get(tool) {
        Some(record) => (hash == record.sha256).then(|| record.clone()),
        // Installed before Downpipe kept records (0.5 and older), when only
        // its button put programs into this folder: Downpipe's, release
        // unknown.
        None => Some(Record {
            sha256: hash,
            ..Record::default()
        }),
    }
}

fn file_hash(path: &Path) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0; 64 * 1024];
    loop {
        match file.read(&mut buf).ok()? {
            0 => break,
            n => hasher.update(&buf[..n]),
        }
    }
    Some(hex::encode(hasher.finalize()))
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn network(e: &reqwest::Error) -> DownloadError {
    let class = if e.is_timeout() {
        "timed out"
    } else if e.is_connect() {
        "could not connect"
    } else {
        "request failed"
    };
    DownloadError::Network(format!("GitHub: {class}"))
}

fn status(code: u16) -> DownloadError {
    match code {
        404 | 410 => DownloadError::NotFound { status: code },
        403 | 429 => DownloadError::Throttled {
            status: code,
            retry_after: None,
        },
        _ => DownloadError::HttpStatus { status: code },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assets_per_system() {
        assert_eq!(
            asset_name(Tool::YtDlp, "windows", "x86_64").as_deref(),
            Some("yt-dlp.exe")
        );
        assert_eq!(
            asset_name(Tool::YtDlp, "linux", "aarch64").as_deref(),
            Some("yt-dlp_linux_aarch64")
        );
        assert_eq!(
            asset_name(Tool::YtDlp, "macos", "aarch64").as_deref(),
            Some("yt-dlp_macos")
        );
        assert_eq!(
            asset_name(Tool::Deno, "windows", "x86_64").as_deref(),
            Some("deno-x86_64-pc-windows-msvc.zip")
        );
        assert_eq!(asset_name(Tool::Deno, "freebsd", "x86_64"), None);
        assert!(available(Tool::Ffmpeg, "linux", "aarch64"));
        assert!(!available(Tool::Ffmpeg, "macos", "aarch64"));
        assert!(!available(Tool::Ffmpeg, "windows", "x86"));
        assert_eq!(download_size(Tool::Ffmpeg, "macos", "x86_64"), None);
        assert_eq!(
            download_size(Tool::Ffmpeg, "windows", "x86_64"),
            Some(171 * MB)
        );
    }

    #[test]
    fn ffmpeg_takes_the_newest_lgpl_release_branch() {
        let sums = "\
            0f8a  ffmpeg-master-latest-win64-gpl-shared.zip\n\
            8366  ffmpeg-master-latest-win64-lgpl.zip\n\
            abff  ffmpeg-n8.1-latest-win64-lgpl-8.1.zip\n\
            8372  ffmpeg-n8.1-latest-win64-lgpl-shared-8.1.zip\n\
            8a7c  ffmpeg-n9.0-latest-win64-gpl-9.0.zip\n\
            0a38  ffmpeg-n9.0-latest-win64-lgpl-9.0.zip\n\
            fea5  ffmpeg-n9.0-latest-winarm64-lgpl-9.0.zip\n\
            ebd9  ffmpeg-master-latest-linux64-lgpl.tar.xz\n\
            3455  ffmpeg-n8.1-latest-linux64-lgpl-8.1.tar.xz\n\
            d6da  ffmpeg-n10.0-latest-linux64-lgpl-10.0.tar.xz\n";
        assert_eq!(
            ffmpeg_asset(sums, "win64").as_deref(),
            Some("ffmpeg-n9.0-latest-win64-lgpl-9.0.zip")
        );
        assert_eq!(
            ffmpeg_asset(sums, "linux64").as_deref(),
            Some("ffmpeg-n10.0-latest-linux64-lgpl-10.0.tar.xz")
        );
        assert_eq!(
            ffmpeg_asset(
                "1  ffmpeg-master-latest-linuxarm64-lgpl.tar.xz\n",
                "linuxarm64"
            )
            .as_deref(),
            Some("ffmpeg-master-latest-linuxarm64-lgpl.tar.xz")
        );
        assert_eq!(ffmpeg_asset(sums, "linuxarm64"), None);
        assert_eq!(
            ffmpeg_release("ffmpeg-n9.0-latest-win64-lgpl-9.0.zip"),
            "n9.0"
        );
        assert_eq!(
            ffmpeg_release("ffmpeg-master-latest-win64-lgpl.zip"),
            "master"
        );
    }

    #[test]
    fn programs_in_archives() {
        let ffmpeg = "ffmpeg-n9.0-latest-win64-lgpl-9.0/bin/ffmpeg.exe";
        assert!(is_program(Tool::Ffmpeg, ffmpeg, "ffmpeg.exe"));
        assert!(!is_program(Tool::Ffmpeg, ffmpeg, "ffprobe.exe"));
        assert!(!is_program(
            Tool::Ffmpeg,
            "x/doc/bin/ffmpeg.exe",
            "ffmpeg.exe"
        ));
        assert!(!is_program(Tool::Ffmpeg, "ffmpeg.exe", "ffmpeg.exe"));
        assert!(is_program(Tool::Deno, "deno.exe", "deno.exe"));
        assert!(!is_program(Tool::Deno, "x/deno.exe", "deno.exe"));
    }

    #[test]
    fn takes_ffmpeg_and_ffprobe_out_of_a_tar_xz() {
        let mut tar = tar::Builder::new(Vec::new());
        let entries: [(&str, &[u8]); 4] = [
            ("ffmpeg-n9.0-latest-linux64-lgpl-9.0/LICENSE.txt", b"LGPL"),
            (
                "ffmpeg-n9.0-latest-linux64-lgpl-9.0/bin/ffmpeg",
                b"ffmpeg build",
            ),
            (
                "ffmpeg-n9.0-latest-linux64-lgpl-9.0/bin/ffprobe",
                b"ffprobe build",
            ),
            ("ffmpeg-n9.0-latest-linux64-lgpl-9.0/bin/ffplay", b"player"),
        ];
        for (name, bytes) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            tar.append_data(&mut header, name, bytes).unwrap();
        }
        let mut xz =
            lzma_rust2::XzWriter::new(Vec::new(), lzma_rust2::XzOptions::default()).unwrap();
        xz.write_all(&tar.into_inner().unwrap()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("ffmpeg.tar.xz");
        std::fs::write(&archive, xz.finish().unwrap()).unwrap();

        let out = |n: &str| dir.path().join(n);
        let wanted = [
            ("ffmpeg".to_string(), out("a")),
            ("ffprobe".to_string(), out("b")),
        ];
        extract_tar_xz(&archive, Tool::Ffmpeg, &wanted).unwrap();
        assert_eq!(std::fs::read(out("a")).unwrap(), b"ffmpeg build");
        assert_eq!(std::fs::read(out("b")).unwrap(), b"ffprobe build");

        // An archive without one of them is refused.
        let wanted = [("ffserver".to_string(), out("c"))];
        let err = extract_tar_xz(&archive, Tool::Ffmpeg, &wanted).unwrap_err();
        assert!(err.to_string().contains("ffserver"), "{err}");
    }

    #[test]
    fn records_survive_a_round_trip_and_tell_own_copies() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(Records::read(dir.path()), Records::default());
        let exe = dir.path().join(Tool::YtDlp.file());
        std::fs::write(&exe, b"build").unwrap();
        let mut records = Records::default();
        records.set(
            Tool::YtDlp,
            Record {
                release: "2026.08.19".into(),
                sha256: file_hash(&exe).unwrap(),
                installed_at: 1,
            },
        );
        records.yt_dlp_checked = Some(5);
        records.write(dir.path()).unwrap();
        assert_eq!(Records::read(dir.path()), records);
        assert_eq!(
            own_copy(dir.path(), Tool::YtDlp).map(|r| r.release),
            Some("2026.08.19".into())
        );
        // Replaced by the user: no longer Downpipe's.
        std::fs::write(&exe, b"the user's own").unwrap();
        assert_eq!(own_copy(dir.path(), Tool::YtDlp), None);
        assert_eq!(own_copy(dir.path(), Tool::Deno), None);
        // From before the records: Downpipe's, release unknown.
        std::fs::write(dir.path().join(Tool::Deno.file()), b"deno").unwrap();
        assert_eq!(
            own_copy(dir.path(), Tool::Deno).map(|r| r.release),
            Some(String::new())
        );
        std::fs::write(dir.path().join(RECORDS), b"{not json").unwrap();
        assert_eq!(Records::read(dir.path()), Records::default());
    }

    #[test]
    fn checksum_files() {
        let sums = "1fa6733c37ea6fb51c99ad8fe785e7b7e5f3246c9b980230329d4fb72ed8d4d6  yt-dlp\n\
                    66674953FE251B89F4D08C5F0E35E0728679BD67AB3D7D05C0562AF101DD3E7A  yt-dlp.exe\n";
        assert_eq!(
            expected_hash(sums, "yt-dlp.exe").as_deref(),
            Some("66674953fe251b89f4d08c5f0e35e0728679bd67ab3d7d05c0562af101dd3e7a")
        );
        assert_eq!(expected_hash(sums, "yt-dlp_x86.exe"), None);
        let powershell = "\r\nAlgorithm : SHA256\r\nHash      : A0C3101B4158D1DFB7D6A78A7BF0F3DE80C96BB423C152BEEC8BEB22786F2238\r\nPath      : C:\\a\\deno\\deno\\target\\release\\deno-x86_64-pc-windows-msvc.zip\r\n";
        assert_eq!(
            expected_hash(powershell, "deno-x86_64-pc-windows-msvc.zip").as_deref(),
            Some("a0c3101b4158d1dfb7d6a78a7bf0f3de80c96bb423c152beec8beb22786f2238")
        );
        let sha256sum = "bf6ee050dc421189520a9ef6dbf9f868cbbdb1e5eda1a28c37dad34e7962a520  deno-x86_64-unknown-linux-gnu.zip\n";
        assert_eq!(
            expected_hash(sha256sum, "deno-x86_64-unknown-linux-gnu.zip").as_deref(),
            Some("bf6ee050dc421189520a9ef6dbf9f868cbbdb1e5eda1a28c37dad34e7962a520")
        );
        assert_eq!(expected_hash("not a hash", "x"), None);
    }
}
