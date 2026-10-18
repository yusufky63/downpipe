//! IDM's site grabber: follow a site's links from one address and collect
//! the files of the wanted types.
//!
//! Pages are read breadth first, the start page at depth 1, so the page
//! limit keeps the pages nearest the start. Only pages of the start's site
//! are followed unless `same_host` is off; files are collected wherever they
//! are hosted, since sites often keep downloads on another host. Before each
//! request to a host the grabber waits the delay (or the host's
//! `Crawl-delay`, when longer), and it asks for nothing `robots.txt`
//! disallows while `respect_robots` is on. No cookies are kept.

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use url::Url;

use crate::fetch::{self, FetchError};
use crate::html::{self, LinkKind};
use crate::robots::Robots;
use crate::{AGENT_TOKEN, url_extension, url_file_name};

pub const MAX_DEPTH: u32 = 5;
pub const MAX_PAGES: u32 = 2000;
/// Sizes are asked for at most this many files; the rest stay unknown.
pub const MAX_SIZED: usize = 500;
/// The grabber stops collecting beyond this many files.
pub const MAX_FILES: usize = 10_000;
const PAGE_CAP: usize = 4 * 1024 * 1024;
const ROBOTS_CAP: usize = 512 * 1024;
const PAGE_TIMEOUT: Duration = Duration::from_secs(30);
const HEAD_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_DELAY: Duration = Duration::from_secs(60);

/// Extensions of server-generated pages: links to them are pages to read,
/// not files to collect.
const PAGE_EXTENSIONS: &[&str] = &[
    "html", "htm", "xhtml", "shtml", "php", "asp", "aspx", "jsp", "jspx", "cfm", "cgi", "pl",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GrabOptions {
    /// Where to start: an http(s) address.
    pub url: String,
    /// 1 reads only the start page; each step follows one more level of links.
    pub depth: u32,
    /// Pages read at most, the start page included.
    pub max_pages: u32,
    /// File types to collect, lower case without dots.
    pub extensions: Vec<String>,
    /// Follow only pages on the start page's host (`www.` aside).
    pub same_host: bool,
    pub respect_robots: bool,
    /// Wait between two requests to the same host.
    pub delay_ms: u64,
    /// Ask each file's size with a `HEAD` request.
    pub check_sizes: bool,
}

impl Default for GrabOptions {
    fn default() -> Self {
        Self {
            url: String::new(),
            depth: 2,
            max_pages: 100,
            extensions: Vec::new(),
            same_host: true,
            respect_robots: true,
            delay_ms: 500,
            check_sizes: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FoundFile {
    pub url: String,
    /// From the link's last path segment.
    pub name: String,
    pub extension: String,
    /// From `HEAD`, when asked and answered.
    pub size: Option<u64>,
    /// The page the link was first seen on.
    pub found_on: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GrabPhase {
    #[default]
    Pages,
    Sizes,
    Done,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct GrabProgress {
    pub phase: GrabPhase,
    pub pages_done: u32,
    /// Pages read plus pages waiting, within the limit.
    pub pages_total: u32,
    pub files: usize,
    pub sized: usize,
    /// Files whose size is being asked for, at most [`MAX_SIZED`].
    pub to_size: usize,
    /// The page or file being requested.
    pub current: Option<String>,
    /// Pages and files left out because robots.txt disallows them.
    pub blocked: u32,
    /// Pages that failed or were not HTML.
    pub failed: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct GrabReport {
    pub files: Vec<FoundFile>,
    pub pages: u32,
    pub blocked: u32,
    pub failed: u32,
    /// Stopped by the user; `files` holds what was found until then.
    pub canceled: bool,
    /// The page limit stopped the grabber before every page was read.
    pub hit_page_limit: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum GrabError {
    #[error("not an http or https address: {0}")]
    InvalidUrl(String),
    #[error("choose at least one file type")]
    NoTypes,
    #[error("robots.txt of this site does not allow reading this page")]
    Blocked,
    #[error("the start page could not be read: {0}")]
    StartPage(String),
    #[error("the start page is not a web page")]
    NotHtml,
}

impl GrabError {
    /// Stable name UIs branch on.
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidUrl(_) => "invalid_url",
            Self::NoTypes => "no_types",
            Self::Blocked => "robots_blocked",
            Self::StartPage(_) => "start_page",
            Self::NotHtml => "not_html",
        }
    }
}

/// Run the grabber. `progress` is called after every request. Cancelling
/// ends it early with what it found so far (`canceled` set).
pub async fn grab(
    client: &Client,
    options: GrabOptions,
    progress: &(dyn Fn(&GrabProgress) + Send + Sync),
    cancel: CancellationToken,
) -> Result<GrabReport, GrabError> {
    let start = Url::parse(options.url.trim())
        .ok()
        .filter(|u| matches!(u.scheme(), "http" | "https") && u.host_str().is_some())
        .ok_or_else(|| GrabError::InvalidUrl(options.url.clone()))?;
    let extensions = crate::normalize_extensions(&options.extensions);
    if extensions.is_empty() {
        return Err(GrabError::NoTypes);
    }
    let depth = options.depth.clamp(1, MAX_DEPTH);
    let max_pages = options.max_pages.clamp(1, MAX_PAGES);
    let mut grabber = Grabber {
        client,
        cancel,
        progress,
        respect_robots: options.respect_robots,
        delay: Duration::from_millis(options.delay_ms).min(MAX_DELAY),
        robots: HashMap::new(),
        last_request: HashMap::new(),
        state: GrabProgress::default(),
    };

    let mut start_page = start.clone();
    start_page.set_fragment(None);
    let site = site_of(&start_page);
    let mut queue: VecDeque<(Url, u32)> = VecDeque::from([(start_page.clone(), 1)]);
    let mut seen_pages: HashSet<Url> = HashSet::from([start_page.clone()]);
    let mut files: Vec<FoundFile> = Vec::new();
    let mut seen_files: HashSet<Url> = HashSet::new();
    let mut canceled = false;
    let mut hit_page_limit = false;
    grabber.state.pages_total = 1;

    while let Some((page, level)) = queue.pop_front() {
        let first = grabber.state.pages_done == 0
            && grabber.state.failed == 0
            && grabber.state.blocked == 0;
        if grabber.state.pages_done >= max_pages {
            hit_page_limit = true;
            break;
        }
        match grabber.read_page(&page).await {
            Ok(Read::Page(found_at, body)) => {
                grabber.state.pages_done += 1;
                // A redirect off the site is not followed further.
                if options.same_host && site_of(&found_at) != site {
                    grabber.report();
                    continue;
                }
                for link in html::links(&body, &found_at) {
                    let url = link.url;
                    let ext = url_extension(&url);
                    if let Some(ext) = ext.as_deref().filter(|e| extensions.iter().any(|x| x == e))
                    {
                        if files.len() < MAX_FILES && seen_files.insert(url.clone()) {
                            files.push(FoundFile {
                                name: url_file_name(&url),
                                extension: ext.to_string(),
                                url: url.to_string(),
                                size: None,
                                found_on: found_at.to_string(),
                            });
                        }
                        continue;
                    }
                    let followable = level < depth
                        && link.kind != LinkKind::Embedded
                        && ext.as_deref().is_none_or(|e| PAGE_EXTENSIONS.contains(&e))
                        && (!options.same_host || site_of(&url) == site);
                    if followable && !seen_pages.contains(&url) {
                        if grabber.state.pages_total >= max_pages {
                            hit_page_limit = true;
                            continue;
                        }
                        seen_pages.insert(url.clone());
                        queue.push_back((url, level + 1));
                        grabber.state.pages_total += 1;
                    }
                }
                grabber.state.files = files.len();
                grabber.report();
            }
            Ok(Read::NotHtml) if first => return Err(GrabError::NotHtml),
            Ok(Read::Blocked) if first => return Err(GrabError::Blocked),
            Err(FetchError::Canceled) => {
                canceled = true;
                break;
            }
            Err(err) if first => return Err(GrabError::StartPage(err.to_string())),
            Ok(Read::Failed(status)) if first => {
                return Err(GrabError::StartPage(format!("HTTP {status}")));
            }
            Ok(Read::NotHtml | Read::Failed(_)) | Err(_) => {
                grabber.state.failed += 1;
                grabber.report();
            }
            Ok(Read::Blocked) => {
                grabber.state.blocked += 1;
                grabber.report();
            }
        }
    }

    if options.check_sizes && !canceled && !files.is_empty() {
        grabber.state.phase = GrabPhase::Sizes;
        grabber.state.to_size = files.len().min(MAX_SIZED);
        grabber.report();
        for file in files.iter_mut().take(MAX_SIZED) {
            let Ok(url) = Url::parse(&file.url) else {
                continue;
            };
            match grabber.size_of(&url).await {
                Ok(size) => file.size = size,
                Err(FetchError::Canceled) => {
                    canceled = true;
                    break;
                }
                Err(_) => {}
            }
            grabber.state.sized += 1;
            grabber.report();
        }
    }

    grabber.state.phase = GrabPhase::Done;
    grabber.state.current = None;
    grabber.report();
    Ok(GrabReport {
        files,
        pages: grabber.state.pages_done,
        blocked: grabber.state.blocked,
        failed: grabber.state.failed,
        canceled,
        hit_page_limit,
    })
}

enum Read {
    /// The URL redirects ended at, and the (possibly cut) HTML.
    Page(Url, String),
    NotHtml,
    Failed(u16),
    Blocked,
}

struct Grabber<'a> {
    client: &'a Client,
    cancel: CancellationToken,
    progress: &'a (dyn Fn(&GrabProgress) + Send + Sync),
    respect_robots: bool,
    delay: Duration,
    /// By origin (`https://host:port`).
    robots: HashMap<String, Robots>,
    last_request: HashMap<String, Instant>,
    state: GrabProgress,
}

impl Grabber<'_> {
    fn report(&self) {
        (self.progress)(&self.state);
    }

    async fn read_page(&mut self, url: &Url) -> Result<Read, FetchError> {
        if !self.allowed(url).await? {
            return Ok(Read::Blocked);
        }
        self.state.current = Some(url.to_string());
        self.report();
        self.wait_turn(url).await?;
        let page = fetch::get_text(
            self.client,
            url,
            "text/html,application/xhtml+xml;q=0.9,*/*;q=0.1",
            PAGE_CAP,
            PAGE_TIMEOUT,
            &self.cancel,
        )
        .await?;
        if !(200..300).contains(&page.status) {
            return Ok(Read::Failed(page.status));
        }
        if !page.is_html() {
            return Ok(Read::NotHtml);
        }
        Ok(Read::Page(page.url, page.body))
    }

    async fn size_of(&mut self, url: &Url) -> Result<Option<u64>, FetchError> {
        if !self.allowed(url).await? {
            self.state.blocked += 1;
            return Ok(None);
        }
        self.state.current = Some(url.to_string());
        self.wait_turn(url).await?;
        fetch::head_size(self.client, url, HEAD_TIMEOUT, &self.cancel).await
    }

    /// Sleep until the host's delay since the last request to it has passed.
    async fn wait_turn(&mut self, url: &Url) -> Result<(), FetchError> {
        let origin = url.origin().ascii_serialization();
        let delay = self
            .robots
            .get(&origin)
            .and_then(|r| r.crawl_delay)
            .map_or(self.delay, |d| d.max(self.delay))
            .min(MAX_DELAY);
        if let Some(last) = self.last_request.get(&origin) {
            fetch::pause(delay.saturating_sub(last.elapsed()), &self.cancel).await?;
        }
        self.last_request.insert(origin, Instant::now());
        Ok(())
    }

    async fn allowed(&mut self, url: &Url) -> Result<bool, FetchError> {
        if !self.respect_robots {
            return Ok(true);
        }
        let origin = url.origin().ascii_serialization();
        if !self.robots.contains_key(&origin) {
            let robots = self.load_robots(url).await?;
            self.robots.insert(origin.clone(), robots);
        }
        let path = match url.query() {
            Some(q) => format!("{}?{q}", url.path()),
            None => url.path().to_string(),
        };
        Ok(self.robots.get(&origin).is_none_or(|r| r.allows(&path)))
    }

    async fn load_robots(&mut self, url: &Url) -> Result<Robots, FetchError> {
        let Ok(robots_url) = url.join("/robots.txt") else {
            return Ok(Robots::allow_all());
        };
        self.state.current = Some(robots_url.to_string());
        self.report();
        self.wait_turn(&robots_url).await?;
        match fetch::get_text(
            self.client,
            &robots_url,
            "text/plain,*/*;q=0.1",
            ROBOTS_CAP,
            PAGE_TIMEOUT,
            &self.cancel,
        )
        .await
        {
            Ok(page) if (200..300).contains(&page.status) => {
                Ok(Robots::parse(&page.body, AGENT_TOKEN))
            }
            // RFC 9309: a server error means "assume everything is disallowed";
            // a missing file (4xx) allows everything.
            Ok(page) if page.status >= 500 => Ok(Robots::disallow_all()),
            Ok(_) => Ok(Robots::allow_all()),
            Err(FetchError::Canceled) => Err(FetchError::Canceled),
            // Unreachable: the page request will fail on its own and say why.
            Err(_) => Ok(Robots::allow_all()),
        }
    }
}

/// The host a page belongs to for "same site": lower case, `www.` removed.
fn site_of(url: &Url) -> String {
    let host = url.host_str().unwrap_or("").to_ascii_lowercase();
    host.strip_prefix("www.")
        .map(str::to_string)
        .unwrap_or(host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_site_ignores_www_and_case() {
        let u = |s: &str| Url::parse(s).unwrap();
        assert_eq!(
            site_of(&u("https://WWW.Example.test/a")),
            site_of(&u("http://example.test/b"))
        );
        assert_ne!(
            site_of(&u("https://cdn.example.test/")),
            site_of(&u("https://example.test/"))
        );
    }

    #[test]
    fn options_have_polite_defaults() {
        let o = GrabOptions::default();
        assert!(o.same_host && o.respect_robots && o.check_sizes);
        assert_eq!(o.delay_ms, 500);
    }
}
