# Site rules

**Settings → Sites** keeps a list of host patterns, each with settings that replace the global ones for downloads from the hosts it matches. Users of other download managers ask for per-site settings (AB Download Manager [#1108](https://github.com/amir1376/ab-download-manager/issues/1108) and [#151](https://github.com/amir1376/ab-download-manager/issues/151), Gopeed [#1552](https://github.com/GopeedLab/gopeed/issues/1552)) and complain about patterns that silently match nothing, with no way to tell why ([#196](https://github.com/amir1376/ab-download-manager/issues/196), [#1353](https://github.com/amir1376/ab-download-manager/issues/1353)).

## Patterns

| Pattern | Matches |
| --- | --- |
| `example.com` | `example.com` only |
| `*.example.com` | `example.com`, `dl.example.com`, `a.b.example.com` |
| `192.168.1.10`, `[::1]` | that address |

A pattern is a host, never a link: `https://example.com/files` is refused with "enter only the host name", the most common reason a pattern matches nothing. Ports are left out (a rule matches its host on every port), `*` may only stand at the start, and `*.com` is refused as too broad. Patterns are stored lower-case and in ASCII, so `*.Bücher.example` and `*.xn--bcher-kva.example` are the same rule and match either spelling of the address.

When several rules match, **the most specific applies**, whatever their order in the list: an exact host before any `*.domain`, then the domain with more labels (`*.dl.example.com` before `*.example.com`). Rules do not combine, so a download always gets exactly one rule or none, and the details dialog names it (**Site rule**). A switched-off rule never applies.

## What a rule can set

Every field is optional; an empty one keeps the global setting.

| Field | Applies | Notes |
| --- | --- | --- |
| Connections | at every start and resume | 1 to 32. A number set for one download still wins. |
| Retries | at every start and resume | Failed attempts in a row per connection, 0 to 100. Network outages are waited out regardless (see [DOWNLOAD_ENGINE.md](DOWNLOAD_ENGINE.md#retries-silence-and-network-outages)). |
| Speed limit | at every start and resume | One limit shared by all downloads the rule applies to, on top of the global limit. Changing it takes effect at once, also for running downloads. |
| User agent, Referer | at every start and resume | Replace the app's user agent and the referer the browser passed on. |
| Proxy | at every start and resume | As in Network, no proxy (not even the system's), or another `http://`, `https://` or `socks5://` proxy. |
| Folder | when the download is added | Inside the default folder unless absolute. A folder chosen for the download wins; a rule's folder is never changed by category sorting. |
| Queue | when the download is added | A queue chosen for the download wins. Removing the queue clears it from the rule. |

Because the rule is looked up again each time a download starts, editing a rule while a download is paused changes how it resumes. Video pages (YouTube, run through yt-dlp) take the proxy and the speed limit (the tighter of the site's and the global one, since yt-dlp has one limit per download).

## Test this address

The field at the top of the tab takes a link or a host name and says which rule its downloads get: `dl.example.com gets the rule *.example.com`, `No rule matches example.org`, or that the text is not an address. It also lists matching rules that are switched off. It works on the rules as edited, before they are saved, and the matching rule is outlined in the list.

The answer comes from the same Rust function that picks the rule when a download starts (`downpipe_core::rules::test_address`, through the `check_site_rules` command), not from a copy in TypeScript, so the tester cannot disagree with what downloads do. The same command reports each rule's problems (bad pattern, duplicate pattern, a referer that is not an http(s) link, a missing or malformed proxy, control characters in the user agent) by stable codes the dialog translates and shows under the field; Save is refused while any remain, and `Manager::update_settings` refuses them too.

## Hosts that refused extra connections

When a host refuses connections beyond some number with `403`, `429` or `503`, the engine continues with the ones it accepts and remembers that number for the host until Downpipe quits (see [DOWNLOAD_ENGINE.md](DOWNLOAD_ENGINE.md#how-many-connections)). The Sites tab lists those hosts (`Manager::refused_hosts`) with **Make it a rule**, which sets the number on the rule for exactly that host, or adds one. A host drops off the list once an enabled rule holds it to that many connections. The memory is shared by every engine the app builds, so changing the proxy or user agent, or a rule with its own proxy, does not forget it.

## Where it lives

- `crates/core/src/rules.rs`: `SiteRule`, pattern parsing, matching, the tester and the problem codes, with unit tests.
- `crates/core/src/manager.rs`: the rule's folder and queue in `add`; at every start the rule is looked up again, its pattern stored in the record's `site_rule` (catalog schema 6), and the request built with its headers, connections, retries and limiter, on an engine derived for its proxy.
- `crates/http-engine`: `DownloadRequest::max_retries` and `DownloadRequest::limiter`, `EngineConfig::no_proxy`, `Engine::derive` and `Engine::connection_limits`.
- `apps/desktop/src/components/SiteRules.tsx` and `lib/siteRules.ts` (drafts, "make it a rule", the tester's verdict, with tests).
- Tests: `crates/core/tests/site_rules.rs` (headers, connections and proxy against the test server, folder and queue, a rule changed while paused, refused settings, a site speed limit, refused hosts), `crates/http-engine/tests/overrides.rs`.
