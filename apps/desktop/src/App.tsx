import { useCallback, useEffect, useEffectEvent, useMemo, useRef, useState, type DragEvent } from "react";

import { AddDialog } from "./components/AddDialog";
import { EmptyState, FilterBar, Header, StatusBar, Toasts, WhenDoneDialog, type Toast } from "./components/Chrome";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { ContextMenu } from "./components/ContextMenu";
import { DetailsDialog } from "./components/DetailsDialog";
import { DownloadList, rowSelector, type RowActions } from "./components/DownloadList";
import { Banner } from "./components/primitives";
import { RedownloadDialog } from "./components/RedownloadDialog";
import { RemoveDialog } from "./components/RemoveDialog";
import { SettingsDialog, type SettingsTab } from "./components/SettingsDialog";
import {
  api,
  errorMessage,
  inTauri,
  onClipboardLink,
  onManagerEvent,
  onResync,
  onWhenDone,
  type QueueMove,
  type Settings,
  type Task,
  type WhenDone,
  type WhenDoneView,
} from "./lib/api";
import { parseUrls } from "./lib/format";
import { buildMenu, linksText, type MenuAction } from "./lib/menu";
import { linksFromList } from "./lib/patterns";
import { EMPTY_SELECTION, clickSelect, contextSelect, edge, focusOf, prune, selectAll, step, type Selection } from "./lib/selection";
import { isBrowserShortcut, isTextField, listKey, type ListKey } from "./lib/shortcuts";
import { DEFAULT_SORT, formatSort, matchesSearch, nextSort, parseSort, sortTasks, type Sort, type SortKey } from "./lib/sort";
import { applyEvent, canPause, canResume, countByFilter, isRunning, matchesFilter, totals, type Filter } from "./lib/tasks";

type Theme = "light" | "dark";
const THEME_KEY = "downpipe:theme";
const SORT_KEY = "downpipe:sort";
const QUEUE_ORDER: Sort = { key: "order", dir: "asc" };

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

/** A copied link joins the links already in the Add dialog, once. */
function withLink(text: string | null, url: string): string {
  if (text === null || text.trim() === "") return url;
  return text.split(/\s+/).includes(url) ? text : `${text.trim()}\n${url}`;
}

/** Move the keyboard focus to a row once it is drawn; focusing scrolls it into view. */
function focusRow(id: string | null) {
  if (id === null) return;
  requestAnimationFrame(() => document.querySelector<HTMLElement>(rowSelector(id))?.focus());
}

let toastSeq = 0;

export function App() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [filter, setFilter] = useState<Filter>("all");
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState("");
  const [queue, setQueue] = useState("");
  const [sort, setSort] = useState<Sort>(storedSort);
  const [selection, setSelection] = useState<Selection>(EMPTY_SELECTION);
  const [menu, setMenu] = useState<{ x: number; y: number; keyboard: boolean } | null>(null);
  const [details, setDetails] = useState<{ id: string; relink: boolean } | null>(null);
  const [addText, setAddText] = useState<string | null>(null);
  // Video pages added here wait for their quality in the confirmation
  // dialog, which then speaks of them rather than of the browser.
  const [askedHere, setAskedHere] = useState<ReadonlySet<string>>(() => new Set());
  const [settingsTab, setSettingsTab] = useState<SettingsTab | null>(null);
  const [removingIds, setRemovingIds] = useState<string[]>([]);
  const [redownloadId, setRedownloadId] = useState<string | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [theme, setTheme] = useState<Theme>(currentTheme);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [whenDoneState, setWhenDoneState] = useState<{ view: WhenDoneView; at: number }>({
    view: { action: "nothing", seconds_left: null },
    at: 0,
  });
  const [now, setNow] = useState(() => Date.now());
  const [dropping, setDropping] = useState(false);
  const searchRef = useRef<HTMLInputElement>(null);
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
    void onClipboardLink((url) => !disposed && setAddText((text) => withLink(text, url))).then(keep);
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
      if (isTextField(e.target)) return;
      const text = e.clipboardData?.getData("text") ?? "";
      if (parseUrls(text).length > 0 || /\[[^\]]+-[^\]]+\]/.test(text)) {
        e.preventDefault();
        setAddText(text.trim());
      }
    };
    window.addEventListener("paste", onPaste);
    return () => window.removeEventListener("paste", onPaste);
  }, []);

  // An app, not a web page: no browser menu (Back, Reload, Inspect) and no
  // browser keys. Text fields keep their own menu for cut, copy and paste.
  useEffect(() => {
    const onContextMenu = (e: MouseEvent) => {
      if (!isTextField(e.target)) e.preventDefault();
    };
    // Capture: runs before any handler can stop the key from reaching the window.
    const onKey = (e: KeyboardEvent) => {
      if (isBrowserShortcut(e, import.meta.env.DEV)) e.preventDefault();
    };
    window.addEventListener("contextmenu", onContextMenu);
    window.addEventListener("keydown", onKey, true);
    return () => {
      window.removeEventListener("contextmenu", onContextMenu);
      window.removeEventListener("keydown", onKey, true);
    };
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

  const queues = useMemo(() => settings?.queues ?? [], [settings]);
  const queueNames = useMemo(() => new Map(queues.map((q) => [q.id, q.name])), [queues]);
  // A queue removed in Settings stops filtering.
  const queueFilter = queueNames.has(queue) ? queue : "";

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
            (queueFilter === "" || t.queue === queueFilter) &&
            matchesSearch(t, query),
        ),
        sort,
      ),
    [tasks, filter, category, queueFilter, query, sort],
  );
  const order = useMemo(() => visible.map((t) => t.id), [visible]);
  const taskIds = useMemo(() => new Set(tasks.map((t) => t.id)), [tasks]);
  // Removed downloads drop out of the selection without an extra render.
  const current = prune(selection, taskIds);
  const selectedSet = useMemo(() => new Set(current.ids), [current.ids]);
  // Only what is shown is acted on: a filter must not hide what Delete removes.
  const selectedTasks = useMemo(() => visible.filter((t) => selectedSet.has(t.id)), [visible, selectedSet]);
  const counts = useMemo(() => countByFilter(tasks), [tasks]);
  const sum = useMemo(() => totals(tasks), [tasks]);
  const narrowed = filter !== "all" || category !== "" || queueFilter !== "" || query.trim() !== "";

  const detailsTask = details ? tasks.find((t) => t.id === details.id) : undefined;
  const removing = tasks.filter((t) => removingIds.includes(t.id));
  const redownloading = tasks.find((t) => t.id === redownloadId);
  const counted = countdown !== null && whenDone.action !== "nothing";
  // One dialog at a time; the others wait their turn.
  const busyDialog = counted || settingsTab !== null || detailsTask !== undefined || removing.length > 0 || redownloading !== undefined;
  const showAdd = addText !== null && settings !== null && !busyDialog;
  const awaiting = useMemo(() => tasks.filter((t) => t.awaiting_confirmation).sort((a, b) => a.created_at - b.created_at), [tasks]);
  const asking = !busyDialog && !showAdd ? awaiting[0] : undefined;
  const menuEntries = useMemo(() => buildMenu(selectedTasks, queues), [selectedTasks, queues]);
  const showMenu = menu !== null && menuEntries.length > 0;
  const closeMenu = useCallback(() => setMenu(null), []);

  const copy = (text: string, what: string) =>
    navigator.clipboard.writeText(text).then(
      () => toast(`Copied ${what}.`, "neutral"),
      () => toast("Could not copy to the clipboard."),
    );

  const moveSelected = (to: QueueMove) => {
    if (selectedTasks.length === 0) return;
    const ids = selectedTasks.map((t) => t.id);
    // Moving only shows in queue order.
    if (sort.key !== QUEUE_ORDER.key || sort.dir !== QUEUE_ORDER.dir) changeSort(QUEUE_ORDER);
    run(() => api.move(ids, to));
    focusRow(focusOf(current, order));
  };

  const onMenuAction = (action: MenuAction) => {
    const one = selectedTasks.length === 1 ? selectedTasks[0] : undefined;
    const ids = selectedTasks.map((t) => t.id);
    if (action.startsWith("queue:")) {
      const target = action.slice("queue:".length);
      run(() => api.setQueue(ids, target));
      return;
    }
    switch (action) {
      case "open":
        if (one) run(() => api.open(one.id));
        break;
      case "reveal":
        if (one) run(() => api.reveal(one.id));
        break;
      case "resume":
        for (const t of selectedTasks.filter(canResume)) run(() => api.resume(t.id));
        break;
      case "pause":
        for (const t of selectedTasks.filter(canPause)) run(() => api.pause(t.id));
        break;
      case "redownload":
        if (one) setRedownloadId(one.id);
        break;
      case "relink":
        if (one) setDetails({ id: one.id, relink: true });
        break;
      case "move_top":
        moveSelected("top");
        break;
      case "move_up":
        moveSelected("up");
        break;
      case "move_down":
        moveSelected("down");
        break;
      case "move_bottom":
        moveSelected("bottom");
        break;
      case "copy_link":
        void copy(linksText(selectedTasks), selectedTasks.length === 1 ? "the link" : `${selectedTasks.length} links`);
        break;
      case "copy_path":
        if (one?.path) void copy(one.path, "the file path");
        break;
      case "properties":
        if (one) setDetails({ id: one.id, relink: false });
        break;
      case "remove":
        setRemovingIds(ids);
        break;
      case "new_queue":
        setSettingsTab("queues");
        break;
    }
  };

  const onListKey = (key: ListKey) => {
    const focus = focusOf(current, order);
    switch (key.kind) {
      case "properties":
        if (focus) setDetails({ id: focus, relink: false });
        break;
      case "remove":
        if (selectedTasks.length > 0) setRemovingIds(selectedTasks.map((t) => t.id));
        break;
      case "copy_link":
        if (selectedTasks.length > 0) onMenuAction("copy_link");
        break;
      case "select_all":
        setSelection(selectAll(order));
        break;
      case "clear":
        setSelection(EMPTY_SELECTION);
        break;
      case "menu": {
        const row = focus ? document.querySelector(rowSelector(focus)) : null;
        if (!row) break;
        const r = row.getBoundingClientRect();
        setMenu({ x: r.left + 32, y: r.top + r.height / 2, keyboard: true });
        break;
      }
      case "move":
        moveSelected(key.to);
        break;
      case "step":
      case "edge": {
        const next = key.kind === "step" ? step(current, order, key.delta, key.extend) : edge(current, order, key.end, key.extend);
        setSelection(next);
        focusRow(focusOf(next, order));
        break;
      }
    }
  };

  const onWindowKey = useEffectEvent((e: KeyboardEvent) => {
    // Dialogs and the menu own the keyboard while they are open.
    if (document.querySelector('[role="dialog"], [data-menu]')) return;
    const mod = e.ctrlKey || e.metaKey;
    if (mod && !e.shiftKey && !e.altKey) {
      const k = e.key.toLowerCase();
      if (k === "f") {
        e.preventDefault();
        searchRef.current?.focus();
        searchRef.current?.select();
        return;
      }
      if (k === "n") {
        e.preventDefault();
        setAddText((text) => text ?? "");
        return;
      }
    }
    // Only on a row or nowhere in particular: a focused button keeps Enter.
    const target = e.target instanceof Element ? e.target : null;
    if (target && target !== document.body && !target.matches("[data-row-id]")) return;
    const key = listKey(e);
    if (!key) return;
    e.preventDefault();
    onListKey(key);
  });
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => onWindowKey(e);
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const rowActions: RowActions = {
    onSelect: (id, mods) => setSelection(clickSelect(current, id, order, mods)),
    onProperties: (id) => {
      setSelection({ ids: [id], anchor: id });
      setDetails({ id, relink: false });
    },
    onMenu: (id, x, y, keyboard) => {
      setSelection(contextSelect(current, id));
      setMenu({ x, y, keyboard });
    },
    onPause: (id) => run(() => api.pause(id)),
    onResume: (id) => run(() => api.resume(id)),
    onReveal: (id) => run(() => api.reveal(id)),
    onDragOut: (id) => run(() => api.startDrag(id)),
  };

  const setWhenDoneAction = (action: WhenDone) =>
    run(async () => {
      setWhenDone(await api.setWhenDone(action));
      if (action !== "nothing" && sum.active + sum.queued === 0) {
        toast("Nothing is downloading yet. It applies once the next downloads finish.", "neutral");
      }
    });

  const chooseQueue = (id: string) => {
    setQueue(id);
    // A queue reads best in the order it runs.
    if (id !== "") changeSort(QUEUE_ORDER);
  };

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
        onSettings={() => setSettingsTab("downloads")}
        hasRunning={tasks.some((t) => isRunning(t.state) || t.state === "queued")}
        hasPaused={tasks.some((t) => (t.state === "paused" && !t.awaiting_confirmation) || t.state === "failed")}
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
        searchRef={searchRef}
        queues={queues}
        queue={queueFilter}
        onQueue={chooseQueue}
        onStartQueue={() => run(() => api.startQueue(queueFilter))}
        onStopQueue={() => run(() => api.stopQueue(queueFilter))}
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
        <div
          className="min-h-0 flex-1 overflow-y-auto"
          // A click below the rows clears the selection, as in a file manager.
          onMouseDown={(e) => {
            if (e.target === e.currentTarget && e.button === 0) setSelection(EMPTY_SELECTION);
          }}
        >
          {visible.length > 0 ? (
            <DownloadList
              tasks={visible}
              selected={selectedSet}
              actions={rowActions}
              sort={sort}
              onSort={(key: SortKey) => changeSort(nextSort(sort, key))}
              queueNames={queueNames}
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
        selected={selectedTasks.length}
        settings={settings}
        whenDone={whenDone.action}
        onWhenDone={setWhenDoneAction}
      />

      {dropping && (
        <div className="pointer-events-none absolute inset-2 z-50 flex items-center justify-center rounded-lg border-2 border-dashed border-primary bg-primary-soft">
          <p className="display-medium text-[18px] text-ink">Drop links or a .txt list to add them</p>
        </div>
      )}

      {showMenu && (
        <ContextMenu
          x={menu.x}
          y={menu.y}
          entries={menuEntries}
          label={selectedTasks.length === 1 ? "Download actions" : `Actions for ${selectedTasks.length} downloads`}
          focusFirst={menu.keyboard}
          onAction={onMenuAction}
          onClose={closeMenu}
        />
      )}

      {detailsTask && (
        <DetailsDialog
          key={detailsTask.id}
          task={detailsTask}
          queueName={detailsTask.queue === "main" ? null : (queueNames.get(detailsTask.queue) ?? null)}
          startRelink={details?.relink ?? false}
          actions={{
            onClose: () => setDetails(null),
            onPause: (id) => run(() => api.pause(id)),
            onResume: (id) => run(() => api.resume(id)),
            onCancel: (id) => run(() => api.cancel(id)),
            onReveal: (id) => run(() => api.reveal(id)),
            onOpen: (id) => run(() => api.open(id)),
            // One dialog at a time: Escape would otherwise close both.
            onRemove: (id) => {
              setDetails(null);
              setRemovingIds([id]);
            },
            onChangeUrl: async (id, url) => {
              await api.changeUrl(id, url);
            },
            onRedownload: (id) => {
              setDetails(null);
              setRedownloadId(id);
            },
          }}
        />
      )}
      {showAdd && (
        <AddDialog
          text={addText}
          onTextChange={setAddText}
          settings={settings}
          onClose={() => setAddText(null)}
          onAdded={(added) => {
            setFilter("all");
            setQuery("");
            setCategory("");
            setQueue("");
            const asked = added.filter((t) => t.awaiting_confirmation).map((t) => t.id);
            if (asked.length > 0) setAskedHere((ids) => new Set([...ids, ...asked]));
            const first = added[0];
            if (first) {
              setSelection({ ids: added.map((t) => t.id), anchor: first.id });
              // The confirmation dialog that follows keeps the focus.
              if (asked.length === 0) focusRow(first.id);
            }
          }}
          onDuplicate={(id) => {
            setAddText(null);
            setFilter("all");
            setQueue("");
            setSelection({ ids: [id], anchor: id });
            focusRow(id);
            toast("That link is already in the list.", "neutral");
          }}
        />
      )}
      {asking && settings && (
        <ConfirmDialog
          key={asking.id}
          task={asking}
          fromApp={askedHere.has(asking.id)}
          waiting={awaiting.length - 1}
          queues={queues}
          onConfirm={async (options) => {
            await api.confirm(asking.id, options);
          }}
          onDiscard={() => run(() => api.remove(asking.id, true))}
          onStopAsking={() =>
            run(async () => {
              setSettings(await api.updateSettings({ ...settings, confirm_browser_downloads: false }));
              toast("Downloads from the browser now start at once. Settings, App turns the question back on.", "neutral");
            })
          }
          onOpenSettings={() => setSettingsTab("media")}
        />
      )}
      {settingsTab !== null && settings && (
        <SettingsDialog settings={settings} initialTab={settingsTab} onClose={() => setSettingsTab(null)} onSaved={setSettings} />
      )}
      {removing.length > 0 && (
        <RemoveDialog
          tasks={removing}
          onClose={() => setRemovingIds([])}
          onConfirm={(deleteFiles) => {
            const ids = removing.map((t) => t.id);
            setRemovingIds([]);
            for (const id of ids) run(() => api.remove(id, deleteFiles));
          }}
        />
      )}
      {redownloading && (
        <RedownloadDialog
          task={redownloading}
          onClose={() => setRedownloadId(null)}
          onConfirm={(replace) => {
            const id = redownloading.id;
            setRedownloadId(null);
            run(() => api.redownload(id, replace));
          }}
        />
      )}
      {counted && (
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
