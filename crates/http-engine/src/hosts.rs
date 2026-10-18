//! File hosts whose share links lead to a page rather than the file.
//!
//! Only hosts whose public files anyone can fetch without an account are
//! handled, and only through the steps a browser user takes without signing
//! in. Hosts that need an account, a premium plan, a captcha or client-side
//! decryption (Mega, Rapidgator and the like) are not, and will not be: see
//! docs/DOWNLOAD_ENGINE.md.
//!
//! * **Google Drive.** Share links (`/file/d/<id>/view`, `open?id=`,
//!   `uc?id=`) become the direct download address. For files Drive cannot
//!   scan for viruses (larger than about 100 MB) it answers with a "can't
//!   scan this file" page whose form carries a confirmation token; the form
//!   is followed, which is what pressing "Download anyway" does. A file
//!   that is not shared publicly ends in a sign-in or "request access"
//!   page, reported as [`DownloadError::PrivateFile`]; Drive's daily
//!   download quota as [`DownloadError::Unavailable`].
//!
//! The download keeps the link the user gave as its identity (the resume
//! data is filed under it), and the host is asked again on every start,
//! because confirmation tokens expire.

use reqwest::header::{CONTENT_TYPE, HeaderMap, RANGE};
use reqwest::{Client, StatusCode};
use url::Url;

use crate::error::{DownloadError, status_error};

/// Pages the confirmation token is looked for in are read up to this size.
const PAGE_CAP: usize = 1024 * 1024;

/// The address to download `url` from: the same URL for ordinary links, the
/// file itself for a recognized host's share link.
pub(crate) async fn resolve(
    client: &Client,
    url: &Url,
    headers: &HeaderMap,
) -> Result<Url, DownloadError> {
    match drive::file_id(url) {
        Some(id) => drive::follow(client, &drive::direct_url(&id), headers).await,
        None => {
            if drive::is_folder(url) {
                return Err(DownloadError::Unavailable(
                    "this is a Google Drive folder; Drive offers folders only as archives it builds in the browser, so open the folder and download its files one by one".into(),
                ));
            }
            Ok(url.clone())
        }
    }
}

pub mod drive {
    use super::*;

    const HOSTS: &[&str] = &[
        "drive.google.com",
        "docs.google.com",
        "drive.usercontent.google.com",
    ];

    fn is_drive_host(url: &Url) -> bool {
        url.host_str()
            .is_some_and(|h| HOSTS.contains(&h.to_ascii_lowercase().as_str()))
    }

    fn valid_id(id: &str) -> bool {
        id.len() >= 10
            && id.len() <= 200
            && id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    }

    /// The file id of a Google Drive file link, if `url` is one. Google
    /// Docs, Sheets and Slides documents are not files and are left alone.
    pub fn file_id(url: &Url) -> Option<String> {
        if !is_drive_host(url) {
            return None;
        }
        let host = url.host_str()?.to_ascii_lowercase();
        let segments: Vec<&str> = url.path_segments()?.filter(|s| !s.is_empty()).collect();
        let query_id = || {
            url.query_pairs()
                .find(|(k, _)| k == "id")
                .map(|(_, v)| v.into_owned())
        };
        let id = match segments.as_slice() {
            // drive.google.com/file/d/<id>/view, also under /u/<n>/.
            ["file", "d", id, ..] | ["u", _, "file", "d", id, ..] if host == "drive.google.com" => {
                Some((*id).to_string())
            }
            ["open"] | ["uc"] | ["u", _, "uc"] if host != "drive.usercontent.google.com" => {
                query_id()
            }
            ["download"] | ["uc"] | ["u", _, "uc"] if host == "drive.usercontent.google.com" => {
                query_id()
            }
            _ => None,
        }?;
        valid_id(&id).then_some(id)
    }

    /// A folder link, which no single download can fetch.
    pub fn is_folder(url: &Url) -> bool {
        is_drive_host(url) && url.path().contains("/folders/")
    }

    /// Where Drive serves a public file's bytes.
    pub fn direct_url(id: &str) -> Url {
        let mut url =
            Url::parse("https://drive.usercontent.google.com/download").expect("static URL");
        url.query_pairs_mut()
            .append_pair("id", id)
            .append_pair("export", "download");
        url
    }

    /// Ask `start` for the file. A file answer is returned as is; the
    /// virus-scan warning page is followed to its confirmed address; a
    /// sign-in, access or quota page becomes the matching error.
    pub async fn follow(
        client: &Client,
        start: &Url,
        headers: &HeaderMap,
    ) -> Result<Url, DownloadError> {
        // One byte is enough to tell a file from a page.
        let mut resp = client
            .get(start.clone())
            .headers(headers.clone())
            .header(RANGE, "bytes=0-0")
            .send()
            .await
            .map_err(|e| DownloadError::from_reqwest(&e))?;
        let html = resp
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|t| t.to_ascii_lowercase().contains("text/html"));
        let landed = resp.url().clone();
        if !html {
            if resp.status().is_success() || resp.status() == StatusCode::RANGE_NOT_SATISFIABLE {
                return Ok(start.clone());
            }
            return Err(status_error(resp.status().as_u16(), None));
        }
        if is_sign_in(&landed) {
            return Err(private());
        }
        let status = resp.status();
        let mut body = Vec::new();
        while let Some(chunk) = resp
            .chunk()
            .await
            .map_err(|e| DownloadError::from_reqwest(&e))?
        {
            body.extend_from_slice(&chunk);
            if body.len() >= PAGE_CAP {
                break;
            }
        }
        let page = String::from_utf8_lossy(&body);
        if let Some(confirmed) = confirm_url(&page, &landed) {
            return Ok(confirmed);
        }
        let lower = page.to_ascii_lowercase();
        if lower.contains("too many users have viewed or downloaded this file")
            || lower.contains("download quota")
        {
            return Err(DownloadError::Unavailable(
                "Google Drive's download quota for this file is used up for today; try again later, or ask its owner for a copy".into(),
            ));
        }
        if status == StatusCode::NOT_FOUND {
            return Err(DownloadError::NotFound { status: 404 });
        }
        if lower.contains("accounts.google.com")
            || lower.contains("request access")
            || lower.contains("you need access")
        {
            return Err(private());
        }
        if !status.is_success() {
            return Err(status_error(status.as_u16(), None));
        }
        Err(DownloadError::Unavailable("Google Drive answered with a page instead of the file; open the link in the browser to see why".into()))
    }

    fn private() -> DownloadError {
        DownloadError::PrivateFile(
            "this Google Drive file is not shared publicly; ask its owner to share it with anyone who has the link, or download it in the browser while signed in".into(),
        )
    }

    fn is_sign_in(url: &Url) -> bool {
        url.host_str()
            .is_some_and(|h| h.eq_ignore_ascii_case("accounts.google.com"))
            || url.path().to_ascii_lowercase().contains("servicelogin")
    }

    /// The address the "Download anyway" form of the warning page submits
    /// to: its action with its hidden fields as the query. The action must
    /// stay on Google Drive or on the host the page came from.
    pub fn confirm_url(page: &str, page_url: &Url) -> Option<Url> {
        let start = find_form(page)?;
        let form_end = page[start..]
            .find("</form")
            .map_or(page.len(), |p| start + p);
        let form = &page[start..form_end];
        let tag_end = form.find('>')?;
        let action = attr(&form[..tag_end], "action").unwrap_or_default();
        let mut url = page_url.join(&action).ok()?;
        if !(is_drive_host(&url) || url.host_str() == page_url.host_str())
            || !matches!(url.scheme(), "http" | "https")
        {
            return None;
        }
        // Older forms carry part of the address in the action's query.
        let mut fields: Vec<(String, String)> = url
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        let mut rest = &form[tag_end..];
        while let Some(p) = rest.to_ascii_lowercase().find("<input") {
            let tag = &rest[p..];
            let end = tag.find('>').unwrap_or(tag.len());
            let input = &tag[..end];
            if attr(input, "type").is_some_and(|t| t.eq_ignore_ascii_case("hidden"))
                && let Some(name) = attr(input, "name")
            {
                let value = attr(input, "value").unwrap_or_default();
                match fields.iter_mut().find(|(k, _)| *k == name) {
                    Some(field) => field.1 = value,
                    None => fields.push((name, value)),
                }
            }
            rest = &tag[end..];
        }
        // Without the token the form would only bring the page back.
        if !fields.iter().any(|(k, _)| k == "confirm" || k == "uuid") {
            return None;
        }
        url.set_query(None);
        {
            let mut q = url.query_pairs_mut();
            for (k, v) in &fields {
                q.append_pair(k, v);
            }
        }
        Some(url)
    }

    fn find_form(page: &str) -> Option<usize> {
        let lower = page.to_ascii_lowercase();
        if let Some(p) = lower
            .find("id=\"download-form\"")
            .or_else(|| lower.find("id='download-form'"))
        {
            return lower[..p].rfind("<form");
        }
        // Older pages: the first form that submits to a download address.
        let mut from = 0;
        while let Some(p) = lower[from..].find("<form") {
            let start = from + p;
            let end = lower[start..].find('>').map_or(lower.len(), |e| start + e);
            if lower[start..end].contains("download") || lower[start..end].contains("/uc") {
                return Some(start);
            }
            from = end;
        }
        None
    }

    /// An attribute's value in a tag, with `&amp;` and friends decoded.
    fn attr(tag: &str, name: &str) -> Option<String> {
        let lower = tag.to_ascii_lowercase();
        let mut from = 0;
        while let Some(p) = lower[from..].find(name) {
            let at = from + p;
            from = at + name.len();
            let before_ok = at == 0 || lower.as_bytes()[at - 1].is_ascii_whitespace();
            let rest = lower[from..].trim_start();
            if !before_ok || !rest.starts_with('=') {
                continue;
            }
            let value_start = tag.len() - rest.len() + 1;
            let raw = tag[value_start..].trim_start();
            let value = match raw.chars().next() {
                Some(q @ ('"' | '\'')) => raw[1..].split(q).next().unwrap_or(""),
                _ => raw
                    .split(|c: char| c.is_ascii_whitespace() || c == '>')
                    .next()
                    .unwrap_or(""),
            };
            return Some(
                value
                    .replace("&amp;", "&")
                    .replace("&quot;", "\"")
                    .replace("&#39;", "'")
                    .replace("&lt;", "<")
                    .replace("&gt;", ">"),
            );
        }
        None
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn id(s: &str) -> Option<String> {
            file_id(&Url::parse(s).unwrap())
        }

        const ID: &str = "1AbCdEfGhIjKlMnOpQrStUvWxYz_-012";

        #[test]
        fn recognizes_share_links() {
            for link in [
                format!("https://drive.google.com/file/d/{ID}/view?usp=sharing"),
                format!("https://drive.google.com/file/d/{ID}/edit"),
                format!("https://drive.google.com/u/1/file/d/{ID}/view"),
                format!("https://drive.google.com/open?id={ID}"),
                format!("https://drive.google.com/uc?export=download&id={ID}"),
                format!("https://docs.google.com/uc?id={ID}&export=download"),
                format!(
                    "https://drive.usercontent.google.com/download?id={ID}&export=download&authuser=0"
                ),
            ] {
                assert_eq!(id(&link).as_deref(), Some(ID), "{link}");
            }
            for other in [
                format!("https://docs.google.com/document/d/{ID}/edit"),
                format!("https://drive.google.com/drive/folders/{ID}"),
                "https://drive.google.com/file/d/short/view".to_string(),
                format!("https://example.com/file/d/{ID}/view"),
                "https://drive.google.com/uc?id=../../etc".to_string(),
            ] {
                assert_eq!(id(&other), None, "{other}");
            }
            assert!(is_folder(
                &Url::parse(&format!("https://drive.google.com/drive/folders/{ID}")).unwrap()
            ));
            assert_eq!(
                direct_url(ID).as_str(),
                format!("https://drive.usercontent.google.com/download?id={ID}&export=download")
            );
        }

        const WARNING: &str = r#"<!DOCTYPE html><html><head><title>Google Drive - Virus scan warning</title></head>
<body><div class="uc-main"><p class="uc-warning-caption">Google Drive can't scan this file for viruses.</p>
<form id="download-form" action="https://drive.usercontent.google.com/download" method="get">
<input type="submit" id="uc-download-link" class="goog-inline-block jfk-button jfk-button-action" value="Download anyway"/>
<input type="hidden" name="id" value="1AbCdEfGhIjKlMnOpQrStUvWxYz_-012">
<input type="hidden" name="export" value="download">
<input type="hidden" name="confirm" value="t">
<input type="hidden" name="uuid" value="a1b2c3d4-e5f6"></form></div></body></html>"#;

        #[test]
        fn reads_the_warning_form() {
            let page_url = direct_url(ID);
            let url = confirm_url(WARNING, &page_url).unwrap();
            assert_eq!(
                url.as_str(),
                format!(
                    "https://drive.usercontent.google.com/download?id={ID}&export=download&confirm=t&uuid=a1b2c3d4-e5f6"
                )
            );
        }

        #[test]
        fn refuses_forms_that_leave_drive_or_lack_a_token() {
            let page_url = direct_url(ID);
            let elsewhere = WARNING.replace(
                "https://drive.usercontent.google.com/download",
                "https://evil.example/download",
            );
            assert_eq!(confirm_url(&elsewhere, &page_url), None);
            let no_token = WARNING
                .replace("name=\"confirm\"", "name=\"x\"")
                .replace("name=\"uuid\"", "name=\"y\"");
            assert_eq!(confirm_url(&no_token, &page_url), None);
            assert_eq!(
                confirm_url("<html><body>nothing</body></html>", &page_url),
                None
            );
        }

        #[test]
        fn reads_older_forms_and_relative_actions() {
            let page = r#"<form action="/uc?export=download&amp;id=X" method='post'><input type=hidden name=confirm value=AbC1><input name="other" value="no"></form>"#;
            let url = confirm_url(
                page,
                &Url::parse("https://drive.google.com/uc?id=X").unwrap(),
            )
            .unwrap();
            assert_eq!(
                url.as_str(),
                "https://drive.google.com/uc?export=download&id=X&confirm=AbC1"
            );
        }
    }
}
