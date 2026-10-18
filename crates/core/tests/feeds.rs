//! Feed subscriptions against a feed served on loopback: new items are
//! added to the chosen queue once, and never again.

use std::path::Path;
use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use downpipe_core::crawl::FeedFilter;
use downpipe_core::feeds::{FeedInput, Feeds, FeedsError};
use downpipe_core::{Manager, Queue, Settings};
use parking_lot::Mutex;

#[derive(Default)]
struct Server {
    feed: Mutex<String>,
    base: Mutex<String>,
}

async fn handle(State(s): State<Arc<Server>>, uri: Uri) -> Response {
    match uri.path() {
        "/feed.xml" => (
            [(header::CONTENT_TYPE, "application/rss+xml")],
            s.feed.lock().clone(),
        )
            .into_response(),
        "/page.html" => (
            [(header::CONTENT_TYPE, "text/html")],
            "<html><body>hi</body></html>",
        )
            .into_response(),
        p if p.starts_with("/media/") => {
            ([(header::CONTENT_TYPE, "audio/mpeg")], vec![7u8; 2048]).into_response()
        }
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn serve() -> Arc<Server> {
    let state = Arc::new(Server::default());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    *state.base.lock() = format!("http://{}", listener.local_addr().unwrap());
    let app = Router::new().fallback(handle).with_state(state.clone());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    state
}

/// An RSS feed with one item per `(guid, title, file)`, newest first.
fn rss(base: &str, items: &[(&str, &str, &str)]) -> String {
    let items: String = items
        .iter()
        .map(|(guid, title, file)| {
            format!(
                "<item><title>{title}</title><guid>{guid}</guid><enclosure url=\"{base}/media/{file}\" type=\"audio/mpeg\" length=\"2048\"/></item>"
            )
        })
        .collect();
    format!(
        "<?xml version=\"1.0\"?><rss version=\"2.0\"><channel><title>Test cast</title>{items}</channel></rss>"
    )
}

async fn manager(data: &Path, downloads: &Path) -> Manager {
    let m = Manager::open(data).await.unwrap();
    let mut queues = m.settings().queues;
    queues.push(Queue {
        id: String::new(),
        name: "Podcasts".into(),
        max_concurrent: 1,
    });
    m.update_settings(Settings {
        download_dir: downloads.to_path_buf(),
        queues,
        ..m.settings()
    })
    .unwrap();
    m
}

fn urls(m: &Manager) -> Vec<String> {
    let mut v: Vec<String> = m.list().into_iter().map(|t| t.record.url).collect();
    v.sort();
    v
}

#[tokio::test]
async fn new_items_are_added_once_to_the_chosen_queue() {
    let server = serve().await;
    let base = server.base.lock().clone();
    *server.feed.lock() = rss(
        &base,
        &[("e2", "Episode 2", "e2.mp3"), ("e1", "Episode 1", "e1.mp3")],
    );
    let tmp = tempfile::tempdir().unwrap();
    let m = manager(&tmp.path().join("data"), &tmp.path().join("dl")).await;
    let queue = m
        .settings()
        .queues
        .iter()
        .find(|q| q.name == "Podcasts")
        .unwrap()
        .id
        .clone();
    let feeds = Feeds::open(&tmp.path().join("data"), m.clone()).unwrap();
    let mut events = feeds.subscribe();

    // Subscribing remembers what is there without downloading it.
    let (feed, first) = feeds
        .add(FeedInput {
            url: format!("{base}/feed.xml"),
            queue: queue.clone(),
            ..FeedInput::default()
        })
        .await
        .unwrap();
    assert_eq!(feed.title, "Test cast");
    assert_eq!(first.items, 2);
    assert!(first.added.is_empty());
    assert!(m.list().is_empty());
    assert_eq!(events.recv().await.unwrap().len(), 1);

    // A new item comes in: only it is added, to the feed's queue.
    *server.feed.lock() = rss(
        &base,
        &[
            ("e3", "Episode 3", "e3.mp3"),
            ("e2", "Episode 2", "e2.mp3"),
            ("e1", "Episode 1", "e1.mp3"),
        ],
    );
    let check = feeds.check(&feed.id).await.unwrap();
    assert_eq!(check.added.len(), 1);
    assert_eq!(urls(&m), [format!("{base}/media/e3.mp3")]);
    assert!(m.list().iter().all(|t| t.record.queue == queue));
    let stored = feeds.get(&feed.id).unwrap();
    assert_eq!((stored.last_added, stored.added_total), (1, 1));
    assert!(stored.last_checked.is_some() && stored.last_error.is_none());

    // Removed from the list, it still is not added again.
    let id = check.added[0].clone();
    m.remove(&id, true).unwrap();
    // Removing a running download finishes once its job has stopped.
    for _ in 0..100 {
        if m.list().is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert!(feeds.check(&feed.id).await.unwrap().added.is_empty());
    assert!(m.list().is_empty());

    // Remembered across a restart of the feeds (same feeds.db).
    drop(feeds);
    let feeds = Feeds::open(&tmp.path().join("data"), m.clone()).unwrap();
    assert!(feeds.check(&feed.id).await.unwrap().added.is_empty());

    // Even when the feed changes the item's id, its file is known.
    *server.feed.lock() = rss(&base, &[("new-guid-for-e3", "Episode 3", "e3.mp3")]);
    assert!(feeds.check(&feed.id).await.unwrap().added.is_empty());
    m.shutdown().await;
}

#[tokio::test]
async fn filters_backlog_and_errors() {
    let server = serve().await;
    let base = server.base.lock().clone();
    *server.feed.lock() = rss(
        &base,
        &[
            ("c", "Bonus: outtakes", "c.mp3"),
            ("b", "Episode 2", "b.zip"),
            ("a", "Episode 1", "a.mp3"),
        ],
    );
    let tmp = tempfile::tempdir().unwrap();
    let m = manager(&tmp.path().join("data"), &tmp.path().join("dl")).await;
    let feeds = Feeds::in_memory(m.clone()).unwrap();

    // Download what is there now too, but only episodes in mp3.
    let filter = FeedFilter {
        extensions: vec!["mp3".into()],
        title_pattern: "^episode".into(),
    };
    let (feed, check) = feeds
        .add(FeedInput {
            url: format!("{base}/feed.xml"),
            filter,
            download_existing: true,
            queue: "nope".into(),
            ..FeedInput::default()
        })
        .await
        .unwrap();
    assert_eq!(check.added.len(), 1);
    assert_eq!(urls(&m), [format!("{base}/media/a.mp3")]);
    assert_eq!(
        feed.queue, "main",
        "an unknown queue falls back to the main queue"
    );

    // Items the filter left out are remembered: widening it applies to new items only.
    let mut input = FeedInput {
        url: feed.url.clone(),
        ..FeedInput::default()
    };
    input.filter = FeedFilter::default();
    feeds.update(&feed.id, input).unwrap();
    assert!(feeds.check(&feed.id).await.unwrap().added.is_empty());

    // The same address twice, a web page and a bad pattern are refused.
    let again = feeds
        .add(FeedInput {
            url: format!("{base}/feed.xml/"),
            ..FeedInput::default()
        })
        .await;
    assert!(matches!(again, Err(FeedsError::Duplicate(_))));
    let page = feeds
        .add(FeedInput {
            url: format!("{base}/page.html"),
            ..FeedInput::default()
        })
        .await
        .unwrap_err();
    assert_eq!(page.code(), "not_a_feed");
    let bad = FeedInput {
        url: format!("{base}/other.xml"),
        filter: FeedFilter {
            extensions: vec![],
            title_pattern: "(".into(),
        },
        ..FeedInput::default()
    };
    assert_eq!(feeds.add(bad).await.unwrap_err().code(), "invalid_pattern");
    assert_eq!(
        feeds
            .add(FeedInput {
                url: "ftp://x/".into(),
                ..FeedInput::default()
            })
            .await
            .unwrap_err()
            .code(),
        "invalid_url"
    );

    // A failing check is recorded on the feed and cleared by the next good one.
    let saved = server.feed.lock().clone();
    *server.feed.lock() = "<html>oops</html>".into();
    assert!(feeds.check(&feed.id).await.is_err());
    assert!(feeds.get(&feed.id).unwrap().last_error.is_some());
    *server.feed.lock() = saved;
    feeds.check(&feed.id).await.unwrap();
    assert!(feeds.get(&feed.id).unwrap().last_error.is_none());

    // Due feeds only: a feed checked a moment ago waits for its interval.
    let before = m.list().len();
    *server.feed.lock() = rss(&base, &[("z", "Episode 9", "z.mp3")]);
    feeds.check_due().await;
    assert_eq!(m.list().len(), before);

    feeds.remove(&feed.id).unwrap();
    assert!(feeds.list().is_empty());
    assert!(matches!(
        feeds.remove(&feed.id),
        Err(FeedsError::NotFound(_))
    ));
    m.shutdown().await;
}

#[tokio::test]
async fn feeds_from_a_backup_are_subscribed_once_without_downloading_their_items() {
    use downpipe_core::backup::BackupFeed;

    let server = serve().await;
    let base = server.base.lock().clone();
    *server.feed.lock() = rss(&base, &[("e1", "Episode 1", "e1.mp3")]);
    let tmp = tempfile::tempdir().unwrap();
    let m = manager(&tmp.path().join("data"), &tmp.path().join("dl")).await;
    let feeds = Feeds::in_memory(m.clone()).unwrap();
    let from_backup = |url: String| BackupFeed {
        url,
        filter: FeedFilter::default(),
        interval_minutes: 1,
        queue: "no-such-queue".into(),
        enabled: true,
    };
    let report = feeds
        .import(vec![
            from_backup(format!("{base}/feed.xml")),
            // Twice in the file, and one that is no feed.
            from_backup(format!("{base}/feed.xml/")),
            from_backup(format!("{base}/page.html")),
            from_backup("file:///etc/passwd".into()),
        ])
        .await;
    assert_eq!((report.added, report.duplicates, report.failed), (1, 1, 2));
    let list = feeds.list();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].queue, "main", "an unknown queue falls back to main");
    assert_eq!(
        list[0].interval_minutes,
        downpipe_core::feeds::MIN_INTERVAL_MINUTES
    );
    assert!(
        m.list().is_empty(),
        "the feed's items are remembered, not downloaded"
    );

    let again = feeds
        .import(vec![from_backup(format!("{base}/feed.xml"))])
        .await;
    assert_eq!((again.added, again.duplicates), (0, 1));
}
