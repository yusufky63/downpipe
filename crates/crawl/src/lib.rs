//! Finding files to download on the web, without a browser.
//!
//! * [`site`] is IDM's site grabber: from one address it follows the links of
//!   the same site up to a depth and a page limit, and collects the links to
//!   files of the wanted types, honouring `robots.txt` and a delay between
//!   requests.
//! * [`feed`] reads RSS and Atom feeds and picks the enclosures and links an
//!   item offers, through a filter on file types and titles.
//!
//! Neither keeps cookies or runs scripts: what a plain HTTP client sees is
//! what they find. Both read at most a few MiB per page, so a hostile or
//! broken server cannot make them hold a large body in memory.

pub mod feed;
mod fetch;
pub mod html;
pub mod robots;
pub mod site;

pub use feed::{
    CompiledFilter, FeedError, FeedFilter, FeedItem, ParsedFeed, fetch_feed, parse_feed,
};
pub use site::{FoundFile, GrabError, GrabOptions, GrabPhase, GrabProgress, GrabReport, grab};

/// The robots.txt product token Downpipe answers to.
pub const AGENT_TOKEN: &str = "Downpipe";

/// Lower-case extension of a URL's last path segment, percent-decoded:
/// `/a/Report%20final.PDF?x=1` -> `pdf`.
pub fn url_extension(url: &url::Url) -> Option<String> {
    let segment = url.path_segments()?.next_back()?;
    let name = percent_encoding::percent_decode_str(segment).decode_utf8_lossy();
    let (stem, ext) = name.rsplit_once('.')?;
    (!stem.is_empty()
        && !ext.is_empty()
        && ext.len() <= 12
        && ext.chars().all(|c| c.is_ascii_alphanumeric()))
    .then(|| ext.to_ascii_lowercase())
}

/// The file name a URL's last path segment gives, percent-decoded.
pub fn url_file_name(url: &url::Url) -> String {
    url.path_segments()
        .and_then(|mut s| s.next_back())
        .filter(|s| !s.is_empty())
        .map(|s| {
            percent_encoding::percent_decode_str(s)
                .decode_utf8_lossy()
                .into_owned()
        })
        .unwrap_or_else(|| url.host_str().unwrap_or("download").to_string())
}

/// Extensions written by people (`.MP3`, ` zip`, `*.pdf`) in the form the
/// filters compare: lower case, without dots, wildcards or duplicates.
pub fn normalize_extensions<I, S>(items: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut out: Vec<String> = Vec::new();
    for item in items {
        for part in item.as_ref().split([',', ' ', ';', '\n', '\t']) {
            let ext = part
                .trim()
                .trim_start_matches('*')
                .trim_start_matches('.')
                .to_ascii_lowercase();
            if !ext.is_empty()
                && ext.len() <= 12
                && ext.chars().all(|c| c.is_ascii_alphanumeric())
                && !out.contains(&ext)
            {
                out.push(ext);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions_of_urls() {
        let u = |s: &str| url::Url::parse(s).unwrap();
        assert_eq!(
            url_extension(&u("https://a.test/x/Report%20final.PDF?x=1")).as_deref(),
            Some("pdf")
        );
        assert_eq!(url_extension(&u("https://a.test/x/")), None);
        assert_eq!(url_extension(&u("https://a.test/.hidden")), None);
        assert_eq!(url_extension(&u("https://a.test/a.b/c")), None);
        assert_eq!(
            url_file_name(&u("https://a.test/x/My%20File.zip")),
            "My File.zip"
        );
        assert_eq!(url_file_name(&u("https://a.test/")), "a.test");
    }

    #[test]
    fn extension_lists_are_cleaned() {
        assert_eq!(
            normalize_extensions(["*.MP3, .zip", "pdf zip", "", "bad/ext"]),
            ["mp3", "zip", "pdf"]
        );
    }
}
