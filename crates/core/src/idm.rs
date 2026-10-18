//! Reading Internet Download Manager's download list, to bring it into
//! Downpipe. Read only: nothing here writes to IDM's files or registry.
//!
//! Two sources:
//!
//! * **An exported list** (IDM: Tasks → Export). The text export has one
//!   address per line; the `.ef2` export has one block per download,
//!   `<`, the address, header lines such as `referer:`, `User-Agent:` or
//!   `cookie:`, then `>`. Only the address and a `filename:` line are
//!   read; headers, cookies, form data and credentials are skipped
//!   without being kept.
//! * **IDM's own list** on Windows: one numbered key per download under
//!   `HKEY_CURRENT_USER\Software\DownloadManager`. Only these values are
//!   read: `Url0` (the address), `FileName` (the name IDM shows),
//!   `LocalPath` and `LocalFileName` (IDM's working folder and file, which
//!   give a folder only when they are outside IDM's own `DwnlData` folder,
//!   and a name with an extension when `FileName` lacks one), `FileSize`
//!   and `Downloaded` (64-bit little-endian byte counts). The keys are
//!   opened for reading only, and values such as `Cookie`, `User`,
//!   `EncPassword`, `CstmHdrs`, `Referer` or `UA` are never queried.

use std::path::{Path, PathBuf};

use downpipe_http::filename;
use serde::Serialize;

use crate::backup::BackupDownload;
use crate::settings::MAIN_QUEUE;
use crate::task::TaskState;

/// An exported list of 100,000 links is a few megabytes.
pub const MAX_LIST_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_ENTRIES: usize = 100_000;
const MAX_URL_LEN: usize = 8 * 1024;

/// One download found in IDM's list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IdmEntry {
    pub url: String,
    /// The name IDM saved (or would save) the file under.
    pub file_name: Option<String>,
    /// The folder IDM saves it to. Exported lists have none.
    pub dir: Option<PathBuf>,
    /// Bytes, when IDM knew the size.
    pub size: Option<u64>,
    /// IDM finished it (every byte downloaded).
    pub finished: bool,
}

impl IdmEntry {
    fn new(url: String) -> Self {
        Self {
            url,
            file_name: None,
            dir: None,
            size: None,
            finished: false,
        }
    }

    /// The download Downpipe adds for it: paused, whether IDM finished it
    /// or not, so nothing starts until the user resumes it. A file IDM
    /// left in the folder is never replaced: a download that finds its
    /// name taken saves under a free one.
    pub fn to_download(&self) -> BackupDownload {
        BackupDownload {
            url: self.url.clone(),
            dir: self.dir.clone().unwrap_or_default(),
            file_name: self.file_name.clone(),
            state: TaskState::Paused,
            total: self.size,
            downloaded: 0,
            queue: MAIN_QUEUE.into(),
            category: None,
            auto_dir: self.dir.is_none(),
            checksum: None,
            media: None,
            site: None,
            created_at: 0,
            completed_at: None,
            speed_limit: None,
        }
    }
}

/// An http(s) address as IDM wrote it, or nothing.
fn web_url(text: &str) -> Option<String> {
    let text = text.trim();
    if text.len() > MAX_URL_LEN {
        return None;
    }
    let url = url::Url::parse(text).ok()?;
    matches!(url.scheme(), "http" | "https").then(|| url.to_string())
}

/// A bare file name, safe for this system, or nothing.
fn file_name(text: &str) -> Option<String> {
    let name = text.trim();
    if name.is_empty() || name.chars().count() > 255 || name.chars().any(char::is_control) {
        return None;
    }
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    (!base.is_empty()).then(|| filename::sanitize(base))
}

/// Text from an exported file: IDM writes UTF-16 with a byte order mark on
/// some versions, UTF-8 (with or without one) on others.
pub fn decode_text(bytes: &[u8]) -> String {
    let utf16 = |bytes: &[u8], le: bool| {
        let units: Vec<u16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&pair| {
                if le {
                    u16::from_le_bytes(pair)
                } else {
                    u16::from_be_bytes(pair)
                }
            })
            .collect();
        String::from_utf16_lossy(&units)
    };
    match bytes {
        [0xFF, 0xFE, rest @ ..] => utf16(rest, true),
        [0xFE, 0xFF, rest @ ..] => utf16(rest, false),
        [0xEF, 0xBB, 0xBF, rest @ ..] => String::from_utf8_lossy(rest).into_owned(),
        _ => String::from_utf8_lossy(bytes).into_owned(),
    }
}

/// Every download in an exported list (text or `.ef2`), each address once,
/// in the file's order. Lines that are not addresses are skipped.
pub fn parse_list(text: &str) -> Vec<IdmEntry> {
    let mut out: Vec<IdmEntry> = Vec::new();
    let mut block: Option<Option<IdmEntry>> = None;
    let push = |out: &mut Vec<IdmEntry>, entry: IdmEntry| {
        if out.len() < MAX_ENTRIES && !out.iter().any(|e| e.url == entry.url) {
            out.push(entry);
        }
    };
    for line in text.lines() {
        let line = line.trim();
        match (&mut block, line) {
            (None, "<") => block = Some(None),
            (Some(entry), ">") => {
                if let Some(entry) = entry.take() {
                    push(&mut out, entry);
                }
                block = None;
            }
            (Some(entry @ None), line) => {
                // The block's first line is the address; a block without
                // one is skipped as a whole.
                *entry = web_url(line).map(IdmEntry::new);
                if entry.is_none() {
                    block = Some(None);
                }
            }
            (Some(Some(entry)), line) => {
                // `name: value` lines. Only the file name is read.
                if let Some((key, value)) = line.split_once(':')
                    && key.trim().eq_ignore_ascii_case("filename")
                {
                    entry.file_name = file_name(value);
                }
            }
            (None, line) => {
                if let Some(url) = web_url(line) {
                    push(&mut out, IdmEntry::new(url));
                }
            }
        }
    }
    out
}

/// A registry value as far as the import cares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegValue {
    Text(String),
    Bytes(Vec<u8>),
    Number(u64),
}

impl RegValue {
    fn text(&self) -> Option<&str> {
        match self {
            Self::Text(t) => Some(t.trim_end_matches('\0')),
            _ => None,
        }
    }

    /// IDM keeps sizes as 8 bytes, little-endian; DWORDs and QWORDs are
    /// read too.
    fn count(&self) -> Option<u64> {
        match self {
            Self::Number(n) => Some(*n),
            Self::Bytes(b) if b.len() == 8 => Some(u64::from_le_bytes(b[..8].try_into().ok()?)),
            Self::Bytes(b) if b.len() == 4 => {
                Some(u64::from(u32::from_le_bytes(b[..4].try_into().ok()?)))
            }
            _ => None,
        }
        // IDM writes -1 for an unknown size.
        .filter(|n| *n > 0 && *n < i64::MAX as u64)
    }
}

/// IDM downloads into a working folder of its own
/// (`...\IDM\DwnlData\<user>\...`) and moves the file to its save folder
/// once it is complete; that save folder is not kept in a value it can be
/// read from. A path in the working folder is never a download's folder.
fn in_work_folder(path: &Path) -> bool {
    path.to_string_lossy()
        .to_ascii_lowercase()
        .replace('/', "\\")
        .contains(r"\idm\dwnldata\")
}

/// A file name as IDM recorded it, unless it is really the tail of a link
/// (`model.bin%22%3B&Expires=...`), which a broken server header leaves.
fn clean_name(text: &str) -> Option<String> {
    let name = file_name(text)?;
    (!name.contains(['%', '&', '=', '?']) && name.chars().count() <= 200).then_some(name)
}

/// The registry values of one IDM download that [`entry_from_values`] reads.
pub const VALUE_NAMES: [&str; 6] = [
    "Url0",
    "LocalFileName",
    "LocalPath",
    "FileName",
    "FileSize",
    "Downloaded",
];

/// One IDM download from its registry values, looked up by name. Nothing
/// without an http(s) address.
pub fn entry_from_values(get: impl Fn(&str) -> Option<RegValue>) -> Option<IdmEntry> {
    let url = web_url(get("Url0")?.text()?)?;
    let mut entry = IdmEntry::new(url);
    let path = |name: &str| {
        get(name)
            .and_then(|v| v.text().map(str::to_string))
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
    };
    let local = path("LocalFileName");
    entry.dir = path("LocalPath")
        .or_else(|| {
            local
                .as_deref()
                .and_then(Path::parent)
                .map(Path::to_path_buf)
        })
        .filter(|dir| !in_work_folder(dir));
    // The name IDM shows, unless it is the tail of a link; the working
    // file's name when that has an extension and the shown one does not.
    let shown = get("FileName").and_then(|v| v.text().and_then(clean_name));
    let working = local
        .as_deref()
        .and_then(Path::file_name)
        .and_then(|n| clean_name(&n.to_string_lossy()));
    let has_extension = |n: &String| crate::settings::extension_of(n).is_some();
    entry.file_name = [shown.clone(), working]
        .into_iter()
        .flatten()
        .find(has_extension)
        .or(shown);
    entry.size = get("FileSize").and_then(|v| v.count());
    let downloaded = get("Downloaded").and_then(|v| v.count());
    entry.finished = matches!((entry.size, downloaded), (Some(size), Some(done)) if done >= size);
    Some(entry)
}

/// IDM's own list on this computer, oldest first. Empty when IDM is not
/// installed (or has never kept a download).
#[cfg(windows)]
pub fn read_registry() -> std::io::Result<Vec<IdmEntry>> {
    registry::read()
}

#[cfg(not(windows))]
pub fn read_registry() -> std::io::Result<Vec<IdmEntry>> {
    Ok(Vec::new())
}

#[cfg(windows)]
mod registry {
    use std::io;

    use windows_sys::Win32::Foundation::{
        ERROR_FILE_NOT_FOUND, ERROR_NO_MORE_ITEMS, ERROR_SUCCESS,
    };
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_READ, REG_BINARY, REG_DWORD, REG_EXPAND_SZ, REG_NONE,
        REG_QWORD, REG_SZ, RRF_RT_ANY, RegCloseKey, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW,
    };

    use super::{IdmEntry, MAX_ENTRIES, RegValue, VALUE_NAMES, entry_from_values};

    const ROOT: &str = r"Software\DownloadManager";
    /// No value the import reads is longer; a bigger one is skipped.
    const MAX_VALUE_BYTES: u32 = 64 * 1024;

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(Some(0)).collect()
    }

    /// Closes the key when dropped.
    struct Key(HKEY);

    impl Drop for Key {
        fn drop(&mut self) {
            // SAFETY: the handle came from RegOpenKeyExW and is closed once.
            unsafe { RegCloseKey(self.0) };
        }
    }

    pub(super) fn read() -> io::Result<Vec<IdmEntry>> {
        let mut handle: HKEY = std::ptr::null_mut();
        let root = wide(ROOT);
        // KEY_READ: the import can look but never change anything.
        // SAFETY: valid null-terminated name and out pointer.
        let status =
            unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, root.as_ptr(), 0, KEY_READ, &mut handle) };
        if status == ERROR_FILE_NOT_FOUND {
            return Ok(Vec::new());
        }
        if status != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(status as i32));
        }
        let key = Key(handle);
        let mut ids: Vec<(u64, String)> = Vec::new();
        for index in 0.. {
            let mut name = [0u16; 256];
            let mut len = name.len() as u32;
            // SAFETY: the buffer and its length match; optional outputs are null.
            let status = unsafe {
                RegEnumKeyExW(
                    key.0,
                    index,
                    name.as_mut_ptr(),
                    &mut len,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            if status == ERROR_NO_MORE_ITEMS || ids.len() >= MAX_ENTRIES {
                break;
            }
            if status != ERROR_SUCCESS {
                continue;
            }
            let name = String::from_utf16_lossy(&name[..len as usize]);
            // Downloads are numbered; other keys hold IDM's settings.
            if let Ok(id) = name.parse::<u64>() {
                ids.push((id, name));
            }
        }
        ids.sort();
        Ok(ids
            .iter()
            .filter_map(|(_, name)| {
                let sub = wide(name);
                entry_from_values(|value| {
                    debug_assert!(VALUE_NAMES.contains(&value));
                    get(&key, &sub, value)
                })
            })
            .collect())
    }

    fn get(key: &Key, sub: &[u16], value: &str) -> Option<RegValue> {
        let value = wide(value);
        let mut kind = 0u32;
        let mut size = 0u32;
        // SAFETY: a size query: null data pointer, valid out pointers.
        let status = unsafe {
            RegGetValueW(
                key.0,
                sub.as_ptr(),
                value.as_ptr(),
                RRF_RT_ANY,
                &mut kind,
                std::ptr::null_mut(),
                &mut size,
            )
        };
        if status != ERROR_SUCCESS || size > MAX_VALUE_BYTES {
            return None;
        }
        let mut data = vec![0u8; size as usize + 2];
        let mut size = data.len() as u32;
        // SAFETY: `data` holds `size` bytes.
        let status = unsafe {
            RegGetValueW(
                key.0,
                sub.as_ptr(),
                value.as_ptr(),
                RRF_RT_ANY,
                &mut kind,
                data.as_mut_ptr().cast(),
                &mut size,
            )
        };
        if status != ERROR_SUCCESS {
            return None;
        }
        data.truncate(size as usize);
        match kind {
            REG_SZ | REG_EXPAND_SZ => {
                let units: Vec<u16> = data
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|&pair| u16::from_le_bytes(pair))
                    .collect();
                Some(RegValue::Text(String::from_utf16_lossy(&units)))
            }
            REG_DWORD if data.len() == 4 => Some(RegValue::Number(u64::from(u32::from_le_bytes(
                data[..4].try_into().ok()?,
            )))),
            REG_QWORD if data.len() == 8 => Some(RegValue::Number(u64::from_le_bytes(
                data[..8].try_into().ok()?,
            ))),
            REG_NONE | REG_BINARY => Some(RegValue::Bytes(data)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn text_exports_list_one_address_per_line() {
        let text = "https://example.org/a.zip\r\n\r\nnot a link\nftp://example.org/b.zip\nhttps://example.org/a.zip\n  http://example.net/c.iso  \n";
        let entries = parse_list(text);
        let urls: Vec<&str> = entries.iter().map(|e| e.url.as_str()).collect();
        assert_eq!(
            urls,
            ["https://example.org/a.zip", "http://example.net/c.iso"]
        );
        assert!(entries.iter().all(|e| e.dir.is_none() && !e.finished));
    }

    #[test]
    fn ef2_blocks_give_the_address_and_name_only() {
        let text = "<\r\nhttps://example.org/get?id=7\r\nreferer: https://example.org/page\r\nUser-Agent: Mozilla/5.0\r\ncookie: sid=secret\r\npostdata: user=me&pass=hunter2\r\nfilename: report:final.pdf\r\n>\r\n<\r\nnot a link\r\nfilename: x.zip\r\n>\r\n<\r\nhttps://example.org/b.zip\r\n>\r\n";
        let entries = parse_list(text);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].url, "https://example.org/get?id=7");
        assert_eq!(
            entries[0].file_name.as_deref(),
            Some(filename::sanitize("report:final.pdf").as_str())
        );
        assert_eq!(entries[1].url, "https://example.org/b.zip");
        let debug = format!("{entries:?}");
        assert!(
            !debug.contains("secret") && !debug.contains("hunter2") && !debug.contains("Mozilla")
        );
    }

    #[test]
    fn utf16_exports_are_decoded() {
        let mut bytes = vec![0xFF, 0xFE];
        for unit in "https://example.org/ü.zip\r\n".encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        let entries = parse_list(&decode_text(&bytes));
        assert_eq!(entries.len(), 1);
        assert!(entries[0].url.starts_with("https://example.org/"));
        assert_eq!(
            decode_text(b"\xEF\xBB\xBFhttps://a.org/x"),
            "https://a.org/x"
        );
    }

    fn values(pairs: &[(&str, RegValue)]) -> impl Fn(&str) -> Option<RegValue> {
        let map: HashMap<String, RegValue> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.clone()))
            .collect();
        move |name| map.get(name).cloned()
    }

    fn abs(path: &str) -> PathBuf {
        std::env::temp_dir().join(path)
    }

    #[test]
    fn registry_values_become_an_entry() {
        let file = abs("idm").join("setup (1).exe");
        let entry = entry_from_values(values(&[
            (
                "Url0",
                RegValue::Text("https://example.org/setup.exe\0".into()),
            ),
            (
                "LocalFileName",
                RegValue::Text(file.to_string_lossy().into_owned()),
            ),
            (
                "FileName",
                RegValue::Text("setup.exe%22%3B&Expires=1".into()),
            ),
            ("FileSize", RegValue::Bytes(1234u64.to_le_bytes().to_vec())),
            (
                "Downloaded",
                RegValue::Bytes(1234u64.to_le_bytes().to_vec()),
            ),
        ]))
        .unwrap();
        assert_eq!(entry.url, "https://example.org/setup.exe");
        assert_eq!(entry.dir.as_deref(), Some(abs("idm").as_path()));
        assert_eq!(
            entry.file_name.as_deref(),
            Some("setup (1).exe"),
            "the tail of a link is not a name"
        );
        assert_eq!(entry.size, Some(1234));
        assert!(entry.finished);

        // Unfinished, no saved path yet: folder and name from the other values.
        let entry = entry_from_values(values(&[
            ("Url0", RegValue::Text("http://example.org/big.iso".into())),
            (
                "LocalPath",
                RegValue::Text(abs("isos").to_string_lossy().into_owned()),
            ),
            ("FileName", RegValue::Text("big.iso".into())),
            ("FileSize", RegValue::Bytes(9000u64.to_le_bytes().to_vec())),
            ("Downloaded", RegValue::Bytes(100u64.to_le_bytes().to_vec())),
        ]))
        .unwrap();
        assert_eq!(entry.dir.as_deref(), Some(abs("isos").as_path()));
        assert_eq!(entry.file_name.as_deref(), Some("big.iso"));
        assert!(!entry.finished);

        // IDM's working folder is not where the file belongs; the name it
        // shows wins when it has an extension.
        let work = abs("IDM").join("DwnlData").join("me").join("big_17");
        let entry = entry_from_values(values(&[
            ("Url0", RegValue::Text("http://example.org/get?id=4".into())),
            (
                "LocalPath",
                RegValue::Text(work.to_string_lossy().into_owned()),
            ),
            (
                "LocalFileName",
                RegValue::Text(work.join("big.iso").to_string_lossy().into_owned()),
            ),
            ("FileName", RegValue::Text("Big Image.iso".into())),
        ]))
        .unwrap();
        assert_eq!(entry.dir, None);
        assert_eq!(entry.file_name.as_deref(), Some("Big Image.iso"));
        // Without an extension in the shown name, the working file's name.
        let entry = entry_from_values(values(&[
            ("Url0", RegValue::Text("http://example.org/get?id=4".into())),
            (
                "LocalFileName",
                RegValue::Text(work.join("big.iso").to_string_lossy().into_owned()),
            ),
            ("FileName", RegValue::Text("big".into())),
        ]))
        .unwrap();
        assert_eq!(
            (entry.dir, entry.file_name.as_deref()),
            (None, Some("big.iso"))
        );

        // Unknown size (-1) and relative paths are dropped.
        let entry = entry_from_values(values(&[
            ("Url0", RegValue::Text("https://example.org/x".into())),
            ("LocalPath", RegValue::Text(r"relative\dir".into())),
            ("FileSize", RegValue::Bytes((-1i64).to_le_bytes().to_vec())),
        ]))
        .unwrap();
        assert_eq!((entry.dir, entry.size, entry.finished), (None, None, false));
    }

    #[test]
    fn registry_entries_without_a_web_address_are_skipped() {
        assert_eq!(entry_from_values(values(&[])), None);
        assert_eq!(
            entry_from_values(values(&[("Url0", RegValue::Text("ftp://a.org/x".into()))])),
            None
        );
        assert_eq!(
            entry_from_values(values(&[("Url0", RegValue::Number(4))])),
            None
        );
    }

    #[test]
    fn only_the_documented_values_are_asked_for() {
        let asked = std::cell::RefCell::new(Vec::new());
        entry_from_values(|name| {
            asked.borrow_mut().push(name.to_string());
            match name {
                "Url0" => Some(RegValue::Text("https://example.org/a".into())),
                _ => None,
            }
        });
        for name in asked.borrow().iter() {
            assert!(VALUE_NAMES.contains(&name.as_str()), "{name}");
        }
    }

    #[test]
    fn entries_are_added_paused() {
        let entry = IdmEntry {
            url: "https://example.org/a.zip".into(),
            file_name: Some("a.zip".into()),
            dir: None,
            size: Some(5),
            finished: true,
        };
        let download = entry.to_download();
        assert_eq!(download.state, TaskState::Paused);
        assert!(download.auto_dir);
        assert_eq!(download.total, Some(5));
    }

    /// Reads this computer's IDM list, if it has one, without asserting on
    /// its contents: it must not fail or change anything.
    #[test]
    fn reading_the_registry_never_fails_without_idm() {
        assert!(read_registry().is_ok());
    }
}
