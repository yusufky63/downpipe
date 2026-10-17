import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { Toasts, type Toast } from "./components/Chrome";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { api, errorMessage, inTauri, onConfirmAgain, onManagerEvent, onResync, type Settings, type Task } from "./lib/api";
import { applyEvent, awaitingIn } from "./lib/tasks";
import { useAppWindow, useThemeFromOtherWindows } from "./lib/window";

let toastSeq = 0;

/**
 * The confirmation window (confirm.html): downloads from the browser wait
 * here for their answer one after another, without the main window. The
 * app shows the window once a question is drawn and hides it once none
 * waits; the core's events keep it in step with the main window's list.
 */
export function ConfirmWindow() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [settings, setSettings] = useState<Settings | null>(null);
  // Bumped when the question comes back from Settings: read it again.
  const [round, setRound] = useState(0);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const native = inTauri();
  useAppWindow();
  useThemeFromOtherWindows();

  const toast = useCallback((text: string) => {
    const id = ++toastSeq;
    setToasts((list) => [...list.slice(-2), { id, text, tone: "danger" }]);
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
        .catch((err: unknown) => toast(errorMessage(err)));

    // Subscribe before the first load so no update falls between the two.
    void onManagerEvent((event) => {
      if (event.type === "settings") setSettings(event.settings);
      else setTasks((list) => applyEvent(list, event));
    }).then(keep);
    void onResync(reload).then(keep);
    void onConfirmAgain(() => !disposed && setRound((n) => n + 1)).then(keep);
    void reload();
    api
      .settings()
      .then((s) => !disposed && setSettings(s))
      .catch((err: unknown) => toast(errorMessage(err)));
    return () => {
      disposed = true;
      unlisten.forEach((fn) => fn());
    };
  }, [native, toast]);

  const waiting = useMemo(() => awaitingIn(tasks, "window"), [tasks]);
  const asking = settings ? waiting[0] : undefined;
  const question = asking ? `${asking.id}:${round}` : null;
  const shown = useRef<string | null>(null);

  // The window takes the height its question needs and shows once it has
  // it, so it never flashes empty or jumps; each next question brings it
  // forward again.
  const fit = useCallback(
    (height: number) => {
      void api
        .fitWindow(height)
        .catch(() => undefined)
        .then(() => {
          if (question === null || shown.current === question) return;
          shown.current = question;
          return api.confirmWindowDrawn();
        })
        .catch(() => undefined);
    },
    [question],
  );

  return (
    <>
      {asking && settings && (
        <ConfirmDialog
          key={question}
          task={asking}
          waiting={waiting.length - 1}
          queues={settings.queues}
          inWindow
          onFit={fit}
          onConfirm={async (options) => {
            await api.confirm(asking.id, options);
            // IDM's download dialog: the question turns into the download's progress.
            if (options.start) api.watchDownload(asking.id).catch(() => undefined);
          }}
          onDiscard={() => run(() => api.remove(asking.id, true))}
          onStopAsking={() =>
            run(async () => {
              setSettings(await api.updateSettings({ ...settings, confirm_browser_downloads: false }));
            })
          }
          onOpenSettings={() => run(api.openMediaSettings)}
        />
      )}
      <Toasts toasts={toasts} onDismiss={(id) => setToasts((list) => list.filter((t) => t.id !== id))} />
    </>
  );
}
