import { describe, expect, it } from "vitest";
import type { SiteRule, SiteRulesCheck } from "./api";
import { fromRuleDraft, newRuleDraft, problemFor, refusedToOffer, ruleForRefusedHost, testVerdict, toRuleDraft } from "./siteRules";

const saved: SiteRule = {
  id: "abc",
  pattern: "*.example.com",
  enabled: true,
  max_connections: 2,
  user_agent: "Agent/1",
  referer: "",
  proxy: "custom",
  proxy_url: "socks5://127.0.0.1:1080",
  speed_limit: 2 * 1024 * 1024,
  max_retries: null,
  download_dir: "Example",
  queue: "night",
};

describe("rule drafts", () => {
  it("round-trip a saved rule", () => {
    const draft = toRuleDraft(saved);
    expect(draft).toMatchObject({ connections: "2", retries: "", limit: "2", unit: "MiB" });
    expect(fromRuleDraft(draft)).toEqual(saved);
  });

  it("keep the global setting for empty fields and clamp numbers", () => {
    const rule = fromRuleDraft({
      ...newRuleDraft(" dl.example.com "),
      connections: "99",
      retries: "-3",
      limit: "0",
      proxy: "direct",
      proxy_url: "http://ignored:8080",
    });
    expect(rule).toMatchObject({
      pattern: "dl.example.com",
      max_connections: 32,
      max_retries: 0,
      speed_limit: null,
      proxy: "direct",
      proxy_url: "",
    });
    expect(fromRuleDraft(newRuleDraft()).max_connections).toBeNull();
  });

  it("give new rules distinct ids before they are saved", () => {
    const a = newRuleDraft();
    const b = newRuleDraft();
    expect(a.id).not.toBe("");
    expect(a.id).not.toBe(b.id);
    expect(toRuleDraft(saved).id).toBe("abc");
  });
});

describe("hosts that refused connections", () => {
  const refused = { host: "dl.example.com", connections: 2 };

  it("become a new rule, or set the number on the host's own rule", () => {
    const added = ruleForRefusedHost([toRuleDraft(saved)], refused);
    expect(added).toHaveLength(2);
    expect(added[1]).toMatchObject({ pattern: "dl.example.com", connections: "2" });

    const own = { ...newRuleDraft("DL.example.com"), enabled: false };
    const updated = ruleForRefusedHost([own], refused);
    expect(updated).toHaveLength(1);
    expect(updated[0]).toMatchObject({ connections: "2", enabled: true });
  });

  it("are offered until a draft holds the host to that many", () => {
    const own = newRuleDraft("dl.example.com");
    expect(refusedToOffer([refused], [own])).toHaveLength(1);
    expect(refusedToOffer([refused], [{ ...own, connections: "4" }])).toHaveLength(1);
    expect(refusedToOffer([refused], [{ ...own, connections: "2" }])).toHaveLength(0);
    expect(refusedToOffer([refused], [{ ...own, connections: "2", enabled: false }])).toHaveLength(1);
  });
});

describe("checks", () => {
  const drafts = [toRuleDraft(saved), toRuleDraft({ ...saved, id: "off", pattern: "dl.example.com", enabled: false })];
  const check = (test: SiteRulesCheck["test"]): SiteRulesCheck => ({
    problems: [{ id: "abc", field: "referer", code: "referer_invalid" }],
    test,
  });

  it("find a field's problem", () => {
    expect(problemFor(check(null), "abc", "referer")).toBe("referer_invalid");
    expect(problemFor(check(null), "abc", "pattern")).toBeNull();
    expect(problemFor(null, "abc", "referer")).toBeNull();
  });

  it("turn the address test into a verdict with patterns", () => {
    expect(testVerdict("  ", check(null), drafts)).toEqual({ kind: "empty" });
    expect(testVerdict("not a link", check({ host: null, rule: null, disabled: [] }), drafts)).toEqual({ kind: "notAddress" });
    expect(testVerdict("https://dl.example.com/a", check({ host: "dl.example.com", rule: "abc", disabled: ["off"] }), drafts)).toEqual({
      kind: "match",
      host: "dl.example.com",
      pattern: "*.example.com",
      disabled: ["dl.example.com"],
    });
    expect(testVerdict("example.org", check({ host: "example.org", rule: null, disabled: [] }), drafts)).toEqual({
      kind: "none",
      host: "example.org",
      disabled: [],
    });
  });
});
