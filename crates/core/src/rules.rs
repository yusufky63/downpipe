//! Site rules: settings that apply to downloads from hosts matching a
//! pattern, such as fewer connections for a host that refuses many, or no
//! proxy for a local mirror.
//!
//! A pattern is a host (`example.com`, which matches only that host) or a
//! domain (`*.example.com`, which matches `example.com` and every host under
//! it). When several rules match, the most specific one applies: an exact
//! host before a domain, a longer domain before a shorter one. Rules never
//! combine, so what a download got is always one rule, which the details
//! dialog names.

use serde::{Deserialize, Serialize};
use url::{Host, Url};

/// Overrides for downloads from the hosts `pattern` matches. Empty or
/// `None` fields keep the global setting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SiteRule {
    /// Stable id; generated for new rules.
    pub id: String,
    /// `example.com` or `*.example.com`; stored lower-case, in ASCII form.
    pub pattern: String,
    pub enabled: bool,
    /// 1 to 32.
    pub max_connections: Option<u32>,
    pub user_agent: String,
    /// Sent as `Referer`, replacing the one the browser passed on.
    pub referer: String,
    pub proxy: RuleProxy,
    /// The proxy for [`RuleProxy::Custom`].
    pub proxy_url: String,
    /// Bytes per second, shared by every download the rule applies to.
    pub speed_limit: Option<u64>,
    /// Failed attempts in a row per connection before the download fails,
    /// 0 to 100.
    pub max_retries: Option<u32>,
    /// Where new downloads go; relative to the download folder unless
    /// absolute.
    pub download_dir: String,
    /// Queue id for new downloads.
    pub queue: String,
}

impl Default for SiteRule {
    fn default() -> Self {
        Self {
            id: String::new(),
            pattern: String::new(),
            enabled: true,
            max_connections: None,
            user_agent: String::new(),
            referer: String::new(),
            proxy: RuleProxy::Inherit,
            proxy_url: String::new(),
            speed_limit: None,
            max_retries: None,
            download_dir: String::new(),
            queue: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleProxy {
    /// The proxy of the network settings.
    #[default]
    Inherit,
    /// No proxy, not even the system's.
    Direct,
    /// `SiteRule::proxy_url`.
    Custom,
}

/// What is wrong with one field of a rule. `code` is stable, so the UI can
/// say it in the user's language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuleProblem {
    pub id: String,
    pub field: &'static str,
    pub code: &'static str,
}

/// A host that refused extra connections this session and that no rule
/// limits yet, offered as a rule in the settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RefusedHost {
    pub host: String,
    /// Connections it accepted.
    pub connections: u32,
}

/// Which rule an address gets, for the "test this address" field.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct RuleTest {
    /// The host read from the address; `None` when it is not an address.
    pub host: Option<String>,
    /// Id of the rule that applies.
    pub rule: Option<String>,
    /// Rules that match but are switched off, so they do not apply.
    pub disabled: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Pattern {
    Host(String),
    /// The domain itself and everything under it.
    Domain(String),
}

impl Pattern {
    fn matches(&self, host: &str) -> bool {
        match self {
            Self::Host(h) => h == host,
            Self::Domain(d) => {
                host == d
                    || host
                        .strip_suffix(d.as_str())
                        .is_some_and(|rest| rest.ends_with('.'))
            }
        }
    }

    /// Higher wins: an exact host over any domain, then the longer name.
    fn specificity(&self) -> (u8, usize) {
        match self {
            Self::Host(h) => (1, h.split('.').count()),
            Self::Domain(d) => (0, d.split('.').count()),
        }
    }
}

/// Lower-case a pattern and write international names in their ASCII form,
/// so `*.Bücher.example` and `*.xn--bcher-kva.example` are the same rule.
/// Anything that is not a pattern is returned trimmed, for
/// [`pattern_problem`] to explain.
pub fn normalize_pattern(input: &str) -> String {
    let trimmed = input.trim().trim_end_matches('.');
    let (wild, rest) = match trimmed.strip_prefix("*.") {
        Some(rest) => (true, rest),
        None => (false, trimmed),
    };
    match parse_host(rest) {
        Some(host) if wild => format!("*.{host}"),
        Some(host) => host,
        None => trimmed.to_string(),
    }
}

/// Why `pattern` (normalized) is not a pattern, as a [`RuleProblem`] code.
pub fn pattern_problem(pattern: &str) -> Option<&'static str> {
    parse_pattern(pattern).err()
}

fn parse_pattern(pattern: &str) -> Result<Pattern, &'static str> {
    // Drafts from the settings dialog are tested before they are saved
    // (and normalized), so `Example.COM.` has to work here too.
    let pattern = normalize_pattern(pattern);
    let pattern = pattern.as_str();
    if pattern.is_empty() {
        return Err("pattern_empty");
    }
    if pattern.contains("://") || pattern.contains('/') {
        // A pasted link: the most common way a pattern silently matches
        // nothing in other download managers.
        return Err("pattern_url");
    }
    let (wild, rest) = match pattern.strip_prefix("*.") {
        Some(rest) => (true, rest),
        None => (false, pattern),
    };
    if rest.contains('*') {
        return Err("pattern_wildcard");
    }
    if rest.contains(':') && !rest.starts_with('[') {
        return Err("pattern_port");
    }
    let host = parse_host(rest).ok_or("pattern_invalid")?;
    if wild {
        // `*.10.0.0.1` would match nothing, and `*.com` everything.
        if host.starts_with('[') || host.parse::<std::net::Ipv4Addr>().is_ok() {
            return Err("pattern_wildcard");
        }
        if !host.contains('.') && host != "localhost" {
            return Err("pattern_too_broad");
        }
        Ok(Pattern::Domain(host))
    } else {
        Ok(Pattern::Host(host))
    }
}

/// `example.com` -> `example.com`; `None` for anything with a port, a path,
/// credentials or characters a host cannot have.
fn parse_host(input: &str) -> Option<String> {
    if input.is_empty() || input.contains(['/', '?', '#', '@', '\\', ' ', '*']) {
        return None;
    }
    let url = Url::parse(&format!("http://{input}/")).ok()?;
    if url.port().is_some() {
        return None;
    }
    match url.host()? {
        Host::Domain(d) if !d.is_empty() => Some(d.to_string()),
        Host::Ipv4(ip) => Some(ip.to_string()),
        Host::Ipv6(ip) => Some(format!("[{ip}]")),
        Host::Domain(_) => None,
    }
}

/// The host a rule is matched against: lower-case, ASCII, IPv6 in brackets.
pub fn host_of(url: &Url) -> Option<String> {
    match url.host()? {
        Host::Domain(d) => Some(d.trim_end_matches('.').to_ascii_lowercase()),
        Host::Ipv4(ip) => Some(ip.to_string()),
        Host::Ipv6(ip) => Some(format!("[{ip}]")),
    }
}

/// The enabled rule for `host`: the most specific match. Rules with invalid
/// patterns never match.
pub fn rule_for_host<'a>(rules: &'a [SiteRule], host: &str) -> Option<&'a SiteRule> {
    rules
        .iter()
        .filter(|r| r.enabled)
        .filter_map(|r| {
            let pattern = parse_pattern(&r.pattern).ok()?;
            pattern.matches(host).then(|| (pattern.specificity(), r))
        })
        // Valid rules have unique patterns, so ties cannot happen; were one
        // to slip through, the first listed would win.
        .fold(
            None::<((u8, usize), &SiteRule)>,
            |best, (spec, r)| match best {
                Some((b, _)) if b >= spec => best,
                _ => Some((spec, r)),
            },
        )
        .map(|(_, r)| r)
}

pub fn rule_for<'a>(rules: &'a [SiteRule], url: &Url) -> Option<&'a SiteRule> {
    rule_for_host(rules, &host_of(url)?)
}

/// Which rule `address` (a link, or just a host) gets.
pub fn test_address(rules: &[SiteRule], address: &str) -> RuleTest {
    let address = address.trim();
    let host = if address.contains("://") {
        Url::parse(address).ok().as_ref().and_then(host_of)
    } else {
        // A bare host, maybe with a path after it.
        let end = address.find(['/', '?', '#']).unwrap_or(address.len());
        let host = &address[..end];
        Url::parse(&format!("http://{host}/"))
            .ok()
            .as_ref()
            .and_then(host_of)
    };
    let Some(host) = host.filter(|h| !h.is_empty()) else {
        return RuleTest::default();
    };
    let disabled = rules
        .iter()
        .filter(|r| !r.enabled)
        .filter(|r| parse_pattern(&r.pattern).is_ok_and(|p| p.matches(&host)))
        .map(|r| r.id.clone())
        .collect();
    RuleTest {
        rule: rule_for_host(rules, &host).map(|r| r.id.clone()),
        host: Some(host),
        disabled,
    }
}

/// Every problem in `rules`, in list order. A queue that no longer exists is
/// not one: the settings forget it, as they do for the schedule.
pub fn problems(rules: &[SiteRule]) -> Vec<RuleProblem> {
    let mut out = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for rule in rules {
        let mut push = |field: &'static str, code: &'static str| {
            out.push(RuleProblem {
                id: rule.id.clone(),
                field,
                code,
            });
        };
        let pattern = normalize_pattern(&rule.pattern);
        if let Some(code) = pattern_problem(&pattern) {
            push("pattern", code);
        } else if seen.contains(&pattern) {
            push("pattern", "pattern_duplicate");
        } else {
            seen.push(pattern);
        }
        if rule.user_agent.chars().any(char::is_control) {
            push("user_agent", "user_agent_invalid");
        }
        let referer = rule.referer.trim();
        if !referer.is_empty()
            && !Url::parse(referer).is_ok_and(|u| matches!(u.scheme(), "http" | "https"))
        {
            push("referer", "referer_invalid");
        }
        if rule.proxy == RuleProxy::Custom {
            let proxy = rule.proxy_url.trim();
            if proxy.is_empty() {
                push("proxy_url", "proxy_missing");
            } else if !is_proxy(proxy) {
                push("proxy_url", "proxy_invalid");
            }
        }
    }
    out
}

/// A problem code in words, for errors outside the settings dialog (which
/// translates the codes itself).
pub fn describe(code: &str) -> &'static str {
    match code {
        "pattern_empty" => "the pattern is empty",
        "pattern_url" => "write only the host, like example.com, not a link",
        "pattern_wildcard" => "`*` can only stand at the start, as in *.example.com",
        "pattern_too_broad" => "the pattern would match a whole top-level domain",
        "pattern_port" => "patterns match hosts on every port; leave the port out",
        "pattern_duplicate" => "another rule has the same pattern",
        "user_agent_invalid" => "the user agent cannot contain control characters",
        "referer_invalid" => "the referer must be an http or https link",
        "proxy_missing" => "the proxy address is missing",
        "proxy_invalid" => "use http://, https:// or socks5://host:port for the proxy",
        _ => "the pattern is not a host name",
    }
}

/// `http://`, `https://` or `socks5://host:port`, like the network setting.
pub fn is_proxy(proxy: &str) -> bool {
    Url::parse(proxy).is_ok_and(|url| {
        matches!(url.scheme(), "http" | "https" | "socks5" | "socks5h")
            && url.host_str().is_some_and(|h| !h.is_empty())
    })
}

/// Normalize what a UI sent: patterns as stored, values in range, ids for
/// new rules. Invalid patterns are kept so [`problems`] can report them.
pub(crate) fn normalize_rules(rules: Vec<SiteRule>) -> Vec<SiteRule> {
    let mut out: Vec<SiteRule> = Vec::with_capacity(rules.len());
    for mut rule in rules {
        rule.id = rule.id.trim().to_string();
        if rule.id.is_empty() || out.iter().any(|r| r.id == rule.id) {
            rule.id = uuid::Uuid::new_v4().simple().to_string();
        }
        rule.pattern = normalize_pattern(&rule.pattern);
        rule.max_connections = rule.max_connections.map(|c| c.clamp(1, 32));
        rule.max_retries = rule.max_retries.map(|c| c.min(100));
        rule.speed_limit = rule.speed_limit.filter(|&s| s > 0);
        rule.user_agent = rule.user_agent.trim().to_string();
        rule.referer = rule.referer.trim().to_string();
        rule.proxy_url = rule.proxy_url.trim().to_string();
        rule.download_dir = rule.download_dir.trim().trim_matches('"').to_string();
        rule.queue = rule.queue.trim().to_string();
        out.push(rule);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(id: &str, pattern: &str) -> SiteRule {
        SiteRule {
            id: id.into(),
            pattern: normalize_pattern(pattern),
            ..SiteRule::default()
        }
    }

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn a_host_matches_only_itself() {
        let rules = [rule("a", "example.com")];
        assert!(rule_for(&rules, &url("https://example.com/f.zip")).is_some());
        assert!(rule_for(&rules, &url("http://EXAMPLE.com:8080/f.zip")).is_some());
        assert!(rule_for(&rules, &url("https://cdn.example.com/f.zip")).is_none());
        assert!(rule_for(&rules, &url("https://notexample.com/f.zip")).is_none());
    }

    #[test]
    fn a_domain_matches_itself_and_below() {
        let rules = [rule("a", "*.example.com")];
        for u in [
            "https://example.com/",
            "https://cdn.example.com/",
            "https://a.b.example.com/",
        ] {
            assert!(rule_for(&rules, &url(u)).is_some(), "{u}");
        }
        assert!(rule_for(&rules, &url("https://badexample.com/")).is_none());
        assert!(rule_for(&rules, &url("https://example.com.evil.net/")).is_none());
    }

    #[test]
    fn the_most_specific_rule_wins_whatever_the_order() {
        let mut rules = vec![
            rule("domain", "*.example.com"),
            rule("deeper", "*.dl.example.com"),
            rule("exact", "dl.example.com"),
        ];
        for _ in 0..3 {
            let id = |u: &str| rule_for(&rules, &url(u)).map(|r| r.id.clone());
            assert_eq!(id("https://dl.example.com/").as_deref(), Some("exact"));
            assert_eq!(id("https://eu.dl.example.com/").as_deref(), Some("deeper"));
            assert_eq!(id("https://www.example.com/").as_deref(), Some("domain"));
            rules.rotate_left(1);
        }
    }

    #[test]
    fn switched_off_rules_do_not_apply_but_are_reported() {
        let mut off = rule("off", "example.com");
        off.enabled = false;
        let rules = [off, rule("on", "*.example.com")];
        let test = test_address(&rules, "https://example.com/file.iso");
        assert_eq!(test.host.as_deref(), Some("example.com"));
        assert_eq!(test.rule.as_deref(), Some("on"));
        assert_eq!(test.disabled, ["off"]);
    }

    #[test]
    fn the_tester_takes_links_and_bare_hosts() {
        let rules = [rule("a", "*.Bücher.example")];
        assert_eq!(rules[0].pattern, "*.xn--bcher-kva.example");
        for address in [
            "https://dl.bücher.example/x.zip",
            "dl.xn--bcher-kva.example",
            " dl.BÜCHER.example/path?q=1 ",
        ] {
            let test = test_address(&rules, address);
            assert_eq!(
                test.host.as_deref(),
                Some("dl.xn--bcher-kva.example"),
                "{address}"
            );
            assert_eq!(test.rule.as_deref(), Some("a"), "{address}");
        }
        let none = test_address(&rules, "https://example.org/");
        assert_eq!(none.host.as_deref(), Some("example.org"));
        assert_eq!(none.rule, None);
        assert_eq!(test_address(&rules, "   "), RuleTest::default());
        assert_eq!(test_address(&rules, "http://"), RuleTest::default());
    }

    #[test]
    fn ip_addresses_and_localhost() {
        let rules = [
            rule("v4", "192.168.1.10"),
            rule("v6", "[::1]"),
            rule("local", "*.localhost"),
        ];
        let id = |u: &str| rule_for(&rules, &url(u)).map(|r| r.id.clone());
        assert_eq!(id("http://192.168.1.10:8000/a").as_deref(), Some("v4"));
        assert_eq!(id("http://[::1]:8000/a").as_deref(), Some("v6"));
        assert_eq!(id("http://app.localhost/a").as_deref(), Some("local"));
    }

    #[test]
    fn bad_patterns_say_why() {
        let code = |p: &str| pattern_problem(&normalize_pattern(p));
        assert_eq!(code("example.com"), None);
        assert_eq!(code(" *.Example.COM. "), None);
        assert_eq!(code(""), Some("pattern_empty"));
        assert_eq!(code("https://example.com/files"), Some("pattern_url"));
        assert_eq!(code("example.com/files"), Some("pattern_url"));
        assert_eq!(code("*example.com"), Some("pattern_wildcard"));
        assert_eq!(code("dl.*.example.com"), Some("pattern_wildcard"));
        assert_eq!(code("*.10.0.0.1"), Some("pattern_wildcard"));
        assert_eq!(code("*"), Some("pattern_wildcard"));
        assert_eq!(code("*.com"), Some("pattern_too_broad"));
        assert_eq!(code("example.com:8080"), Some("pattern_port"));
        assert_eq!(code("exa mple.com"), Some("pattern_invalid"));
        assert_eq!(code("user@example.com"), Some("pattern_invalid"));
    }

    #[test]
    fn problems_cover_every_field() {
        let rules = vec![
            rule("ok", "example.com"),
            rule("dup", "EXAMPLE.com"),
            SiteRule {
                id: "bad".into(),
                pattern: "*.example.org".into(),
                user_agent: "a\nb".into(),
                referer: "ftp://example.org/".into(),
                proxy: RuleProxy::Custom,
                proxy_url: "localhost:8080".into(),
                ..SiteRule::default()
            },
            SiteRule {
                id: "empty-proxy".into(),
                pattern: "example.net".into(),
                proxy: RuleProxy::Custom,
                ..SiteRule::default()
            },
        ];
        let found: Vec<(String, &str)> = problems(&rules)
            .into_iter()
            .map(|p| (p.id, p.code))
            .collect();
        let expected = [
            ("dup", "pattern_duplicate"),
            ("bad", "user_agent_invalid"),
            ("bad", "referer_invalid"),
            ("bad", "proxy_invalid"),
            ("empty-proxy", "proxy_missing"),
        ];
        assert_eq!(
            found,
            expected
                .iter()
                .map(|(i, c)| (i.to_string(), *c))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn normalizing_fills_ids_and_clamps() {
        let rules = normalize_rules(vec![
            SiteRule {
                pattern: " Example.COM ".into(),
                max_connections: Some(99),
                max_retries: Some(1000),
                speed_limit: Some(0),
                ..SiteRule::default()
            },
            SiteRule {
                id: "x".into(),
                max_connections: Some(0),
                ..SiteRule::default()
            },
            SiteRule {
                id: "x".into(),
                ..SiteRule::default()
            },
        ]);
        assert_eq!(rules[0].pattern, "example.com");
        assert!(!rules[0].id.is_empty());
        assert_eq!(rules[0].max_connections, Some(32));
        assert_eq!(rules[0].max_retries, Some(100));
        assert_eq!(rules[0].speed_limit, None);
        assert_eq!(rules[1].max_connections, Some(1));
        assert_ne!(rules[2].id, "x", "a repeated id is replaced");
    }

    #[test]
    fn missing_fields_take_defaults() {
        let r: SiteRule = serde_json::from_str(r#"{"pattern": "example.com"}"#).unwrap();
        assert!(r.enabled);
        assert_eq!(r.proxy, RuleProxy::Inherit);
        assert_eq!(r.max_connections, None);
    }
}
