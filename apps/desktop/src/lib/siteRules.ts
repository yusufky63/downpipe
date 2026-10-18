// Site rules as the settings dialog edits them. Matching and validation stay
// in Rust (downpipe_core::rules), reached through `api.checkSiteRules`, so
// the "test this address" answer is the one downloads get.
import type { RefusedHost, RuleProblem, RuleProblemCode, RuleProxy, SiteRule, SiteRulesCheck } from "./api";
import { bytesToRate, rateToBytes, type RateUnit } from "./format";

/** A rule as edited: numbers stay strings until saved, so a field can be empty. */
export interface RuleDraft {
  key: number;
  id: string;
  pattern: string;
  enabled: boolean;
  connections: string;
  retries: string;
  limit: string;
  unit: RateUnit;
  user_agent: string;
  referer: string;
  proxy: RuleProxy;
  proxy_url: string;
  download_dir: string;
  queue: string;
}

let nextKey = 0;

/**
 * New rules get their id here rather than when saved: the checks report
 * problems by id, and a rule that is not saved yet needs one too.
 */
function newId(): string {
  return `rule-${Date.now().toString(36)}-${(++nextKey).toString(36)}`;
}

export function toRuleDraft(rule: SiteRule): RuleDraft {
  const rate = bytesToRate(rule.speed_limit ?? 0);
  return {
    key: ++nextKey,
    id: rule.id || newId(),
    pattern: rule.pattern,
    enabled: rule.enabled,
    connections: rule.max_connections === null ? "" : String(rule.max_connections),
    retries: rule.max_retries === null ? "" : String(rule.max_retries),
    limit: rate.value > 0 ? String(rate.value) : "",
    unit: rate.unit,
    user_agent: rule.user_agent,
    referer: rule.referer,
    proxy: rule.proxy,
    proxy_url: rule.proxy_url,
    download_dir: rule.download_dir,
    queue: rule.queue,
  };
}

export function newRuleDraft(pattern = ""): RuleDraft {
  return toRuleDraft({
    id: "",
    pattern,
    enabled: true,
    max_connections: null,
    user_agent: "",
    referer: "",
    proxy: "inherit",
    proxy_url: "",
    speed_limit: null,
    max_retries: null,
    download_dir: "",
    queue: "",
  });
}

/** An empty or unreadable number keeps the global setting. */
function whole(text: string, min: number, max: number): number | null {
  const n = Number.parseInt(text.trim(), 10);
  if (!Number.isFinite(n)) return null;
  return Math.min(max, Math.max(min, n));
}

export function fromRuleDraft(d: RuleDraft): SiteRule {
  const limit = rateToBytes(Number.parseFloat(d.limit), d.unit);
  return {
    id: d.id,
    pattern: d.pattern.trim(),
    enabled: d.enabled,
    max_connections: whole(d.connections, 1, 32),
    user_agent: d.user_agent.trim(),
    referer: d.referer.trim(),
    proxy: d.proxy,
    proxy_url: d.proxy === "custom" ? d.proxy_url.trim() : "",
    speed_limit: limit > 0 ? limit : null,
    max_retries: whole(d.retries, 0, 100),
    download_dir: d.download_dir.trim(),
    queue: d.queue,
  };
}

/**
 * "Make it a rule" for a host that refused extra connections: the rule for
 * exactly that host gets the number, or a new one is added at the end.
 */
export function ruleForRefusedHost(drafts: RuleDraft[], refused: RefusedHost): RuleDraft[] {
  const host = refused.host.toLowerCase();
  const connections = String(refused.connections);
  const at = drafts.findIndex((d) => d.pattern.trim().toLowerCase() === host);
  if (at >= 0) return drafts.map((d, i) => (i === at ? { ...d, connections, enabled: true } : d));
  return [...drafts, { ...newRuleDraft(host), connections }];
}

/** Hosts still worth offering: not already held to that many by a draft for exactly that host. */
export function refusedToOffer(refused: RefusedHost[], drafts: RuleDraft[]): RefusedHost[] {
  return refused.filter((r) => {
    const draft = drafts.find((d) => d.pattern.trim().toLowerCase() === r.host.toLowerCase());
    if (!draft || !draft.enabled) return true;
    const max = whole(draft.connections, 1, 32);
    return max === null || max > r.connections;
  });
}

export function problemFor(
  check: SiteRulesCheck | null,
  id: string,
  field: RuleProblem["field"],
): RuleProblemCode | null {
  return check?.problems.find((p) => p.id === id && p.field === field)?.code ?? null;
}

/** What the "test this address" field says, before translation. */
export type TestVerdict =
  | { kind: "empty" }
  | { kind: "notAddress" }
  | { kind: "none"; host: string; disabled: string[] }
  | { kind: "match"; host: string; pattern: string; disabled: string[] };

export function testVerdict(address: string, check: SiteRulesCheck | null, drafts: RuleDraft[]): TestVerdict {
  if (!address.trim()) return { kind: "empty" };
  const test = check?.test;
  if (!test?.host) return { kind: "notAddress" };
  const pattern = (id: string) => drafts.find((d) => d.id === id)?.pattern.trim() ?? id;
  const disabled = test.disabled.map(pattern);
  if (!test.rule) return { kind: "none", host: test.host, disabled };
  return { kind: "match", host: test.host, pattern: pattern(test.rule), disabled };
}
