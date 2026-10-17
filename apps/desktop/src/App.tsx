import { useCallback, useEffect, useMemo, useState, type DragEvent } from "react";

import { AddDialog } from "./components/AddDialog";
import { EmptyState, FilterBar, Header, StatusBar, Toasts, WhenDoneDialog, type Toast } from "./components/Chrome";
import { DetailsDialog } from "./components/DetailsDialog";
import { DownloadList } from "./components/DownloadList";
import { Banner } from "./components/primitives";
import { RemoveDialog } from "./components/RemoveDialog";
import { SettingsDialog } from "./components/SettingsDialog";
import {
  api,
  errorMessage,
  inTauri,
  onManagerEvent,
  onResync,
  onWhenDone,
  type Settings,
  type Task,
  type WhenDone,
  type WhenDoneView,
} from "./lib/api";
import { parseUrls } from "./lib/format";
import { linksFromList } from "./lib/patterns";
import { DEFAULT_SORT, formatSort, matchesSearch, nextSort, parseSort, sortTasks, type Sort, type SortKey } from "./lib/sort";
import { applyEvent, canResume, countByFilter, isRunning, matchesFilter, totals, type Filter } from "./lib/tasks";

type Theme = "light" | "dark";
const THEME_KEY = "downpipe:theme";
const SORT_KEY = "downpipe:sort";

function currentTheme(): Theme {
  const attr = document.documentElement.getAttribute("data-theme");
  if (attr === "light" || attr === "dark") return attr;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function storedSort(): Sort {
  try {
    return parseSort(localStorage.getItem(SORT_KEY));
  } catch {
    return DEFAULT_SORT;
  }
}

/** Links and .txt lists dragged in from a browser or the file manager. */
function draggedLinks(e: DragEvent): boolean {
  const types = Array.from(e.dataTransfer.types);
  return types.includes("text/uri-list") || types.includes("text/plain") || types.includes("Files");
}

let toastSeq = 0;

export function App() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [filter, setFilter] = useState<Filter>("all");
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState("");
  const [sort, setSort] = useState<Sort>(storedSort);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [addText, setAddText] = useState<string | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [removingId, setRemovingId] = useState<string | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [theme, setTheme] = useState<Theme>(currentTheme);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [whenDoneState, setWhenDoneState] = useState<{ view: WhenDoneView; at: number }>({
    view: { action: "nothing", seconds_left: null },
    at: 0,
  });
  const [now, setNow] = useState(() => Date.now());
  const [dropping, setDropping] = useState(false);
  const native = inTauri();

  const whenDone = whenDoneState.view;
  const setWhenDone = useCallback((view: WhenDoneView) => setWhenDoneState({ view, at: Date.now() }), []);
  // The app sends the seconds left once; count down locally from there.
  const countdown =
    whenDone.seconds_left === null ? null : Math.max(0, whenDone.seconds_left - Math.floor(Math.max(0, now - whenDoneState.at) / 1000));

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
    const keep = (fn: () => void) => (disposed ? fn() : unlisten.push(fn));
    const reload = () =>
      api
        .list()
        .then((list) => !disposed && setTasks(list))
        .catch((err: unknown) => setLoadError(errorMessage(err)));

    // Subscribe before the first load so no update falls between the two.
    void onManagerEvent((event) => {
      if (event.type === "settings") setSettings(event.settings);
      else setTasks((list) => applyEvent(list, event));
    }).then(keep);
    void onResync(reload).then(keep);
    void onWhenDone((view) => !disposed && setWhenDone(view)).then(keep);
    void reload();
    api
      .settings()
      .then((s) => !disposed && setSettings(s))
      .catch((err: unknown) => setLoadError(errorMessage(err)));
    api
      .whenDone()
      .then((v) => !disposed && setWhenDone(v))
      .catch(() => undefined);
    return () => {
      disposed = true;
      unlisten.forEach((fn) => fn());
    };
  }, [native, setWhenDone]);

  const counting = whenDone.seconds_left !== null;
  useEffect(() => {
    if (!counting) return;
    const timer = window.setInterval(() => setNow(Date.now()), 250);
    return () => window.clearInterval(timer);
  }, [counting]);

  // Ctrl+V anywhere outside a text field opens the add dialog with the links.
  useEffect(() => {
    const onPaste = (e: ClipboardEvent) => {
      const target = e.target as HTMLElement | null;
      if (target?.closest("input, textarea, select, [contenteditable]")) return;
      const text = e.clipboardData?.getData("text") ?? "";
      if (parseUrls(text).length > 0 || /\[[^\]]+-[^\]]+\]/.test(text)) {
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

  const changeSort = (next: Sort) => {
    setSort(next);
    try {
      localStorage.setItem(SORT_KEY, formatSort(next));
    } catch {
      // Not remembered; still applies now.
    }
  };

  const onDrop = async (e: DragEvent<HTMLDivElement>) => {
    setDropping(false);
    if (!draggedLinks(e)) return;
    e.preventDefault();
    const parts: string[] = [];
    for (const file of Array.from(e.dataTransfer.files)) {
      if (file.type === "text/plain" || file.name.toLowerCase().endsWith(".txt")) {
        if (file.size <= 2 * 1024 * 1024) parts.push(linksFromList(await file.text()));
      }
    }
    const text = e.dataTransfer.getData("text/uri-list") || e.dataTransfer.getData("text/plain");
    if (text) parts.push(linksFromList(text));
    const joined = parts.filter(Boolean).join("\n");
    if (parseUrls(joined).length > 0) setAddText(joined);
    else toast("Drop links, or a .txt file with one link per line.", "neutral");
  };

  const categoryNames = useMemo(() => {
    const names = new Set<string>(settings?.use_categories ? settings.categories.map((c) => c.name) : []);
    for (const t of tasks) if (t.category) names.add(t.category);
    return [...names];
  }, [settings, tasks]);

  const visible = useMemo(
    () =>
      sortTasks(
        tasks.filter(
          (t) =>
            matchesFilter(t, filter) &&
            (category === "" || (category === "-" ? !t.category : t.category === category)) &&
            matchesSearch(t, query),
        ),
        sort,
      ),
    [tasks, filter, category, query, sort],
  );
  const counts = useMemo(() => countByFilter(tasks), [tasks]);
  const sum = useMemo(() => totals(tasks), [tasks]);
  const selected = tasks.find((t) => t.id === selectedId) ?? null;
  const removing = tasks.find((t) => t.id === removingId) ?? null;
  const narrowed = filter !== "all" || category !== "" || query.trim() !== "";

  const rowActions = {
    onSelect: (id: string) => setSelectedId(id),
    onPause: (id: string) => run(() => api.pause(id)),
    onResume: (id: string) => run(() => api.resume(id)),
    onReveal: (id: string) => run(() => api.reveal(id)),
    onRemove: (id: string) => setRemovingId(id),
    onOpen: (id: string) => run(() => api.open(id)),
    onDragOut: (id: string) => run(() => api.startDrag(id)),
  };

  const setWhenDoneAction = (action: WhenDone) =>
    run(async () => {
      setWhenDone(await api.setWhenDone(action));
      if (action !== "nothing" && sum.active + sum.queued === 0) {
        toast("Nothing is downloading yet. It applies once the next downloads finish.", "neutral");
      }
    });

  return (
    <div
      className="relative flex h-full flex-col"
      onDragOver={(e) => {
        if (!draggedLinks(e)) return;
        e.preventDefault();
        e.dataTransfer.dropEffect = "copy";
        setDropping(true);
      }}
      onDragLeave={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget as Node | null)) setDropping(false);
      }}
      onDrop={(e) => void onDrop(e)}
    >
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
        query={query}
        onQuery={setQuery}
        category={category}
        categories={categoryNames}
        onCategory={setCategory}
        sortIsDefault={sort.key === DEFAULT_SORT.key && sort.dir === DEFAULT_SORT.dir}
        onResetSort={() => changeSort(DEFAULT_SORT)}
        onExport={() =>
          run(async () => {
            const count = await api.exportLinks();
            if (count !== null) toast(`Saved ${count} ${count === 1 ? "link" : "links"}.`, "neutral");
          })
        }
        canExport={tasks.length > 0}
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
            <DownloadList
              tasks={visible}
              selectedId={selectedId}
              actions={rowActions}
              sort={sort}
              onSort={(key: SortKey) => changeSort(nextSort(sort, key))}
            />
          ) : (
            <EmptyState filtered={tasks.length > 0 && narrowed} onAdd={() => setAddText("")} />
          )}
        </div>
      </main>

      <StatusBar
        speed={sum.speed}
        active={sum.active}
        queued={sum.queued}
        settings={settings}
        whenDone={whenDone.action}
        onWhenDone={setWhenDoneAction}
      />

      {dropping && (
        <div className="pointer-events-none absolute inset-2 z-50 flex items-center justify-center rounded-lg border-2 border-dashed border-primary bg-primary-soft">
          <p className="display-medium text-[18px] text-ink">Drop links or a .txt list to add them</p>
        </div>
      )}

      {selected && (
        <DetailsDialog
          task={selected}
          actions={{
            ...rowActions,
            onClose: () => setSelectedId(null),
            onCancel: (id) => run(() => api.cancel(id)),
            onResume: (id) => (canResume(selected) ? run(() => api.resume(id)) : undefined),
            // One dialog at a time: Escape would otherwise close both.
            onRemove: (id) => {
              setSelectedId(null);
              setRemovingId(id);
            },
            onChangeUrl: async (id, url) => {
              await api.changeUrl(id, url);
            },
          }}
        />
      )}
      {addText !== null && settings && (
        <AddDialog
          initialText={addText}
          settings={settings}
          onClose={() => setAddText(null)}
          onAdded={(added) => {
            setFilter("all");
            setQuery("");
            setCategory("");
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
      {countdown !== null && whenDone.action !== "nothing" && (
        <WhenDoneDialog
          action={whenDone.action}
          secondsLeft={countdown}
          onCancel={() => setWhenDoneAction("nothing")}
          onNow={() => run(api.whenDoneNow)}
        />
      )}
      <Toasts toasts={toasts} onDismiss={(id) => setToasts((list) => list.filter((t) => t.id !== id))} />
    </div>
  );
}
