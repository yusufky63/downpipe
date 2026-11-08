//! Downpipe's own updates: whether a newer release is out, and on Windows
//! its installer, checked before anything runs.
//!
//! The latest release is found the way the tools are (`releases/latest` on
//! github.com, which redirects to its tag), so the check needs no GitHub API
//! and sends nothing but the request itself. The installer is checked
//! against the SHA-256 the same release lists in `SHA256SUMS.txt`; that
//! catches a damaged or altered download, not a compromised release, the
//! same promise as for yt-dlp and Deno.

use std::path::{Path, PathBuf};

use downpipe_http::DownloadError;
use serde::Serialize;
use url::Url;

use crate::install::{expected_hash, fetch_file, fetch_text, latest_release};

/// Where Downpipe's releases are.
pub const REPO: &str = "yusufky63/downpipe";

/// A release newer than the running one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Update {
    /// `0.5.6`
    pub version: String,
    /// The release's page, for its notes and the other packages.
    pub page: String,
    /// The installer this system can run from the app (Windows), if any;
    /// elsewhere the page is where to get the package.
    pub installer: Option<String>,
}

/// The newest release, if it is newer than `current`.
pub async fn check(
    client: &reqwest::Client,
    base: &Url,
    current: &str,
) -> Result<Option<Update>, DownloadError> {
    let tag = latest_release(client, base, REPO).await?;
    let version = tag.trim_start_matches('v').to_string();
    if !is_newer(&version, current) {
        return Ok(None);
    }
    let page = base
        .join(&format!("{REPO}/releases/tag/{tag}"))
        .map_err(|e| DownloadError::InvalidUrl(e.to_string()))?;
    Ok(Some(Update {
        installer: installer_name(&version, std::env::consts::OS, std::env::consts::ARCH),
        version,
        page: page.to_string(),
    }))
}

/// Whether `candidate` (`0.5.6`) is a later version than `current`. Parts
/// are compared as numbers; a part that is not a number (`0.6.0-beta`) makes
/// the candidate count as not newer, so a test build is never offered.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    let parse = |v: &str| -> Option<Vec<u64>> {
        v.trim_start_matches('v')
            .split('.')
            .map(|p| p.parse().ok())
            .collect()
    };
    match (parse(candidate), parse(current)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

/// The installer a release ships for this system, as the release names it.
pub fn installer_name(version: &str, os: &str, arch: &str) -> Option<String> {
    (os == "windows" && arch == "x86_64").then(|| format!("Downpipe_{version}_x64-setup.exe"))
}

/// Download the update's installer into `dir`, checked against the
/// release's `SHA256SUMS.txt`. A failed check leaves nothing behind.
pub async fn download_installer(
    client: &reqwest::Client,
    base: &Url,
    update: &Update,
    dir: &Path,
) -> Result<PathBuf, DownloadError> {
    let name = update.installer.as_deref().ok_or_else(|| {
        DownloadError::Unavailable(
            "there is no installer for this system; get the package from the release page".into(),
        )
    })?;
    let file = |f: &str| {
        base.join(&format!("{REPO}/releases/download/v{}/{f}", update.version))
            .map_err(|e| DownloadError::InvalidUrl(e.to_string()))
    };
    let sums = fetch_text(client, file("SHA256SUMS.txt")?).await?;
    let expected = expected_hash(&sums, name).ok_or_else(|| {
        DownloadError::Extractor(format!("the release does not list a SHA-256 for {name}"))
    })?;
    tokio::fs::create_dir_all(dir).await?;
    let staged = dir.join(format!(".{name}.download"));
    let actual = match fetch_file(client, file(name)?, &staged).await {
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
    let target = dir.join(name);
    tokio::fs::rename(&staged, &target).await?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_later_plain_version_is_newer() {
        assert!(is_newer("0.5.6", "0.5.5"));
        assert!(is_newer("0.10.0", "0.9.9"), "numbers, not text");
        assert!(is_newer("v1.0.0", "0.5.5"));
        assert!(!is_newer("0.5.5", "0.5.5"));
        assert!(!is_newer("0.5.4", "0.5.5"));
        assert!(
            !is_newer("0.6.0-beta", "0.5.5"),
            "a test build is not offered"
        );
    }

    #[test]
    fn the_installer_is_the_windows_setup() {
        assert_eq!(
            installer_name("0.5.6", "windows", "x86_64").as_deref(),
            Some("Downpipe_0.5.6_x64-setup.exe")
        );
        assert_eq!(installer_name("0.5.6", "linux", "x86_64"), None);
        assert_eq!(installer_name("0.5.6", "macos", "aarch64"), None);
    }
}
