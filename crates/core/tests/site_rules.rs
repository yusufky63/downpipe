//! Site rules: what a rule changes for the downloads it matches, when it is
//! looked up, and the hosts the engine offers as rules.

use std::path::Path;
use std::time::{Duration, Instant};

use downpipe_core::http::{EngineConfig, KIB, MIB};
use downpipe_core::{
    AddOptions, CoreError, Manager, Queue, RefusedHost, RuleProxy, Settings, SiteRule,
    TaskSnapshot, TaskState,
};
use downpipe_test_server::{TestServer, content};

fn config() -> EngineConfig {
    EngineConfig {
        max_connections: 2,
        initial_connections: 2,
        min_split_size: 64 * KIB,
        checkpoint_interval: Duration::from_millis(100),
        retry_base_delay: Duration::from_millis(50),
        retry_max_delay: Duration::from_millis(200),
        ..EngineConfig::default()
    }
}

async fn open(data: &Path, downloads: &Path) -> Manager {
    let manager = Manager::open_with(data, config()).await.expect("open");
    manager
        .update_settings(Settings {
            download_dir: downloads.to_path_buf(),
            ..manager.settings()
        })
        .unwrap();
    manager
}

fn rule(pattern: &str) -> SiteRule {
    SiteRule {
        pattern: pattern.into(),
        ..SiteRule::default()
    }
}

fn set_rules(manager: &Manager, rules: Vec<SiteRule>) -> Settings {
    manager
        .update_settings(Settings {
            site_rules: rules,
            ..manager.settings()
        })
        .expect("rules accepted")
}

async fn wait_for(
    manager: &Manager,
    id: &str,
    cond: impl Fn(&TaskSnapshot) -> bool,
) -> TaskSnapshot {
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            if let Some(task) = manager.get(id)
                && cond(&task)
            {
                return task;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("not reached; last: {:?}", manager.get(id)))
}

fn done(t: &TaskSnapshot) -> bool {
    t.record.state == TaskState::Completed
}

#[tokio::test]
async fn a_rule_sets_headers_connections_and_skips_the_proxy() {
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let manager = open(data.path(), out.path()).await;
    // Nobody listens there: only a rule without a proxy gets through.
    manager
        .update_settings(Settings {
            proxy: "http://127.0.0.1:9".into(),
            max_connections_per_download: 8,
            ..manager.settings()
        })
        .unwrap();
    set_rules(
        &manager,
        vec![SiteRule {
            max_connections: Some(1),
            user_agent: "RuleAgent/1.0".into(),
            referer: "https://pages.example/download".into(),
            proxy: RuleProxy::Direct,
            ..rule("127.0.0.1")
        }],
    );

    let size = 2 * MIB;
    let task = manager
        .add(AddOptions {
            url: server.url(&format!("/file/{size}")),
            // What the browser passed on is replaced by the rule's.
            headers: vec![("referer".into(), "https://browser.example/".into())],
            ..AddOptions::default()
        })
        .unwrap();
    assert_eq!(task.record.site_rule.as_deref(), Some("127.0.0.1"));
    let finished = wait_for(&manager, &task.record.id, done).await;
    let path = finished.record.path.unwrap();
    assert_eq!(std::fs::read(path).unwrap(), content(size, 0));
    assert_eq!(finished.record.site_rule.as_deref(), Some("127.0.0.1"));

    let requests = server.state.requests();
    assert!(!requests.is_empty());
    for r in &requests {
        assert_eq!(r.user_agent.as_deref(), Some("RuleAgent/1.0"));
        assert_eq!(r.referer.as_deref(), Some("https://pages.example/download"));
    }
    assert_eq!(
        server.state.max_concurrent(),
        1,
        "the rule's one connection"
    );

    // Without the rule, the dead proxy is used again.
    set_rules(&manager, Vec::new());
    let task = manager
        .add(AddOptions {
            url: server.url("/file/1000?again"),
            ..AddOptions::default()
        })
        .unwrap();
    assert_eq!(task.record.site_rule, None);
    let failed = wait_for(&manager, &task.record.id, |t| {
        t.record.state == TaskState::Failed
    })
    .await;
    assert_eq!(failed.record.error_kind.as_deref(), Some("network"));
}

#[tokio::test]
async fn new_downloads_take_the_folder_and_queue_of_the_most_specific_rule() {
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let manager = open(data.path(), out.path()).await;
    let mut settings = manager.settings();
    settings.queues.push(Queue {
        id: "night".into(),
        name: "Night".into(),
        max_concurrent: 1,
    });
    let absolute = out.path().join("Mirrors");
    settings.site_rules = vec![
        SiteRule {
            download_dir: "Example".into(),
            ..rule("*.example.com")
        },
        SiteRule {
            download_dir: absolute.to_string_lossy().into_owned(),
            queue: "night".into(),
            ..rule("dl.example.com")
        },
    ];
    manager.update_settings(settings).unwrap();

    let add = |url: &str, dir: Option<&Path>| {
        manager
            .add(AddOptions {
                url: url.into(),
                dir: dir.map(Path::to_path_buf),
                paused: true,
                ..AddOptions::default()
            })
            .unwrap()
            .record
    };
    let exact = add("https://dl.example.com/disk.iso", None);
    assert_eq!(exact.dir, absolute);
    assert_eq!(exact.queue, "night");
    assert!(!exact.auto_dir, "a rule's folder is not moved by category");
    assert_eq!(exact.site_rule.as_deref(), Some("dl.example.com"));

    let domain = add("https://www.example.com/disk.iso", None);
    assert_eq!(
        domain.dir,
        out.path().join("Example"),
        "relative to downloads"
    );
    assert_eq!(domain.queue, "main");
    assert_eq!(domain.site_rule.as_deref(), Some("*.example.com"));

    let chosen = tempfile::tempdir().unwrap();
    let given = add("https://dl.example.com/other.iso", Some(chosen.path()));
    assert_eq!(given.dir, chosen.path(), "a folder chosen for it wins");

    let none = add("https://example.org/disk.iso", None);
    assert_eq!(none.dir, out.path().join("Disk images"));
    assert_eq!(none.site_rule, None);

    // Removing the queue leaves the rule without one.
    let mut settings = manager.settings();
    settings.queues.retain(|q| q.id != "night");
    let saved = manager.update_settings(settings).unwrap();
    assert_eq!(saved.site_rules[1].queue, "");
}

#[tokio::test]
async fn a_rule_changed_while_paused_applies_to_the_resume() {
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let manager = open(data.path(), out.path()).await;
    set_rules(
        &manager,
        vec![SiteRule {
            id: "local".into(),
            user_agent: "First/1.0".into(),
            ..rule("127.0.0.1")
        }],
    );
    let size = 4 * MIB;
    let task = manager
        .add(AddOptions {
            url: server.url(&format!("/file/{size}?rate=1048576")),
            ..AddOptions::default()
        })
        .unwrap();
    let id = task.record.id.clone();
    wait_for(&manager, &id, |t| t.record.downloaded >= 256 * KIB).await;
    manager.pause(&id).unwrap();
    wait_for(&manager, &id, |t| t.record.state == TaskState::Paused).await;
    let seen = server.state.requests().len();

    set_rules(
        &manager,
        vec![SiteRule {
            id: "local".into(),
            user_agent: "Second/2.0".into(),
            ..rule("127.0.0.1")
        }],
    );
    manager.resume(&id).unwrap();
    let finished = wait_for(&manager, &id, done).await;
    assert_eq!(
        std::fs::read(finished.record.path.unwrap()).unwrap(),
        content(size, 0)
    );
    let requests = server.state.requests();
    assert!(requests.len() > seen);
    assert!(
        requests[seen..]
            .iter()
            .all(|r| r.user_agent.as_deref() == Some("Second/2.0")),
        "the resume uses the rule as it is now"
    );
}

#[tokio::test]
async fn invalid_rules_are_refused_with_the_reason() {
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let manager = open(data.path(), out.path()).await;
    let cases = [
        (rule("https://example.com/files"), "not a link"),
        (rule("*.com"), "top-level domain"),
        (
            SiteRule {
                proxy: RuleProxy::Custom,
                proxy_url: "nonsense".into(),
                ..rule("example.com")
            },
            "socks5://",
        ),
        (
            SiteRule {
                referer: "javascript:alert(1)".into(),
                ..rule("example.com")
            },
            "http or https",
        ),
    ];
    for (bad, why) in cases {
        let pattern = bad.pattern.clone();
        let err = manager
            .update_settings(Settings {
                site_rules: vec![bad],
                ..manager.settings()
            })
            .unwrap_err();
        match err {
            CoreError::InvalidSetting(msg) => assert!(msg.contains(why), "{pattern}: {msg}"),
            other => panic!("{pattern}: {other:?}"),
        }
    }
    let err = manager
        .update_settings(Settings {
            site_rules: vec![rule("Example.com"), rule("example.com.")],
            ..manager.settings()
        })
        .unwrap_err();
    assert!(err.to_string().contains("same pattern"), "{err}");
    assert!(manager.settings().site_rules.is_empty());
}

#[tokio::test]
async fn a_site_speed_limit_holds_its_downloads_back() {
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let manager = open(data.path(), out.path()).await;
    set_rules(
        &manager,
        vec![SiteRule {
            speed_limit: Some(MIB),
            ..rule("127.0.0.1")
        }],
    );
    let size = 768 * KIB;
    let start = Instant::now();
    let task = manager
        .add(AddOptions {
            url: server.url(&format!("/file/{size}")),
            ..AddOptions::default()
        })
        .unwrap();
    wait_for(&manager, &task.record.id, done).await;
    // 0.75 MiB at 1 MiB/s, less the quarter second of burst.
    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(400), "{elapsed:?}");
}

#[tokio::test]
async fn hosts_that_refused_connections_are_offered_until_a_rule_covers_them() {
    let server = TestServer::start().await.unwrap();
    let data = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let manager = open(data.path(), out.path()).await;
    assert!(manager.refused_hosts().is_empty());

    let task = manager
        .add(AddOptions {
            url: server.url(&format!(
                "/file/{}?max_conns=1&limit_status=503&rate=4194304",
                4 * MIB
            )),
            ..AddOptions::default()
        })
        .unwrap();
    wait_for(&manager, &task.record.id, done).await;
    assert_eq!(
        manager.refused_hosts(),
        [RefusedHost {
            host: "127.0.0.1".into(),
            connections: 1,
        }]
    );

    // A rule that allows more does not cover it; one at the limit does.
    set_rules(
        &manager,
        vec![SiteRule {
            max_connections: Some(4),
            ..rule("127.0.0.1")
        }],
    );
    assert_eq!(manager.refused_hosts().len(), 1);
    set_rules(
        &manager,
        vec![SiteRule {
            max_connections: Some(1),
            ..rule("127.0.0.1")
        }],
    );
    assert!(manager.refused_hosts().is_empty());

    // Changing the user agent rebuilds the engine; what hosts refused stays.
    manager
        .update_settings(Settings {
            user_agent: "Other/1.0".into(),
            site_rules: Vec::new(),
            ..manager.settings()
        })
        .unwrap();
    assert_eq!(manager.refused_hosts().len(), 1);
}
