import { useCallback, useEffect, useMemo, useState } from "react";

import { AddDialog } from "./components/AddDialog";
import { EmptyState, FilterBar, Header, StatusBar, Toasts, type Toast } from "./components/Chrome";
import { DetailPanel } from "./components/DetailPanel";
import { DownloadList } from "./components/DownloadList";
import { Banner } from "./components/primitives";
import { RemoveDialog } from "./components/RemoveDialog";
import { SettingsDialog } from "./components/SettingsDialog";
import { api, errorMessage, inTauri, onManagerEvent, onResync, type Settings, type Task } from "./lib/api";
import { parseUrls } from "./lib/format";
import { applyEvent, canResume, countByFilter, isRunning, matchesFilter, sortForDisplay, totals, type Filter } from "./lib/tasks";

type Theme = "light" | "dark";
const THEME_KEY = "downpipe:theme";

function currentTheme(): Theme {
  const attr = document.documentElement.getAttribute("data-theme");
  if (attr === "light" || attr === "dark") return attr;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

let toastSeq = 0;

export function App() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [filter, setFilter] = useState<Filter>("all");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [addText, setAddText] = useState<string | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [removingId, setRemovingId] = useState<string | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [theme, setTheme] = useState<Theme>(currentTheme);
  const [loadError, setLoadError] = useState<string | null>(null);
  const native = inTauri();

  const toast = useCallback((text: string, tone: Toast["tone"] = "danger") => {
    const id = ++toastSeq;
    setToasts((list) => [...list.slice(-3), { id, text, tone }]);
    window.setTimeout(() => setToasts((list) => list.filter((t) => t.id !== id)), 6000);
  }, []);

  const run = useCallback(
    (action: () => Promise<unknown>) => {
      action().catch((err: unknown) => toast(errorMessage(err)));
    },
    [toast],
  );

  useEffect(() => {
    if (!native) return;
    let disposed = false;
    const unlisten: (() => void)[] = [];
    const reload = () =>
      api
        .list()
        .then((list) => !disposed && setTasks(list))
        .catch((err: unknown) => setLoadError(errorMessage(err)));

    // Subscribe before the first load so no update falls between the two.
    void onManagerEvent((event) => {
      if (event.type === "settings") setSettings(event.settings);
      else setTasks((list) => applyEvent(list, event));
    }).then((fn) => (disposed ? fn() : unlisten.push(fn)));
    void onResync(reload).then((fn) => (disposed ? fn() : unlisten.push(fn)));
    void reload();
    api
      .settings()
      .then((s) => !disposed && setSettings(s))
      .catch((err: unknown) => setLoadError(errorMessage(err)));
    return () => {
      disposed = true;
      unlisten.forEach((fn) => fn());
    };
  }, [native]);

  // Ctrl+V anywhere outside a text field opens the add dialog with the links.
  useEffect(() => {
    const onPaste = (e: ClipboardEvent) => {
      const target = e.target as HTMLElement | null;
      if (target?.closest("input, textarea, select, [contenteditable]")) return;
      const text = e.clipboardData?.getData("text") ?? "";
      if (parseUrls(text).length > 0) {
        e.preventDefault();
        setAddText(text.trim());
      }
    };
    window.addEventListener("paste", onPaste);
    return () => window.removeEventListener("paste", onPaste);
  }, []);

  // Follow the system theme until the user picks one explicitly.
  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => {
      if (!document.documentElement.hasAttribute("data-theme")) setTheme(media.matches ? "dark" : "light");
    };
    media.addEventListener("change", onChange);
    return () => media.removeEventListener("change", onChange);
  }, []);

  const toggleTheme = () => {
    const next: Theme = theme === "dark" ? "light" : "dark";
    document.documentElement.setAttribute("data-theme", next);
    try {
      localStorage.setItem(THEME_KEY, next);
    } catch {
      // Storage may be unavailable; the choice still applies until restart.
    }
    setTheme(next);
  };

  const sorted = useMemo(() => sortForDisplay(tasks), [tasks]);
  const visible = useMemo(() => sorted.filter((t) => matchesFilter(t, filter)), [sorted, filter]);
  const counts = useMemo(() => countByFilter(tasks), [tasks]);
  const sum = useMemo(() => totals(tasks), [tasks]);
  const selected = tasks.find((t) => t.id === selectedId) ?? null;
  const removing = tasks.find((t) => t.id === removingId) ?? null;

  const rowActions = {
    onSelect: (id: string) => setSelectedId((cur) => (cur === id ? null : id)),
    onPause: (id: string) => run(() => api.pause(id)),
    onResume: (id: string) => run(() => api.resume(id)),
    onReveal: (id: string) => run(() => api.reveal(id)),
    onRemove: (id: string) => setRemovingId(id),
    onOpen: (id: string) => run(() => api.open(id)),
  };

  return (
    <div className="flex h-full flex-col">
      <Header
        theme={theme}
        onToggleTheme={toggleTheme}
        onAdd={() => setAddText("")}
        onPauseAll={() => run(api.pauseAll)}
        onResumeAll={() => run(api.resumeAll)}
        onSettings={() => setShowSettings(true)}
        hasRunning={tasks.some((t) => isRunning(t.state) || t.state === "queued")}
        hasPaused={tasks.some((t) => t.state === "paused" || t.state === "failed")}
      />
      <FilterBar
        filter={filter}
        counts={counts}
        onFilter={setFilter}
        onClearFinished={() => run(api.clearFinished)}
        canClear={tasks.some((t) => t.state === "completed" || t.state === "canceled")}
      />

      <main className="flex min-h-0 flex-1 flex-col">
        {!native && (
          <div className="p-4">
            <Banner tone="warning">This window is the Downpipe interface without its engine. Start the desktop app with pnpm desktop to download.</Banner>
          </div>
        )}
        {loadError && (
          <div className="p-4">
            <Banner tone="danger">Could not load downloads: {loadError}</Banner>
          </div>
        )}
        <div className="min-h-0 flex-1 overflow-y-auto">
          {visible.length > 0 ? (
            <DownloadList tasks={visible} selectedId={selectedId} actions={rowActions} />
          ) : (
            <EmptyState filtered={tasks.length > 0} onAdd={() => setAddText("")} />
          )}
        </div>
        {selected && (
          <div className="max-h-[50%] shrink-0 overflow-y-auto border-t border-line bg-surface pt-4">
            <DetailPanel
              task={selected}
              actions={{
                ...rowActions,
                onClose: () => setSelectedId(null),
                onCancel: (id) => run(() => api.cancel(id)),
                onResume: (id) => (canResume(selected) ? run(() => api.resume(id)) : undefined),
              }}
            />
          </div>
        )}
      </main>

      <StatusBar speed={sum.speed} active={sum.active} queued={sum.queued} settings={settings} />

      {addText !== null && settings && (
        <AddDialog
          initialText={addText}
          settings={settings}
          onClose={() => setAddText(null)}
          onAdded={(added) => {
            setFilter("all");
            if (added.length === 1 && added[0]) setSelectedId(added[0].id);
          }}
          onDuplicate={(id) => {
            setAddText(null);
            setFilter("all");
            setSelectedId(id);
            toast("That link is already in the list.", "neutral");
          }}
        />
      )}
      {showSettings && settings && <SettingsDialog settings={settings} onClose={() => setShowSettings(false)} onSaved={setSettings} />}
      {removing && (
        <RemoveDialog
          task={removing}
          onClose={() => setRemovingId(null)}
          onConfirm={(deleteFiles) => {
            const id = removing.id;
            setRemovingId(null);
            if (selectedId === id) setSelectedId(null);
            run(() => api.remove(id, deleteFiles));
          }}
        />
      )}
      <Toasts toasts={toasts} onDismiss={(id) => setToasts((list) => list.filter((t) => t.id !== id))} />
    </div>
  );
}
