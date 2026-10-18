// RSS and Atom subscriptions: the list with its last check, and a form to
// add or change one.
import { useEffect, useState, type FormEvent } from "react";

import { useI18n } from "../i18n";
import type { MessageKey } from "../i18n/core";
import { api, errorMessage, isCommandError, onFeeds, type Feed, type FeedInput, type Settings } from "../lib/api";
import { INTERVALS, NEW_FEED, feedName, inputOf, intervalParts, withTypes } from "../lib/feeds";
import { formatDate } from "../lib/format";
import { Badge, Banner, Button, Dialog, Field, Switch, inputClass } from "./primitives";

type Form = { id: string | null; input: FeedInput; types: string; error: string | null; busy: boolean };

const ERROR_KEYS: Record<string, MessageKey> = {
  duplicate_feed: "feeds.errors.duplicate_feed",
  not_a_feed: "feeds.errors.not_a_feed",
  invalid_url: "feeds.errors.invalid_url",
};

export function FeedsDialog({ settings, onClose }: { settings: Settings; onClose: () => void }) {
  const { t } = useI18n();
  const [feeds, setFeeds] = useState<Feed[] | null>(null);
  const [form, setForm] = useState<Form | null>(null);
  const [removing, setRemoving] = useState<string | null>(null);
  const [checking, setChecking] = useState<Set<string>>(() => new Set());
  const [notice, setNotice] = useState<{ text: string; tone: "neutral" | "danger" } | null>(null);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void onFeeds((list) => !disposed && setFeeds(list)).then((fn) => (disposed ? fn() : (unlisten = fn)));
    api
      .feeds()
      .then((list) => !disposed && setFeeds(list))
      .catch((err: unknown) => !disposed && setNotice({ text: errorMessage(err), tone: "danger" }));
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const message = (err: unknown) => {
    const key = isCommandError(err) ? ERROR_KEYS[err.code] : undefined;
    return key ? t(key) : errorMessage(err);
  };

  const intervalLabel = (minutes: number) => {
    const { value, unit } = intervalParts(minutes);
    return t(`feeds.interval.${unit}`, { count: value });
  };

  const save = async (e?: FormEvent) => {
    e?.preventDefault();
    if (!form) return;
    const input = withTypes(form.input, form.types);
    setForm({ ...form, busy: true, error: null });
    try {
      if (form.id) {
        await api.updateFeed(form.id, input);
        setNotice(null);
      } else {
        const { check } = await api.addFeed(input);
        setNotice({
          text: check.added.length > 0 ? t("feeds.form.subscribed", { count: check.added.length }) : t("feeds.form.subscribedNone"),
          tone: "neutral",
        });
      }
      setForm(null);
    } catch (err) {
      setForm({ ...form, busy: false, error: message(err) });
    }
  };

  const check = async (feed: Feed) => {
    setChecking((s) => new Set(s).add(feed.id));
    try {
      const result = await api.checkFeed(feed.id);
      setNotice({
        text:
          result.added.length > 0
            ? t("feeds.checkResult.added", { count: result.added.length, name: feedName(feed) })
            : t("feeds.checkResult.none", { name: feedName(feed) }),
        tone: "neutral",
      });
    } catch (err) {
      setNotice({ text: message(err), tone: "danger" });
    } finally {
      setChecking((s) => {
        const next = new Set(s);
        next.delete(feed.id);
        return next;
      });
    }
  };

  const remove = async (id: string) => {
    setRemoving(null);
    try {
      await api.removeFeed(id);
    } catch (err) {
      setNotice({ text: message(err), tone: "danger" });
    }
  };

  const queueName = (id: string) => settings.queues.find((q) => q.id === id)?.name ?? id;

  if (form) {
    const set = (patch: Partial<FeedInput>) => setForm({ ...form, input: { ...form.input, ...patch } });
    return (
      <Dialog
        eyebrow={t("feeds.eyebrow")}
        title={form.id ? t("feeds.form.editTitle") : t("feeds.form.newTitle")}
        onClose={() => setForm(null)}
        footer={
          <>
            <Button variant="ghost" onClick={() => setForm(null)}>
              {t("common.cancel")}
            </Button>
            <Button variant="primary" onClick={() => void save()} disabled={form.busy || form.input.url.trim() === ""}>
              {form.id ? t("feeds.form.save") : t("feeds.form.subscribe")}
            </Button>
          </>
        }
      >
        <form onSubmit={save} className="flex flex-col gap-5">
          <Field label={t("feeds.form.url")} error={form.error}>
            {(id) => (
              <input
                id={id}
                value={form.input.url}
                onChange={(e) => set({ url: e.target.value })}
                placeholder="https://example.org/feed.xml"
                spellCheck={false}
                className={`${inputClass} h-9 font-mono text-[13px]`}
              />
            )}
          </Field>
          <Field label={t("feeds.form.types")} hint={t("feeds.form.typesHint")}>
            {(id) => (
              <input
                id={id}
                value={form.types}
                onChange={(e) => setForm({ ...form, types: e.target.value })}
                placeholder="mp3, m4a"
                spellCheck={false}
                className={`${inputClass} h-9 font-mono text-[13px]`}
              />
            )}
          </Field>
          <Field label={t("feeds.form.pattern")} hint={t("feeds.form.patternHint")}>
            {(id) => (
              <input
                id={id}
                value={form.input.filter.title_pattern}
                onChange={(e) => set({ filter: { ...form.input.filter, title_pattern: e.target.value } })}
                spellCheck={false}
                className={`${inputClass} h-9 font-mono text-[13px]`}
              />
            )}
          </Field>
          <div className="grid grid-cols-1 gap-5 sm:grid-cols-2">
            <Field label={t("feeds.form.interval")}>
              {(id) => (
                <select
                  id={id}
                  value={form.input.interval_minutes}
                  onChange={(e) => set({ interval_minutes: Number(e.target.value) })}
                  className={`${inputClass} h-9`}
                >
                  {/* A value set elsewhere stays selectable. */}
                  {[...new Set<number>([...INTERVALS, form.input.interval_minutes])]
                    .sort((a, b) => a - b)
                    .map((m) => (
                      <option key={m} value={m}>
                        {intervalLabel(m)}
                      </option>
                    ))}
                </select>
              )}
            </Field>
            <Field label={t("feeds.form.queue")}>
              {(id) => (
                <select id={id} value={form.input.queue} onChange={(e) => set({ queue: e.target.value })} className={`${inputClass} h-9`}>
                  {settings.queues.map((q) => (
                    <option key={q.id} value={q.id}>
                      {q.name}
                    </option>
                  ))}
                </select>
              )}
            </Field>
          </div>
          <Switch label={t("feeds.form.enabled")} hint={t("feeds.form.enabledHint")} checked={form.input.enabled} onChange={(enabled) => set({ enabled })} />
          {!form.id && (
            <Switch
              label={t("feeds.form.existing")}
              hint={t("feeds.form.existingHint")}
              checked={form.input.download_existing}
              onChange={(download_existing) => set({ download_existing })}
            />
          )}
        </form>
      </Dialog>
    );
  }

  return (
    <Dialog
      eyebrow={t("feeds.eyebrow")}
      title={t("feeds.title")}
      onClose={onClose}
      size="lg"
      focusFirstField={false}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t("common.close")}
          </Button>
          <Button variant="primary" onClick={() => setForm({ id: null, input: NEW_FEED, types: "", error: null, busy: false })}>
            {t("feeds.add")}
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-4">
        <p className="text-[13px] text-ink-secondary">{t("feeds.intro")}</p>
        {notice && <Banner tone={notice.tone}>{notice.text}</Banner>}
        {feeds !== null && feeds.length === 0 && <p className="text-[14px] text-ink-muted">{t("feeds.empty")}</p>}
        {feeds !== null && feeds.length > 0 && (
          <ul className="flex flex-col divide-y divide-line rounded-md border border-line">
            {feeds.map((feed) => (
              <li key={feed.id} className="flex flex-col gap-2 px-4 py-3">
                <div className="flex min-w-0 flex-col gap-2 sm:flex-row sm:items-start sm:justify-between sm:gap-3">
                  <div className="flex min-w-0 flex-col gap-0.5">
                    <div className="flex min-w-0 items-center gap-2">
                      <span className="truncate text-[14px] font-medium text-ink" title={feedName(feed)}>
                        {feedName(feed)}
                      </span>
                      {!feed.enabled && <Badge tone="neutral">{t("feeds.off")}</Badge>}
                    </div>
                    <span className="truncate font-mono text-[12px] text-ink-muted" title={feed.url}>
                      {feed.url}
                    </span>
                  </div>
                  <div className="-ml-3 flex shrink-0 items-center gap-1 sm:ml-0">
                    <Button size="sm" variant="ghost" onClick={() => void check(feed)} disabled={checking.has(feed.id)}>
                      {t("feeds.checkNow")}
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => setForm({ id: feed.id, input: inputOf(feed), types: feed.filter.extensions.join(", "), error: null, busy: false })}
                    >
                      {t("feeds.edit")}
                    </Button>
                    <Button size="sm" variant="ghost" onClick={() => setRemoving(feed.id)}>
                      {t("common.remove")}
                    </Button>
                  </div>
                </div>
                <p className="text-[12px] text-ink-secondary">
                  {[
                    feed.last_checked ? t("feeds.checked", { date: formatDate(feed.last_checked) }) : t("feeds.notChecked"),
                    intervalLabel(feed.interval_minutes),
                    queueName(feed.queue),
                    feed.filter.extensions.length > 0 ? feed.filter.extensions.join(", ") : null,
                    t("feeds.addedTotal", { count: feed.added_total }),
                  ]
                    .filter(Boolean)
                    .join(" · ")}
                </p>
                {feed.last_error && <p className="text-[12px] text-danger-fg">{t("feeds.error", { error: feed.last_error })}</p>}
                {removing === feed.id && (
                  <div className="flex flex-wrap items-center gap-2 rounded-md bg-surface-muted px-3 py-2">
                    <span className="mr-auto text-[13px] text-ink">{t("feeds.confirmRemove", { name: feedName(feed) })}</span>
                    <Button size="sm" variant="ghost" onClick={() => setRemoving(null)}>
                      {t("common.keep")}
                    </Button>
                    <Button size="sm" variant="danger" onClick={() => void remove(feed.id)}>
                      {t("common.remove")}
                    </Button>
                  </div>
                )}
              </li>
            ))}
          </ul>
        )}
      </div>
    </Dialog>
  );
}
