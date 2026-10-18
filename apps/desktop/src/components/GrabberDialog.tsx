// IDM's site grabber: an address and filters, the crawl's progress, then the
// files found with checkboxes and sizes, added to a queue.
import { useEffect, useEffectEvent, useMemo, useRef, useState, type FormEvent } from "react";

import { useI18n } from "../i18n";
import type { MessageKey } from "../i18n/core";
import {
  MAIN_QUEUE,
  api,
  errorMessage,
  isCommandError,
  onGrabEvent,
  type GrabEvent,
  type GrabOptions,
  type GrabProgress,
  type GrabReport,
  type Settings,
  type Task,
} from "../lib/api";
import { formatBytes } from "../lib/format";
import {
  GRAB_DEFAULTS,
  MAX_DEPTH,
  MAX_PAGES,
  MIN_DEPTH,
  clampInt,
  filterFiles,
  grabExtensions,
  grabFraction,
  selectedSize,
  toggleShown,
} from "../lib/grabber";
import { Banner, Button, Dialog, Field, Switch, inputClass } from "./primitives";

/** Files whose size the grabber asks for at most (downpipe_crawl::site::MAX_SIZED). */
const MAX_SIZED = 500;

type Stage =
  | { kind: "form"; error: string | null }
  | { kind: "running"; id: string; progress: GrabProgress | null; stopping: boolean }
  | { kind: "results"; report: GrabReport };

const ERROR_KEYS: Record<string, MessageKey> = {
  invalid_url: "grabber.errors.invalid_url",
  no_types: "grabber.errors.no_types",
  robots_blocked: "grabber.errors.robots_blocked",
  not_html: "grabber.errors.not_html",
};

export function GrabberDialog({
  settings,
  onClose,
  onAdded,
  onNotice,
}: {
  settings: Settings;
  onClose: () => void;
  onAdded: (tasks: Task[]) => void;
  /** A short message for the app's toasts once files were added. */
  onNotice: (text: string) => void;
}) {
  const { t } = useI18n();
  const [url, setUrl] = useState("");
  const [depth, setDepth] = useState(String(GRAB_DEFAULTS.depth));
  const [pages, setPages] = useState(String(GRAB_DEFAULTS.max_pages));
  const [delay, setDelay] = useState(String(GRAB_DEFAULTS.delay_ms));
  const [chosenCategories, setChosenCategories] = useState<Set<string>>(() => new Set());
  const [custom, setCustom] = useState("");
  const [sameHost, setSameHost] = useState(GRAB_DEFAULTS.same_host);
  const [robots, setRobots] = useState(GRAB_DEFAULTS.respect_robots);
  const [sizes, setSizes] = useState(GRAB_DEFAULTS.check_sizes);
  const [stage, setStage] = useState<Stage>({ kind: "form", error: null });
  const [chosen, setChosen] = useState<Set<string>>(() => new Set());
  const [query, setQuery] = useState("");
  const [queue, setQueue] = useState(MAIN_QUEUE);
  const [paused, setPaused] = useState(false);
  const [adding, setAdding] = useState(false);
  const [addError, setAddError] = useState<string | null>(null);
  const running = useRef<string | null>(null);

  const onEvent = useEffectEvent((event: GrabEvent) => {
    if (event.id !== running.current) return;
    if (event.kind === "progress") {
      setStage((s) => (s.kind === "running" ? { ...s, progress: event.progress } : s));
    } else if (event.kind === "done") {
      running.current = null;
      setChosen(new Set(event.report.files.map((f) => f.url)));
      setStage({ kind: "results", report: event.report });
    } else {
      running.current = null;
      const key = ERROR_KEYS[event.code];
      const text = key ? t(key) : event.code === "start_page" ? t("grabber.errors.start_page", { error: event.message }) : event.message;
      setStage({ kind: "form", error: text });
    }
  });

  // Events of this dialog's grab only; a grab left behind is stopped on close.
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void onGrabEvent((event) => onEvent(event)).then((fn) => (disposed ? fn() : (unlisten = fn)));
    return () => {
      disposed = true;
      unlisten?.();
      if (running.current) void api.cancelGrab(running.current).catch(() => undefined);
    };
  }, []);

  const categories = settings.categories;
  const extensions = grabExtensions(categories, chosenCategories, custom);

  const start = async (e?: FormEvent) => {
    e?.preventDefault();
    const options: GrabOptions = {
      url: url.trim(),
      depth: clampInt(depth, MIN_DEPTH, MAX_DEPTH, GRAB_DEFAULTS.depth),
      max_pages: clampInt(pages, 1, MAX_PAGES, GRAB_DEFAULTS.max_pages),
      extensions,
      same_host: sameHost,
      respect_robots: robots,
      delay_ms: clampInt(delay, 0, 60000, GRAB_DEFAULTS.delay_ms),
      check_sizes: sizes,
    };
    if (!/^https?:\/\/\S+$/i.test(options.url)) {
      setStage({ kind: "form", error: t("grabber.errors.invalid_url") });
      return;
    }
    if (extensions.length === 0) {
      setStage({ kind: "form", error: t("grabber.errors.no_types") });
      return;
    }
    try {
      const id = await api.startGrab(options);
      running.current = id;
      setStage({ kind: "running", id, progress: null, stopping: false });
    } catch (err) {
      setStage({ kind: "form", error: errorMessage(err) });
    }
  };

  const stop = () => {
    if (stage.kind !== "running") return;
    setStage({ ...stage, stopping: true });
    void api.cancelGrab(stage.id).catch(() => undefined);
  };

  const report = stage.kind === "results" ? stage.report : null;
  const shown = useMemo(() => (report ? filterFiles(report.files, query) : []), [report, query]);
  const total = report ? selectedSize(report.files, chosen) : { bytes: 0, unknown: 0 };
  const count = report ? report.files.filter((f) => chosen.has(f.url)).length : 0;

  const addChosen = async () => {
    if (!report) return;
    setAdding(true);
    setAddError(null);
    const added: Task[] = [];
    const problems: string[] = [];
    let listed = 0;
    for (const file of report.files.filter((f) => chosen.has(f.url))) {
      try {
        added.push(await api.add({ url: file.url, queue, paused, from_app: true }));
      } catch (err) {
        if (isCommandError(err) && err.code === "duplicate") listed += 1;
        else problems.push(t("add.errors.forLink", { url: file.url, error: errorMessage(err) }));
      }
    }
    setAdding(false);
    if (added.length > 0) onAdded(added);
    const parts = [t("grabber.results.added", { count: added.length })];
    if (listed > 0) parts.push(t("grabber.results.alreadyListed", { count: listed }));
    if (problems.length > 0) {
      setAddError(problems.join("\n"));
      return;
    }
    onNotice(parts.join(" "));
    onClose();
  };

  const toggleCategory = (name: string) =>
    setChosenCategories((set) => {
      const next = new Set(set);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });

  if (stage.kind === "running") {
    const p = stage.progress;
    const fraction = p ? grabFraction(p) : 0;
    return (
      <Dialog
        eyebrow={t("grabber.eyebrow")}
        title={url.trim()}
        onClose={onClose}
        focusFirstField={false}
        footer={
          <Button variant="secondary" onClick={stop} disabled={stage.stopping}>
            {t("grabber.stop")}
          </Button>
        }
      >
        <div className="flex flex-col gap-3" aria-live="polite">
          <div className="relative h-1.5 w-full overflow-hidden rounded-full bg-surface-muted">
            <div className="bar-fill absolute inset-y-0 left-0 bg-primary" style={{ width: `${Math.round(fraction * 100)}%` }} />
          </div>
          <div className="flex flex-wrap items-baseline justify-between gap-2 text-[13px]">
            <span className="text-ink">
              {p?.phase === "sizes"
                ? t("grabber.progress.sizes", { done: p.sized, count: p.to_size })
                : t("grabber.progress.pages", { done: p?.pages_done ?? 0, count: p?.pages_total ?? 1 })}
            </span>
            <span className="num font-mono text-ink-secondary">{t("grabber.progress.files", { count: p?.files ?? 0 })}</span>
          </div>
          <p className="truncate font-mono text-[12px] text-ink-muted" title={p?.current ?? ""}>
            {p?.current ?? " "}
          </p>
        </div>
      </Dialog>
    );
  }

  if (report) {
    const allShown = shown.length > 0 && shown.every((f) => chosen.has(f.url));
    const sizeText = formatBytes(total.bytes);
    return (
      <Dialog
        eyebrow={t("grabber.eyebrow")}
        title={t("grabber.results.title", { count: report.files.length })}
        onClose={onClose}
        size="lg"
        focusFirstField={false}
        footer={
          <>
            {report.files.length > 0 && (
              <label className="mr-auto flex items-center gap-2 text-[13px] text-ink-secondary">
                <input type="checkbox" checked={paused} onChange={(e) => setPaused(e.target.checked)} className="size-4 accent-primary" />
                {t("add.addPaused")}
              </label>
            )}
            <Button variant="ghost" onClick={() => setStage({ kind: "form", error: null })}>
              {t("grabber.again")}
            </Button>
            <Button variant="primary" onClick={() => void addChosen()} disabled={adding || count === 0}>
              {t("grabber.results.add", { count })}
            </Button>
          </>
        }
      >
        <div className="flex flex-col gap-3">
          {report.canceled && <Banner tone="neutral">{t("grabber.results.canceled")}</Banner>}
          {report.hit_page_limit && <Banner tone="neutral">{t("grabber.results.pageLimit")}</Banner>}
          {report.blocked > 0 && <Banner tone="neutral">{t("grabber.results.blocked", { count: report.blocked })}</Banner>}
          {report.failed > 0 && <Banner tone="warning">{t("grabber.results.failed", { count: report.failed })}</Banner>}
          {addError && <Banner tone="danger">{addError}</Banner>}
          {report.files.length === 0 ? (
            <p className="text-[14px] text-ink-secondary">{t("grabber.results.none")}</p>
          ) : (
            <>
              <div className="flex flex-wrap items-center gap-2">
                <input
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                  placeholder={t("grabber.results.filter")}
                  aria-label={t("grabber.results.filter")}
                  spellCheck={false}
                  className={`${inputClass} h-8 min-w-0 flex-1 text-[13px]`}
                />
                {settings.queues.length > 1 && (
                  <select
                    value={queue}
                    onChange={(e) => setQueue(e.target.value)}
                    aria-label={t("add.queue.label")}
                    className={`${inputClass} h-8 w-auto max-w-40 text-[13px]`}
                  >
                    {settings.queues.map((q) => (
                      <option key={q.id} value={q.id}>
                        {q.name}
                      </option>
                    ))}
                  </select>
                )}
              </div>
              <div className="max-h-[46vh] overflow-auto rounded-md border border-line">
                <table className="w-full table-fixed text-[13px]">
                  <thead className="sticky top-0 bg-surface-muted text-left">
                    <tr>
                      <th className="w-9 px-3 py-2">
                        <input
                          type="checkbox"
                          checked={allShown}
                          onChange={() => setChosen((c) => toggleShown(c, shown))}
                          aria-label={t("grabber.results.selectAll")}
                          className="size-4 accent-primary"
                        />
                      </th>
                      <th className="label px-2 py-2">{t("grabber.results.name")}</th>
                      <th className="label w-24 px-2 py-2 text-right">{t("grabber.results.size")}</th>
                      <th className="label hidden w-[38%] px-2 py-2 sm:table-cell">{t("grabber.results.foundOn")}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {shown.map((f) => (
                      <tr key={f.url} className="border-t border-line hover:bg-surface-muted">
                        <td className="px-3 py-1.5">
                          <input
                            type="checkbox"
                            checked={chosen.has(f.url)}
                            onChange={() =>
                              setChosen((c) => {
                                const next = new Set(c);
                                if (next.has(f.url)) next.delete(f.url);
                                else next.add(f.url);
                                return next;
                              })
                            }
                            aria-label={f.name}
                            className="size-4 accent-primary"
                          />
                        </td>
                        <td className="truncate px-2 py-1.5 text-ink" title={f.url}>
                          {f.name}
                        </td>
                        <td className="num px-2 py-1.5 text-right font-mono text-ink-secondary">
                          {f.size === null ? t("common.none") : formatBytes(f.size)}
                        </td>
                        <td className="hidden truncate px-2 py-1.5 font-mono text-[12px] text-ink-muted sm:table-cell" title={f.found_on}>
                          {f.found_on}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              <p className="text-[12px] text-ink-muted">
                {t("grabber.results.selected", { count, size: sizeText })}
                {total.unknown > 0 ? ` · ${t("grabber.results.unknownSizes", { count: total.unknown })}` : ""}
              </p>
            </>
          )}
        </div>
      </Dialog>
    );
  }

  return (
    <Dialog
      eyebrow={t("grabber.eyebrow")}
      title={t("grabber.title")}
      onClose={onClose}
      size="lg"
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t("common.cancel")}
          </Button>
          <Button variant="primary" onClick={() => void start()} disabled={url.trim() === "" || extensions.length === 0}>
            {t("grabber.start")}
          </Button>
        </>
      }
    >
      <form onSubmit={start} className="flex flex-col gap-5">
        <Field label={t("grabber.address.label")} hint={t("grabber.address.hint")} error={stage.kind === "form" ? stage.error : null}>
          {(id) => (
            <input
              id={id}
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder="https://example.org/downloads/"
              spellCheck={false}
              className={`${inputClass} h-9 font-mono text-[13px]`}
            />
          )}
        </Field>

        <div className="grid grid-cols-1 gap-5 sm:grid-cols-3">
          <Field label={t("grabber.depth.label")} hint={t("grabber.depth.hint")}>
            {(id) => (
              <select id={id} value={depth} onChange={(e) => setDepth(e.target.value)} className={`${inputClass} h-9`}>
                {[1, 2, 3, 4, 5].map((d) => (
                  <option key={d} value={d}>
                    {d}
                  </option>
                ))}
              </select>
            )}
          </Field>
          <Field label={t("grabber.pages.label")} hint={t("grabber.pages.hint")}>
            {(id) => (
              <input
                id={id}
                type="number"
                min={1}
                max={MAX_PAGES}
                value={pages}
                onChange={(e) => setPages(e.target.value)}
                className={`${inputClass} num h-9 font-mono`}
              />
            )}
          </Field>
          <Field label={t("grabber.delay.label")} hint={t("grabber.delay.hint")}>
            {(id) => (
              <input
                id={id}
                type="number"
                min={0}
                max={60000}
                step={100}
                value={delay}
                onChange={(e) => setDelay(e.target.value)}
                className={`${inputClass} num h-9 font-mono`}
              />
            )}
          </Field>
        </div>

        <fieldset className="flex min-w-0 flex-col gap-2">
          <legend className="label mb-1.5">{t("grabber.types.label")}</legend>
          <div className="flex flex-wrap gap-x-4 gap-y-2">
            {categories.map((c) => (
              <label key={c.name} className="flex items-center gap-2 text-[13px] text-ink" title={c.extensions.join(", ")}>
                <input type="checkbox" checked={chosenCategories.has(c.name)} onChange={() => toggleCategory(c.name)} className="size-4 accent-primary" />
                {c.name}
              </label>
            ))}
          </div>
          <input
            value={custom}
            onChange={(e) => setCustom(e.target.value)}
            aria-label={t("grabber.types.custom")}
            placeholder={`${t("grabber.types.custom")}: ${t("grabber.types.customPlaceholder")}`}
            spellCheck={false}
            className={`${inputClass} h-9 font-mono text-[13px]`}
          />
          {extensions.length > 0 && <p className="truncate font-mono text-[12px] text-ink-muted">{extensions.join(" ")}</p>}
        </fieldset>

        <div className="flex flex-col gap-4">
          <Switch label={t("grabber.sameSite.label")} hint={t("grabber.sameSite.hint")} checked={sameHost} onChange={setSameHost} />
          <Switch label={t("grabber.robots.label")} hint={t("grabber.robots.hint")} checked={robots} onChange={setRobots} />
          <Switch label={t("grabber.sizes.label")} hint={t("grabber.sizes.hint", { count: MAX_SIZED })} checked={sizes} onChange={setSizes} />
        </div>
      </form>
    </Dialog>
  );
}
