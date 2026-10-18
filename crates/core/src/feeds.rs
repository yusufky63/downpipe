//! RSS and Atom subscriptions: feeds checked at an interval whose new
//! enclosures and file links are added to a queue.
//!
//! Each feed remembers every item it has seen (by the item's id and by the
//! links it offered) in `feeds.db`, next to the download catalog but in a
//! file of its own, so an item is added once and never again: not when the
//! download is removed from the list, not when the feed reorders or
//! republishes its items, not after a restart. Items the filter leaves out
//! are remembered too, so changing the filter applies to new items only.
//!
//! A new subscription remembers what the feed holds already without adding
//! it, unless the user asks to download those items too.

use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use downpipe_crawl::feed::{CompiledFilter, FeedError, FeedFilter, ParsedFeed, fetch_feed};
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::backup::BackupFeed;
use crate::manager::{CoreError, Manager};
use crate::settings::MAIN_QUEUE;
use crate::task::{AddOptions, TaskId};

const DB_FILE: &str = "feeds.db";
pub const MIN_INTERVAL_MINUTES: u32 = 5;
pub const MAX_INTERVAL_MINUTES: u32 = 7 * 24 * 60;
pub const DEFAULT_INTERVAL_MINUTES: u32 = 60;
/// How often the loop looks for feeds that are due.
const TICK: Duration = Duration::from_secs(60);

/// A subscription as the UI shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Feed {
    pub id: String,
    pub url: String,
    /// The feed's own title, from the last successful check.
    pub title: String,
    pub filter: FeedFilter,
    pub interval_minutes: u32,
    /// Queue new downloads go to.
    pub queue: String,
    pub enabled: bool,
    pub created_at: i64,
    pub last_checked: Option<i64>,
    /// Why the last check failed; cleared by a successful one.
    pub last_error: Option<String>,
    /// Downloads added by the last check that added any.
    pub last_added: u32,
    pub added_total: u64,
}

/// What the user enters to subscribe or to change a subscription.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct FeedInput {
    pub url: String,
    pub filter: FeedFilter,
    pub interval_minutes: u32,
    pub queue: String,
    pub enabled: bool,
    /// When subscribing: also add the matching items the feed holds now.
    pub download_existing: bool,
}

impl Default for FeedInput {
    fn default() -> Self {
        Self {
            url: String::new(),
            filter: FeedFilter::default(),
            interval_minutes: DEFAULT_INTERVAL_MINUTES,
            queue: MAIN_QUEUE.into(),
            enabled: true,
            download_existing: false,
        }
    }
}

/// What importing a backup's feeds did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct FeedImport {
    pub added: usize,
    /// Already subscribed to.
    pub duplicates: usize,
    /// Refused by the checks, or not readable now.
    pub failed: usize,
}

/// What one check did.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct FeedCheck {
    /// Items in the feed.
    pub items: usize,
    /// Downloads added, in the order they were queued.
    pub added: Vec<TaskId>,
}

#[derive(Debug, thiserror::Error)]
pub enum FeedsError {
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error(transparent)]
    Feed(#[from] FeedError),
    #[error("database error: {0}")]
    Store(#[from] rusqlite::Error),
    #[error("no feed with id {0}")]
    NotFound(String),
    #[error("this feed is already in the list")]
    Duplicate(String),
    #[error("not an http or https address: {0}")]
    InvalidUrl(String),
    #[error("this feed is being checked already")]
    Busy,
}

impl FeedsError {
    /// Stable name UIs branch on.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Core(e) => e.code(),
            Self::Feed(e) => e.code(),
            Self::Store(_) => "internal",
            Self::NotFound(_) => "not_found",
            Self::Duplicate(_) => "duplicate_feed",
            Self::InvalidUrl(_) => "invalid_url",
            Self::Busy => "busy",
        }
    }
}

/// The subscriptions and the loop that checks them.
#[derive(Debug, Clone)]
pub struct Feeds {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    manager: Manager,
    store: Mutex<Store>,
    events: broadcast::Sender<Vec<Feed>>,
    /// Feeds whose check is running, so a manual check and the loop do not
    /// add the same item twice.
    checking: Mutex<HashSet<String>>,
    stop: CancellationToken,
}

impl Feeds {
    /// Open (or create) `feeds.db` in `data_dir`.
    pub fn open(data_dir: &Path, manager: Manager) -> Result<Self, FeedsError> {
        let _ = std::fs::create_dir_all(data_dir);
        Self::with_store(
            Store::open(Connection::open(data_dir.join(DB_FILE))?)?,
            manager,
        )
    }

    /// A store that lives in memory only, for tests.
    pub fn in_memory(manager: Manager) -> Result<Self, FeedsError> {
        Self::with_store(Store::open(Connection::open_in_memory()?)?, manager)
    }

    fn with_store(store: Store, manager: Manager) -> Result<Self, FeedsError> {
        let (events, _) = broadcast::channel(16);
        Ok(Self {
            inner: Arc::new(Inner {
                manager,
                store: Mutex::new(store),
                events,
                checking: Mutex::new(HashSet::new()),
                stop: CancellationToken::new(),
            }),
        })
    }

    /// The whole list after every change.
    pub fn subscribe(&self) -> broadcast::Receiver<Vec<Feed>> {
        self.inner.events.subscribe()
    }

    pub fn list(&self) -> Vec<Feed> {
        self.inner.store.lock().all().unwrap_or_else(|e| {
            tracing::warn!("could not read the feeds: {e}");
            Vec::new()
        })
    }

    fn changed(&self) {
        let _ = self.inner.events.send(self.list());
    }

    /// Subscribe: read the feed once (so a mistyped address or a web page is
    /// refused right away), then remember or add what it holds.
    pub async fn add(&self, input: FeedInput) -> Result<(Feed, FeedCheck), FeedsError> {
        let input = self.clean(input)?;
        let filter = input.filter.compile()?;
        if self.list().iter().any(|f| same_url(&f.url, &input.url)) {
            return Err(FeedsError::Duplicate(input.url));
        }
        let parsed = fetch_feed(
            &self.inner.manager.http_client(),
            &input.url,
            &self.inner.stop,
        )
        .await?;
        let now = unix_now();
        let mut feed = Feed {
            id: uuid::Uuid::new_v4().simple().to_string(),
            url: input.url.clone(),
            title: parsed.title.clone(),
            filter: input.filter.clone(),
            interval_minutes: input.interval_minutes,
            queue: input.queue.clone(),
            enabled: input.enabled,
            created_at: now,
            last_checked: Some(now),
            last_error: None,
            last_added: 0,
            added_total: 0,
        };
        self.inner.store.lock().upsert(&feed)?;
        let check = self.take_items(&mut feed, &parsed, &filter, input.download_existing)?;
        self.changed();
        Ok((feed, check))
    }

    /// Change a subscription. What it has seen stays seen.
    pub fn update(&self, id: &str, input: FeedInput) -> Result<Feed, FeedsError> {
        let input = self.clean(input)?;
        input.filter.compile()?;
        let mut feed = self.get(id)?;
        if self
            .list()
            .iter()
            .any(|f| f.id != id && same_url(&f.url, &input.url))
        {
            return Err(FeedsError::Duplicate(input.url));
        }
        feed.url = input.url;
        feed.filter = input.filter;
        feed.interval_minutes = input.interval_minutes;
        feed.queue = input.queue;
        feed.enabled = input.enabled;
        self.inner.store.lock().upsert(&feed)?;
        self.changed();
        Ok(feed)
    }

    /// Unsubscribe and forget what the feed had seen. Downloads it added
    /// stay in the list.
    pub fn remove(&self, id: &str) -> Result<(), FeedsError> {
        if !self.inner.store.lock().delete(id)? {
            return Err(FeedsError::NotFound(id.into()));
        }
        self.changed();
        Ok(())
    }

    /// Subscribe to the feeds of a backup, one after the other. A feed this
    /// list has already is skipped, and one that cannot be read now is left
    /// out and counted. As when subscribing, each feed's current items are
    /// remembered rather than downloaded.
    pub async fn import(&self, feeds: Vec<BackupFeed>) -> FeedImport {
        let mut out = FeedImport::default();
        for feed in feeds {
            let feed = match feed.validated() {
                Ok(feed) => feed,
                Err(why) => {
                    tracing::debug!("feed import: skipped a feed: {why}");
                    out.failed += 1;
                    continue;
                }
            };
            if self.list().iter().any(|f| same_url(&f.url, &feed.url)) {
                out.duplicates += 1;
                continue;
            }
            let input = FeedInput {
                url: feed.url,
                filter: feed.filter,
                interval_minutes: feed.interval_minutes,
                queue: feed.queue,
                enabled: feed.enabled,
                download_existing: false,
            };
            match self.add(input).await {
                Ok(_) => out.added += 1,
                Err(FeedsError::Duplicate(_)) => out.duplicates += 1,
                Err(err) => {
                    tracing::warn!("feed import: could not subscribe: {err}");
                    out.failed += 1;
                }
            }
        }
        out
    }

    pub fn get(&self, id: &str) -> Result<Feed, FeedsError> {
        self.inner
            .store
            .lock()
            .get(id)?
            .ok_or_else(|| FeedsError::NotFound(id.into()))
    }

    /// Check one feed now and add its new items.
    pub async fn check(&self, id: &str) -> Result<FeedCheck, FeedsError> {
        if !self.inner.checking.lock().insert(id.to_string()) {
            return Err(FeedsError::Busy);
        }
        let result = self.check_inner(id).await;
        self.inner.checking.lock().remove(id);
        result
    }

    async fn check_inner(&self, id: &str) -> Result<FeedCheck, FeedsError> {
        let mut feed = self.get(id)?;
        let fetched = match feed.filter.compile() {
            Ok(filter) => fetch_feed(
                &self.inner.manager.http_client(),
                &feed.url,
                &self.inner.stop,
            )
            .await
            .map(|parsed| (filter, parsed)),
            Err(e) => Err(e),
        };
        feed.last_checked = Some(unix_now());
        let (filter, parsed) = match fetched {
            Ok(ok) => ok,
            Err(FeedError::Canceled) => return Err(FeedError::Canceled.into()),
            Err(err) => {
                feed.last_error = Some(err.to_string());
                // The feed may have been removed while it was being read.
                if self.inner.store.lock().get(id)?.is_some() {
                    self.inner.store.lock().upsert(&feed)?;
                    self.changed();
                }
                return Err(err.into());
            }
        };
        if self.inner.store.lock().get(id)?.is_none() {
            return Err(FeedsError::NotFound(id.into()));
        }
        feed.last_error = None;
        if !parsed.title.is_empty() {
            feed.title = parsed.title.clone();
        }
        let check = self.take_items(&mut feed, &parsed, &filter, true)?;
        self.changed();
        Ok(check)
    }

    /// Check every enabled feed whose interval has passed. Failures are
    /// recorded on the feed and do not stop the others.
    pub async fn check_due(&self) {
        let now = unix_now();
        let due: Vec<String> = self
            .list()
            .into_iter()
            .filter(|f| {
                f.enabled
                    && f.last_checked
                        .is_none_or(|t| now - t >= i64::from(f.interval_minutes) * 60)
            })
            .map(|f| f.id)
            .collect();
        for id in due {
            if self.inner.stop.is_cancelled() {
                return;
            }
            match self.check(&id).await {
                Ok(check) if !check.added.is_empty() => {
                    tracing::info!(feed = %id, added = check.added.len(), "feed added downloads");
                }
                Ok(_) | Err(FeedsError::Busy) => {}
                Err(err) => tracing::info!(feed = %id, "feed check failed: {err}"),
            }
        }
    }

    /// Check due feeds every minute until [`stop`](Self::stop).
    pub async fn run(self) {
        loop {
            tokio::select! {
                _ = self.inner.stop.cancelled() => return,
                _ = tokio::time::sleep(TICK) => {}
            }
            self.check_due().await;
        }
    }

    pub fn stop(&self) {
        self.inner.stop.cancel();
    }

    fn clean(&self, mut input: FeedInput) -> Result<FeedInput, FeedsError> {
        input.url = input.url.trim().to_string();
        let parsed =
            url::Url::parse(&input.url).map_err(|_| FeedsError::InvalidUrl(input.url.clone()))?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
            return Err(FeedsError::InvalidUrl(input.url));
        }
        input.interval_minutes = input
            .interval_minutes
            .clamp(MIN_INTERVAL_MINUTES, MAX_INTERVAL_MINUTES);
        input.filter.extensions = downpipe_crawl::normalize_extensions(&input.filter.extensions);
        input.filter.title_pattern = input.filter.title_pattern.trim().to_string();
        if self.inner.manager.settings().queue(&input.queue).is_none() {
            input.queue = MAIN_QUEUE.into();
        }
        Ok(input)
    }

    /// Remember the feed's items, adding the new ones that pass the filter
    /// when `add_new` is set. Oldest first, so a backlog queues in order.
    fn take_items(
        &self,
        feed: &mut Feed,
        parsed: &ParsedFeed,
        filter: &CompiledFilter,
        add_new: bool,
    ) -> Result<FeedCheck, FeedsError> {
        let mut check = FeedCheck {
            items: parsed.items.len(),
            added: Vec::new(),
        };
        for item in parsed.items.iter().rev() {
            let links = filter.pick(item);
            let mut keys: Vec<String> = Vec::new();
            if !item.id.trim().is_empty() {
                keys.push(format!("item:{}", item.id.trim()));
            }
            // Every link the item offers, filtered out or not, so a feed that
            // renames its items' ids still finds them seen.
            for link in item
                .enclosures
                .iter()
                .map(|e| e.url.as_str())
                .chain(item.link.as_deref())
            {
                keys.push(format!("url:{link}"));
            }
            if keys.is_empty() {
                continue;
            }
            let seen = self.inner.store.lock().any_seen(&feed.id, &keys)?;
            if !seen && add_new {
                for link in &links {
                    match self.inner.manager.add(AddOptions {
                        url: link.clone(),
                        queue: Some(feed.queue.clone()),
                        ..AddOptions::default()
                    }) {
                        Ok(task) => check.added.push(task.record.id),
                        // Already in the list (added by hand, or by another feed).
                        Err(CoreError::Duplicate(_)) => {}
                        Err(err) => tracing::info!(feed = %feed.id, "feed link not added: {err}"),
                    }
                }
            }
            self.inner
                .store
                .lock()
                .mark_seen(&feed.id, &keys, unix_now())?;
        }
        if !check.added.is_empty() {
            feed.last_added = check.added.len() as u32;
            feed.added_total += check.added.len() as u64;
        }
        self.inner.store.lock().upsert(feed)?;
        Ok(check)
    }
}

/// Two addresses of the same feed: case of the scheme and host, and a
/// trailing slash, aside.
fn same_url(a: &str, b: &str) -> bool {
    match (url::Url::parse(a), url::Url::parse(b)) {
        (Ok(a), Ok(b)) => a.as_str().trim_end_matches('/') == b.as_str().trim_end_matches('/'),
        _ => a == b,
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

#[derive(Debug)]
struct Store {
    conn: Connection,
}

impl Store {
    fn open(conn: Connection) -> rusqlite::Result<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS feeds (
                id         TEXT PRIMARY KEY,
                created_at INTEGER NOT NULL,
                data       TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS feed_seen (
                feed_id TEXT NOT NULL,
                key     TEXT NOT NULL,
                seen_at INTEGER NOT NULL,
                PRIMARY KEY (feed_id, key)
             ) WITHOUT ROWID;",
        )?;
        Ok(Self { conn })
    }

    fn all(&self) -> rusqlite::Result<Vec<Feed>> {
        let mut stmt = self
            .conn
            .prepare("SELECT data FROM feeds ORDER BY created_at, id")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        let mut out = Vec::new();
        for row in rows {
            match serde_json::from_str(&row?) {
                Ok(feed) => out.push(feed),
                Err(e) => tracing::warn!("skipping an unreadable feed: {e}"),
            }
        }
        Ok(out)
    }

    fn get(&self, id: &str) -> rusqlite::Result<Option<Feed>> {
        let data: Option<String> = self
            .conn
            .query_row("SELECT data FROM feeds WHERE id = ?1", [id], |r| r.get(0))
            .optional()?;
        Ok(data.and_then(|d| serde_json::from_str(&d).ok()))
    }

    fn upsert(&self, feed: &Feed) -> rusqlite::Result<()> {
        let data = serde_json::to_string(feed)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        self.conn.execute(
            "INSERT INTO feeds (id, created_at, data) VALUES (?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET data = excluded.data",
            params![feed.id, feed.created_at, data],
        )?;
        Ok(())
    }

    fn delete(&self, id: &str) -> rusqlite::Result<bool> {
        self.conn
            .execute("DELETE FROM feed_seen WHERE feed_id = ?1", [id])?;
        Ok(self.conn.execute("DELETE FROM feeds WHERE id = ?1", [id])? > 0)
    }

    fn any_seen(&self, feed_id: &str, keys: &[String]) -> rusqlite::Result<bool> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT 1 FROM feed_seen WHERE feed_id = ?1 AND key = ?2")?;
        for key in keys {
            if stmt.exists(params![feed_id, key])? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn mark_seen(&self, feed_id: &str, keys: &[String], at: i64) -> rusqlite::Result<()> {
        let mut stmt = self.conn.prepare_cached(
            "INSERT OR IGNORE INTO feed_seen (feed_id, key, seen_at) VALUES (?1, ?2, ?3)",
        )?;
        for key in keys {
            stmt.execute(params![feed_id, key, at])?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_feed_addresses() {
        assert!(same_url("https://Pod.test/feed/", "https://pod.test/feed"));
        assert!(!same_url("https://pod.test/a", "https://pod.test/b"));
    }

    #[test]
    fn store_round_trip_and_seen_keys() {
        let store = Store::open(Connection::open_in_memory().unwrap()).unwrap();
        let feed = Feed {
            id: "f1".into(),
            url: "https://pod.test/feed".into(),
            title: "Pod".into(),
            filter: FeedFilter::default(),
            interval_minutes: 60,
            queue: "main".into(),
            enabled: true,
            created_at: 1,
            last_checked: None,
            last_error: None,
            last_added: 0,
            added_total: 0,
        };
        store.upsert(&feed).unwrap();
        assert_eq!(store.all().unwrap(), std::slice::from_ref(&feed));
        let keys = vec!["item:a".to_string(), "url:x".to_string()];
        assert!(!store.any_seen("f1", &keys).unwrap());
        store.mark_seen("f1", &keys[1..], 5).unwrap();
        assert!(store.any_seen("f1", &keys).unwrap());
        assert!(!store.any_seen("f2", &keys).unwrap());
        assert!(store.delete("f1").unwrap());
        assert!(!store.any_seen("f1", &keys).unwrap());
        assert!(store.get("f1").unwrap().is_none());
    }
}
