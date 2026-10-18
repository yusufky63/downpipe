import { useCallback, useEffect, useEffectEvent, useMemo, useRef, useState } from "react";

import { Toasts, type Toast } from "./components/Chrome";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { DuplicateDialog } from "./components/DuplicateDialog";
import { LocaleProvider, useLocale } from "./i18n";
import { api, errorMessage, inTauri, onConfirmAgain, onManagerEvent, onResync, type Settings, type Task } from "./lib/api";
import { rememberedAnswer, type DuplicateChoice } from "./lib/duplicates";
import { applyEvent, awaitingIn, canResume } from "./lib/tasks";
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
  // Duplicates the user chose to add as a copy: their usual question follows.
  const [copies, setCopies] = useState<ReadonlySet<string>>(() => new Set());
  const native = inTauri();
  const locale = useLocale(settings?.language ?? "");
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

  // The browser sent a link already in the list: ask about that first, as IDM does.
  const existing = asking?.duplicate_of ? tasks.find((t) => t.id === asking.duplicate_of) : undefined;
  // Answered before with "Don't ask again". A copy goes on to the usual
  // question; the other answers need no window at all.
  const remembered = settings ? rememberedAnswer(settings) : null;
  const askDuplicate = asking !== undefined && existing !== undefined && !copies.has(asking.id) && remembered !== "copy";

  /** Drops the held copy and turns to the download already in the list. */
  const turnToExisting = (held: Task, old: Task, choice: Exclude<DuplicateChoice, "copy">) =>
    run(async () => {
      await api.remove(held.id, true);
      if (choice === "overwrite" && old.state === "completed") await api.redownload(old.id, true);
      else if (canResume(old)) await api.resume(old.id);
      // Its window: progress while it runs, Open and Open folder once done.
      await api.watchDownload(old.id);
    });

  const answered = useRef<string | null>(null);
  const answerRemembered = useEffectEvent(() => {
    if (!askDuplicate || !asking || !existing || remembered === null) return;
    if (answered.current === asking.id) return;
    answered.current = asking.id;
    turnToExisting(asking, existing, remembered);
  });
  useEffect(() => answerRemembered(), [asking?.id, askDuplicate, remembered]);

  return (
    <LocaleProvider locale={locale}>
      {askDuplicate ? (
        remembered === null && (
          <DuplicateDialog
            key={`duplicate-${question}`}
            existing={existing}
            inWindow
            onFit={fit}
            onCancel={() => run(() => api.remove(asking.id, true))}
            onChoose={(choice, remember) => {
              if (remember && settings) run(async () => setSettings(await api.updateSettings({ ...settings, duplicate_answer: choice })));
              if (choice === "copy") setCopies((ids) => new Set(ids).add(asking.id));
              else turnToExisting(asking, existing, choice);
            }}
          />
        )
      ) : asking && settings && (
        <ConfirmDialog
          key={question}
          task={asking}
          waiting={waiting.length - 1}
          queues={settings.queues}
          settings={settings}
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
    </LocaleProvider>
  );
}
