use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub download_dir: PathBuf,
    /// Downloads that run at the same time; the rest wait in the queue.
    pub max_concurrent_downloads: u32,
    /// Upper bound of connections per download.
    pub max_connections_per_download: u32,
    /// Global limit in bytes per second, 0 = unlimited.
    pub speed_limit: u64,
    /// Desktop app: at every start, point the browsers' Native Messaging
    /// registration (current user only) at the host shipped with the app.
    pub register_browser_host: bool,
    /// Desktop app: a system notification when a download finishes or fails
    /// while the window is not in front.
    pub notify_on_finish: bool,
    /// Desktop app: closing the window keeps Downpipe running in the tray,
    /// so the browser can still hand downloads over.
    pub close_to_tray: bool,
    /// Desktop app: start (in the tray) when the user signs in.
    pub start_with_system: bool,
    /// `http://`, `https://` or `socks5://` proxy. Empty: the system proxy.
    pub proxy: String,
    /// Empty: Downpipe's own user agent.
    pub user_agent: String,
    /// Sort new downloads into category folders by file type.
    pub use_categories: bool,
    pub categories: Vec<Category>,
    pub schedule: Schedule,
}

/// A file-type group with its own folder, like IDM's categories.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Category {
    pub name: String,
    /// Lower-case, without dots.
    pub extensions: Vec<String>,
    /// Empty: a folder named after the category inside the download folder.
    pub folder: String,
}

/// Start and stop the queue at fixed times of day (local time).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Schedule {
    pub enabled: bool,
    /// `HH:MM`: resume paused and failed downloads.
    pub start: String,
    /// `HH:MM`, or empty for no stop time: pause everything.
    pub stop: String,
    /// Monday first.
    pub days: [bool; 7],
}

impl Default for Schedule {
    fn default() -> Self {
        Self {
            enabled: false,
            start: "02:00".into(),
            stop: "07:00".into(),
            days: [true; 7],
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            download_dir: default_download_dir(),
            max_concurrent_downloads: 3,
            max_connections_per_download: 8,
            speed_limit: 0,
            register_browser_host: true,
            notify_on_finish: true,
            close_to_tray: true,
            start_with_system: false,
            proxy: String::new(),
            user_agent: String::new(),
            use_categories: true,
            categories: default_categories(),
            schedule: Schedule::default(),
        }
    }
}

impl Settings {
    /// Clamp values a UI might send out of range.
    pub fn normalized(mut self) -> Self {
        self.max_concurrent_downloads = self.max_concurrent_downloads.clamp(1, 16);
        self.max_connections_per_download = self.max_connections_per_download.clamp(1, 32);
        if self.download_dir.as_os_str().is_empty() {
            self.download_dir = default_download_dir();
        }
        self.proxy = self.proxy.trim().to_string();
        self.user_agent = self.user_agent.trim().to_string();
        self.categories = self
            .categories
            .into_iter()
            .map(Category::normalized)
            .filter(|c| !c.name.is_empty())
            .collect();
        if !crate::schedule::is_time(&self.schedule.start) {
            self.schedule.start = Schedule::default().start;
        }
        if !self.schedule.stop.trim().is_empty() && !crate::schedule::is_time(&self.schedule.stop) {
            self.schedule.stop = String::new();
        }
        self
    }

    /// The category a file name belongs to, if categories are on.
    pub fn category_for(&self, file_name: &str) -> Option<&Category> {
        if !self.use_categories {
            return None;
        }
        let ext = extension_of(file_name)?;
        self.categories.iter().find(|c| c.extensions.contains(&ext))
    }

    /// Where downloads of `category` are saved.
    pub fn category_dir(&self, category: &Category) -> PathBuf {
        let folder = category.folder.trim();
        if folder.is_empty() {
            self.download_dir
                .join(downpipe_http::filename::sanitize(&category.name))
        } else if Path::new(folder).is_absolute() {
            PathBuf::from(folder)
        } else {
            self.download_dir.join(folder)
        }
    }
}

impl Category {
    fn normalized(mut self) -> Self {
        self.name = self.name.trim().to_string();
        self.folder = self.folder.trim().to_string();
        let mut seen = Vec::new();
        for ext in self.extensions {
            let ext = ext.trim().trim_start_matches('.').to_ascii_lowercase();
            if !ext.is_empty() && !seen.contains(&ext) {
                seen.push(ext);
            }
        }
        self.extensions = seen;
        self
    }
}

/// `report.final.PDF` -> `pdf`. Names without a dot have no extension.
pub fn extension_of(file_name: &str) -> Option<String> {
    let name = file_name.rsplit(['/', '\\']).next().unwrap_or(file_name);
    let (stem, ext) = name.rsplit_once('.')?;
    (!stem.is_empty() && !ext.is_empty() && ext.len() <= 12).then(|| ext.to_ascii_lowercase())
}

fn category(name: &str, extensions: &[&str]) -> Category {
    Category {
        name: name.into(),
        extensions: extensions.iter().map(|e| (*e).to_string()).collect(),
        folder: String::new(),
    }
}

/// IDM's set, plus disk images, which are common enough to deserve a folder.
pub fn default_categories() -> Vec<Category> {
    vec![
        category(
            "Compressed",
            &[
                "zip", "rar", "7z", "tar", "gz", "tgz", "bz2", "xz", "zst", "lz", "lzma", "cab",
                "arj",
            ],
        ),
        category(
            "Documents",
            &[
                "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "ods", "odp", "rtf",
                "txt", "csv", "epub", "md",
            ],
        ),
        category(
            "Music",
            &[
                "mp3", "flac", "wav", "aac", "ogg", "oga", "m4a", "opus", "wma", "aif", "aiff",
            ],
        ),
        category(
            "Video",
            &[
                "mp4", "mkv", "webm", "avi", "mov", "m4v", "wmv", "flv", "mpg", "mpeg", "3gp",
            ],
        ),
        category(
            "Programs",
            &[
                "exe", "msi", "msix", "appx", "dmg", "pkg", "deb", "rpm", "appimage", "apk",
            ],
        ),
        category(
            "Disk images",
            &["iso", "img", "vhd", "vhdx", "qcow2", "vmdk"],
        ),
    ]
}

pub fn default_download_dir() -> PathBuf {
    dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_out_of_range_values() {
        let s = Settings {
            download_dir: PathBuf::new(),
            max_concurrent_downloads: 0,
            max_connections_per_download: 500,
            speed_limit: 7,
            register_browser_host: false,
            proxy: "  socks5://127.0.0.1:1080 ".into(),
            categories: vec![
                Category {
                    name: " Books ".into(),
                    extensions: vec![".EPUB".into(), "epub".into(), " ".into(), "mobi".into()],
                    folder: String::new(),
                },
                Category::default(),
            ],
            schedule: Schedule {
                start: "25:00".into(),
                stop: "nope".into(),
                ..Schedule::default()
            },
            ..Settings::default()
        }
        .normalized();
        assert_eq!(s.max_concurrent_downloads, 1);
        assert_eq!(s.max_connections_per_download, 32);
        assert!(!s.download_dir.as_os_str().is_empty());
        assert_eq!(s.speed_limit, 7);
        assert_eq!(s.proxy, "socks5://127.0.0.1:1080");
        assert_eq!(s.categories.len(), 1, "unnamed categories are dropped");
        assert_eq!(s.categories[0].name, "Books");
        assert_eq!(s.categories[0].extensions, ["epub", "mobi"]);
        assert_eq!(s.schedule.start, "02:00");
        assert_eq!(s.schedule.stop, "");
    }

    #[test]
    fn missing_fields_take_defaults() {
        let s: Settings = serde_json::from_str(r#"{"speed_limit": 1024}"#).unwrap();
        assert_eq!(s.speed_limit, 1024);
        assert_eq!(s.max_concurrent_downloads, 3);
        // Catalogs written before the setting existed keep the bridge registered.
        assert!(s.register_browser_host);
        assert!(s.close_to_tray && s.notify_on_finish && s.use_categories);
        assert!(!s.start_with_system && !s.schedule.enabled);
        assert_eq!(s.categories, default_categories());
    }

    #[test]
    fn classifies_by_extension() {
        let s = Settings {
            download_dir: PathBuf::from("D:/Downloads"),
            ..Settings::default()
        };
        assert_eq!(
            s.category_for("ubuntu.ISO").map(|c| c.name.as_str()),
            Some("Disk images")
        );
        assert_eq!(
            s.category_for("backup.tar.gz").map(|c| c.name.as_str()),
            Some("Compressed")
        );
        assert_eq!(s.category_for("README"), None);
        assert_eq!(s.category_for(".bashrc"), None);
        let video = s.category_for("clip.mp4").unwrap();
        assert_eq!(
            s.category_dir(video),
            PathBuf::from("D:/Downloads").join("Video")
        );

        let custom = Category {
            name: "Work".into(),
            extensions: vec![],
            folder: "Projects/Work".into(),
        };
        assert_eq!(
            s.category_dir(&custom),
            PathBuf::from("D:/Downloads").join("Projects/Work")
        );

        let off = Settings {
            use_categories: false,
            ..s
        };
        assert_eq!(off.category_for("clip.mp4"), None);
    }

    #[test]
    fn extensions() {
        assert_eq!(extension_of("a/b/c.Zip").as_deref(), Some("zip"));
        assert_eq!(extension_of("noext"), None);
        assert_eq!(extension_of("trailing."), None);
        assert_eq!(extension_of("x.thisisfartoolong"), None);
    }
}
