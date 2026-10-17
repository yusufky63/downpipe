// App frame: header, filter bar, status bar, empty state and toasts.
import { Moon, Pause, Play, Plus, Settings as SettingsIcon, Sun, X } from "lucide-react";

import type { Settings } from "../lib/api";
import { formatSpeed } from "../lib/format";
import { FILTERS, type Filter } from "../lib/tasks";
import { Button, IconButton } from "./primitives";

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
  return (
    <header className="flex h-14 shrink-0 items-center gap-3 border-b border-line bg-canvas px-4">
      <Logo />
      <span className="display-medium text-[18px]">Downpipe</span>
      <div className="ml-auto flex items-center gap-1.5">
        <Button size="sm" variant="ghost" onClick={onPauseAll} disabled={!hasRunning} className="max-sm:px-2">
          <Pause size={16} strokeWidth={1.75} />
          <span className="max-sm:sr-only">Pause all</span>
        </Button>
        <Button size="sm" variant="ghost" onClick={onResumeAll} disabled={!hasPaused} className="max-sm:px-2">
          <Play size={16} strokeWidth={1.75} />
          <span className="max-sm:sr-only">Resume all</span>
        </Button>
        <IconButton label={theme === "dark" ? "Use light theme" : "Use dark theme"} onClick={onToggleTheme}>
          {theme === "dark" ? <Sun size={18} strokeWidth={1.75} /> : <Moon size={18} strokeWidth={1.75} />}
        </IconButton>
        <IconButton label="Settings" onClick={onSettings}>
          <SettingsIcon size={18} strokeWidth={1.75} />
        </IconButton>
        <Button variant="primary" onClick={onAdd} className="ml-1">
          <Plus size={16} strokeWidth={1.75} />
          <span className="max-sm:sr-only">Add link</span>
        </Button>
      </div>
    </header>
  );
}

export function FilterBar({
  filter,
  counts,
  onFilter,
  onClearFinished,
  canClear,
}: {
  filter: Filter;
  counts: Record<Filter, number>;
  onFilter: (f: Filter) => void;
  onClearFinished: () => void;
  canClear: boolean;
}) {
  return (
    <div className="scrollbar-none flex shrink-0 items-center gap-1 overflow-x-auto border-b border-line px-4 py-2">
      <div role="tablist" aria-label="Filter downloads" className="flex items-center gap-1">
        {FILTERS.map(({ id, label }) => {
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
              {label}
              <span className={`num font-mono text-[12px] ${active ? "text-primary" : "text-ink-muted"}`}>{counts[id]}</span>
            </button>
          );
        })}
      </div>
      <Button size="sm" variant="ghost" className="ml-auto" onClick={onClearFinished} disabled={!canClear}>
        Clear finished
      </Button>
    </div>
  );
}

export function StatusBar({ speed, active, queued, settings }: { speed: number; active: number; queued: number; settings: Settings | null }) {
  return (
    <footer className="flex h-8 shrink-0 items-center gap-4 overflow-hidden border-t border-line bg-surface px-4 font-mono text-[12px] text-ink-muted">
      <span className="num shrink-0">
        <span className="text-ink-secondary">{active}</span> downloading
      </span>
      <span className="num shrink-0">
        <span className="text-ink-secondary">{queued}</span> queued
      </span>
      <span className="num shrink-0 text-ink-secondary">{speed > 0 ? formatSpeed(speed) : "idle"}</span>
      {settings && (
        <span className="ml-auto hidden truncate sm:inline">
          {settings.speed_limit > 0 ? `limit ${formatSpeed(settings.speed_limit)}` : "no speed limit"} · {settings.max_concurrent_downloads} at a time ·{" "}
          {settings.download_dir}
        </span>
      )}
    </footer>
  );
}

export function EmptyState({ filtered, onAdd }: { filtered: boolean; onAdd: () => void }) {
  if (filtered) {
    return <p className="px-4 py-10 text-center text-[14px] text-ink-muted">Nothing here with this filter.</p>;
  }
  return (
    <div className="anim-rise mx-auto flex max-w-[520px] flex-col items-start gap-4 px-6 py-16">
      <span className="eyebrow">01 — Queue</span>
      <h1 className="display-medium text-[24px] leading-tight">Nothing downloading yet</h1>
      <p className="text-[15px] text-ink-secondary">
        Paste a link anywhere in this window, or add one with the button. Downpipe splits large files across several connections only while that makes them
        faster, and picks up where it left off after a pause, a restart or a crash.
      </p>
      <div className="flex items-center gap-3">
        <Button variant="primary" onClick={onAdd}>
          <Plus size={16} strokeWidth={1.75} />
          Add a link
        </Button>
        <span className="font-mono text-[12px] text-ink-muted">or Ctrl+V</span>
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
  return (
    <div className="pointer-events-none fixed right-4 bottom-12 z-1200 flex w-[min(380px,calc(100vw-32px))] flex-col gap-2" aria-live="polite">
      {toasts.map((t) => (
        <div
          key={t.id}
          role="status"
          className={`anim-rise pointer-events-auto flex items-start gap-2 rounded-md border bg-canvas px-3 py-2.5 text-[13px] shadow-popover ${
            t.tone === "danger" ? "border-danger text-danger-fg" : "border-line text-ink"
          }`}
        >
          <span className="flex-1 whitespace-pre-line">{t.text}</span>
          <IconButton label="Dismiss" className="size-6" onClick={() => onDismiss(t.id)}>
            <X size={14} strokeWidth={1.75} />
          </IconButton>
        </div>
      ))}
    </div>
  );
}
