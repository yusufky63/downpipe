//! Which pages a site provider takes, and the one link each is kept under.
//!
//! Only pages listed here become site downloads: the browser extension
//! offers its button on the same pages (`apps/browser-extension/src/lib/sites.ts`,
//! checked against the same fixture), and the app refuses a `page` message
//! for anything else. Adding a site yt-dlp supports is one more entry in
//! [`recognize`] and in the fixture.
//!
//! A playlist or a channel is not one download but a list to choose from
//! ([`recognize_list`], fixture `lists.json`): the app asks yt-dlp for its
//! videos and adds the ones the user picks, each as a page of its own.

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

/// What kind of list a [`ListPage`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKind {
    Playlist,
    Channel,
}

impl ListKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Playlist => "playlist",
            Self::Channel => "channel",
        }
    }
}

/// A playlist or channel whose videos a site provider can list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListPage {
    pub site: &'static str,
    pub kind: ListKind,
    /// The list's own page: `playlist?list=...`, or a channel's videos tab
    /// (its Shorts or live tab when that was the link), because a channel's
    /// home page lists its tabs rather than its videos.
    pub url: Url,
}

/// Playlists that belong to whoever is signed in (Watch later, Liked
/// videos, Liked music) and are empty for Downpipe, which never sends
/// cookies.
const PRIVATE_LISTS: &[&str] = &["WL", "LL", "LM"];

/// The playlist or channel behind a link, if a provider lists it. A video
/// link that also names a playlist (`watch?v=...&list=...`) is the video.
pub fn recognize_list(url: &Url) -> Option<ListPage> {
    if !matches!(url.scheme(), "http" | "https") || recognize(url).is_some() {
        return None;
    }
    let host = url.host_str()?.to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host);
    if !matches!(host, "youtube.com" | "m.youtube.com" | "music.youtube.com") {
        return None;
    }
    let path: Vec<&str> = url.path_segments()?.filter(|s| !s.is_empty()).collect();
    let page = |kind, path: String| ListPage {
        site: YOUTUBE,
        kind,
        url: Url::parse(&format!("https://www.youtube.com/{path}"))
            .expect("checked parts make a valid URL"),
    };
    let (first, rest) = path.split_first()?;
    if *first == "playlist" {
        let id = url
            .query_pairs()
            .find(|(k, _)| k == "list")
            .map(|(_, v)| v.into_owned())?;
        return (list_id(&id) && !PRIVATE_LISTS.contains(&id.as_str()))
            .then(|| page(ListKind::Playlist, format!("playlist?list={id}")));
    }
    // `@handle/<tab>`, `channel/<id>/<tab>`, `c/<name>/<tab>`, `user/<name>/<tab>`.
    let (channel, tab) = if let Some(handle) = first.strip_prefix('@') {
        (
            channel_name(handle).then(|| (*first).to_string())?,
            rest.first(),
        )
    } else {
        let name = rest.first()?;
        let ok = match *first {
            "channel" => channel_id(name),
            "c" | "user" => channel_name(name),
            _ => false,
        };
        (ok.then(|| format!("{first}/{name}"))?, rest.get(1))
    };
    // Shorts and live keep their tab; any other tab lists the videos.
    let tab = tab
        .copied()
        .filter(|t| matches!(*t, "shorts" | "streams"))
        .unwrap_or("videos");
    Some(page(ListKind::Channel, format!("{channel}/{tab}")))
}

/// [`recognize_list`] for text that may not be a URL.
pub fn recognize_list_link(raw: &str) -> Option<ListPage> {
    Url::parse(raw.trim())
        .ok()
        .as_ref()
        .and_then(recognize_list)
}

/// Playlist ids: `PL` and 32 more characters, `OLAK5uy_...` albums, `UU`
/// uploads and the like; the URL-safe base64 alphabet either way.
fn list_id(id: &str) -> bool {
    (2..=64).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// `UC` and 22 characters.
fn channel_id(id: &str) -> bool {
    id.len() == 24
        && id.starts_with("UC")
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// A handle or legacy channel name as it stands in the path, where the URL
/// parser keeps non-ASCII letters percent-encoded.
fn channel_name(name: &str) -> bool {
    (1..=300).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'%'))
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

    #[derive(Deserialize)]
    struct ListCase {
        url: String,
        kind: Option<String>,
        canonical: Option<String>,
    }

    /// The same cases the app's and the extension's `sites.test.ts` read.
    #[test]
    fn shared_list_fixture() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/lists.json");
        let cases: Vec<ListCase> = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert!(cases.len() > 10);
        for case in cases {
            let list = recognize_list_link(&case.url);
            assert_eq!(
                list.as_ref().map(|l| l.kind.as_str().to_string()),
                case.kind,
                "{}",
                case.url
            );
            assert_eq!(
                list.map(|l| l.url.to_string()),
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
