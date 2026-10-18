//! What a `.torrent` file (or a magnet link's metadata) describes: the
//! name, the files and their sizes.

use std::path::{Component, Path, PathBuf};

use downpipe_http::DownloadError;
use serde::Serialize;

/// Largest `.torrent` file accepted. Real ones are kilobytes to a few
/// megabytes (huge torrents with small pieces); anything bigger is not a
/// torrent.
pub const MAX_TORRENT_FILE: usize = 16 * 1024 * 1024;

/// One file of a torrent, in the torrent's order (its index is what the
/// file selection refers to).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TorrentFile {
    /// Path inside the torrent, `/`-separated.
    pub path: String,
    pub size: u64,
    /// BEP 47 padding: filler that keeps pieces aligned, never shown or
    /// written as a file of its own.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub padding: bool,
}

/// A torrent's metadata, checked for names that could leave the download
/// folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Metadata {
    pub info_hash: String,
    /// The torrent's name: the file of a single-file torrent, the folder of
    /// a multi-file one.
    pub name: String,
    pub files: Vec<TorrentFile>,
    pub total: u64,
    /// Private torrents (BEP 27) only talk to their trackers: no DHT, PEX
    /// or local discovery.
    pub private: bool,
}

impl Metadata {
    /// Read a `.torrent` file.
    pub fn parse(bytes: &[u8]) -> Result<Self, DownloadError> {
        if bytes.len() > MAX_TORRENT_FILE {
            return Err(invalid("the file is too large to be a torrent"));
        }
        let torrent = librqbit::torrent_from_bytes(bytes)
            .map_err(|e| invalid(&format!("it is not a valid torrent file ({e})")))?;
        let info_hash = torrent.info_hash.as_string();
        let info = torrent
            .info
            .data
            .validate()
            .map_err(|e| invalid(&e.to_string()))?;
        let mut files = Vec::new();
        for details in info.iter_file_details() {
            let parts = details.filename.to_vec();
            check_parts(&parts)?;
            files.push(TorrentFile {
                path: parts.join("/"),
                size: details.len,
                padding: details.attrs().padding,
            });
        }
        let name = match info.name() {
            Some(name) if !name.trim().is_empty() => name.into_owned(),
            _ => files
                .first()
                .map(|f| f.path.rsplit('/').next().unwrap_or(&f.path).to_string())
                .unwrap_or_else(|| "torrent".into()),
        };
        Ok(Self {
            info_hash,
            name,
            total: files.iter().filter(|f| !f.padding).map(|f| f.size).sum(),
            private: info.info().private,
            files,
        })
    }

    /// More than one real file: the torrent gets a folder of its own.
    pub fn is_folder(&self) -> bool {
        self.files.iter().filter(|f| !f.padding).count() > 1
            || self.files.first().is_some_and(|f| f.path.contains('/'))
    }

    /// Where the torrent's data goes inside `dir`: a folder named after the
    /// torrent for several files, else `dir` itself.
    pub fn output_dir(&self, dir: &Path) -> PathBuf {
        if self.is_folder() {
            dir.join(downpipe_http::filename::sanitize(&self.name))
        } else {
            dir.to_path_buf()
        }
    }

    /// The finished download as the list shows it: the folder, or the one
    /// file.
    pub fn output_path(&self, dir: &Path) -> PathBuf {
        let out = self.output_dir(dir);
        if self.is_folder() {
            return out;
        }
        match self.files.iter().find(|f| !f.padding) {
            Some(file) => out.join(local_path(&file.path)),
            None => out,
        }
    }

    /// Bytes of the chosen files (all files when `only` is `None`).
    pub fn selected_size(&self, only: Option<&[usize]>) -> u64 {
        self.files
            .iter()
            .enumerate()
            .filter(|(i, f)| !f.padding && only.is_none_or(|o| o.contains(i)))
            .map(|(_, f)| f.size)
            .sum()
    }
}

/// A torrent path (`a/b.txt`) as a relative path on this system.
pub fn local_path(path: &str) -> PathBuf {
    path.split('/').collect()
}

/// Every part must stay a plain name on this system: on Windows `C:`,
/// `a:stream` (an alternate data stream) or a reserved device name would
/// escape the folder or write somewhere unexpected.
fn check_parts(parts: &[String]) -> Result<(), DownloadError> {
    for part in parts {
        let bad = part.is_empty()
            || part.contains(['/', '\\', '\0'])
            || (cfg!(windows) && (part.contains(':') || is_reserved(part)))
            || Path::new(part)
                .components()
                .any(|c| !matches!(c, Component::Normal(_)));
        if bad {
            return Err(invalid(&format!(
                "it contains a file name that is not safe to write (`{part}`)"
            )));
        }
    }
    Ok(())
}

/// Windows device names, which name a device whatever the extension.
fn is_reserved(part: &str) -> bool {
    let stem = part
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end()
        .to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.len() == 4
            && stem.as_bytes()[3].is_ascii_digit()
            && stem.as_bytes()[3] != b'0')
}

pub(crate) fn invalid(why: &str) -> DownloadError {
    DownloadError::Torrent(format!("this torrent cannot be used: {why}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_names() {
        assert!(is_reserved("CON"));
        assert!(is_reserved("nul.txt"));
        assert!(is_reserved("com1"));
        assert!(!is_reserved("com0"));
        assert!(!is_reserved("console.log"));
        assert!(!is_reserved("readme"));
    }

    #[test]
    fn unsafe_parts_are_refused() {
        assert!(check_parts(&["a".into(), "b.txt".into()]).is_ok());
        assert!(check_parts(&["..".into()]).is_err());
        assert!(check_parts(&[".".into()]).is_err());
        assert!(check_parts(&["".into()]).is_err());
        assert!(check_parts(&["a\\b".into()]).is_err());
        if cfg!(windows) {
            assert!(check_parts(&["C:".into(), "x".into()]).is_err());
            assert!(check_parts(&["file.txt:stream".into()]).is_err());
            assert!(check_parts(&["aux".into()]).is_err());
        }
    }
}
