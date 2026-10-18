import { ExternalLink, FolderOpen, Pause, Play } from "lucide-react";
import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";

import { Button, Dialog } from "./components/primitives";
import { SegmentBar } from "./components/SegmentBar";
import { LocaleProvider, useI18n, useLocale } from "./i18n";
import { api, errorMessage, inTauri, onManagerEvent, onResync, type Settings, type Task } from "./lib/api";
import { progressEyebrow, progressPhase, resumeText, sizeText, speedText, statusText, windowTitle } from "./lib/progress";
import { canPause, canResume, displayName } from "./lib/tasks";
import { useAppWindow, useThemeFromOtherWindows } from "./lib/window";

/**
 * A download's progress window (confirm.html with a `progress-<id>` label):
 * IDM's download dialog. It follows one download through the core's events
 * until it is hidden; every button acts on the download by id, and closing
 * the window never stops the download.
 */
export function ProgressWindow({ id }: { id: string }) {
  // Undefined until the list is read; null once the download left it.
  const [task, setTask] = useState<Task | null | undefined>(undefined);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // Only for the language; the window has no other settings of its own.
  const [settings, setSettings] = useState<Settings | null>(null);
  const shown = useRef(false);
  const locale = useLocale(settings?.language ?? "");
  const { t } = useI18n();
  useAppWindow();
  useThemeFromOtherWindows();

  useEffect(() => {
    if (!inTauri()) return;
    let disposed = false;
    const unlisten: (() => void)[] = [];
    const keep = (fn: () => void) => (disposed ? fn() : unlisten.push(fn));
    const reload = () =>
      api
        .list()
        .then((list) => !disposed && setTask(list.find((t) => t.id === id) ?? null))
        .catch((err: unknown) => setError(errorMessage(err)));
    // Subscribe before the first load so no update falls between the two.
    void onManagerEvent((event) => {
      if (event.type === "updated" && event.task.id === id) setTask(event.task);
      else if (event.type === "removed" && event.id === id) setTask(null);
      else if (event.type === "settings") setSettings(event.settings);
    }).then(keep);
    void onResync(reload).then(keep);
    void reload();
    api
      .settings()
      .then((s) => !disposed && setSettings(s))
      // Without them the window follows the system's language, which is no reason to show an error.
      .catch(() => undefined);
    return () => {
      disposed = true;
      unlisten.forEach((fn) => fn());
    };
  }, [id]);

  // The language is part of the title, so it is made again when that changes.
  const title = task === undefined ? null : windowTitle(task);
  useEffect(() => {
    if (title !== null) api.setProgressTitle(title).catch(() => undefined);
  }, [title]);

  // Shown once it is drawn at its height, as the confirmation window is.
  const fit = useCallback((height: number) => {
    void api
      .fitWindow(height)
      .catch(() => undefined)
      .then(() => {
        if (shown.current) return;
        shown.current = true;
        return api.progressWindowDrawn();
      })
      .catch(() => undefined);
  }, []);

  if (task === undefined) return null;

  const hide = () => {
    api.closeProgressWindow().catch(() => undefined);
  };
  const act = (action: () => Promise<unknown>, thenClose = false) => {
    setBusy(true);
    setError(null);
    action()
      .then(() => {
        if (thenClose) hide();
      })
      .catch((err: unknown) => setError(errorMessage(err)))
      .finally(() => setBusy(false));
  };

  const phase = progressPhase(task);
  const tone = phase === "done" ? "positive" : phase === "failed" ? "danger" : phase === "active" ? "primary" : "neutral";

  return (
    <LocaleProvider locale={locale}>
      <Dialog
        eyebrow={progressEyebrow(task)}
        title={task ? displayName(task) : t("confirm.progress.download")}
        frame="window"
        compact
        onFit={fit}
        focusFirstField={false}
        // Escape and the window's own close button only hide it.
        onClose={hide}
        footer={
          <>
            {task === null || phase === "done" || phase === "failed" || phase === "canceled" ? (
              <Button variant="ghost" size="sm" className="mr-auto" onClick={hide}>
                {t("common.close")}
              </Button>
            ) : (
              <Button variant="ghost" size="sm" className="mr-auto" onClick={hide} title={t("confirm.progress.hideTitle")}>
                {t("confirm.progress.hide")}
              </Button>
            )}
            {task && phase === "done" && (
              <>
                <Button size="sm" disabled={busy} onClick={() => act(() => api.reveal(task.id), true)}>
                  <FolderOpen size={14} strokeWidth={1.75} />
                  {t("confirm.progress.openFolder")}
                </Button>
                <Button variant="primary" size="sm" disabled={busy} onClick={() => act(() => api.open(task.id), true)}>
                  <ExternalLink size={14} strokeWidth={1.75} />
                  {t("common.open")}
                </Button>
              </>
            )}
            {task && (phase === "active" || phase === "paused") && (
              <Button
                variant="danger"
                size="sm"
                disabled={busy}
                title={t("confirm.progress.cancelTitle")}
                onClick={() => act(() => api.cancel(task.id), true)}
              >
                {t("common.cancel")}
              </Button>
            )}
            {task && canPause(task) && (
              <Button size="sm" disabled={busy} onClick={() => act(() => api.pause(task.id))}>
                <Pause size={14} strokeWidth={1.75} />
                {task.swarm?.seeding ? t("torrent.stopSeeding") : t("common.pause")}
              </Button>
            )}
            {task && canResume(task) && (
              <Button variant="primary" size="sm" disabled={busy} onClick={() => act(() => api.resume(task.id))}>
                <Play size={14} strokeWidth={1.75} />
                {phase === "failed" ? t("common.retry") : phase === "canceled" ? t("confirm.progress.startAgain") : t("common.resume")}
              </Button>
            )}
          </>
        }
      >
        {task === null ? (
          <p className="text-[13px] text-ink-secondary">{t("confirm.progress.gone")}</p>
        ) : (
          <div className="flex flex-col gap-3">
            <div className="grid grid-cols-[auto_minmax(0,1fr)] items-baseline gap-x-3 gap-y-1.5">
              <Row label={t("confirm.progress.labels.url")}>
                <span className="block truncate font-mono text-[12px] text-ink-secondary select-text" title={task.url}>
                  {task.url}
                </span>
              </Row>
              <Row label={t("confirm.progress.labels.status")}>
                <span className={`block truncate ${phase === "failed" ? "text-danger-fg" : "text-ink"}`} title={statusText(task)}>
                  {statusText(task)}
                </span>
              </Row>
              <Row label={t("confirm.progress.labels.size")}>
                <span className="num font-mono text-[12px] text-ink">{sizeText(task)}</span>
              </Row>
              {speedText(task) !== null && (
                <Row label={t("confirm.progress.labels.speed")}>
                  <span className="num font-mono text-[12px] text-ink">{speedText(task)}</span>
                </Row>
              )}
              {resumeText(task) !== null && (
                <Row label={t("confirm.progress.labels.resume")}>
                  <span className="text-ink-secondary">{resumeText(task)}</span>
                </Row>
              )}
              {phase === "done" && task.path && (
                <Row label={t("confirm.progress.labels.savedTo")}>
                  <span className="block truncate font-mono text-[12px] text-ink-secondary select-text" title={task.path}>
                    {task.path}
                  </span>
                </Row>
              )}
            </div>
            <SegmentBar segments={task.segments} total={task.total} downloaded={task.downloaded} height="h-2.5" tone={tone} />
            {error && <p className="text-[12px] text-danger-fg">{error}</p>}
          </div>
        )}
      </Dialog>
    </LocaleProvider>
  );
}

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <>
      <span className="label">{label}</span>
      <div className="min-w-0 text-[13px]">{children}</div>
    </>
  );
}
