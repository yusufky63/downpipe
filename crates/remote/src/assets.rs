//! The web page's files. The desktop app hands over the files its own
//! build bundled; tests and development read them from a folder.

use std::path::PathBuf;

/// The page every browser gets at `/`.
pub const PAGE: &str = "remote.html";

/// One file of the web page.
#[derive(Debug, Clone)]
pub struct Asset {
    pub bytes: Vec<u8>,
    pub mime: String,
}

/// Where the web page's files come from. Only [`PAGE`], `icon.svg` and
/// files directly in `assets/` are ever asked for, with plain names.
pub trait Assets: Send + Sync + std::fmt::Debug + 'static {
    fn get(&self, path: &str) -> Option<Asset>;
}

/// Files of a built `dist` folder.
#[derive(Debug, Clone)]
pub struct DirAssets {
    root: PathBuf,
}

impl DirAssets {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl Assets for DirAssets {
    fn get(&self, path: &str) -> Option<Asset> {
        if !is_servable(path) {
            return None;
        }
        let bytes = std::fs::read(self.root.join(path)).ok()?;
        Some(Asset {
            bytes,
            mime: mime_for(path).to_string(),
        })
    }
}

/// No web page, only the API (a build without the page's files).
#[derive(Debug, Clone, Copy, Default)]
pub struct NoAssets;

impl Assets for NoAssets {
    fn get(&self, _path: &str) -> Option<Asset> {
        None
    }
}

/// `remote.html`, `icon.svg` or `assets/<name>` with a plain file name, so
/// no request can reach another file of the bundle, let alone the disk.
pub fn is_servable(path: &str) -> bool {
    if path == PAGE || path == "icon.svg" {
        return true;
    }
    let Some(name) = path.strip_prefix("assets/") else {
        return false;
    };
    !name.is_empty()
        && name.len() <= 128
        && !name.starts_with('.')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        && !name.ends_with(".html")
}

pub fn mime_for(path: &str) -> &'static str {
    match path.rsplit_once('.').map(|(_, ext)| ext) {
        Some("html") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        Some("json") => "application/json",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serves_only_the_page_and_its_assets() {
        for ok in [
            "remote.html",
            "icon.svg",
            "assets/remote-Bx1_9.js",
            "assets/dm-sans-latin-400.woff2",
        ] {
            assert!(is_servable(ok), "{ok}");
        }
        for refused in [
            "index.html",
            "confirm.html",
            "assets/../index.html",
            "assets/sub/x.js",
            "assets/.hidden",
            "assets/",
            "assets/x.html",
            "../secrets",
            "assets/a%2f..",
        ] {
            assert!(!is_servable(refused), "{refused}");
        }
    }
}
