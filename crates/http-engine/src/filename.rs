//! Turning untrusted server/URL data into a safe local file name.
//!
//! Server-supplied names are attacker-controlled: they can contain `../`,
//! drive letters, control characters, invisible direction marks or Windows
//! device names. Everything that reaches the file system goes through
//! [`sanitize`].

use std::path::{Path, PathBuf};

use percent_encoding::percent_decode_str;
use url::Url;

const MAX_NAME_BYTES: usize = 200;
const FALLBACK_NAME: &str = "download";

/// Extract the file name from a `Content-Disposition` header value.
///
/// Follows RFC 6266 with the leniency browsers show real servers:
/// `filename*` (RFC 5987, any order, continuations `filename*0*=`) wins over
/// `filename`; a plain `filename` may be raw UTF-8, percent-encoded UTF-8 or
/// an RFC 2047 encoded word; the disposition type may be missing and a
/// closing quote may be forgotten.
pub fn from_content_disposition(value: &str) -> Option<String> {
    let mut plain = None;
    let mut extended = None;
    // filename*0, filename*1*, ...: (index, encoded, raw value).
    let mut pieces: Vec<(u32, bool, String)> = Vec::new();

    for (i, param) in split_params(value).into_iter().enumerate() {
        let Some((key, raw)) = param.split_once('=') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        // The first part is the disposition type, unless a server left the
        // type out and started with the parameter.
        if i == 0 && key != "filename" && key != "filename*" {
            continue;
        }
        let raw = raw.trim();
        match key.as_str() {
            "filename*" => {
                if let Some(v) = decode_ext_value(raw) {
                    extended = Some(v);
                }
            }
            "filename" => plain = Some(decode_plain(&unquote(raw))),
            k => {
                if let Some(rest) = k.strip_prefix("filename*") {
                    let (index, encoded) = match rest.strip_suffix('*') {
                        Some(n) => (n, true),
                        None => (rest, false),
                    };
                    if let Ok(index) = index.parse::<u32>() {
                        pieces.push((index, encoded, raw.to_string()));
                    }
                }
            }
        }
    }

    extended
        .or_else(|| join_continuations(pieces))
        .or(plain)
        .map(|name| last_path_component(&name).to_string())
        .filter(|name| !name.trim().is_empty())
}

/// Best-effort name from the last path segment of a URL.
pub fn from_url(url: &Url) -> Option<String> {
    let segment = url.path_segments()?.rev().find(|s| !s.is_empty())?;
    // A segment that is not UTF-8 once decoded (GBK, Shift-JIS links) keeps
    // its percent signs: readable enough, and better than U+FFFD marks.
    let decoded = match percent_decode_str(segment).decode_utf8() {
        Ok(s) => s.into_owned(),
        Err(_) => segment.to_string(),
    };
    let name = last_path_component(&decoded).to_string();
    (!name.trim().is_empty()).then_some(name)
}

/// The name to save a response under, sanitized: `Content-Disposition`
/// first, then the URLs. A redirect often ends at a storage URL named by a
/// hash (`/blob/9f8a7b`), while the link the user had says `setup.exe`; so
/// the URL whose name has an extension wins, the final one on a tie. An
/// extension is added from the MIME type when the name has none.
pub fn pick(
    content_disposition: Option<&str>,
    request_url: &Url,
    final_url: &Url,
    content_type: Option<&str>,
) -> String {
    let from_urls = || {
        let last = from_url(final_url);
        let first = from_url(request_url);
        match (last, first) {
            (Some(l), _) if has_extension(&l) => Some(l),
            (_, Some(f)) if has_extension(&f) => Some(f),
            (l, f) => l.or(f),
        }
    };
    let mut name = content_disposition
        .and_then(from_content_disposition)
        .or_else(from_urls)
        .unwrap_or_else(|| FALLBACK_NAME.to_string());
    if !has_extension(&name)
        && let Some(ext) = content_type.and_then(extension_for_mime)
    {
        name = format!("{name}.{ext}");
    }
    sanitize(&name)
}

/// The name ends in something that reads as a file type: `.zip`, `.tar.gz`,
/// not `v1.2` or `report.` (a version number is not a type).
pub fn has_extension(name: &str) -> bool {
    match split_extension(name) {
        (_, Some(ext)) => {
            !ext.is_empty()
                && ext.len() <= 10
                && ext.chars().all(|c| c.is_alphanumeric())
                && !ext.chars().all(|c| c.is_ascii_digit())
        }
        _ => false,
    }
}

/// The name says the file is not a web page (an archive, a program, media,
/// a document). A web page arriving under such a name is a login or
/// "link expired" page, not the file.
pub fn is_binary_name(name: &str) -> bool {
    const BINARY: &[&str] = &[
        "7z", "aac", "apk", "appimage", "avi", "bin", "bz2", "cab", "deb", "dmg", "doc", "docx",
        "epub", "exe", "flac", "gz", "img", "iso", "jar", "jpeg", "jpg", "m4a", "m4v", "mkv",
        "mov", "mp3", "mp4", "msi", "msix", "odt", "ogg", "opus", "pdf", "pkg", "png", "ppt",
        "pptx", "rar", "rpm", "tar", "tgz", "torrent", "ts", "wav", "webm", "webp", "xls", "xlsx",
        "xz", "zip", "zst",
    ];
    match split_extension(name) {
        (_, Some(ext)) => BINARY.contains(&ext.to_ascii_lowercase().as_str()),
        _ => false,
    }
}

/// Extension to append when the name has none but the MIME type is known.
pub fn extension_for_mime(mime: &str) -> Option<&'static str> {
    let essence = mime.split(';').next()?.trim().to_ascii_lowercase();
    Some(match essence.as_str() {
        "application/zip" | "application/x-zip-compressed" => "zip",
        "application/pdf" => "pdf",
        "application/gzip" | "application/x-gzip" => "gz",
        "application/x-7z-compressed" => "7z",
        "application/x-tar" => "tar",
        "application/x-xz" => "xz",
        "application/x-bzip2" => "bz2",
        "application/vnd.rar" | "application/x-rar-compressed" => "rar",
        "application/x-msdownload" | "application/vnd.microsoft.portable-executable" => "exe",
        "application/x-msi" => "msi",
        "application/x-iso9660-image" => "iso",
        "application/x-apple-diskimage" => "dmg",
        "application/vnd.android.package-archive" => "apk",
        "application/x-debian-package" | "application/vnd.debian.binary-package" => "deb",
        "application/x-bittorrent" => "torrent",
        "application/epub+zip" => "epub",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => "docx",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => "xlsx",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation" => "pptx",
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
        "audio/wav" | "audio/x-wav" => "wav",
        "video/mp4" => "mp4",
        "video/webm" => "webm",
        "video/quicktime" => "mov",
        "video/x-matroska" => "mkv",
        _ => return None,
    })
}

/// Make a name safe on every supported OS. Never returns an empty string.
pub fn sanitize(name: &str) -> String {
    let name = last_path_component(name);
    let out: String = name
        .chars()
        .filter(|&c| !is_invisible_mark(c))
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();

    let mut out = finish(&out);
    if is_reserved_windows_name(&out) {
        out.insert(0, '_');
    }
    finish(&truncate_preserving_extension(&out, MAX_NAME_BYTES))
}

/// Windows silently strips trailing dots and spaces, which can make two
/// different names collide or hide an extension; leading spaces are
/// invisible in every file manager.
fn finish(name: &str) -> String {
    let trimmed = name.trim_end_matches(['.', ' ']).trim_start_matches(' ');
    if trimmed.is_empty() || trimmed.chars().all(|c| c == '.') {
        FALLBACK_NAME.to_string()
    } else {
        trimmed.to_string()
    }
}

/// Zero-width and direction marks. U+202E turns `exe.pdf` into what reads
/// as `fdp.exe`, the classic way to disguise a program; none of these has a
/// place in a file name. Joiners (U+200C, U+200D) stay: Persian and Indic
/// scripts and emoji need them.
fn is_invisible_mark(c: char) -> bool {
    matches!(
        c,
        '\u{061C}'
            | '\u{200B}'
            | '\u{200E}'
            | '\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'
            | '\u{2066}'..='\u{2069}'
            | '\u{FEFF}'
    )
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
    let upper = stem.to_uppercase();
    if matches!(
        upper.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) {
        return true;
    }
    // COM1-9 and LPT1-9, and since Windows treats the superscript digits as
    // digits there, COM¹ to LPT³ as well.
    let mut chars = upper.chars();
    let prefix: String = chars.by_ref().take(3).collect();
    let digit: Vec<char> = chars.collect();
    (prefix == "COM" || prefix == "LPT")
        && matches!(digit.as_slice(), ['1'..='9' | '¹' | '²' | '³'])
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
    let inner = match raw.strip_prefix('"') {
        // A forgotten closing quote must not end up in the name.
        Some(rest) => rest.strip_suffix('"').unwrap_or(rest),
        None => return raw.to_string(),
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

/// A plain `filename` value, which RFC 6266 says is ISO-8859-1 but servers
/// fill however they like. Two encodings are common enough to undo: RFC
/// 2047 words (`=?UTF-8?B?...?=`, from mail libraries) and percent-encoded
/// UTF-8 (what browsers decode too). Percent signs that do not decode to
/// non-ASCII UTF-8 are kept as typed: `100%.txt`, `foo-%41.html`.
fn decode_plain(value: &str) -> String {
    if let Some(decoded) = decode_rfc2047(value) {
        return decoded;
    }
    if value.contains('%') {
        let bytes: Vec<u8> = percent_decode_str(value).collect();
        if !bytes.is_ascii()
            && let Ok(s) = String::from_utf8(bytes)
        {
            return s;
        }
    }
    value.to_string()
}

/// RFC 5987 `ext-value`: `charset'lang'pct-encoded`.
fn decode_ext_value(raw: &str) -> Option<String> {
    // Quotes are not allowed here, but some servers add them anyway.
    let raw = unquote(raw);
    let mut parts = raw.splitn(3, '\'');
    let charset = parts.next()?;
    let _lang = parts.next()?;
    let encoded = parts.next()?;
    decode_charset(charset, &percent_decode_str(encoded).collect::<Vec<u8>>())
}

fn decode_charset(charset: &str, bytes: &[u8]) -> Option<String> {
    match charset.trim().to_ascii_lowercase().as_str() {
        "utf-8" | "utf8" | "us-ascii" | "" => String::from_utf8(bytes.to_vec()).ok(),
        // Windows-1252 differs from Latin-1 only in 0x80-0x9F, which are
        // control characters in Latin-1 and get replaced by sanitize anyway.
        "iso-8859-1" | "latin1" | "windows-1252" => {
            Some(bytes.iter().map(|&b| b as char).collect())
        }
        _ => None,
    }
}

/// RFC 2231 continuations: `filename*0*=UTF-8''a%20; filename*1="b.txt"`.
/// The charset comes with the first piece; pieces marked `*` are
/// percent-encoded, the others are plain (possibly quoted) text.
fn join_continuations(mut pieces: Vec<(u32, bool, String)>) -> Option<String> {
    if pieces.is_empty() {
        return None;
    }
    pieces.sort_by_key(|p| p.0);
    if pieces.iter().enumerate().any(|(i, p)| p.0 != i as u32) {
        return None;
    }
    let mut charset = String::from("utf-8");
    let mut bytes = Vec::new();
    for (index, encoded, raw) in pieces {
        if encoded {
            let mut value = raw.as_str();
            if index == 0 {
                let mut parts = value.splitn(3, '\'');
                let (Some(cs), Some(_lang), Some(rest)) =
                    (parts.next(), parts.next(), parts.next())
                else {
                    return None;
                };
                charset = cs.to_string();
                value = rest;
            }
            bytes.extend(percent_decode_str(value));
        } else {
            bytes.extend_from_slice(unquote(&raw).as_bytes());
        }
    }
    decode_charset(&charset, &bytes)
}

/// One or more RFC 2047 encoded words making up the whole value.
fn decode_rfc2047(value: &str) -> Option<String> {
    let value = value.trim();
    if !value.starts_with("=?") || !value.ends_with("?=") {
        return None;
    }
    let mut out = String::new();
    // Whitespace between adjacent encoded words is not part of the text.
    for word in value.split_whitespace() {
        let inner = word.strip_prefix("=?")?.strip_suffix("?=")?;
        let mut parts = inner.splitn(3, '?');
        let (charset, encoding, text) = (parts.next()?, parts.next()?, parts.next()?);
        let bytes = match encoding.to_ascii_uppercase().as_str() {
            "B" => base64(text)?,
            "Q" => quoted_printable(text)?,
            _ => return None,
        };
        out.push_str(&decode_charset(charset, &bytes)?);
    }
    Some(out)
}

fn base64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let mut acc = 0u32;
    let mut bits = 0;
    for c in text.bytes().filter(|&c| c != b'=') {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

fn quoted_printable(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len());
    let mut bytes = text.bytes();
    while let Some(b) = bytes.next() {
        match b {
            b'_' => out.push(b' '),
            b'=' => {
                let hex = [bytes.next()?, bytes.next()?];
                let hex = std::str::from_utf8(&hex).ok()?;
                out.push(u8::from_str_radix(hex, 16).ok()?);
            }
            b => out.push(b),
        }
    }
    Some(out)
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

    /// Real-world and RFC 6266 test-suite headers (greenbytes.de/tech/tc2231
    /// and what browsers accept) with the name each should give.
    const HEADERS: &[(&str, Option<&str>)] = &[
        ("attachment; filename=report.pdf", Some("report.pdf")),
        (r#"attachment; filename="foo.html""#, Some("foo.html")),
        (
            r#"attachment; filename="a \"b\"; c.zip""#,
            Some(r#"a "b"; c.zip"#),
        ),
        (r#"attachment; filename="a;b.txt""#, Some("a;b.txt")),
        (r#"attachment; FILENAME="foo.html""#, Some("foo.html")),
        (r#"attachment; filename = "foo.html""#, Some("foo.html")),
        ("attachment; filename=foo bar.html", Some("foo bar.html")),
        ("filename=foo.html", Some("foo.html")),
        (r#"attachment; filename="foo.html"#, Some("foo.html")),
        // Raw UTF-8, as many servers send it (read from the header's bytes).
        (r#"attachment; filename="foo-ä.html""#, Some("foo-ä.html")),
        // Percent signs that are not encoded UTF-8 stay.
        (
            r#"attachment; filename="foo-%41.html""#,
            Some("foo-%41.html"),
        ),
        (r#"attachment; filename="100%.txt""#, Some("100%.txt")),
        (
            r#"attachment; filename="foo-%c3%a4-%e2%82%ac.html""#,
            Some("foo-ä-€.html"),
        ),
        (
            "attachment; filename*=UTF-8''foo-%c3%a4-%e2%82%ac.html",
            Some("foo-ä-€.html"),
        ),
        (
            "attachment; filename*=iso-8859-1''foo-%E4.html",
            Some("foo-ä.html"),
        ),
        (
            "attachment; filename*=UTF-8''foo-a%cc%88.html",
            Some("foo-a\u{308}.html"),
        ),
        (
            r#"attachment; filename*="UTF-8''foo-%c3%a4.html""#,
            Some("foo-ä.html"),
        ),
        (
            "attachment; filename*=UTF-8'en'%e2%82%ac%20rates",
            Some("€ rates"),
        ),
        ("attachment; filename*=UTF8''x%C3%A9.txt", Some("xé.txt")),
        (
            "attachment;filename*=utf-8''%e6%97%a5%e6%9c%ac.pdf",
            Some("日本.pdf"),
        ),
        (
            "attachment; filename*=UTF-8''%E4%B8%AD%E6%96%87.zip",
            Some("中文.zip"),
        ),
        (
            "attachment; filename*=UTF-8''%D0%BE%D1%82%D1%87%D0%B5%D1%82.docx",
            Some("отчет.docx"),
        ),
        (
            "attachment; filename*=UTF-8''%D8%AA%D9%82%D8%B1%D9%8A%D8%B1.pdf",
            Some("تقرير.pdf"),
        ),
        (
            r#"attachment; filename="foo.html"; filename*=UTF-8''foo-%c3%a4.html"#,
            Some("foo-ä.html"),
        ),
        (
            r#"attachment; filename*=UTF-8''foo-%c3%a4.html; filename="foo.html""#,
            Some("foo-ä.html"),
        ),
        // Broken extended values fall back to the plain name.
        (
            r#"attachment; filename*=UTF-8''%ff%fe.txt; filename="ok.txt""#,
            Some("ok.txt"),
        ),
        (
            r#"attachment; filename*=foo''bar.txt; filename="ok.txt""#,
            Some("ok.txt"),
        ),
        (
            r#"attachment; filename*=nonsense; filename="ok.txt""#,
            Some("ok.txt"),
        ),
        (
            r#"attachment; filename*0="foo."; filename*1="html""#,
            Some("foo.html"),
        ),
        (
            r#"attachment; filename*0*=UTF-8''foo-%c3%a4; filename*1=".html""#,
            Some("foo-ä.html"),
        ),
        (
            r#"attachment; filename*1="html"; filename*0="foo.""#,
            Some("foo.html"),
        ),
        (
            r#"attachment; filename="=?UTF-8?B?w7Z6ZXQucGRm?=""#,
            Some("özet.pdf"),
        ),
        (
            r#"attachment; filename="=?utf-8?Q?=C3=B6zet_rapor.pdf?=""#,
            Some("özet rapor.pdf"),
        ),
        (r#"attachment; filename="../../etc/passwd""#, Some("passwd")),
        (
            r#"attachment; filename="C:\Windows\evil.exe""#,
            Some("evil.exe"),
        ),
        ("inline", None),
        ("attachment", None),
        (r#"attachment; filename="""#, None),
        (r#"attachment; filename="   ""#, None),
        (r#"attachment; filename="dir/""#, None),
    ];

    #[test]
    fn content_disposition_table() {
        for (header, want) in HEADERS {
            assert_eq!(
                from_content_disposition(header).as_deref(),
                *want,
                "header: {header}"
            );
        }
    }

    /// Every name survives a trip through `filename*`, whatever its script.
    #[test]
    fn extended_names_round_trip() {
        let mut rng = fastrand::Rng::with_seed(5987);
        let pool: Vec<char> = "abcXYZ019 ._-()[]+,;=&'%#@!~äöüßçğışéèñøåæœ中文日本語한국어\
                               отчётتقريرहिन्दीไทย€£¥😀\u{200d}"
            .chars()
            .collect();
        for _ in 0..2000 {
            let len = rng.usize(1..24);
            let name: String = (0..len).map(|_| pool[rng.usize(..pool.len())]).collect();
            if name.trim().is_empty() {
                continue;
            }
            let encoded: String = name
                .bytes()
                .map(|b| {
                    if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                        (b as char).to_string()
                    } else {
                        format!("%{b:02X}")
                    }
                })
                .collect();
            let header =
                format!("attachment; filename=\"fallback.bin\"; filename*=UTF-8''{encoded}");
            assert_eq!(
                from_content_disposition(&header).as_deref(),
                Some(name.as_str())
            );
        }
    }

    #[test]
    fn url_names() {
        let url = Url::parse("https://example.org/pub/ubuntu%2024.04.iso?x=1").unwrap();
        assert_eq!(from_url(&url).as_deref(), Some("ubuntu 24.04.iso"));
        let url = Url::parse("https://example.org/dir/").unwrap();
        assert_eq!(from_url(&url).as_deref(), Some("dir"));
        let url = Url::parse("https://example.org/").unwrap();
        assert_eq!(from_url(&url), None);
        let url = Url::parse("https://example.org/%E6%97%A5%E6%9C%AC.zip").unwrap();
        assert_eq!(from_url(&url).as_deref(), Some("日本.zip"));
        // GBK, not UTF-8: kept encoded rather than turned into U+FFFD.
        let url = Url::parse("https://example.org/%C4%E3%BA%C3.txt").unwrap();
        assert_eq!(from_url(&url).as_deref(), Some("%C4%E3%BA%C3.txt"));
    }

    #[test]
    fn picking_a_name_across_redirects() {
        let u = |s: &str| Url::parse(s).unwrap();
        let link = u("https://example.org/dl/setup.exe?token=1");
        let blob = u("https://cdn.example.net/blob/9f8a7b6c5d");
        assert_eq!(pick(None, &link, &blob, None), "setup.exe");
        let named = u("https://cdn.example.net/files/setup-1.2.exe");
        assert_eq!(pick(None, &link, &named, None), "setup-1.2.exe");
        assert_eq!(
            pick(Some("attachment; filename=real.msi"), &link, &named, None),
            "real.msi"
        );
        let bare = u("https://example.org/get");
        assert_eq!(pick(None, &bare, &bare, Some("application/zip")), "get.zip");
        assert_eq!(
            pick(
                None,
                &u("https://example.org/"),
                &u("https://example.org/"),
                None
            ),
            "download"
        );
        let version = u("https://example.org/release/v1.2");
        assert_eq!(
            pick(None, &version, &version, Some("application/zip")),
            "v1.2.zip"
        );
        let page = u("https://example.org/notes.txt");
        assert_eq!(pick(None, &page, &page, Some("text/plain")), "notes.txt");
    }

    #[test]
    fn extensions() {
        assert!(has_extension("a.zip"));
        assert!(has_extension("a.tar.gz"));
        assert!(has_extension("файл.docx"));
        assert!(!has_extension("v1.2"));
        assert!(!has_extension("README"));
        assert!(!has_extension(".bashrc"));
        assert!(!has_extension("name.with space"));
        assert!(is_binary_name("Setup.EXE"));
        assert!(is_binary_name("movie.mkv"));
        assert!(!is_binary_name("index.html"));
        assert!(!is_binary_name("download"));
        assert!(!is_binary_name("notes.txt"));
    }

    #[test]
    fn sanitizes_hostile_names() {
        let cases = [
            ("a<b>c:d|e?f*.txt", "a_b_c_d_e_f_.txt"),
            ("..\\..\\boot.ini", "boot.ini"),
            ("CON", "_CON"),
            ("con.tar.gz", "_con.tar.gz"),
            ("com1.txt", "_com1.txt"),
            ("COM¹.txt", "_COM¹.txt"),
            ("lpt³", "_lpt³"),
            ("LPT9", "_LPT9"),
            ("NUL .txt", "_NUL .txt"),
            ("CON. . ", "_CON"),
            ("com0.txt", "com0.txt"),
            ("com10.txt", "com10.txt"),
            ("console.txt", "console.txt"),
            ("name. . ", "name"),
            ("  spaced  .txt  ", "spaced  .txt"),
            ("..", "download"),
            ("", "download"),
            ("tab\there", "tab_here"),
            ("evil\u{202E}gpj.exe", "evilgpj.exe"),
            ("zero\u{200B}width", "zerowidth"),
            ("\u{FEFF}bom.txt", "bom.txt"),
            ("\u{202E}\u{202E}", "download"),
            // Joiners are part of scripts and emoji.
            ("می\u{200C}خواهم.pdf", "می\u{200C}خواهم.pdf"),
            ("名前:テスト?.mp4", "名前_テスト_.mp4"),
            ("отчёт 2024.docx", "отчёт 2024.docx"),
        ];
        for (input, want) in cases {
            assert_eq!(sanitize(input), want, "input: {input:?}");
        }
    }

    #[test]
    fn truncates_long_names_keeping_extension() {
        let long = format!("{}.mp4", "ş".repeat(300));
        let out = sanitize(&long);
        assert!(out.len() <= MAX_NAME_BYTES);
        assert!(out.ends_with(".mp4"));
        // A cut that lands on a dot or space must not leave it at the end.
        let long = format!("{}. {}", "a".repeat(198), "b".repeat(100));
        let out = sanitize(&long);
        assert!(!out.ends_with(['.', ' ']), "{out:?}");
    }

    /// Whatever goes in, what comes out is a usable name on Windows, macOS
    /// and Linux, and sanitizing it again changes nothing.
    #[test]
    fn sanitize_properties_hold_for_random_input() {
        let mut rng = fastrand::Rng::with_seed(6266);
        let pool: Vec<char> = "aZ09 ._-~<>:\"/\\|?*\t\n\r\0\u{7f}\u{85}\u{202E}\u{200B}\u{FEFF}\
                               \u{2066}CONPRNAUXNULLPTCOM¹²³äö中文😀"
            .chars()
            .collect();
        for _ in 0..5000 {
            let len = rng.usize(0..40);
            let mut input: String = (0..len).map(|_| pool[rng.usize(..pool.len())]).collect();
            if rng.u8(..8) == 0 {
                input = input.repeat(rng.usize(5..30));
            }
            let out = sanitize(&input);
            assert!(!out.is_empty(), "{input:?}");
            assert!(out.len() <= MAX_NAME_BYTES, "{input:?}");
            assert!(
                !out.chars()
                    .any(|c| "<>:\"/\\|?*".contains(c) || c.is_control() || is_invisible_mark(c)),
                "{input:?} -> {out:?}"
            );
            assert!(
                !out.ends_with(['.', ' ']) && !out.starts_with(' '),
                "{input:?} -> {out:?}"
            );
            assert!(out != "." && out != "..");
            assert!(!is_reserved_windows_name(&out), "{input:?} -> {out:?}");
            assert_eq!(sanitize(&out), out, "not idempotent for {input:?}");
        }
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
