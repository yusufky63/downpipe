//! Backup and restore: the settings (queues and categories included) and the
//! download list in one JSON file, and the rules for reading one back.
//!
//! A backup is read as untrusted input: it may come from another machine,
//! another version or another person. Every download in it is checked on
//! its own and dropped when it does not hold up, paths of another system
//! fall back to this one's folders, and the programs Downpipe runs (FFmpeg,
//! yt-dlp, Deno) are never taken from a file. Nothing in a backup carries
//! headers or cookies: the catalog never stores them, so there are none to
//! export.

use std::path::{Path, PathBuf};

use downpipe_http::{Checksum, filename, media::MediaKind};
use downpipe_providers::pages;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::settings::{Category, MAIN_QUEUE, Queue, Settings};
use crate::task::{MediaInfo, SiteInfo, TaskRecord, TaskState};

/// The `format` field, so a file picked by mistake is told apart from an
/// unreadable backup.
pub const FORMAT: &str = "downpipe-backup";
/// Bumped when a backup stops being readable by older versions. Fields may
/// be added without it; readers ignore what they do not know.
pub const VERSION: u32 = 1;
/// A list of 100,000 downloads is about 60 MB of JSON.
pub const MAX_BACKUP_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_DOWNLOADS: usize = 100_000;
/// Signed CDN links run to a few kilobytes, as in the browser bridge.
const MAX_URL_LEN: usize = 8 * 1024;
const MAX_PATH_LEN: usize = 4096;
const MAX_NAME_LEN: usize = 255;
const MAX_LABEL_LEN: usize = 128;

/// Why a file cannot be read as a backup at all.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum BackupError {
    #[error("the file is larger than {} MB", MAX_BACKUP_BYTES / (1024 * 1024))]
    TooLarge,
    #[error("the file is not a Downpipe backup")]
    NotABackup,
    #[error("the backup was made by a newer Downpipe (format version {0})")]
    NewerVersion(u32),
    #[error("the backup is damaged: {0}")]
    Damaged(String),
    #[error("the backup lists more than {MAX_DOWNLOADS} downloads")]
    TooManyDownloads,
}

impl BackupError {
    /// Stable name UIs branch on.
    pub fn code(&self) -> &'static str {
        match self {
            Self::TooLarge => "backup_too_large",
            Self::NotABackup => "not_a_backup",
            Self::NewerVersion(_) => "backup_too_new",
            Self::Damaged(_) => "backup_damaged",
            Self::TooManyDownloads => "backup_too_large",
        }
    }
}

/// One backup file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Backup {
    pub format: String,
    pub version: u32,
    #[serde(default)]
    pub app_version: String,
    /// Seconds since the Unix epoch.
    #[serde(default)]
    pub exported_at: i64,
    pub settings: Settings,
    pub downloads: Vec<BackupDownload>,
    /// Downloads in the file that did not pass the checks. Not written.
    #[serde(skip)]
    pub invalid: usize,
}

/// One download as a backup keeps it: where it came from, where it goes,
/// and how far it got. No id (the importing catalog makes its own), no
/// error text and nothing kept in memory only (headers, credentials).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupDownload {
    pub url: String,
    pub dir: PathBuf,
    /// The final name for a finished download, else the chosen one, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    pub state: TaskState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    #[serde(default)]
    pub downloaded: u64,
    #[serde(default = "main_queue")]
    pub queue: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Downpipe picked the folder (see `TaskRecord::auto_dir`).
    #[serde(default)]
    pub auto_dir: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<MediaInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub site: Option<SiteInfo>,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<i64>,
}

fn main_queue() -> String {
    MAIN_QUEUE.to_string()
}

impl BackupDownload {
    pub fn from_record(record: &TaskRecord) -> Self {
        let finished_name = record
            .path
            .as_deref()
            .filter(|_| record.state == TaskState::Completed)
            .and_then(Path::file_name)
            .map(|n| n.to_string_lossy().into_owned());
        Self {
            url: record.url.clone(),
            dir: record.dir.clone(),
            file_name: finished_name.or_else(|| record.file_name.clone()),
            state: record.state,
            total: record.total,
            downloaded: record.downloaded,
            queue: record.queue.clone(),
            category: record.category.clone(),
            auto_dir: record.auto_dir,
            checksum: record.checksum.clone(),
            media: record.media.clone(),
            site: record.site.clone(),
            created_at: record.created_at,
            completed_at: record.completed_at,
        }
    }

    /// Check one download from a file and bring it into the shape the
    /// catalog keeps. `Err` names the first problem.
    pub fn validated(self) -> Result<Self, String> {
        let url = url::Url::parse(self.url.trim()).map_err(|e| format!("url: {e}"))?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(format!("url: `{}` links are not supported", url.scheme()));
        }
        // Video pages are kept under their canonical link, as `add` does.
        let page = pages::recognize(&url);
        let url = page.as_ref().map_or(url, |p| p.url.clone());
        let key = downpipe_http::redact_url(&url);
        if key.len() > MAX_URL_LEN {
            return Err("url: too long".into());
        }
        let file_name = match self.file_name.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(name) if name.chars().count() > MAX_NAME_LEN => {
                return Err("file_name: too long".into());
            }
            Some(name) => Some(filename::sanitize(name)),
        };
        // A path from another system (or none) is replaced by this one's
        // folder when the download is added; `auto_dir` says so.
        let dir_ok = self.dir.is_absolute()
            && self.dir.as_os_str().len() <= MAX_PATH_LEN
            && !self.dir.to_string_lossy().chars().any(char::is_control);
        let state = match self.state {
            TaskState::Completed | TaskState::Canceled => self.state,
            // Nothing starts by itself after an import.
            _ => TaskState::Paused,
        };
        let total = self.total;
        let downloaded = match total {
            Some(total) => self.downloaded.min(total),
            None => self.downloaded,
        };
        let checksum = self
            .checksum
            .as_deref()
            .and_then(|c| c.parse::<Checksum>().ok())
            .map(|c| c.to_string());
        let queue = Some(self.queue.trim())
            .filter(|q| !q.is_empty() && q.len() <= MAX_LABEL_LEN)
            .unwrap_or(MAIN_QUEUE)
            .to_string();
        let category = self.category.and_then(label);
        let site = page.map(|p| SiteInfo {
            site: p.site.to_string(),
            selection: self.site.as_ref().and_then(|s| s.selection.clone()),
            quality: self
                .site
                .as_ref()
                .and_then(|s| s.quality.clone())
                .and_then(label),
        });
        let media = match (&site, self.media) {
            (Some(_), _) => None,
            (None, Some(m)) => Some(MediaInfo {
                quality: m.quality.and_then(label),
                ..m
            }),
            (None, None) => MediaKind::from_url(&url).map(MediaInfo::new),
        };
        Ok(Self {
            url: key,
            dir: if dir_ok { self.dir } else { PathBuf::new() },
            file_name,
            state,
            total,
            downloaded,
            queue,
            category,
            auto_dir: self.auto_dir || !dir_ok,
            checksum,
            media,
            site,
            created_at: self.created_at.max(0),
            completed_at: self.completed_at.map(|t| t.max(0)),
        })
    }
}

/// A short single-line text, or nothing.
fn label(text: String) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()
        && text.chars().count() <= MAX_LABEL_LEN
        && !text.chars().any(char::is_control))
    .then(|| text.to_string())
}

impl Backup {
    /// What `Manager::backup` writes.
    pub fn new(
        app_version: &str,
        exported_at: i64,
        settings: &Settings,
        records: &[TaskRecord],
    ) -> Self {
        Self {
            format: FORMAT.into(),
            version: VERSION,
            app_version: app_version.into(),
            exported_at,
            settings: exported_settings(settings),
            downloads: records.iter().map(BackupDownload::from_record).collect(),
            invalid: 0,
        }
    }

    pub fn to_json(&self) -> String {
        // Serializing plain data cannot fail.
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Read and check a backup. The structure must hold; downloads that do
    /// not are left out and counted in `invalid`.
    pub fn parse(bytes: &[u8]) -> Result<Self, BackupError> {
        if bytes.len() as u64 > MAX_BACKUP_BYTES {
            return Err(BackupError::TooLarge);
        }
        let value: Value = serde_json::from_slice(bytes).map_err(|_| BackupError::NotABackup)?;
        let Some(object) = value.as_object() else {
            return Err(BackupError::NotABackup);
        };
        if object.get("format").and_then(Value::as_str) != Some(FORMAT) {
            return Err(BackupError::NotABackup);
        }
        let version = object
            .get("version")
            .and_then(Value::as_u64)
            .ok_or_else(|| BackupError::Damaged("version is missing".into()))?;
        if version > u64::from(VERSION) {
            return Err(BackupError::NewerVersion(
                u32::try_from(version).unwrap_or(u32::MAX),
            ));
        }
        let settings: Settings = match object.get("settings") {
            Some(s) => serde_json::from_value(s.clone())
                .map_err(|e| BackupError::Damaged(format!("settings: {e}")))?,
            None => return Err(BackupError::Damaged("settings are missing".into())),
        };
        let raw = match object.get("downloads") {
            Some(Value::Array(items)) => items,
            Some(_) => return Err(BackupError::Damaged("downloads is not a list".into())),
            None => return Err(BackupError::Damaged("downloads are missing".into())),
        };
        if raw.len() > MAX_DOWNLOADS {
            return Err(BackupError::TooManyDownloads);
        }
        let mut downloads = Vec::with_capacity(raw.len());
        let mut invalid = 0;
        for item in raw {
            match serde_json::from_value::<BackupDownload>(item.clone())
                .map_err(|e| e.to_string())
                .and_then(BackupDownload::validated)
            {
                Ok(download) => downloads.push(download),
                Err(why) => {
                    tracing::debug!("backup: skipped a download: {why}");
                    invalid += 1;
                }
            }
        }
        Ok(Self {
            format: FORMAT.into(),
            version: VERSION,
            app_version: object
                .get("app_version")
                .and_then(Value::as_str)
                .and_then(|v| label(v.to_string()))
                .unwrap_or_default(),
            exported_at: object
                .get("exported_at")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .max(0),
            settings,
            downloads,
            invalid,
        })
    }
}

/// The settings as a backup writes them: without the paths of the
/// programs Downpipe runs, which belong to one machine and are never
/// imported anyway.
fn exported_settings(settings: &Settings) -> Settings {
    Settings {
        ffmpeg_path: String::new(),
        yt_dlp_path: String::new(),
        deno_path: String::new(),
        ..settings.clone()
    }
}

/// The settings after importing `imported` over `current`:
///
/// * the imported values win, except the programs Downpipe runs (FFmpeg,
///   yt-dlp, Deno), which keep their current paths, and folders that are
///   not absolute paths on this system, which keep the current folder;
/// * queues are matched by id and categories by name: imported ones replace
///   the same-named ones and are added otherwise, and the ones only this
///   installation has are kept.
///
/// The result is normalized; proxy and user agent are checked by
/// `Manager::update_settings` like any change.
pub fn merge_settings(current: &Settings, imported: &Settings) -> Settings {
    let mut merged = imported.clone();
    merged.ffmpeg_path = current.ffmpeg_path.clone();
    merged.yt_dlp_path = current.yt_dlp_path.clone();
    merged.deno_path = current.deno_path.clone();
    if !imported.download_dir.is_absolute() {
        merged.download_dir = current.download_dir.clone();
    }
    merged.queues = merge_by(&current.queues, &imported.queues, |q: &Queue| q.id.clone());
    let categories: Vec<Category> = imported
        .categories
        .iter()
        .cloned()
        .map(|mut c| {
            if looks_absolute(&c.folder) && !Path::new(c.folder.trim()).is_absolute() {
                // A folder of another system: the category's folder inside
                // the download folder instead.
                c.folder = String::new();
            }
            c
        })
        .collect();
    merged.categories = merge_by(&current.categories, &categories, |c: &Category| {
        c.name.trim().to_lowercase()
    });
    merged.normalized()
}

/// `current` in its order with same-keyed entries replaced by `imported`'s,
/// then the imported entries it did not have.
fn merge_by<T: Clone, K: PartialEq>(
    current: &[T],
    imported: &[T],
    key: impl Fn(&T) -> K,
) -> Vec<T> {
    let mut out: Vec<T> = current
        .iter()
        .map(|c| {
            imported
                .iter()
                .find(|i| key(i) == key(c))
                .unwrap_or(c)
                .clone()
        })
        .collect();
    for item in imported {
        if !out.iter().any(|o| key(o) == key(item)) {
            out.push(item.clone());
        }
    }
    out
}

/// A path written on Windows (`C:\`, `\\server`) or a Unix-like system (`/`).
fn looks_absolute(path: &str) -> bool {
    let path = path.trim();
    let bytes = path.as_bytes();
    path.starts_with('/')
        || path.starts_with("\\\\")
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'\\' | b'/'))
}

/// What a backup holds, for the import preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BackupSummary {
    pub app_version: String,
    pub exported_at: i64,
    /// Downloads that passed the checks.
    pub downloads: usize,
    /// Of those, the ones whose link is not in the list yet.
    pub new_downloads: usize,
    /// Downloads that did not pass the checks and will be left out.
    pub invalid: usize,
    /// Queues besides the main queue.
    pub queues: usize,
    pub categories: usize,
}

/// What to take from a backup.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct ImportChoice {
    pub settings: bool,
    pub downloads: bool,
}

/// What an import did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ImportReport {
    pub settings: bool,
    pub added: usize,
    /// Links already in the list (or twice in the file), left alone.
    pub duplicates: usize,
    /// Entries that did not pass the checks.
    pub invalid: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn download(value: Value) -> Result<BackupDownload, String> {
        serde_json::from_value::<BackupDownload>(value)
            .map_err(|e| e.to_string())
            .and_then(BackupDownload::validated)
    }

    fn abs(dir: &str) -> String {
        std::env::temp_dir()
            .join(dir)
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn downloads_are_checked_one_by_one() {
        let ok = download(json!({
            "url": "https://user:pw@example.org/a.zip#part",
            "dir": abs("dl"),
            "file_name": "  a?.zip ",
            "state": "downloading",
            "total": 10,
            "downloaded": 99,
            "queue": "",
            "checksum": "md5:nope",
            "category": "Compressed",
        }))
        .unwrap();
        assert!(
            ok.url.starts_with("https://example.org/a.zip"),
            "{}",
            ok.url
        );
        assert!(!ok.url.contains("pw"), "credentials are never kept");
        assert_eq!(ok.state, TaskState::Paused, "nothing starts by itself");
        assert_eq!(ok.downloaded, 10);
        assert_eq!(ok.queue, MAIN_QUEUE);
        assert_eq!(ok.checksum, None);
        assert_eq!(
            ok.file_name.as_deref(),
            Some(filename::sanitize("a?.zip").as_str())
        );
        assert!(!ok.auto_dir);

        for bad in [
            json!({"url": "javascript:alert(1)", "dir": abs("x"), "state": "paused"}),
            json!({"url": "file:///etc/passwd", "dir": abs("x"), "state": "paused"}),
            json!({"url": "not a url", "dir": abs("x"), "state": "paused"}),
            json!({"url": "https://example.org/a", "dir": abs("x"), "state": "exploded"}),
            json!({"url": "https://example.org/a", "state": "paused"}),
            json!({"url": format!("https://example.org/{}", "a".repeat(9000)), "dir": abs("x"), "state": "paused"}),
            json!({"url": "https://example.org/a", "dir": abs("x"), "state": "paused", "file_name": "n".repeat(300)}),
        ] {
            assert!(download(bad.clone()).is_err(), "{bad}");
        }
    }

    #[test]
    fn paths_of_another_system_fall_back_to_this_one() {
        let foreign = if cfg!(windows) {
            "/home/me/Downloads"
        } else {
            r"C:\Users\me\Downloads"
        };
        let d = download(
            json!({"url": "https://example.org/a.zip", "dir": foreign, "state": "completed"}),
        )
        .unwrap();
        assert_eq!(d.dir, PathBuf::new());
        assert!(d.auto_dir);
        assert_eq!(d.state, TaskState::Completed);
    }

    #[test]
    fn video_pages_and_streams_keep_their_kind() {
        let page = download(json!({
            "url": "https://youtu.be/jNQXAC9IVRw?t=4",
            "dir": abs("v"),
            "state": "paused",
            "site": {"site": "Anything", "quality": "1080p\nevil"},
        }))
        .unwrap();
        assert_eq!(page.url, "https://www.youtube.com/watch?v=jNQXAC9IVRw");
        let site = page.site.unwrap();
        assert_eq!(
            site.site, "YouTube",
            "the site comes from the link, not the file"
        );
        assert_eq!(site.quality, None);

        let stream = download(
            json!({"url": "https://a.org/v/master.m3u8", "dir": abs("v"), "state": "failed"}),
        )
        .unwrap();
        assert_eq!(stream.media.map(|m| m.kind), Some(MediaKind::Hls));

        let not_a_page = download(json!({
            "url": "https://example.org/a.zip", "dir": abs("v"), "state": "paused",
            "site": {"site": "YouTube"},
        }))
        .unwrap();
        assert_eq!(not_a_page.site, None);
    }

    #[test]
    fn structure_errors_are_told_apart() {
        assert_eq!(Backup::parse(b"hello"), Err(BackupError::NotABackup));
        assert_eq!(
            Backup::parse(br#"{"format":"other"}"#),
            Err(BackupError::NotABackup)
        );
        assert_eq!(Backup::parse(b"[1,2]"), Err(BackupError::NotABackup));
        assert_eq!(
            Backup::parse(
                br#"{"format":"downpipe-backup","version":7,"settings":{},"downloads":[]}"#
            ),
            Err(BackupError::NewerVersion(7))
        );
        assert!(matches!(
            Backup::parse(br#"{"format":"downpipe-backup","version":1,"downloads":[]}"#),
            Err(BackupError::Damaged(_))
        ));
        assert!(matches!(
            Backup::parse(
                br#"{"format":"downpipe-backup","version":1,"settings":{},"downloads":{}}"#
            ),
            Err(BackupError::Damaged(_))
        ));
        let big = vec![b' '; MAX_BACKUP_BYTES as usize + 1];
        assert_eq!(Backup::parse(&big), Err(BackupError::TooLarge));
    }

    #[test]
    fn bad_entries_are_counted_and_the_rest_kept() {
        let text = json!({
            "format": FORMAT,
            "version": 1,
            "app_version": "0.4.0",
            "exported_at": 1_760_000_000,
            "settings": {"max_concurrent_downloads": 5, "future_setting": true},
            "downloads": [
                {"url": "https://example.org/a.zip", "dir": abs("d"), "state": "paused"},
                {"url": "ftp://example.org/b.zip", "dir": abs("d"), "state": "paused"},
                "not even an object",
            ],
        });
        let backup = Backup::parse(text.to_string().as_bytes()).unwrap();
        assert_eq!(backup.downloads.len(), 1);
        assert_eq!(backup.invalid, 2);
        assert_eq!(backup.settings.max_concurrent_downloads, 5);
    }

    #[test]
    fn a_backup_round_trips_without_tool_paths() {
        let settings = Settings {
            ffmpeg_path: r"C:\tools\ffmpeg.exe".into(),
            yt_dlp_path: "/usr/bin/yt-dlp".into(),
            ..Settings::default()
        };
        let backup = Backup::new("0.5.0", 42, &settings, &[]);
        let json = backup.to_json();
        assert!(!json.contains("ffmpeg.exe") && !json.contains("/usr/bin/yt-dlp"));
        let back = Backup::parse(json.as_bytes()).unwrap();
        assert_eq!(back.app_version, "0.5.0");
        assert_eq!(back.exported_at, 42);
        assert_eq!(back.settings.categories, settings.categories);
    }

    #[test]
    fn settings_merge_keeps_what_only_this_installation_has() {
        let current = Settings {
            yt_dlp_path: "/opt/yt-dlp".into(),
            queues: vec![
                Queue {
                    id: MAIN_QUEUE.into(),
                    name: "Main".into(),
                    max_concurrent: 3,
                },
                Queue {
                    id: "night".into(),
                    name: "Night".into(),
                    max_concurrent: 1,
                },
            ],
            categories: vec![Category {
                name: "Books".into(),
                extensions: vec!["epub".into()],
                folder: String::new(),
            }],
            ..Settings::default()
        };
        let foreign = if cfg!(windows) {
            "/mnt/videos"
        } else {
            r"D:\Videos"
        };
        let imported = Settings {
            yt_dlp_path: r"C:\evil.exe".into(),
            download_dir: PathBuf::from("relative/dir"),
            max_concurrent_downloads: 6,
            queues: vec![
                Queue {
                    id: MAIN_QUEUE.into(),
                    name: "Main".into(),
                    max_concurrent: 6,
                },
                Queue {
                    id: "night".into(),
                    name: "Nightly".into(),
                    max_concurrent: 2,
                },
                Queue {
                    id: "music".into(),
                    name: "Music".into(),
                    max_concurrent: 1,
                },
            ],
            categories: vec![
                Category {
                    name: "books".into(),
                    extensions: vec!["mobi".into()],
                    folder: String::new(),
                },
                Category {
                    name: "Video".into(),
                    extensions: vec!["mp4".into()],
                    folder: foreign.into(),
                },
            ],
            ..Settings::default()
        };
        let merged = merge_settings(&current, &imported);
        assert_eq!(
            merged.yt_dlp_path, "/opt/yt-dlp",
            "programs never come from a file"
        );
        assert_eq!(merged.download_dir, current.download_dir);
        assert_eq!(merged.max_concurrent_downloads, 6);
        let queues: Vec<(&str, &str)> = merged
            .queues
            .iter()
            .map(|q| (q.id.as_str(), q.name.as_str()))
            .collect();
        assert_eq!(
            queues,
            [("main", "Main"), ("night", "Nightly"), ("music", "Music")]
        );
        assert_eq!(merged.categories.len(), 2);
        assert_eq!(merged.categories[0].extensions, ["mobi"]);
        assert_eq!(
            merged.categories[1].folder, "",
            "a folder of another system is dropped"
        );
    }
}
