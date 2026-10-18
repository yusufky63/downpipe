// App frame: header, filter bar, selection bar, status bar, empty state and toasts.
import { Download, Moon, MoreHorizontal, Pause, Play, Plus, Search, Settings as SettingsIcon, Sun, Trash2, X } from "lucide-react";
import type { MouseEvent, RefObject } from "react";

import { useI18n } from "../i18n";
import type { Queue, Settings, WhenDone } from "../lib/api";
import { categoryLabel } from "../lib/categories";
import { formatSpeed } from "../lib/format";
import { FILTERS, filterLabel, type Filter } from "../lib/tasks";
import { Button, Dialog, IconButton } from "./primitives";

export function Logo() {
  return (
    <svg viewBox="0 0 1024 1024" className="size-7 shrink-0" aria-hidden="true">
      <rect width="1024" height="1024" rx="232" className="fill-primary-fill" />
      <g fill="none" stroke="currentColor" strokeWidth="80" strokeLinecap="round" strokeLinejoin="round" className="text-primary-contrast">
        <path d="M320 224 V336 L512 528" />
        <path d="M704 224 V336 L512 528" />
        <path d="M512 224 V800" />
        <path d="M368 656 L512 800 L656 656" />
      </g>
    </svg>
  );
}

export function Header({
  theme,
  onToggleTheme,
  onAdd,
  onPauseAll,
  onResumeAll,
  onSettings,
  hasRunning,
  hasPaused,
}: {
  theme: "light" | "dark";
  onToggleTheme: () => void;
  onAdd: () => void;
  onPauseAll: () => void;
  onResumeAll: () => void;
  onSettings: () => void;
  hasRunning: boolean;
  hasPaused: boolean;
}) {
  const { t } = useI18n();
  return (
    <header className="flex h-14 shrink-0 items-center gap-3 border-b border-line bg-canvas px-4">
      <Logo />
      <span className="display-medium text-[18px]">{t("common.appName")}</span>
      <div className="ml-auto flex items-center gap-1.5">
        <Button size="sm" variant="ghost" onClick={onPauseAll} disabled={!hasRunning} className="max-sm:px-2">
          <Pause size={16} strokeWidth={1.75} />
          <span className="max-sm:sr-only">{t("chrome.pauseAll")}</span>
        </Button>
        <Button size="sm" variant="ghost" onClick={onResumeAll} disabled={!hasPaused} className="max-sm:px-2">
          <Play size={16} strokeWidth={1.75} />
          <span className="max-sm:sr-only">{t("chrome.resumeAll")}</span>
        </Button>
        <IconButton label={theme === "dark" ? t("chrome.lightTheme") : t("chrome.darkTheme")} onClick={onToggleTheme}>
          {theme === "dark" ? <Sun size={18} strokeWidth={1.75} /> : <Moon size={18} strokeWidth={1.75} />}
        </IconButton>
        <IconButton label={t("chrome.settings")} onClick={onSettings}>
          <SettingsIcon size={18} strokeWidth={1.75} />
        </IconButton>
        <Button variant="primary" onClick={onAdd} className="ml-1">
          <Plus size={16} strokeWidth={1.75} />
          <span className="max-sm:sr-only">{t("chrome.addLink")}</span>
        </Button>
      </div>
    </header>
  );
}

const SELECT = "h-8 max-w-28 rounded-md border border-line-strong bg-canvas px-2 text-[13px] text-ink transition-colors duration-150 focus:border-primary sm:max-w-36";

export function FilterBar({
  filter,
  counts,
  onFilter,
  onClearFinished,
  canClear,
  query,
  onQuery,
  category,
  categories,
  onCategory,
  sortIsDefault,
  onResetSort,
  onExport,
  canExport,
  searchRef,
  queues,
  queue,
  onQueue,
  onStartQueue,
  onStopQueue,
}: {
  filter: Filter;
  counts: Record<Filter, number>;
  onFilter: (f: Filter) => void;
  onClearFinished: () => void;
  canClear: boolean;
  query: string;
  onQuery: (q: string) => void;
  /** "" = all categories, "-" = none. */
  category: string;
  categories: string[];
  onCategory: (c: string) => void;
  sortIsDefault: boolean;
  onResetSort: () => void;
  onExport: () => void;
  canExport: boolean;
  /** Ctrl+F focuses the search field. */
  searchRef: RefObject<HTMLInputElement | null>;
  queues: Queue[];
  /** "" = every queue. */
  queue: string;
  onQueue: (id: string) => void;
  onStartQueue: () => void;
  onStopQueue: () => void;
}) {
  const { t } = useI18n();
  return (
    <div className="flex shrink-0 flex-wrap items-center gap-x-1 gap-y-2 border-b border-line px-4 py-2">
      <div role="tablist" aria-label={t("chrome.filterLabel")} className="scrollbar-none flex max-w-full min-w-0 items-center gap-1 overflow-x-auto">
        {FILTERS.map((id) => {
          const active = filter === id;
          return (
            <button
              key={id}
              type="button"
              role="tab"
              aria-selected={active}
              onClick={() => onFilter(id)}
              className={`inline-flex h-8 shrink-0 items-center gap-2 rounded-md border px-2.5 text-[13px] transition-colors duration-150 ${
                active ? "border-primary bg-primary-soft text-ink" : "border-transparent text-ink-secondary hover:bg-surface-muted hover:text-ink"
              }`}
            >
              {filterLabel(id)}
              <span className={`num font-mono text-[12px] ${active ? "text-primary" : "text-ink-muted"}`}>{counts[id]}</span>
            </button>
          );
        })}
      </div>
      <div className="ml-auto flex min-w-0 flex-1 flex-wrap items-center justify-end gap-1.5 sm:flex-none">
        <label className="relative min-w-36 flex-1 sm:w-56 sm:flex-none">
          <span className="sr-only">{t("chrome.searchLabel")}</span>
          <Search size={14} strokeWidth={1.75} className="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-ink-muted" aria-hidden="true" />
          <input
            ref={searchRef}
            type="search"
            value={query}
            onChange={(e) => onQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") onQuery("");
            }}
            placeholder={t("chrome.search")}
            title={t("chrome.searchTitle")}
            spellCheck={false}
            className="h-8 w-full rounded-md border border-line-strong bg-canvas pr-2 pl-8 text-[13px] text-ink placeholder:text-ink-muted transition-colors duration-150 focus:border-primary"
          />
        </label>
        {queues.length > 1 && (
          <select aria-label={t("chrome.queue")} value={queue} onChange={(e) => onQueue(e.target.value)} className={SELECT}>
            <option value="">{t("chrome.allQueues")}</option>
            {queues.map((q) => (
              <option key={q.id} value={q.id}>
                {q.name}
              </option>
            ))}
          </select>
        )}
        {queue !== "" && (
          <>
            <IconButton label={t("chrome.startQueue")} onClick={onStartQueue}>
              <Play size={16} strokeWidth={1.75} />
            </IconButton>
            <IconButton label={t("chrome.stopQueue")} onClick={onStopQueue}>
              <Pause size={16} strokeWidth={1.75} />
            </IconButton>
          </>
        )}
        {categories.length > 0 && (
          <select aria-label={t("chrome.category")} value={category} onChange={(e) => onCategory(e.target.value)} className={SELECT}>
            <option value="">{t("chrome.allTypes")}</option>
            {categories.map((c) => (
              <option key={c} value={c}>
                {categoryLabel(c)}
              </option>
            ))}
            <option value="-">{t("chrome.uncategorized")}</option>
          </select>
        )}
        {!sortIsDefault && (
          <Button size="sm" variant="ghost" onClick={onResetSort} title={t("chrome.newestFirstTitle")}>
            {t("chrome.newestFirst")}
          </Button>
        )}
        <IconButton label={t("chrome.exportLinks")} onClick={onExport} disabled={!canExport}>
          <Download size={16} strokeWidth={1.75} />
        </IconButton>
        <Button size="sm" variant="ghost" onClick={onClearFinished} disabled={!canClear}>
          {t("chrome.clearFinished")}
        </Button>
      </div>
    </div>
  );
}

/**
 * Floats over the bottom of the list while several downloads are selected,
 * so ticking boxes never moves the rows under the pointer.
 */
export function SelectionBar({
  count,
  canResume,
  canPause,
  onResume,
  onPause,
  onRemove,
  onMore,
  onClear,
}: {
  count: number;
  canResume: boolean;
  canPause: boolean;
  onResume: () => void;
  onPause: () => void;
  onRemove: () => void;
  /** Opens the right-click menu for the selection at this point. */
  onMore: (x: number, y: number, keyboard: boolean) => void;
  onClear: () => void;
}) {
  const { t } = useI18n();
  const more = (e: MouseEvent<HTMLButtonElement>) => {
    const r = e.currentTarget.getBoundingClientRect();
    onMore(r.left, r.top - 4, e.detail === 0);
  };
  return (
    <div
      role="toolbar"
      aria-label={t("list.bar.label")}
      className="anim-rise absolute bottom-11 left-1/2 z-50 flex max-w-[calc(100%-32px)] -translate-x-1/2 items-center gap-1 overflow-x-auto rounded-lg border border-line bg-canvas px-2 py-1.5 shadow-popover scrollbar-none"
    >
      <span className="num shrink-0 px-2 font-mono text-[12px] text-primary">{t("list.bar.count", { count })}</span>
      <span className="h-5 w-px shrink-0 bg-line" aria-hidden="true" />
      <Button size="sm" variant="ghost" onClick={onResume} disabled={!canResume}>
        <Play size={16} strokeWidth={1.75} />
        <span className="max-sm:sr-only">{t("list.bar.resume")}</span>
      </Button>
      <Button size="sm" variant="ghost" onClick={onPause} disabled={!canPause}>
        <Pause size={16} strokeWidth={1.75} />
        <span className="max-sm:sr-only">{t("list.bar.pause")}</span>
      </Button>
      <Button size="sm" variant="ghost" onClick={onRemove} className="text-danger-fg hover:text-danger-fg">
        <Trash2 size={16} strokeWidth={1.75} />
        <span className="max-sm:sr-only">{t("list.bar.remove")}</span>
      </Button>
      <Button size="sm" variant="ghost" onClick={more} aria-haspopup="menu">
        <MoreHorizontal size={16} strokeWidth={1.75} />
        <span className="max-sm:sr-only">{t("list.bar.more")}</span>
      </Button>
      <IconButton label={t("list.bar.clear")} onClick={onClear}>
        <X size={16} strokeWidth={1.75} />
      </IconButton>
    </div>
  );
}

const WHEN_DONE: readonly WhenDone[] = ["nothing", "quit", "sleep", "shutdown"];

export function WhenDoneSelect({ value, onChange }: { value: WhenDone; onChange: (w: WhenDone) => void }) {
  const { t } = useI18n();
  return (
    <label className={`inline-flex shrink-0 items-center gap-1.5 ${value === "nothing" ? "" : "text-primary"}`}>
      <span className="max-sm:sr-only">{t("chrome.whenDone.label")}</span>
      <select
        value={value}
        onChange={(e) => onChange(e.target.value as WhenDone)}
        className="h-6 rounded-sm border border-line bg-canvas px-1 font-mono text-[12px] text-ink focus:border-primary"
      >
        {WHEN_DONE.map((w) => (
          <option key={w} value={w}>
            {t(`chrome.whenDone.${w}`)}
          </option>
        ))}
      </select>
    </label>
  );
}

export function WhenDoneDialog({
  action,
  secondsLeft,
  onCancel,
  onNow,
}: {
  action: WhenDone;
  secondsLeft: number;
  onCancel: () => void;
  onNow: () => void;
}) {
  const { t } = useI18n();
  const title =
    action === "shutdown" ? t("chrome.whenDone.titleShutdown") : action === "sleep" ? t("chrome.whenDone.titleSleep") : t("chrome.whenDone.titleQuit");
  return (
    <Dialog
      eyebrow={t("chrome.whenDone.eyebrow")}
      title={title}
      onClose={onCancel}
      footer={
        <>
          <Button variant="ghost" onClick={onNow}>
            {t("chrome.whenDone.now")}
          </Button>
          <Button variant="primary" onClick={onCancel}>
            {t("common.cancel")}
          </Button>
        </>
      }
    >
      <p className="num text-[15px] text-ink-secondary">{t("chrome.whenDone.countdown", { count: secondsLeft })}</p>
    </Dialog>
  );
}

export function StatusBar({
  speed,
  active,
  queued,
  selected,
  settings,
  whenDone,
  onWhenDone,
}: {
  speed: number;
  active: number;
  queued: number;
  /** Rows selected in the list; shown from two on. */
  selected: number;
  settings: Settings | null;
  whenDone: WhenDone;
  onWhenDone: (w: WhenDone) => void;
}) {
  const { t } = useI18n();
  return (
    <footer className="flex h-8 shrink-0 items-center gap-4 overflow-hidden border-t border-line bg-surface px-4 font-mono text-[12px] text-ink-muted">
      <span className="num shrink-0">{t("chrome.status.downloading", { count: active })}</span>
      <span className="num shrink-0 max-sm:hidden">{t("chrome.status.queued", { count: queued })}</span>
      <span className="num shrink-0 text-ink-secondary">{speed > 0 ? formatSpeed(speed) : t("chrome.status.idle")}</span>
      <WhenDoneSelect value={whenDone} onChange={onWhenDone} />
      {selected > 1 && <span className="num shrink-0 text-primary max-sm:hidden">{t("chrome.status.selected", { count: selected })}</span>}
      {settings && (
        <span className="ml-auto hidden truncate sm:inline">
          {settings.speed_limit > 0 ? t("chrome.status.limit", { speed: formatSpeed(settings.speed_limit) }) : t("chrome.status.noLimit")} ·{" "}
          {t("chrome.status.atATime", { count: settings.max_concurrent_downloads })} · {settings.download_dir}
        </span>
      )}
    </footer>
  );
}

export function EmptyState({ filtered, onAdd }: { filtered: boolean; onAdd: () => void }) {
  const { t } = useI18n();
  if (filtered) {
    return <p className="px-4 py-10 text-center text-[14px] text-ink-muted">{t("chrome.empty.filtered")}</p>;
  }
  return (
    <div className="anim-rise mx-auto flex max-w-[520px] flex-col items-start gap-4 px-6 py-16">
      <span className="eyebrow">{t("chrome.empty.eyebrow")}</span>
      <h1 className="display-medium text-[24px] leading-tight">{t("chrome.empty.title")}</h1>
      <p className="text-[15px] text-ink-secondary">{t("chrome.empty.body")}</p>
      <div className="flex items-center gap-3">
        <Button variant="primary" onClick={onAdd}>
          <Plus size={16} strokeWidth={1.75} />
          {t("chrome.empty.add")}
        </Button>
        <span className="font-mono text-[12px] text-ink-muted">{t("chrome.empty.orPaste")}</span>
      </div>
    </div>
  );
}

export interface Toast {
  id: number;
  text: string;
  tone: "danger" | "neutral";
}

export function Toasts({ toasts, onDismiss }: { toasts: Toast[]; onDismiss: (id: number) => void }) {
  const { t } = useI18n();
  return (
    <div className="pointer-events-none fixed right-4 bottom-12 z-1200 flex w-[min(380px,calc(100vw-32px))] flex-col gap-2" aria-live="polite">
      {toasts.map((toast) => (
        <div
          key={toast.id}
          role="status"
          className={`anim-rise pointer-events-auto flex items-start gap-2 rounded-md border bg-canvas px-3 py-2.5 text-[13px] shadow-popover ${
            toast.tone === "danger" ? "border-danger text-danger-fg" : "border-line text-ink"
          }`}
        >
          <span className="flex-1 whitespace-pre-line">{toast.text}</span>
          <IconButton label={t("common.dismiss")} className="size-6" onClick={() => onDismiss(toast.id)}>
            <X size={14} strokeWidth={1.75} />
          </IconButton>
        </div>
      ))}
    </div>
  );
}
