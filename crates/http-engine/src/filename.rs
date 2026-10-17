//! Turning untrusted server/URL data into a safe local file name.
//!
//! Server-supplied names are attacker-controlled: they can contain `../`,
//! drive letters, control characters or Windows device names. Everything that
//! reaches the file system goes through [`sanitize`].

use std::path::{Path, PathBuf};

use percent_encoding::percent_decode_str;
use url::Url;

const MAX_NAME_BYTES: usize = 200;
const FALLBACK_NAME: &str = "download";

/// Extract the file name from a `Content-Disposition` header value.
/// `filename*` (RFC 5987, UTF-8 capable) wins over plain `filename`.
pub fn from_content_disposition(value: &str) -> Option<String> {
    let mut plain = None;
    let mut extended = None;

    for param in split_params(value).into_iter().skip(1) {
        let Some((key, raw)) = param.split_once('=') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let raw = raw.trim();
        match key.as_str() {
            "filename*" => extended = decode_ext_value(raw),
            "filename" => plain = Some(unquote(raw)),
            _ => {}
        }
    }

    extended
        .or(plain)
        .map(|name| last_path_component(&name).to_string())
        .filter(|name| !name.trim().is_empty())
}

/// Best-effort name from the last path segment of a URL.
pub fn from_url(url: &Url) -> Option<String> {
    let segment = url.path_segments()?.rev().find(|s| !s.is_empty())?;
    let decoded = percent_decode_str(segment).decode_utf8_lossy().into_owned();
    let name = last_path_component(&decoded).to_string();
    (!name.trim().is_empty()).then_some(name)
}

/// Extension to append when the name has none but the MIME type is known.
pub fn extension_for_mime(mime: &str) -> Option<&'static str> {
    let essence = mime.split(';').next()?.trim().to_ascii_lowercase();
    Some(match essence.as_str() {
        "application/zip" => "zip",
        "application/pdf" => "pdf",
        "application/gzip" | "application/x-gzip" => "gz",
        "application/x-7z-compressed" => "7z",
        "application/x-tar" => "tar",
        "application/vnd.rar" | "application/x-rar-compressed" => "rar",
        "application/x-msdownload" | "application/vnd.microsoft.portable-executable" => "exe",
        "application/x-msi" => "msi",
        "application/x-iso9660-image" => "iso",
        "application/json" => "json",
        "text/plain" => "txt",
        "text/csv" => "csv",
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "audio/mpeg" => "mp3",
        "audio/ogg" => "ogg",
        "audio/flac" => "flac",
        "video/mp4" => "mp4",
        "video/webm" => "webm",
        "video/x-matroska" => "mkv",
        _ => return None,
    })
}

/// Make a name safe on every supported OS. Never returns an empty string.
pub fn sanitize(name: &str) -> String {
    let name = last_path_component(name);
    let mut out: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();

    // Windows silently strips trailing dots/spaces, which can make two
    // different names collide or hide an extension.
    let trimmed_len = out.trim_end_matches(['.', ' ']).len();
    out.truncate(trimmed_len);
    let out = out.trim_start().to_string();
    let mut out = if out.is_empty() || out.chars().all(|c| c == '.') {
        FALLBACK_NAME.to_string()
    } else {
        out
    };

    if is_reserved_windows_name(&out) {
        out.insert(0, '_');
    }
    truncate_preserving_extension(&out, MAX_NAME_BYTES)
}

/// `name.ext`, `name (1).ext`, `name (2).ext`, ... — first one where neither
/// the final file nor its `.part` exists yet.
pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let (stem, ext) = split_extension(name);
    let candidate = |n: u32| {
        if n == 0 {
            name.to_string()
        } else {
            match ext {
                Some(ext) => format!("{stem} ({n}).{ext}"),
                None => format!("{stem} ({n})"),
            }
        }
    };
    for n in 0..10_000 {
        let path = dir.join(candidate(n));
        if !path.exists() && !crate::meta::part_path(&path).exists() {
            return path;
        }
    }
    dir.join(format!("{stem}-{}", fastrand::u64(..)))
}

fn is_reserved_windows_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or(name).trim_end();
    let upper = stem.to_ascii_uppercase();
    matches!(
        upper.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ((upper.starts_with("COM") || upper.starts_with("LPT"))
        && upper.len() == 4
        && upper.as_bytes()[3].is_ascii_digit()
        && upper.as_bytes()[3] != b'0')
}

fn truncate_preserving_extension(name: &str, max_bytes: usize) -> String {
    if name.len() <= max_bytes {
        return name.to_string();
    }
    let (stem, ext) = split_extension(name);
    let ext_len = ext.map_or(0, |e| e.len() + 1);
    let mut budget = max_bytes.saturating_sub(ext_len);
    while !stem.is_char_boundary(budget.min(stem.len())) {
        budget -= 1;
    }
    let stem = &stem[..budget.min(stem.len())];
    match ext {
        Some(ext) if ext.len() < 16 => format!("{stem}.{ext}"),
        _ => stem.to_string(),
    }
}

fn split_extension(name: &str) -> (&str, Option<&str>) {
    match name.rfind('.') {
        Some(0) | None => (name, None),
        Some(i) => (&name[..i], Some(&name[i + 1..])),
    }
}

fn last_path_component(name: &str) -> &str {
    name.rsplit(['/', '\\']).next().unwrap_or(name)
}

fn unquote(raw: &str) -> String {
    let Some(inner) = raw.strip_prefix('"').and_then(|r| r.strip_suffix('"')) else {
        return raw.to_string();
    };
    // Only `\"` and `\\` are treated as escapes. Servers routinely send raw
    // Windows paths (`C:\dir\file`); keeping those backslashes lets the
    // directory part be stripped instead of glued onto the name.
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, chars.peek()) {
            ('\\', Some(&next)) if next == '"' || next == '\\' => {
                out.push(next);
                chars.next();
            }
            _ => out.push(c),
        }
    }
    out
}

/// RFC 5987 `ext-value`: `charset'lang'pct-encoded`.
fn decode_ext_value(raw: &str) -> Option<String> {
    let raw = unquote(raw);
    let mut parts = raw.splitn(3, '\'');
    let charset = parts.next()?.to_ascii_lowercase();
    let _lang = parts.next()?;
    let encoded = parts.next()?;
    let bytes: Vec<u8> = percent_decode_str(encoded).collect();
    match charset.as_str() {
        "utf-8" => String::from_utf8(bytes).ok(),
        "iso-8859-1" => Some(bytes.iter().map(|&b| b as char).collect()),
        _ => None,
    }
}

/// Split on `;` while respecting quoted strings.
fn split_params(value: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut in_quotes = false;
    let mut escaped = false;
    for (i, c) in value.char_indices() {
        match c {
            _ if escaped => escaped = false,
            '\\' if in_quotes => escaped = true,
            '"' => in_quotes = !in_quotes,
            ';' if !in_quotes => {
                parts.push(&value[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&value[start..]);
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_disposition_plain_and_quoted() {
        assert_eq!(
            from_content_disposition("attachment; filename=report.pdf").as_deref(),
            Some("report.pdf")
        );
        assert_eq!(
            from_content_disposition(r#"attachment; filename="a \"b\"; c.zip""#).as_deref(),
            Some(r#"a "b"; c.zip"#)
        );
    }

    #[test]
    fn content_disposition_prefers_extended() {
        let header =
            r#"attachment; filename="fallback.txt"; filename*=UTF-8''%C3%B6zet%20rapor.txt"#;
        assert_eq!(
            from_content_disposition(header).as_deref(),
            Some("özet rapor.txt")
        );
    }

    #[test]
    fn content_disposition_strips_directories() {
        assert_eq!(
            from_content_disposition(r#"attachment; filename="../../etc/passwd""#).as_deref(),
            Some("passwd")
        );
        assert_eq!(
            from_content_disposition(r#"attachment; filename="C:\Windows\evil.exe""#).as_deref(),
            Some("evil.exe")
        );
        assert_eq!(from_content_disposition("inline"), None);
    }

    #[test]
    fn url_names() {
        let url = Url::parse("https://example.org/pub/ubuntu%2024.04.iso?x=1").unwrap();
        assert_eq!(from_url(&url).as_deref(), Some("ubuntu 24.04.iso"));
        let url = Url::parse("https://example.org/dir/").unwrap();
        assert_eq!(from_url(&url).as_deref(), Some("dir"));
        let url = Url::parse("https://example.org/").unwrap();
        assert_eq!(from_url(&url), None);
    }

    #[test]
    fn sanitizes_hostile_names() {
        assert_eq!(sanitize("a<b>c:d|e?f*.txt"), "a_b_c_d_e_f_.txt");
        assert_eq!(sanitize("..\\..\\boot.ini"), "boot.ini");
        assert_eq!(sanitize("CON"), "_CON");
        assert_eq!(sanitize("com1.txt"), "_com1.txt");
        assert_eq!(sanitize("com0.txt"), "com0.txt");
        assert_eq!(sanitize("console.txt"), "console.txt");
        assert_eq!(sanitize("name. . "), "name");
        assert_eq!(sanitize(".."), "download");
        assert_eq!(sanitize(""), "download");
        assert_eq!(sanitize("tab\there"), "tab_here");
    }

    #[test]
    fn truncates_long_names_keeping_extension() {
        let long = format!("{}.mp4", "ş".repeat(300));
        let out = sanitize(&long);
        assert!(out.len() <= MAX_NAME_BYTES);
        assert!(out.ends_with(".mp4"));
    }

    #[test]
    fn unique_path_skips_existing_and_partial_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("file.zip"), b"x").unwrap();
        std::fs::write(dir.path().join("file (1).zip.part"), b"x").unwrap();
        assert_eq!(
            unique_path(dir.path(), "file.zip"),
            dir.path().join("file (2).zip")
        );
        assert_eq!(unique_path(dir.path(), "new"), dir.path().join("new"));
    }

    #[test]
    fn mime_extensions() {
        assert_eq!(
            extension_for_mime("application/pdf; charset=binary"),
            Some("pdf")
        );
        assert_eq!(extension_for_mime("application/octet-stream"), None);
    }
}
