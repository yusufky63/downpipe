//! Resume state stored next to the partial file (`name.part.meta`).
//!
//! The meta file — not a database — is the source of truth for which bytes are
//! on disk, because it is written right after the data it describes is synced.
//! It records the validator (ETag / Last-Modified) and size the bytes belong
//! to, so a resume never stitches together two different versions of a file.

use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::DownloadError;
use crate::segments::ByteRange;

pub const META_VERSION: u32 = 1;
const PART_SUFFIX: &str = ".part";
const META_SUFFIX: &str = ".part.meta";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeMeta {
    pub version: u32,
    /// Request URL with credentials stripped (see [`crate::redact_url`]).
    pub url: String,
    /// URL after redirects, also redacted. Informational only: resumes go
    /// through `url` again because redirect targets are often short-lived.
    pub final_url: String,
    pub file_name: String,
    pub total_size: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    /// Half-open byte ranges `[start, end)` that are durably on disk.
    pub completed: Vec<ByteRange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl ResumeMeta {
    pub fn completed_bytes(&self) -> u64 {
        self.completed.iter().map(ByteRange::len).sum()
    }

    /// Validator for `If-Range`. Only strong validators are allowed there
    /// (RFC 9110 §13.1.5), so weak ETags fall back to Last-Modified.
    pub fn if_range_validator(&self) -> Option<&str> {
        match &self.etag {
            Some(etag) if !etag.starts_with("W/") => Some(etag),
            _ => self.last_modified.as_deref(),
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = unix_now();
    }
}

pub fn part_path(final_path: &Path) -> PathBuf {
    with_suffix(final_path, PART_SUFFIX)
}

pub fn meta_path(final_path: &Path) -> PathBuf {
    with_suffix(final_path, META_SUFFIX)
}

/// Inverse of [`meta_path`] / [`part_path`]: the final path a `.part` or
/// `.part.meta` file belongs to.
pub fn final_path_for(path: &Path) -> Option<PathBuf> {
    let name = path.file_name()?.to_str()?;
    let stem = name
        .strip_suffix(META_SUFFIX)
        .or_else(|| name.strip_suffix(PART_SUFFIX))?;
    Some(path.with_file_name(stem))
}

pub fn load(path: &Path) -> Result<ResumeMeta, DownloadError> {
    let raw = fs::read(path)?;
    let meta: ResumeMeta =
        serde_json::from_slice(&raw).map_err(|e| DownloadError::CorruptMeta(e.to_string()))?;
    if meta.version != META_VERSION {
        return Err(DownloadError::CorruptMeta(format!(
            "unsupported meta version {}",
            meta.version
        )));
    }
    if meta
        .completed
        .iter()
        .any(|r| r.start > r.end || r.end > meta.total_size)
    {
        return Err(DownloadError::CorruptMeta("range outside file".into()));
    }
    Ok(meta)
}

/// Write-then-rename so a crash mid-write leaves the previous meta intact.
pub fn save(path: &Path, meta: &ResumeMeta) -> Result<(), DownloadError> {
    let tmp = with_suffix(path, ".tmp");
    let json = serde_json::to_vec_pretty(meta).map_err(std::io::Error::other)?;
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(&json)?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn remove(final_path: &Path) {
    let _ = fs::remove_file(meta_path(final_path));
}

/// Look for an unfinished download of `url` in `dir`.
pub fn find_for_url(dir: &Path, url: &str) -> Option<(PathBuf, ResumeMeta)> {
    let entries = fs::read_dir(dir).ok()?;
    entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(META_SUFFIX))
        })
        .filter_map(|p| load(&p).ok().map(|m| (p, m)))
        .filter(|(_, m)| m.url == url)
        .max_by_key(|(_, m)| m.updated_at)
        .and_then(|(p, m)| final_path_for(&p).map(|f| (f, m)))
}

pub(crate) fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name: OsString = path.file_name().map(OsString::from).unwrap_or_default();
    name.push(suffix);
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(url: &str) -> ResumeMeta {
        ResumeMeta {
            version: META_VERSION,
            url: url.into(),
            final_url: url.into(),
            file_name: "file.iso".into(),
            total_size: 100,
            etag: Some("\"abc\"".into()),
            last_modified: None,
            content_type: None,
            completed: vec![ByteRange::new(0, 40), ByteRange::new(60, 70)],
            checksum: None,
            created_at: 1,
            updated_at: 2,
        }
    }

    #[test]
    fn paths_round_trip() {
        let f = Path::new("/tmp/a b.iso");
        assert_eq!(part_path(f), Path::new("/tmp/a b.iso.part"));
        assert_eq!(meta_path(f), Path::new("/tmp/a b.iso.part.meta"));
        assert_eq!(final_path_for(&meta_path(f)).as_deref(), Some(f));
        assert_eq!(final_path_for(&part_path(f)).as_deref(), Some(f));
        assert_eq!(final_path_for(Path::new("/tmp/a.iso")), None);
    }

    #[test]
    fn save_load_and_find() {
        let dir = tempfile::tempdir().unwrap();
        let final_path = dir.path().join("file.iso");
        let meta = sample("https://example.org/file.iso");
        save(&meta_path(&final_path), &meta).unwrap();
        assert_eq!(load(&meta_path(&final_path)).unwrap(), meta);
        assert_eq!(meta.completed_bytes(), 50);

        let (found, found_meta) = find_for_url(dir.path(), "https://example.org/file.iso").unwrap();
        assert_eq!(found, final_path);
        assert_eq!(found_meta, meta);
        assert!(find_for_url(dir.path(), "https://example.org/other").is_none());
    }

    #[test]
    fn rejects_ranges_outside_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.part.meta");
        let mut meta = sample("u");
        meta.completed.push(ByteRange::new(90, 200));
        save(&path, &meta).unwrap();
        assert!(matches!(load(&path), Err(DownloadError::CorruptMeta(_))));
    }

    #[test]
    fn weak_etag_is_not_used_for_if_range() {
        let mut meta = sample("u");
        assert_eq!(meta.if_range_validator(), Some("\"abc\""));
        meta.etag = Some("W/\"abc\"".into());
        assert_eq!(meta.if_range_validator(), None);
        meta.last_modified = Some("Fri, 16 Oct 2026 12:00:00 GMT".into());
        assert_eq!(
            meta.if_range_validator(),
            Some("Fri, 16 Oct 2026 12:00:00 GMT")
        );
    }
}
