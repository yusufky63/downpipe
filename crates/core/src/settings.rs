use std::path::PathBuf;

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
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            download_dir: default_download_dir(),
            max_concurrent_downloads: 3,
            max_connections_per_download: 8,
            speed_limit: 0,
            register_browser_host: true,
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
        self
    }
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
        }
        .normalized();
        assert_eq!(s.max_concurrent_downloads, 1);
        assert_eq!(s.max_connections_per_download, 32);
        assert!(!s.download_dir.as_os_str().is_empty());
        assert_eq!(s.speed_limit, 7);
    }

    #[test]
    fn missing_fields_take_defaults() {
        let s: Settings = serde_json::from_str(r#"{"speed_limit": 1024}"#).unwrap();
        assert_eq!(s.speed_limit, 1024);
        assert_eq!(s.max_concurrent_downloads, 3);
        // Catalogs written before the setting existed keep the bridge registered.
        assert!(s.register_browser_host);
    }
}
