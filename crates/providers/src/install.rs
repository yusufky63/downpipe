//! Downloading yt-dlp and Deno into Downpipe's tools folder, only when the
//! user presses the button for it in the settings.
//!
//! Both come from their projects' latest GitHub release: the release is
//! found by following `releases/latest`, the file is the official build for
//! this system, and it is checked against the SHA-256 the project publishes
//! with the release (`SHA2-256SUMS` for yt-dlp, `<file>.sha256sum` for Deno)
//! before it replaces anything. A failed check leaves the old copy in place.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use downpipe_http::DownloadError;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;

use crate::tools::exe_name;

/// Where releases are looked up; tests point this at a local server.
pub const GITHUB: &str = "https://github.com/";
/// No official build of either is near this; anything larger is refused.
const MAX_DOWNLOAD: u64 = 400 * 1024 * 1024;
const MAX_CHECKSUMS: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tool {
    YtDlp,
    Deno,
}

impl Tool {
    fn repo(self) -> &'static str {
        match self {
            Self::YtDlp => "yt-dlp/yt-dlp",
            Self::Deno => "denoland/deno",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::YtDlp => "yt-dlp",
            Self::Deno => "Deno",
        }
    }

    fn file(self) -> String {
        exe_name(match self {
            Self::YtDlp => "yt-dlp",
            Self::Deno => "deno",
        })
    }
}

/// What an install put where.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Installed {
    pub tool: Tool,
    pub path: PathBuf,
    /// The release tag ("2026.08.19", "v2.9.7").
    pub release: String,
    pub sha256: String,
}

/// The release asset for this system, or `None` where the project ships
/// no build.
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
    }
}

/// Download the latest official build of `tool` into `dir`.
pub async fn install(
    client: &reqwest::Client,
    base: &Url,
    dir: &Path,
    tool: Tool,
) -> Result<Installed, DownloadError> {
    let asset =
        asset_name(tool, std::env::consts::OS, std::env::consts::ARCH).ok_or_else(|| {
            DownloadError::ToolMissing(format!(
                "{} has no official build for this system; install it yourself and set its path.",
                tool.name()
            ))
        })?;
    let release = latest_release(client, base, tool.repo()).await?;
    let download = |file: &str| -> Result<Url, DownloadError> {
        base.join(&format!(
            "{}/releases/download/{release}/{file}",
            tool.repo()
        ))
        .map_err(|e| DownloadError::InvalidUrl(e.to_string()))
    };

    let sums_name = match tool {
        Tool::YtDlp => "SHA2-256SUMS".to_string(),
        Tool::Deno => format!("{asset}.sha256sum"),
    };
    let sums = fetch_text(client, download(&sums_name)?).await?;
    let expected = expected_hash(&sums, &asset).ok_or_else(|| {
        DownloadError::Extractor(format!(
            "the {} release does not list a SHA-256 for {asset}",
            tool.name()
        ))
    })?;

    tokio::fs::create_dir_all(dir).await?;
    let staged = dir.join(format!(".{asset}.download"));
    let actual = fetch_file(client, download(&asset)?, &staged).await;
    let actual = match actual {
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

    let target = dir.join(tool.file());
    let result = match tool {
        Tool::YtDlp => place(&staged, &target).await,
        Tool::Deno => {
            let unpacked = dir.join(format!(".{}.download", tool.file()));
            let (zip, out) = (staged.clone(), unpacked.clone());
            let name = tool.file();
            let extracted = tokio::task::spawn_blocking(move || extract(&zip, &name, &out))
                .await
                .map_err(|e| DownloadError::Io(std::io::Error::other(e)))?;
            let _ = tokio::fs::remove_file(&staged).await;
            match extracted {
                Ok(()) => place(&unpacked, &target).await,
                Err(e) => {
                    let _ = tokio::fs::remove_file(&unpacked).await;
                    Err(e)
                }
            }
        }
    };
    let _ = tokio::fs::remove_file(&staged).await;
    result?;
    Ok(Installed {
        tool,
        path: target,
        release,
        sha256: expected,
    })
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
) -> Result<String, DownloadError> {
    let resp = client.get(url).send().await.map_err(|e| network(&e))?;
    if !resp.status().is_success() {
        return Err(status(resp.status().as_u16()));
    }
    if resp.content_length().is_some_and(|n| n > MAX_DOWNLOAD) {
        return Err(DownloadError::Extractor(
            "the download is larger than expected".into(),
        ));
    }
    let mut file = std::fs::File::create(path)?;
    let mut hasher = Sha256::new();
    let mut written: u64 = 0;
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
    }
    file.sync_all()?;
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

/// Copy `name` (at the archive's top level) out of a zip file.
fn extract(zip: &Path, name: &str, out: &Path) -> Result<(), DownloadError> {
    let invalid = |e: zip::result::ZipError| {
        DownloadError::Extractor(format!("the archive is not readable: {e}"))
    };
    let mut archive = zip::ZipArchive::new(std::fs::File::open(zip)?).map_err(invalid)?;
    let mut entry = archive.by_name(name).map_err(invalid)?;
    if entry.size() > MAX_DOWNLOAD {
        return Err(DownloadError::Extractor(
            "the archive's program is larger than expected".into(),
        ));
    }
    let mut file = std::fs::File::create(out)?;
    std::io::copy(&mut entry, &mut file)?;
    file.sync_all()?;
    Ok(())
}

/// Move a checked file into place, executable.
async fn place(from: &Path, to: &Path) -> Result<(), DownloadError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(from, std::fs::Permissions::from_mode(0o755)).await?;
    }
    tokio::fs::rename(from, to).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            DownloadError::Extractor(format!(
                "{} is in use; pause downloads from sites and try again",
                to.display()
            ))
        } else {
            DownloadError::Io(e)
        }
    })
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
