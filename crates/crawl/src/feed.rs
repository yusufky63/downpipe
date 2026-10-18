//! RSS 2.0, RSS 1.0 (RDF) and Atom feeds: their items, and the files an
//! item offers.
//!
//! An item's files are its enclosures (`<enclosure>`, Atom's
//! `rel="enclosure"` links, Media RSS `<media:content>`), plus its own link
//! when that points at a file rather than a page. The filter then keeps the
//! wanted file types and the titles that match a pattern.

use std::time::Duration;

use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use url::Url;

use crate::fetch::{self, FetchError};
use crate::url_extension;

const FEED_CAP: usize = 8 * 1024 * 1024;
const FEED_TIMEOUT: Duration = Duration::from_secs(30);
const MEDIA_RSS: &str = "http://search.yahoo.com/mrss/";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ParsedFeed {
    pub title: String,
    pub items: Vec<FeedItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FeedItem {
    /// `guid` / `id`, else the link, else the first enclosure: what makes the
    /// item the same item on the next check.
    pub id: String,
    pub title: String,
    /// The item's own link (a page, usually).
    pub link: Option<String>,
    pub enclosures: Vec<Enclosure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Enclosure {
    pub url: String,
    /// MIME type the feed announces, lower case.
    pub mime: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum FeedError {
    #[error("not an http or https address: {0}")]
    InvalidUrl(String),
    #[error("the feed could not be read: {0}")]
    Fetch(String),
    #[error("the server answered HTTP {0}")]
    Status(u16),
    #[error("this address is a web page, not an RSS or Atom feed")]
    NotAFeed,
    #[error("the feed is not valid XML: {0}")]
    Xml(String),
    #[error("the feed is larger than 8 MiB")]
    TooLarge,
    #[error("the title pattern is not valid: {0}")]
    Pattern(String),
    #[error("stopped")]
    Canceled,
}

impl FeedError {
    /// Stable name UIs branch on.
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidUrl(_) => "invalid_url",
            Self::Fetch(_) => "network",
            Self::Status(_) => "http_status",
            Self::NotAFeed => "not_a_feed",
            Self::Xml(_) => "invalid_feed",
            Self::TooLarge => "too_large",
            Self::Pattern(_) => "invalid_pattern",
            Self::Canceled => "canceled",
        }
    }
}

/// Fetch and parse the feed at `url`.
pub async fn fetch_feed(
    client: &Client,
    url: &str,
    cancel: &CancellationToken,
) -> Result<ParsedFeed, FeedError> {
    let url = Url::parse(url.trim())
        .ok()
        .filter(|u| matches!(u.scheme(), "http" | "https"))
        .ok_or_else(|| FeedError::InvalidUrl(url.to_string()))?;
    let page = fetch::get_text(
        client,
        &url,
        "application/rss+xml, application/atom+xml, application/rdf+xml;q=0.9, application/xml;q=0.8, text/xml;q=0.8, */*;q=0.1",
        FEED_CAP,
        FEED_TIMEOUT,
        cancel,
    )
    .await
    .map_err(|e| match e {
        FetchError::Canceled => FeedError::Canceled,
        other => FeedError::Fetch(other.to_string()),
    })?;
    if !(200..300).contains(&page.status) {
        return Err(FeedError::Status(page.status));
    }
    if page.truncated {
        return Err(FeedError::TooLarge);
    }
    parse_feed(&page.body, &page.url)
}

/// Parse a feed document. Relative links resolve against `base`.
pub fn parse_feed(text: &str, base: &Url) -> Result<ParsedFeed, FeedError> {
    let text = text.trim_start_matches('\u{feff}').trim_start();
    // roxmltree refuses DTDs, which also keeps entity expansion attacks out.
    let doc = roxmltree::Document::parse(text).map_err(|e| {
        if text
            .get(..15)
            .is_some_and(|s| s.to_ascii_lowercase().starts_with("<!doctype html"))
            || text.starts_with("<html")
        {
            FeedError::NotAFeed
        } else {
            FeedError::Xml(e.to_string())
        }
    })?;
    let root = doc.root_element();
    match root.tag_name().name() {
        "rss" => {
            let channel = child(root, "channel").ok_or(FeedError::NotAFeed)?;
            let items = channel
                .children()
                .filter(|n| n.has_tag_name("item"))
                .map(|n| rss_item(n, base))
                .collect();
            Ok(ParsedFeed {
                title: child_text(channel, "title"),
                items,
            })
        }
        // RSS 1.0: items are siblings of the channel.
        "RDF" => {
            let title = child(root, "channel")
                .map(|c| child_text(c, "title"))
                .unwrap_or_default();
            let items = root
                .children()
                .filter(|n| n.tag_name().name() == "item")
                .map(|n| rss_item(n, base))
                .collect();
            Ok(ParsedFeed { title, items })
        }
        "feed" => {
            let items = root
                .children()
                .filter(|n| n.tag_name().name() == "entry")
                .map(|n| atom_entry(n, base))
                .collect();
            Ok(ParsedFeed {
                title: child_text(root, "title"),
                items,
            })
        }
        _ => Err(FeedError::NotAFeed),
    }
}

fn child<'a, 'i>(node: roxmltree::Node<'a, 'i>, name: &str) -> Option<roxmltree::Node<'a, 'i>> {
    // Namespaced elements (`media:title`) are not what the plain names mean.
    node.children().find(|n| {
        n.is_element() && n.tag_name().name() == name && n.tag_name().namespace() != Some(MEDIA_RSS)
    })
}

fn child_text(node: roxmltree::Node<'_, '_>, name: &str) -> String {
    child(node, name).map(text_of).unwrap_or_default()
}

fn text_of(node: roxmltree::Node<'_, '_>) -> String {
    let joined: String = node
        .descendants()
        .filter(|n| n.is_text())
        .filter_map(|n| n.text())
        .collect();
    joined.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn resolve(base: &Url, value: &str) -> Option<String> {
    let mut url = base.join(value.trim()).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    url.set_fragment(None);
    Some(url.to_string())
}

fn media_enclosures(node: roxmltree::Node<'_, '_>, base: &Url, out: &mut Vec<Enclosure>) {
    // <media:content> directly or inside <media:group>.
    for n in node
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().namespace() == Some(MEDIA_RSS))
    {
        if n.tag_name().name() == "content"
            && let Some(url) = n.attribute("url").and_then(|u| resolve(base, u))
            && !out.iter().any(|e| e.url == url)
        {
            out.push(Enclosure {
                url,
                mime: n.attribute("type").map(|t| t.trim().to_ascii_lowercase()),
            });
        }
    }
}

fn rss_item(node: roxmltree::Node<'_, '_>, base: &Url) -> FeedItem {
    let link = child(node, "link")
        .map(text_of)
        .and_then(|l| resolve(base, &l))
        .or_else(|| {
            // RSS 1.0 also names the item by its rdf:about.
            node.attributes()
                .find(|a| a.name() == "about")
                .and_then(|a| resolve(base, a.value()))
        });
    let mut enclosures = Vec::new();
    for n in node
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "enclosure")
    {
        if let Some(url) = n.attribute("url").and_then(|u| resolve(base, u))
            && !enclosures.iter().any(|e: &Enclosure| e.url == url)
        {
            enclosures.push(Enclosure {
                url,
                mime: n.attribute("type").map(|t| t.trim().to_ascii_lowercase()),
            });
        }
    }
    media_enclosures(node, base, &mut enclosures);
    let guid = child_text(node, "guid");
    let id = first_non_empty([
        guid,
        link.clone().unwrap_or_default(),
        enclosures
            .first()
            .map(|e| e.url.clone())
            .unwrap_or_default(),
    ]);
    FeedItem {
        id,
        title: child_text(node, "title"),
        link,
        enclosures,
    }
}

fn atom_entry(node: roxmltree::Node<'_, '_>, base: &Url) -> FeedItem {
    let mut link = None;
    let mut enclosures = Vec::new();
    for n in node
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "link")
    {
        let Some(href) = n.attribute("href").and_then(|h| resolve(base, h)) else {
            continue;
        };
        match n.attribute("rel").unwrap_or("alternate") {
            "enclosure" => {
                if !enclosures.iter().any(|e: &Enclosure| e.url == href) {
                    enclosures.push(Enclosure {
                        url: href,
                        mime: n.attribute("type").map(|t| t.trim().to_ascii_lowercase()),
                    });
                }
            }
            "alternate" if link.is_none() => link = Some(href),
            _ => {}
        }
    }
    media_enclosures(node, base, &mut enclosures);
    let id = first_non_empty([
        child_text(node, "id"),
        link.clone().unwrap_or_default(),
        enclosures
            .first()
            .map(|e| e.url.clone())
            .unwrap_or_default(),
    ]);
    FeedItem {
        id,
        title: child_text(node, "title"),
        link,
        enclosures,
    }
}

fn first_non_empty<const N: usize>(values: [String; N]) -> String {
    values
        .into_iter()
        .find(|v| !v.trim().is_empty())
        .unwrap_or_default()
}

/// Which items a subscription downloads.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FeedFilter {
    /// File types to download, lower case without dots. Empty: any file.
    pub extensions: Vec<String>,
    /// A regular expression the item's title must contain (case does not
    /// matter). Empty: any title.
    pub title_pattern: String,
}

impl FeedFilter {
    pub fn compile(&self) -> Result<CompiledFilter, FeedError> {
        let pattern = self.title_pattern.trim();
        let title = if pattern.is_empty() {
            None
        } else {
            Some(
                regex::RegexBuilder::new(pattern)
                    .case_insensitive(true)
                    .size_limit(1 << 20)
                    .build()
                    .map_err(|e| FeedError::Pattern(e.to_string()))?,
            )
        };
        Ok(CompiledFilter {
            extensions: crate::normalize_extensions(&self.extensions),
            title,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CompiledFilter {
    extensions: Vec<String>,
    title: Option<Regex>,
}

impl CompiledFilter {
    /// The links of `item` to download; empty when the filter leaves it out.
    pub fn pick(&self, item: &FeedItem) -> Vec<String> {
        if self
            .title
            .as_ref()
            .is_some_and(|re| !re.is_match(&item.title))
        {
            return Vec::new();
        }
        let mut candidates: Vec<(String, Option<String>)> = item
            .enclosures
            .iter()
            .map(|e| (e.url.clone(), extension_for(&e.url, e.mime.as_deref())))
            .collect();
        // The item's link counts when it is a file, not a page.
        if let Some(link) = &item.link
            && let Some(ext) = Url::parse(link).ok().as_ref().and_then(url_extension)
            && !matches!(
                ext.as_str(),
                "html" | "htm" | "xhtml" | "shtml" | "php" | "asp" | "aspx" | "jsp" | "cfm"
            )
            && !candidates.iter().any(|(u, _)| u == link)
        {
            candidates.push((link.clone(), Some(ext)));
        }
        candidates
            .into_iter()
            .filter(|(_, ext)| {
                self.extensions.is_empty()
                    || ext.as_ref().is_some_and(|e| self.extensions.contains(e))
            })
            .map(|(url, _)| url)
            .collect()
    }
}

/// The file type of an enclosure: from its URL, else from its MIME type
/// (podcast hosts often hide the name behind a tracking redirect).
fn extension_for(url: &str, mime: Option<&str>) -> Option<String> {
    Url::parse(url)
        .ok()
        .as_ref()
        .and_then(url_extension)
        .or_else(|| {
            let ext = match mime?.split(';').next()?.trim() {
                "audio/mpeg" | "audio/mp3" => "mp3",
                "audio/mp4" | "audio/x-m4a" | "audio/m4a" => "m4a",
                "audio/aac" => "aac",
                "audio/ogg" => "ogg",
                "audio/opus" => "opus",
                "audio/flac" | "audio/x-flac" => "flac",
                "audio/wav" | "audio/x-wav" => "wav",
                "video/mp4" => "mp4",
                "video/x-m4v" => "m4v",
                "video/webm" => "webm",
                "video/quicktime" => "mov",
                "video/x-matroska" => "mkv",
                "application/pdf" => "pdf",
                "application/zip" => "zip",
                "application/epub+zip" => "epub",
                "application/x-bittorrent" => "torrent",
                _ => return None,
            };
            Some(ext.to_string())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Url {
        Url::parse("https://pod.test/feed.xml").unwrap()
    }

    const RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
  <channel>
    <title>Example  Podcast</title>
    <item>
      <title>Episode 2: Rust</title>
      <link>https://pod.test/ep2</link>
      <guid isPermaLink="false">ep-2</guid>
      <enclosure url="/media/ep2.mp3" length="123" type="audio/mpeg"/>
    </item>
    <item>
      <title><![CDATA[Episode 1 & intro]]></title>
      <link>https://pod.test/files/ep1.pdf</link>
      <media:content url="https://cdn.test/track?id=1" type="audio/mpeg"/>
      <media:title>not the title</media:title>
    </item>
    <item><title>No files</title><link>https://pod.test/news.html</link></item>
  </channel>
</rss>"#;

    #[test]
    fn reads_rss_items_and_enclosures() {
        let feed = parse_feed(RSS, &base()).unwrap();
        assert_eq!(feed.title, "Example Podcast");
        assert_eq!(feed.items.len(), 3);
        let ep2 = &feed.items[0];
        assert_eq!(ep2.id, "ep-2");
        assert_eq!(ep2.title, "Episode 2: Rust");
        assert_eq!(
            ep2.enclosures,
            [Enclosure {
                url: "https://pod.test/media/ep2.mp3".into(),
                mime: Some("audio/mpeg".into())
            }]
        );
        let ep1 = &feed.items[1];
        assert_eq!(ep1.title, "Episode 1 & intro");
        assert_eq!(ep1.id, "https://pod.test/files/ep1.pdf");
        assert_eq!(ep1.enclosures[0].url, "https://cdn.test/track?id=1");
    }

    #[test]
    fn filter_picks_types_and_titles() {
        let feed = parse_feed(RSS, &base()).unwrap();
        let any = FeedFilter::default().compile().unwrap();
        assert_eq!(any.pick(&feed.items[0]), ["https://pod.test/media/ep2.mp3"]);
        assert_eq!(
            any.pick(&feed.items[1]),
            [
                "https://cdn.test/track?id=1",
                "https://pod.test/files/ep1.pdf"
            ]
        );
        assert!(
            any.pick(&feed.items[2]).is_empty(),
            "a page link is not a file"
        );

        let mp3 = FeedFilter {
            extensions: vec![".MP3".into()],
            title_pattern: String::new(),
        }
        .compile()
        .unwrap();
        // The hidden name comes from the announced MIME type.
        assert_eq!(mp3.pick(&feed.items[1]), ["https://cdn.test/track?id=1"]);

        let rust = FeedFilter {
            extensions: vec![],
            title_pattern: "rust|go".into(),
        }
        .compile()
        .unwrap();
        assert_eq!(rust.pick(&feed.items[0]).len(), 1);
        assert!(rust.pick(&feed.items[1]).is_empty());

        let bad = FeedFilter {
            extensions: vec![],
            title_pattern: "(".into(),
        }
        .compile();
        assert!(matches!(bad, Err(FeedError::Pattern(_))));
    }

    #[test]
    fn reads_atom() {
        let atom = r#"<feed xmlns="http://www.w3.org/2005/Atom"><title type="text">Releases</title>
            <entry><id>tag:x,1</id><title>v1.0</title>
              <link href="https://x.test/v1"/>
              <link rel="enclosure" type="application/zip" href="dl/v1.zip"/>
            </entry></feed>"#;
        let feed = parse_feed(atom, &Url::parse("https://x.test/releases.atom").unwrap()).unwrap();
        assert_eq!(feed.title, "Releases");
        assert_eq!(feed.items[0].id, "tag:x,1");
        assert_eq!(feed.items[0].link.as_deref(), Some("https://x.test/v1"));
        assert_eq!(feed.items[0].enclosures[0].url, "https://x.test/dl/v1.zip");
    }

    #[test]
    fn reads_rss_1() {
        let rdf = r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns="http://purl.org/rss/1.0/">
            <channel rdf:about="https://r.test/"><title>R</title></channel>
            <item rdf:about="https://r.test/a.pdf"><title>A</title><link>https://r.test/a.pdf</link></item></rdf:RDF>"#;
        let feed = parse_feed(rdf, &base()).unwrap();
        assert_eq!(feed.title, "R");
        assert_eq!(
            FeedFilter::default()
                .compile()
                .unwrap()
                .pick(&feed.items[0]),
            ["https://r.test/a.pdf"]
        );
    }

    #[test]
    fn refuses_pages_and_dtds() {
        assert!(matches!(
            parse_feed("<!DOCTYPE html><html><body>x</body></html>", &base()),
            Err(FeedError::NotAFeed)
        ));
        assert!(matches!(
            parse_feed("<html><body/></html>", &base()),
            Err(FeedError::NotAFeed)
        ));
        let bomb = r#"<?xml version="1.0"?><!DOCTYPE rss [<!ENTITY a "aaaa">]><rss><channel><title>&a;</title></channel></rss>"#;
        assert!(parse_feed(bomb, &base()).is_err());
        assert!(matches!(
            parse_feed("not xml", &base()),
            Err(FeedError::Xml(_))
        ));
    }
}
