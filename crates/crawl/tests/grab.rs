//! The site grabber against a small site on loopback. `127.0.0.1` and
//! `localhost` reach the same server under two host names, which stands in
//! for "another site".

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use downpipe_crawl::{GrabError, GrabOptions, GrabPhase, grab};
use parking_lot::Mutex;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
struct Hit {
    method: Method,
    path: String,
    cookie: bool,
    at: Instant,
}

#[derive(Default)]
struct Site {
    hits: Mutex<Vec<Hit>>,
    robots: Mutex<Option<String>>,
}

async fn handle(
    State(site): State<Arc<Site>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Response {
    let path = uri.path().to_string();
    site.hits.lock().push(Hit {
        method: method.clone(),
        path: path.clone(),
        cookie: headers.contains_key(header::COOKIE),
        at: Instant::now(),
    });
    let port = headers
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.rsplit(':').next())
        .unwrap_or("80")
        .to_string();
    let html = |body: String| {
        (
            [
                (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                (header::SET_COOKIE, "session=abc; Path=/"),
            ],
            body,
        )
            .into_response()
    };
    match path.as_str() {
        "/robots.txt" => match site.robots.lock().clone() {
            Some(text) => ([(header::CONTENT_TYPE, "text/plain")], text).into_response(),
            None => StatusCode::NOT_FOUND.into_response(),
        },
        "/" => html(format!(
            r#"<a href="a.zip">A</a> <a href="/files/b.pdf">B</a> <a href="docs/">Docs</a>
               <a href="/private/">Private</a> <a href="http://localhost:{port}/other.html">Other</a>
               <a href="http://localhost:{port}/ext.zip">Ext</a> <img src="logo.png"> <a href="a.zip#again">A</a>
               <a href="/missing.html">Missing</a> <a href="/data.json">Data</a>"#
        )),
        "/docs/" => {
            html(r#"<a href="c.zip">C</a><a href="deep.html">Deep</a><a href="/">Home</a>"#.into())
        }
        "/docs/deep.html" => html(r#"<a href="d.zip">D</a>"#.into()),
        "/private/" => html(r#"<a href="secret.zip">S</a>"#.into()),
        "/other.html" => html(r#"<a href="other.zip">O</a>"#.into()),
        "/data.json" => ([(header::CONTENT_TYPE, "application/json")], "{}").into_response(),
        p if p.ends_with(".zip") || p.ends_with(".pdf") || p.ends_with(".png") => {
            let body = vec![b'x'; 1000 + p.len()];
            ([(header::CONTENT_TYPE, "application/octet-stream")], body).into_response()
        }
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn serve(robots: Option<&str>) -> (SocketAddr, Arc<Site>) {
    let site = Arc::new(Site::default());
    *site.robots.lock() = robots.map(str::to_string);
    let app = Router::new().fallback(handle).with_state(site.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (addr, site)
}

fn client() -> reqwest::Client {
    reqwest::Client::builder().no_proxy().build().unwrap()
}

fn options(addr: SocketAddr, depth: u32) -> GrabOptions {
    GrabOptions {
        url: format!("http://{addr}/"),
        depth,
        max_pages: 50,
        extensions: vec!["zip".into(), "PDF".into()],
        delay_ms: 0,
        ..GrabOptions::default()
    }
}

fn names(report: &downpipe_crawl::GrabReport) -> Vec<String> {
    let mut v: Vec<String> = report.files.iter().map(|f| f.name.clone()).collect();
    v.sort();
    v
}

#[tokio::test]
async fn collects_files_of_the_site_within_the_depth() {
    let (addr, site) = serve(Some("User-agent: *\nDisallow: /private/\n")).await;
    let last = Arc::new(Mutex::new(None));
    let seen = last.clone();
    let report = grab(
        &client(),
        options(addr, 2),
        &move |p| *seen.lock() = Some(p.clone()),
        CancellationToken::new(),
    )
    .await
    .unwrap();

    // Files on the other host count; its pages are not followed, the
    // private folder is skipped, deep.html is beyond depth 2.
    assert_eq!(names(&report), ["a.zip", "b.pdf", "c.zip", "ext.zip"]);
    assert!(!report.canceled && !report.hit_page_limit);
    assert_eq!(report.pages, 2, "the start page and docs/");
    assert_eq!(report.blocked, 1);
    assert_eq!(
        report.failed, 1,
        "missing.html (404); data.json is no page to follow"
    );
    let a = report.files.iter().find(|f| f.name == "a.zip").unwrap();
    assert_eq!(a.size, Some(1000 + "/a.zip".len() as u64));
    assert_eq!(a.extension, "zip");
    assert_eq!(a.found_on, format!("http://{addr}/"));
    assert!(report.files.iter().all(|f| f.size.is_some()));

    let hits = site.hits.lock().clone();
    let paths: Vec<&str> = hits.iter().map(|h| h.path.as_str()).collect();
    assert_eq!(paths[0], "/robots.txt", "robots.txt is read first");
    assert!(!paths.contains(&"/private/"), "{paths:?}");
    assert!(!paths.contains(&"/other.html"), "{paths:?}");
    assert!(
        !paths.contains(&"/logo.png"),
        "embedded images are neither files nor pages here"
    );
    assert!(!paths.contains(&"/data.json"));
    assert!(
        hits.iter().all(|h| !h.cookie),
        "no cookie is ever sent back"
    );
    assert!(hits.iter().filter(|h| h.method == Method::HEAD).count() >= 4);
    // robots.txt once per host: 127.0.0.1 and localhost (for ext.zip's size).
    assert_eq!(paths.iter().filter(|p| **p == "/robots.txt").count(), 2);

    let progress = last.lock().clone().unwrap();
    assert_eq!(progress.phase, GrabPhase::Done);
    assert_eq!(progress.files, 4);
    assert_eq!(progress.sized, 4);
}

#[tokio::test]
async fn deeper_levels_other_hosts_and_robots_off() {
    let (addr, _site) = serve(Some("User-agent: *\nDisallow: /private/\n")).await;
    let report = grab(
        &client(),
        options(addr, 3),
        &|_| {},
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(
        names(&report),
        ["a.zip", "b.pdf", "c.zip", "d.zip", "ext.zip"]
    );

    let mut o = options(addr, 2);
    o.respect_robots = false;
    o.same_host = false;
    o.check_sizes = false;
    let report = grab(&client(), o, &|_| {}, CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(
        names(&report),
        [
            "a.zip",
            "b.pdf",
            "c.zip",
            "ext.zip",
            "other.zip",
            "secret.zip"
        ]
    );
    assert!(report.files.iter().all(|f| f.size.is_none()));
}

#[tokio::test]
async fn page_limit_keeps_the_nearest_pages() {
    let (addr, _site) = serve(None).await;
    let mut o = options(addr, 5);
    o.max_pages = 1;
    let report = grab(&client(), o, &|_| {}, CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(names(&report), ["a.zip", "b.pdf", "ext.zip"]);
    assert!(report.hit_page_limit);
    assert_eq!(report.pages, 1);
}

#[tokio::test]
async fn waits_between_requests_to_a_host() {
    let (addr, site) = serve(None).await;
    let mut o = options(addr, 2);
    o.delay_ms = 150;
    o.check_sizes = false;
    grab(&client(), o, &|_| {}, CancellationToken::new())
        .await
        .unwrap();
    let hits = site.hits.lock().clone();
    assert!(hits.len() >= 4);
    for pair in hits.windows(2) {
        let gap = pair[1].at - pair[0].at;
        assert!(
            gap >= Duration::from_millis(140),
            "{} -> {} after {gap:?}",
            pair[0].path,
            pair[1].path
        );
    }
}

#[tokio::test]
async fn cancelling_returns_what_was_found() {
    let (addr, _site) = serve(None).await;
    let cancel = CancellationToken::new();
    let stop = cancel.clone();
    let mut o = options(addr, 3);
    o.delay_ms = 2000;
    let task = tokio::spawn(async move {
        grab(
            &client(),
            o,
            &move |p| {
                if p.pages_done >= 1 {
                    stop.cancel();
                }
            },
            cancel,
        )
        .await
    });
    let report = tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(report.canceled);
    assert_eq!(report.pages, 1);
    assert_eq!(names(&report), ["a.zip", "b.pdf", "ext.zip"]);
    assert!(report.files.iter().all(|f| f.size.is_none()));
}

#[tokio::test]
async fn start_page_problems_are_errors() {
    let (addr, _site) = serve(Some("User-agent: downpipe\nDisallow: /docs/\n")).await;
    let run = |url: String, exts: Vec<String>| {
        let o = GrabOptions {
            url,
            extensions: exts,
            delay_ms: 0,
            ..GrabOptions::default()
        };
        async move { grab(&client(), o, &|_| {}, CancellationToken::new()).await }
    };
    let zip = || vec!["zip".to_string()];
    assert!(matches!(
        run(format!("http://{addr}/nope.html"), zip()).await,
        Err(GrabError::StartPage(_))
    ));
    assert!(matches!(
        run(format!("http://{addr}/docs/"), zip()).await,
        Err(GrabError::Blocked)
    ));
    assert!(matches!(
        run(format!("http://{addr}/data.json"), zip()).await,
        Err(GrabError::NotHtml)
    ));
    assert!(matches!(
        run("ftp://x.test/".into(), zip()).await,
        Err(GrabError::InvalidUrl(_))
    ));
    assert!(matches!(
        run(format!("http://{addr}/"), vec![]).await,
        Err(GrabError::NoTypes)
    ));
}
