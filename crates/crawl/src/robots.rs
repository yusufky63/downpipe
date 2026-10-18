//! `robots.txt` (RFC 9309): which paths a crawler may fetch.
//!
//! The group for the most specific user agent that names Downpipe applies,
//! else the `*` group. Within it the longest matching rule wins, and `Allow`
//! wins a tie. `*` matches any run of characters and a trailing `$` anchors
//! the end, as Google and Bing read them.

use std::time::Duration;

/// The rules that apply to Downpipe on one host.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Robots {
    rules: Vec<Rule>,
    /// `Crawl-delay`, which RFC 9309 leaves out but many sites still set.
    pub crawl_delay: Option<Duration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Rule {
    allow: bool,
    pattern: String,
}

impl Robots {
    /// Everything allowed: no robots.txt, or one that could not be read.
    pub fn allow_all() -> Self {
        Self::default()
    }

    /// Nothing allowed: the server failed (5xx) while answering for its
    /// robots.txt, which RFC 9309 says to treat as a full disallow.
    pub fn disallow_all() -> Self {
        Self {
            rules: vec![Rule {
                allow: false,
                pattern: "/".into(),
            }],
            crawl_delay: None,
        }
    }

    /// The rules of `text` for the agent whose product token is `agent`.
    pub fn parse(text: &str, agent: &str) -> Self {
        let agent = agent.to_ascii_lowercase();
        // (agents of the group, rules, delay)
        let mut groups: Vec<(Vec<String>, Vec<Rule>, Option<Duration>)> = Vec::new();
        let mut in_agents = false;
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let key = key.trim().to_ascii_lowercase();
            let value = value.trim();
            match key.as_str() {
                "user-agent" => {
                    if !in_agents {
                        groups.push((Vec::new(), Vec::new(), None));
                        in_agents = true;
                    }
                    if let Some(g) = groups.last_mut() {
                        g.0.push(value.to_ascii_lowercase());
                    }
                }
                "allow" | "disallow" => {
                    in_agents = false;
                    // An empty Disallow allows everything: no rule at all.
                    if let Some(g) = groups.last_mut()
                        && !value.is_empty()
                    {
                        g.1.push(Rule {
                            allow: key == "allow",
                            pattern: value.to_string(),
                        });
                    }
                }
                "crawl-delay" => {
                    in_agents = false;
                    if let Some(g) = groups.last_mut()
                        && let Ok(secs) = value.parse::<f64>()
                        && secs.is_finite()
                        && secs >= 0.0
                    {
                        g.2 = Some(Duration::from_secs_f64(secs.min(3600.0)));
                    }
                }
                _ => {}
            }
        }
        // The longest agent name that is part of our token is the most specific.
        let named = groups
            .iter()
            .filter_map(|g| {
                g.0.iter()
                    .filter(|a| *a != "*" && !a.is_empty() && agent.contains(a.as_str()))
                    .map(String::len)
                    .max()
                    .map(|len| (len, g))
            })
            .max_by_key(|(len, _)| *len)
            .map(|(len, _)| len);
        let picked: Vec<_> = match named {
            Some(best) => groups
                .iter()
                .filter(|g| {
                    g.0.iter()
                        .any(|a| a.len() == best && *a != "*" && agent.contains(a.as_str()))
                })
                .collect(),
            None => groups
                .iter()
                .filter(|g| g.0.iter().any(|a| a == "*"))
                .collect(),
        };
        let mut robots = Self::default();
        for g in picked {
            robots.rules.extend(g.1.iter().cloned());
            robots.crawl_delay = robots.crawl_delay.max(g.2);
        }
        robots
    }

    /// Whether a URL path (with its query) may be fetched.
    pub fn allows(&self, path_and_query: &str) -> bool {
        let mut best: Option<(usize, bool)> = None;
        for rule in &self.rules {
            if matches(&rule.pattern, path_and_query) {
                let len = rule.pattern.len();
                best = match best {
                    Some((l, a)) if l > len || (l == len && a) => Some((l, a)),
                    _ => Some((len, rule.allow)),
                };
            }
        }
        best.is_none_or(|(_, allow)| allow)
    }
}

/// Whether a robots.txt path pattern matches the start of `path`.
fn matches(pattern: &str, path: &str) -> bool {
    let (pattern, anchored) = match pattern.strip_suffix('$') {
        Some(p) => (p, true),
        None => (pattern, false),
    };
    let parts: Vec<&str> = pattern.split('*').collect();
    let mut pos = 0;
    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            if !path.starts_with(part) {
                return false;
            }
            pos = part.len();
            continue;
        }
        let last = i == parts.len() - 1;
        if last && anchored {
            // The final piece must end the path, after what came before.
            return path.len() >= pos + part.len() && path.ends_with(part);
        }
        match path[pos..].find(part) {
            Some(p) => pos += p + part.len(),
            None => return false,
        }
    }
    !anchored || pos == path.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "# comment\n\
        User-agent: *\n\
        Disallow: /private/\n\
        Allow: /private/open/\n\
        Disallow: /*.php$\n\
        Crawl-delay: 2\n\
        \n\
        User-agent: BadBot\n\
        Disallow: /\n";

    #[test]
    fn star_group_applies_to_downpipe() {
        let r = Robots::parse(TEXT, "Downpipe");
        assert!(r.allows("/"));
        assert!(r.allows("/files/a.zip"));
        assert!(!r.allows("/private/a.zip"));
        assert!(r.allows("/private/open/a.zip"));
        assert!(!r.allows("/index.php"));
        assert!(r.allows("/index.php?x=1"));
        assert_eq!(r.crawl_delay, Some(Duration::from_secs(2)));
    }

    #[test]
    fn a_group_naming_downpipe_replaces_the_star_group() {
        let text = format!("{TEXT}\nUser-agent: downpipe\nUser-agent: other\nDisallow: /files/\n");
        let r = Robots::parse(&text, "Downpipe");
        assert!(!r.allows("/files/a.zip"));
        assert!(r.allows("/private/a.zip"));
        assert_eq!(r.crawl_delay, None);
    }

    #[test]
    fn longest_rule_wins_and_allow_wins_ties() {
        let r = Robots::parse(
            "User-agent: *\nDisallow: /a\nAllow: /a\nDisallow: /ab*\nAllow: /abc",
            "Downpipe",
        );
        assert!(r.allows("/a"));
        assert!(!r.allows("/abd"));
        assert!(r.allows("/abcd"));
    }

    #[test]
    fn empty_and_missing_rules_allow_everything() {
        assert!(Robots::parse("User-agent: *\nDisallow:\n", "Downpipe").allows("/x"));
        assert!(Robots::parse("", "Downpipe").allows("/x"));
        assert!(Robots::parse("garbage\n:::\n", "Downpipe").allows("/x"));
        assert!(!Robots::disallow_all().allows("/x"));
    }

    #[test]
    fn wildcards() {
        assert!(matches("/*/b", "/a/b/c"));
        assert!(matches("/a*", "/a"));
        assert!(matches("/*.zip$", "/x/y.zip"));
        assert!(!matches("/*.zip$", "/x/y.zip?z"));
        assert!(matches("/x$", "/x"));
        assert!(!matches("/x$", "/xy"));
        assert!(!matches("/a*b*c$", "/abx"));
    }
}
