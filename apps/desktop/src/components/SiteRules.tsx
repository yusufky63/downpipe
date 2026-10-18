import { ChevronDown, Plus, Trash2 } from "lucide-react";
import { Fragment, useEffect, useId, useState, type ReactNode } from "react";

import { useI18n } from "../i18n";
import { api, type RefusedHost, type RuleProxy, type SiteRulesCheck } from "../lib/api";
import { formatSpeed, rateToBytes, type RateUnit } from "../lib/format";
import { newRuleDraft, problemFor, refusedToOffer, ruleForRefusedHost, testVerdict, type RuleDraft } from "../lib/siteRules";
import { FolderInput } from "./FolderInput";
import { Badge, Button, Field, IconButton, inputClass } from "./primitives";

/** A translated sentence with elements at its {placeholders}. */
function fill(text: string, parts: Record<string, ReactNode>): ReactNode {
  return text.split(/\{(\w+)\}/).map((piece, i) => (i % 2 === 1 ? <Fragment key={i}>{parts[piece] ?? `{${piece}}`}</Fragment> : piece));
}

const mono = (text: string) => <span className="font-mono text-ink">{text}</span>;

/** Settings, Sites. The dialog owns the drafts and the check, so Save can refuse rules with problems. */
export function SiteRules({
  drafts,
  onChange,
  address,
  onAddress,
  check,
  queues,
}: {
  drafts: RuleDraft[];
  onChange: (drafts: RuleDraft[]) => void;
  address: string;
  onAddress: (address: string) => void;
  check: SiteRulesCheck | null;
  /** Saved queues, for the queue choice. */
  queues: { id: string; name: string }[];
}) {
  const { t } = useI18n();
  const testId = useId();
  const testHintId = useId();
  const [refused, setRefused] = useState<RefusedHost[]>([]);
  // New rules open for editing; saved ones show their summary.
  const [open, setOpen] = useState<Set<number>>(() => new Set());

  useEffect(() => {
    let live = true;
    api
      .refusedHosts()
      .then((hosts) => live && setRefused(hosts))
      .catch(() => live && setRefused([]));
    return () => {
      live = false;
    };
  }, []);

  const update = (key: number, patch: Partial<RuleDraft>) => onChange(drafts.map((d) => (d.key === key ? { ...d, ...patch } : d)));
  const toggle = (key: number) =>
    setOpen((keys) => {
      const next = new Set(keys);
      if (!next.delete(key)) next.add(key);
      return next;
    });
  const add = (next: RuleDraft[]) => {
    const added = next.find((d) => !drafts.some((old) => old.key === d.key));
    if (added) setOpen((keys) => new Set(keys).add(added.key));
    onChange(next);
  };

  const verdict = testVerdict(address, check, drafts);
  const matched = check?.test?.rule ?? null;
  const offered = refusedToOffer(refused, drafts);

  return (
    <>
      <p className="text-[13px] text-ink-secondary">{t("sites.intro")}</p>

      <div className="flex flex-col gap-1.5">
        <label htmlFor={testId} className="label">
          {t("sites.test")}
        </label>
        <input
          id={testId}
          value={address}
          placeholder={t("sites.testPlaceholder")}
          spellCheck={false}
          aria-describedby={testHintId}
          onChange={(e) => onAddress(e.target.value)}
          className={`${inputClass} h-9 font-mono text-[13px]`}
        />
        <div id={testHintId} aria-live="polite" className="text-[12px]">
          {verdict.kind === "empty" && <p className="text-ink-muted">{t("sites.testHint")}</p>}
          {verdict.kind === "notAddress" && <p className="text-warning-fg">{t("sites.testNotAddress")}</p>}
          {verdict.kind === "none" && <p className="text-ink-secondary">{fill(t("sites.testNone"), { host: mono(verdict.host) })}</p>}
          {verdict.kind === "match" && (
            <p className="text-ink-secondary">{fill(t("sites.testMatch"), { host: mono(verdict.host), pattern: mono(verdict.pattern) })}</p>
          )}
          {(verdict.kind === "none" || verdict.kind === "match") && verdict.disabled.length > 0 && (
            <p className="text-warning-fg">{fill(t("sites.testDisabled"), { patterns: mono(verdict.disabled.join(", ")) })}</p>
          )}
        </div>
      </div>

      {offered.length > 0 && (
        <section aria-label={t("sites.refusedHeading")} className="flex flex-col gap-2 rounded-md border border-line p-3">
          <h3 className="label">{t("sites.refusedHeading")}</h3>
          <p className="text-[12px] text-ink-muted">{t("sites.refusedIntro")}</p>
          <ul className="flex flex-col gap-1.5">
            {offered.map((host) => (
              <li key={host.host} className="flex flex-wrap items-center justify-between gap-2">
                <span className="min-w-0 text-[13px] break-all text-ink-secondary">
                  {fill(t("sites.refusedHost", { count: host.connections }), { host: mono(host.host) })}
                </span>
                <Button size="sm" onClick={() => add(ruleForRefusedHost(drafts, host))}>
                  {t("sites.makeRule")}
                </Button>
              </li>
            ))}
          </ul>
        </section>
      )}

      {drafts.length === 0 ? (
        <p className="text-[13px] text-ink-muted">{t("sites.empty")}</p>
      ) : (
        <ul aria-label={t("sites.label")} className="flex flex-col gap-3">
          {drafts.map((d) => (
            <RuleItem
              key={d.key}
              draft={d}
              open={open.has(d.key)}
              matched={matched === d.id}
              check={check}
              queues={queues}
              onToggle={() => toggle(d.key)}
              onChange={(patch) => update(d.key, patch)}
              onRemove={() => onChange(drafts.filter((x) => x.key !== d.key))}
            />
          ))}
        </ul>
      )}
      <div className="flex flex-col gap-2">
        <div>
          <Button size="sm" onClick={() => add([...drafts, newRuleDraft()])}>
            <Plus size={16} strokeWidth={1.75} />
            {t("sites.add")}
          </Button>
        </div>
        <p className="text-[12px] text-ink-muted">{t("sites.patternHelp")}</p>
        <p className="text-[12px] text-ink-muted">{t("sites.applies")}</p>
      </div>
    </>
  );
}

function RuleItem({
  draft: d,
  open,
  matched,
  check,
  queues,
  onToggle,
  onChange,
  onRemove,
}: {
  draft: RuleDraft;
  open: boolean;
  matched: boolean;
  check: SiteRulesCheck | null;
  queues: { id: string; name: string }[];
  onToggle: () => void;
  onChange: (patch: Partial<RuleDraft>) => void;
  onRemove: () => void;
}) {
  const { t } = useI18n();
  const patternError = problemFor(check, d.id, "pattern");
  const problem = (field: "user_agent" | "referer" | "proxy_url") => {
    const code = problemFor(check, d.id, field);
    return code ? t(`sites.problems.${code}`) : null;
  };
  const name = d.pattern.trim();
  const queueName = queues.find((q) => q.id === d.queue)?.name;
  const panelId = useId();

  return (
    <li className={`rounded-md border p-3 ${matched ? "border-primary" : "border-line"}`}>
      <div className="grid grid-cols-[auto_minmax(0,1fr)_auto_auto] items-center gap-2">
        <input
          type="checkbox"
          role="switch"
          aria-label={name ? t("sites.use", { pattern: name }) : t("sites.useUnnamed")}
          checked={d.enabled}
          onChange={(e) => onChange({ enabled: e.target.checked })}
          className="switch"
        />
        <input
          aria-label={t("sites.pattern")}
          aria-invalid={patternError !== null}
          value={d.pattern}
          placeholder={t("sites.patternPlaceholder")}
          spellCheck={false}
          autoCapitalize="off"
          onChange={(e) => onChange({ pattern: e.target.value })}
          className={`${inputClass} h-8 font-mono text-[13px] ${patternError ? "border-danger-fg" : ""}`}
        />
        <IconButton
          label={name ? t("sites.edit", { pattern: name }) : t("sites.editUnnamed")}
          aria-expanded={open}
          aria-controls={panelId}
          onClick={onToggle}
        >
          <ChevronDown size={16} strokeWidth={1.75} className={`transition-transform duration-150 ${open ? "rotate-180" : ""}`} />
        </IconButton>
        <IconButton label={name ? t("sites.remove", { pattern: name }) : t("sites.removeUnnamed")} tone="danger" onClick={onRemove}>
          <Trash2 size={16} strokeWidth={1.75} />
        </IconButton>
      </div>
      {patternError && <p className="mt-1.5 text-[13px] text-danger-fg">{t(`sites.problems.${patternError}`)}</p>}
      <div className="mt-1.5 flex flex-wrap items-center gap-2">
        {matched && <Badge tone="primary">{t("sites.matches")}</Badge>}
        <p className="min-w-0 text-[12px] break-words text-ink-muted">{summary(d, queueName, t)}</p>
      </div>

      {open && (
        <div id={panelId} className="mt-3 grid grid-cols-1 gap-4 border-t border-line pt-3 sm:grid-cols-2">
          <Field label={t("sites.connections")} hint={t("sites.connectionsHint")}>
            {(id) => (
              <input
                id={id}
                type="number"
                min={1}
                max={32}
                value={d.connections}
                placeholder={t("sites.keep")}
                onChange={(e) => onChange({ connections: e.target.value })}
                className={`${inputClass} num h-9 font-mono`}
              />
            )}
          </Field>
          <Field label={t("sites.retries")} hint={t("sites.retriesHint")}>
            {(id) => (
              <input
                id={id}
                type="number"
                min={0}
                max={100}
                value={d.retries}
                placeholder={t("sites.keep")}
                onChange={(e) => onChange({ retries: e.target.value })}
                className={`${inputClass} num h-9 font-mono`}
              />
            )}
          </Field>
          <Field label={t("sites.speedLimit")} hint={t("sites.speedLimitHint")}>
            {(id) => (
              <div className="flex gap-2">
                <input
                  id={id}
                  type="number"
                  min={0}
                  step="any"
                  value={d.limit}
                  onChange={(e) => onChange({ limit: e.target.value })}
                  className={`${inputClass} num h-9 font-mono`}
                />
                <select
                  aria-label={t("settings.downloads.unit")}
                  value={d.unit}
                  onChange={(e) => onChange({ unit: e.target.value as RateUnit })}
                  className={`${inputClass} h-9 w-28 font-mono`}
                >
                  <option value="KiB">KiB/s</option>
                  <option value="MiB">MiB/s</option>
                </select>
              </div>
            )}
          </Field>
          <Field label={t("sites.queue")}>
            {(id) => (
              <select id={id} value={d.queue} onChange={(e) => onChange({ queue: e.target.value })} className={`${inputClass} h-9`}>
                <option value="">{t("sites.queueDefault")}</option>
                {queues.map((q) => (
                  <option key={q.id} value={q.id}>
                    {q.name}
                  </option>
                ))}
              </select>
            )}
          </Field>
          <div className="sm:col-span-2">
            <Field label={t("sites.userAgent")} error={problem("user_agent")}>
              {(id) => (
                <input
                  id={id}
                  value={d.user_agent}
                  placeholder={t("sites.keep")}
                  spellCheck={false}
                  onChange={(e) => onChange({ user_agent: e.target.value })}
                  className={`${inputClass} h-9 font-mono text-[13px]`}
                />
              )}
            </Field>
          </div>
          <div className="sm:col-span-2">
            <Field label={t("sites.referer")} hint={t("sites.refererHint")} error={problem("referer")}>
              {(id) => (
                <input
                  id={id}
                  value={d.referer}
                  placeholder="https://example.com/downloads"
                  spellCheck={false}
                  onChange={(e) => onChange({ referer: e.target.value })}
                  className={`${inputClass} h-9 font-mono text-[13px]`}
                />
              )}
            </Field>
          </div>
          <Field label={t("sites.proxy")}>
            {(id) => (
              <select id={id} value={d.proxy} onChange={(e) => onChange({ proxy: e.target.value as RuleProxy })} className={`${inputClass} h-9`}>
                <option value="inherit">{t("sites.proxyInherit")}</option>
                <option value="direct">{t("sites.proxyDirect")}</option>
                <option value="custom">{t("sites.proxyCustom")}</option>
              </select>
            )}
          </Field>
          {d.proxy === "custom" && (
            <Field label={t("sites.proxyUrl")} error={problem("proxy_url")}>
              {(id) => (
                <input
                  id={id}
                  value={d.proxy_url}
                  placeholder="socks5://127.0.0.1:1080"
                  spellCheck={false}
                  onChange={(e) => onChange({ proxy_url: e.target.value })}
                  className={`${inputClass} h-9 font-mono text-[13px]`}
                />
              )}
            </Field>
          )}
          <div className="sm:col-span-2">
            <Field label={t("sites.folder")} hint={t("sites.folderHint")}>
              {(id) => <FolderInput id={id} value={d.download_dir} placeholder={t("sites.keep")} onChange={(download_dir) => onChange({ download_dir })} />}
            </Field>
          </div>
        </div>
      )}
    </li>
  );
}

/** What a rule changes, in one line. */
function summary(d: RuleDraft, queueName: string | undefined, t: ReturnType<typeof useI18n>["t"]): string {
  const parts: string[] = [];
  if (!d.enabled) parts.push(t("sites.summary.off"));
  const connections = Number.parseInt(d.connections, 10);
  if (Number.isFinite(connections)) parts.push(t("sites.summary.connections", { count: Math.min(32, Math.max(1, connections)) }));
  const limit = rateToBytes(Number.parseFloat(d.limit), d.unit);
  if (limit > 0) parts.push(formatSpeed(limit));
  const retries = Number.parseInt(d.retries, 10);
  if (Number.isFinite(retries)) parts.push(t("sites.summary.retries", { count: Math.min(100, Math.max(0, retries)) }));
  if (d.proxy === "direct") parts.push(t("sites.summary.noProxy"));
  if (d.proxy === "custom" && d.proxy_url.trim()) parts.push(t("sites.summary.proxy", { proxy: d.proxy_url.trim() }));
  if (d.user_agent.trim()) parts.push(t("sites.summary.userAgent"));
  if (d.referer.trim()) parts.push(t("sites.summary.referer"));
  if (d.download_dir.trim()) parts.push(t("sites.summary.folder", { folder: d.download_dir.trim() }));
  if (d.queue && queueName) parts.push(t("sites.summary.queue", { queue: queueName }));
  if (parts.length === (d.enabled ? 0 : 1)) parts.push(t("sites.summary.nothing"));
  return parts.join(" · ");
}
