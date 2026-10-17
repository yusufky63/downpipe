//! Which pages a site provider takes, and the one link each is kept under.
//!
//! Only pages listed here become site downloads: the browser extension
//! offers its button on the same pages (`apps/browser-extension/src/lib/sites.ts`,
//! checked against the same fixture), and the app refuses a `page` message
//! for anything else. Adding a site yt-dlp supports is one more entry in
//! [`recognize`] and in the fixture.

use url::Url;

/// A page whose video a site provider downloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// The site as people name it ("YouTube").
    pub site: &'static str,
    /// The video's id on that site.
    pub id: String,
    /// The page without tracking, timestamps or playlists, so the same
    /// video sent twice is recognized as a duplicate.
    pub url: Url,
}

pub const YOUTUBE: &str = "YouTube";

/// The page behind a link, if a provider takes it.
pub fn recognize(url: &Url) -> Option<Page> {
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let host = url.host_str()?.to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host);
    let mut path = url.path_segments()?.filter(|s| !s.is_empty());
    let id = match host {
        "youtube.com" | "m.youtube.com" | "music.youtube.com" => match path.next()? {
            "watch" => url
                .query_pairs()
                .find(|(k, _)| k == "v")
                .map(|(_, v)| v.into_owned())?,
            "shorts" | "embed" | "live" | "v" => path.next()?.to_string(),
            _ => return None,
        },
        "youtube-nocookie.com" => match path.next()? {
            "embed" => path.next()?.to_string(),
            _ => return None,
        },
        "youtu.be" => path.next()?.to_string(),
        _ => return None,
    };
    youtube_id(&id).then(|| Page {
        site: YOUTUBE,
        url: Url::parse(&format!("https://www.youtube.com/watch?v={id}"))
            .expect("a valid id makes a valid URL"),
        id,
    })
}

/// [`recognize`] for text that may not be a URL.
pub fn recognize_link(raw: &str) -> Option<Page> {
    Url::parse(raw.trim()).ok().as_ref().and_then(recognize)
}

/// YouTube video ids are 11 characters of the URL-safe base64 alphabet.
fn youtube_id(id: &str) -> bool {
    id.len() == 11
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Case {
        url: String,
        site: Option<String>,
        canonical: Option<String>,
    }

    /// The same cases the extension's `sites.test.ts` reads.
    #[test]
    fn shared_fixture() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pages.json");
        let cases: Vec<Case> = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert!(cases.len() > 10);
        for case in cases {
            let page = recognize_link(&case.url);
            assert_eq!(
                page.as_ref().map(|p| p.site.to_string()),
                case.site,
                "{}",
                case.url
            );
            assert_eq!(
                page.map(|p| p.url.to_string()),
                case.canonical,
                "{}",
                case.url
            );
        }
    }

    #[test]
    fn keeps_the_id() {
        let page = recognize_link("https://youtu.be/jNQXAC9IVRw?si=abc").unwrap();
        assert_eq!(page.id, "jNQXAC9IVRw");
        assert_eq!(page.site, YOUTUBE);
    }
}
