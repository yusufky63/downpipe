use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::rules::{SiteRule, normalize_rules};

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
    /// Desktop app: offer to download http(s) links copied to the clipboard
    /// when their file type belongs to a category or they are video pages.
    pub watch_clipboard: bool,
    /// Desktop app: keep the computer from sleeping while downloads run.
    /// The display may still turn off.
    pub keep_awake: bool,
    /// Downloads sent by the browser wait until the user confirms their
    /// name, folder and queue (IDM's "Download File Info").
    pub confirm_browser_downloads: bool,
    /// Interface language as a code such as `tr` or `pt-BR`; empty follows
    /// the system. The desktop app decides which codes it has.
    pub language: String,
    /// Download queues. The first is always the main queue, whose limit is
    /// `max_concurrent_downloads`.
    pub queues: Vec<Queue>,
    /// FFmpeg used to merge separate video and audio of streams. Empty:
    /// `ffmpeg` on `PATH`, if there is one. Downpipe does not ship FFmpeg.
    pub ffmpeg_path: String,
    /// yt-dlp for videos on pages such as YouTube. Empty: the copy in
    /// Downpipe's tools folder (downloaded when the user asks), else
    /// `yt-dlp` on `PATH`.
    pub yt_dlp_path: String,
    /// Deno, which yt-dlp needs for YouTube. Empty: the tools folder, then
    /// `PATH` (and Node 22 or newer when there is no Deno at all).
    pub deno_path: String,
    /// Per-site overrides; see [`crate::rules`].
    pub site_rules: Vec<SiteRule>,
    /// The answer to "this link is already in the list" once the user chose
    /// "Don't ask again": one of [`DUPLICATE_ANSWERS`]. `ask` asks every
    /// time. The desktop app applies it; the core still holds a browser's
    /// duplicate for confirmation, so the answer is given in one place.
    pub duplicate_answer: String,
    /// Remote control from a phone or another computer (off by default).
    pub remote: RemoteSettings,
    /// BitTorrent: seeding, upload limit and how peers are found.
    pub torrent: TorrentSettings,
}

/// BitTorrent settings. The download limit is `Settings::speed_limit`, the
/// same as for everything else.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TorrentSettings {
    /// Keep uploading a finished torrent to other peers until a limit below
    /// is reached. Off: stop as soon as it finishes.
    pub seed: bool,
    /// Stop seeding once this many percent of the downloaded size was
    /// uploaded (100 = ratio 1.0); 0 for no ratio limit.
    pub seed_ratio_percent: u32,
    /// Stop seeding after this many minutes; 0 for no time limit.
    pub seed_minutes: u32,
    /// Upload limit in bytes per second, 0 = unlimited.
    pub upload_limit: u64,
    /// TCP port for incoming peers; 0 picks a free one at every start.
    pub listen_port: u16,
    /// Find peers through the distributed hash table.
    pub dht: bool,
    /// Find peers on the local network.
    pub lsd: bool,
    /// Ask the router to forward the port (UPnP). Off by default: it opens
    /// a port on the router.
    pub upnp: bool,
}

impl Default for TorrentSettings {
    fn default() -> Self {
        Self {
            seed: true,
            // Give back what was taken, for at most two hours.
            seed_ratio_percent: 100,
            seed_minutes: 120,
            upload_limit: 0,
            listen_port: 0,
            dht: true,
            lsd: true,
            upnp: false,
        }
    }
}

/// Port the remote control listens on unless the user picks another.
pub const DEFAULT_REMOTE_PORT: u16 = 9420;

/// The optional remote control: an HTTP API and a small web page served by
/// the desktop app (`downpipe-remote`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RemoteSettings {
    pub enabled: bool,
    /// IP address to listen on: `127.0.0.1` (this computer only) unless the
    /// user explicitly picks a network address such as `0.0.0.0`.
    pub bind: String,
    pub port: u16,
    /// SHA-256 of the access token, lower-case hex. The token itself is
    /// shown once when it is made and never stored, so neither the catalog
    /// nor the settings the UI reads can give it away.
    pub token_hash: String,
}

impl Default for RemoteSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            bind: "127.0.0.1".into(),
            port: DEFAULT_REMOTE_PORT,
            token_hash: String::new(),
        }
    }
}

impl RemoteSettings {
    fn normalized(mut self) -> Self {
        let bind = self.bind.trim();
        self.bind = match bind.parse::<std::net::IpAddr>() {
            Ok(ip) => ip.to_string(),
            Err(_) => Self::default().bind,
        };
        // Ports below 1024 need administrator rights on some systems.
        if self.port < 1024 {
            self.port = DEFAULT_REMOTE_PORT;
        }
        self.token_hash = self.token_hash.trim().to_ascii_lowercase();
        if self.token_hash.len() != 64 || !self.token_hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            self.token_hash.clear();
        }
        // Without a token nobody could sign in, so it cannot be on.
        if self.token_hash.is_empty() {
            self.enabled = false;
        }
        self
    }

    /// Listens on an address other machines can reach.
    pub fn is_network(&self) -> bool {
        self.bind
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| !ip.is_loopback())
    }
}

/// The values of [`Settings::duplicate_answer`]: ask, a numbered copy, the
/// old file downloaded again in place, or the download already in the list.
pub const DUPLICATE_ANSWERS: [&str; 4] = ["ask", "copy", "overwrite", "existing"];

/// The queue every download belongs to unless moved.
pub const MAIN_QUEUE: &str = "main";

/// A named queue with its own number of downloads at once.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Queue {
    /// Stable id; generated from the name for new queues.
    pub id: String,
    pub name: String,
    /// 1 to 16. Ignored for the main queue, which uses
    /// `Settings::max_concurrent_downloads`.
    pub max_concurrent: u32,
}

impl Default for Queue {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            max_concurrent: 1,
        }
    }
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
    /// Queue id the schedule starts and stops; empty for every queue.
    pub queue: String,
}

impl Default for Schedule {
    fn default() -> Self {
        Self {
            enabled: false,
            start: "02:00".into(),
            stop: "07:00".into(),
            days: [true; 7],
            queue: String::new(),
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
            watch_clipboard: false,
            keep_awake: true,
            confirm_browser_downloads: true,
            language: String::new(),
            queues: default_queues(),
            ffmpeg_path: String::new(),
            yt_dlp_path: String::new(),
            deno_path: String::new(),
            site_rules: Vec::new(),
            duplicate_answer: "ask".into(),
            remote: RemoteSettings::default(),
            torrent: TorrentSettings::default(),
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
        self.language = self.language.trim().to_string();
        // A language tag; anything else is treated as "follow the system".
        if self.language.len() > 16
            || !self
                .language
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            self.language.clear();
        }
        self.user_agent = self.user_agent.trim().to_string();
        // Pasted paths often come quoted from a file manager.
        self.ffmpeg_path = self.ffmpeg_path.trim().trim_matches('"').to_string();
        self.yt_dlp_path = self.yt_dlp_path.trim().trim_matches('"').to_string();
        self.deno_path = self.deno_path.trim().trim_matches('"').to_string();
        // An answer a newer version knows and this one does not asks again.
        if !DUPLICATE_ANSWERS.contains(&self.duplicate_answer.as_str()) {
            self.duplicate_answer = "ask".into();
        }

        self.remote = std::mem::take(&mut self.remote).normalized();
        // Ratio 100.0 and 30 days are far beyond anything useful.
        self.torrent.seed_ratio_percent = self.torrent.seed_ratio_percent.min(10_000);
        self.torrent.seed_minutes = self.torrent.seed_minutes.min(60 * 24 * 30);
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
        self.queues = normalize_queues(
            std::mem::take(&mut self.queues),
            self.max_concurrent_downloads,
        );
        if !self.schedule.queue.is_empty()
            && !self.queues.iter().any(|q| q.id == self.schedule.queue)
        {
            self.schedule.queue = String::new();
        }
        self.site_rules = normalize_rules(std::mem::take(&mut self.site_rules));
        for rule in &mut self.site_rules {
            if !rule.queue.is_empty() && !self.queues.iter().any(|q| q.id == rule.queue) {
                rule.queue = String::new();
            }
        }
        self
    }

    /// The rule that applies to downloads from `url`, if any.
    pub fn site_rule_for(&self, url: &url::Url) -> Option<&SiteRule> {
        crate::rules::rule_for(&self.site_rules, url)
    }

    /// Where `dir` of a rule or category points: absolute as it is,
    /// otherwise inside the download folder.
    pub fn resolve_dir(&self, folder: &str) -> PathBuf {
        let folder = folder.trim();
        if Path::new(folder).is_absolute() {
            PathBuf::from(folder)
        } else {
            self.download_dir.join(folder)
        }
    }

    pub fn queue(&self, id: &str) -> Option<&Queue> {
        self.queues.iter().find(|q| q.id == id)
    }

    /// Downloads a queue may run at once.
    pub fn queue_limit(&self, id: &str) -> usize {
        if id == MAIN_QUEUE {
            return self.max_concurrent_downloads.max(1) as usize;
        }
        self.queue(id)
            .map_or(1, |q| q.max_concurrent.max(1) as usize)
    }

    /// Lower-case extensions of every category, for the clipboard watcher.
    pub fn watched_extensions(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .categories
            .iter()
            .flat_map(|c| c.extensions.iter().cloned())
            .collect();
        out.sort();
        out.dedup();
        out
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
        } else {
            self.resolve_dir(folder)
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

pub fn default_queues() -> Vec<Queue> {
    vec![Queue {
        id: MAIN_QUEUE.into(),
        name: "Main".into(),
        max_concurrent: 3,
    }]
}

/// Main queue first, unique ids, named queues only, limits in range.
fn normalize_queues(queues: Vec<Queue>, main_limit: u32) -> Vec<Queue> {
    let mut out: Vec<Queue> = Vec::new();
    let mut main = None;
    for mut q in queues {
        q.name = q.name.trim().to_string();
        q.id = q.id.trim().to_string();
        q.max_concurrent = q.max_concurrent.clamp(1, 16);
        if q.id == MAIN_QUEUE {
            main.get_or_insert(q);
            continue;
        }
        if q.name.is_empty() {
            continue;
        }
        if q.id.is_empty() || out.iter().any(|o| o.id == q.id) {
            q.id = unique_queue_id(&q.name, &out);
        }
        out.push(q);
    }
    let mut main = main.unwrap_or_else(|| default_queues().remove(0));
    if main.name.is_empty() {
        main.name = "Main".into();
    }
    main.max_concurrent = main_limit.clamp(1, 16);
    out.insert(0, main);
    out
}

fn unique_queue_id(name: &str, taken: &[Queue]) -> String {
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let base = if slug.is_empty() || slug == MAIN_QUEUE {
        "queue".to_string()
    } else {
        slug
    };
    let mut id = base.clone();
    let mut n = 2;
    while taken.iter().any(|q| q.id == id) || id == MAIN_QUEUE {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
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
            yt_dlp_path: r#" "C:\Tools\yt-dlp.exe" "#.into(),
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
        assert_eq!(s.yt_dlp_path, r"C:\Tools\yt-dlp.exe");
        assert_eq!(s.categories.len(), 1, "unnamed categories are dropped");
        assert_eq!(s.categories[0].name, "Books");
        assert_eq!(s.categories[0].extensions, ["epub", "mobi"]);
        assert_eq!(s.schedule.start, "02:00");
        assert_eq!(s.schedule.stop, "");
    }

    #[test]
    fn duplicate_answer_is_kept_only_when_known() {
        for answer in DUPLICATE_ANSWERS {
            let s = Settings {
                duplicate_answer: answer.into(),
                ..Settings::default()
            }
            .normalized();
            assert_eq!(s.duplicate_answer, answer);
        }
        let s = Settings {
            duplicate_answer: "rename".into(),
            ..Settings::default()
        }
        .normalized();
        assert_eq!(s.duplicate_answer, "ask");
        let s: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(s.duplicate_answer, "ask");
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
        // Conservative seeding: ratio 1.0 or two hours, no UPnP.
        assert!(s.torrent.seed && s.torrent.dht && !s.torrent.upnp);
        assert_eq!(s.torrent.seed_ratio_percent, 100);
        assert_eq!(s.torrent.seed_minutes, 120);
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
    fn queues_keep_main_first_and_get_ids() {
        let s = Settings {
            max_concurrent_downloads: 4,
            queues: vec![
                Queue {
                    id: String::new(),
                    name: " Night downloads ".into(),
                    max_concurrent: 0,
                },
                Queue {
                    id: "night-downloads".into(),
                    name: "Other".into(),
                    max_concurrent: 99,
                },
                Queue::default(),
            ],
            schedule: Schedule {
                queue: "gone".into(),
                ..Schedule::default()
            },
            ..Settings::default()
        }
        .normalized();
        let ids: Vec<&str> = s.queues.iter().map(|q| q.id.as_str()).collect();
        assert_eq!(
            ids,
            ["main", "night-downloads", "other"],
            "a clashing id is rebuilt from the name"
        );
        assert_eq!(s.queues[0].name, "Main");
        assert_eq!(s.queues[1].max_concurrent, 1);
        assert_eq!(s.queues[2].max_concurrent, 16);
        assert_eq!(
            s.queue_limit("main"),
            4,
            "main follows max_concurrent_downloads"
        );
        assert_eq!(s.queue_limit("night-downloads"), 1);
        assert_eq!(s.queue_limit("missing"), 1);
        assert_eq!(s.schedule.queue, "", "unknown queue falls back to all");
    }

    #[test]
    fn language_is_a_tag_or_empty() {
        let pick = |language: &str| {
            Settings {
                language: language.into(),
                ..Settings::default()
            }
            .normalized()
            .language
        };
        assert_eq!(pick(" pt-BR "), "pt-BR");
        assert_eq!(pick(""), "");
        assert_eq!(pick("../../etc"), "", "not a language tag");
    }

    #[test]
    fn clipboard_watches_every_category_type() {
        let s = Settings::default();
        let exts = s.watched_extensions();
        assert!(exts.contains(&"iso".to_string()) && exts.contains(&"mp4".to_string()));
        assert!(
            exts.windows(2).all(|w| w[0] < w[1]),
            "sorted, no duplicates"
        );
    }

    #[test]
    fn remote_control_needs_a_token_and_a_valid_address() {
        let pick = |remote: RemoteSettings| {
            Settings {
                remote,
                ..Settings::default()
            }
            .normalized()
            .remote
        };
        let off = Settings::default().remote;
        assert!(!off.enabled && off.bind == "127.0.0.1" && !off.is_network());

        let hash = "AB".repeat(32);
        let on = pick(RemoteSettings {
            enabled: true,
            bind: " 0.0.0.0 ".into(),
            port: 80,
            token_hash: hash.clone(),
        });
        assert!(on.enabled && on.is_network());
        assert_eq!(on.bind, "0.0.0.0");
        assert_eq!(on.port, DEFAULT_REMOTE_PORT, "privileged ports fall back");
        assert_eq!(on.token_hash, hash.to_ascii_lowercase());

        let bad = pick(RemoteSettings {
            enabled: true,
            bind: "my-computer".into(),
            port: 50000,
            token_hash: "not a hash".into(),
        });
        assert_eq!(bad.bind, "127.0.0.1", "only IP addresses");
        assert!(bad.token_hash.is_empty());
        assert!(!bad.enabled, "no token, no remote control");
        assert_eq!(bad.port, 50000);
    }

    #[test]
    fn extensions() {
        assert_eq!(extension_of("a/b/c.Zip").as_deref(), Some("zip"));
        assert_eq!(extension_of("noext"), None);
        assert_eq!(extension_of("trailing."), None);
        assert_eq!(extension_of("x.thisisfartoolong"), None);
    }
}
