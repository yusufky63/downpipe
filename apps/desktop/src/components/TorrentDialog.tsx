import { FolderOpen } from "lucide-react";
import { useEffect, useEffectEvent, useId, useState, type FormEvent } from "react";

import { useI18n } from "../i18n";
import { api, errorMessage, onConfirmClose, type ConfirmOptions, type Queue, type Task, type TorrentChoices } from "../lib/api";
import { formatBytes, sentence, withStop } from "../lib/format";
import { displayName, knownSize } from "../lib/tasks";
import { chosenSummary, initialChosen, isMagnet } from "../lib/torrent";
import { Banner, Button, Dialog, inputClass } from "./primitives";

type Inspection = { state: "loading" } | { state: "ready"; choices: TorrentChoices } | { state: "failed"; message: string };

/**
 * The confirmation for a torrent: which of its files to download, where and
 * in which queue. A magnet link's file list comes from peers first, which
 * can take a while; started before that, every file is downloaded.
 */
export function TorrentDialog({
  task,
  waiting,
  queues,
  inWindow = false,
  onFit,
  onConfirm,
  onDiscard,
  onStopAsking,
}: {
  task: Task;
  /** More downloads waiting after this one. */
  waiting: number;
  queues: Queue[];
  inWindow?: boolean;
  onFit?: (height: number) => void;
  onConfirm: (options: ConfirmOptions) => Promise<void>;
  onDiscard: () => void;
  onStopAsking?: () => void;
}) {
  const { t } = useI18n();
  const [dir, setDir] = useState(task.dir);
  const [queue, setQueue] = useState(task.queue);
  const [inspection, setInspection] = useState<Inspection>({ state: "loading" });
  const [chosen, setChosen] = useState<Set<number>>(new Set());
  const [stopAsking, setStopAsking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const ids = useId();

  useEffect(() => {
    let current = true;
    api
      .inspectTorrent(task.id)
      .then((choices) => {
        if (!current) return;
        setChosen(initialChosen(choices));
        setInspection({ state: "ready", choices });
      })
      .catch((err: unknown) => {
        if (current) setInspection({ state: "failed", message: withStop(sentence(errorMessage(err))) });
      });
    return () => {
      current = false;
    };
  }, [task.id]);

  const choices = inspection.state === "ready" ? inspection.choices : null;
  const nothing = choices !== null && chosen.size === 0;

  const answer = async (start: boolean, e?: FormEvent) => {
    e?.preventDefault();
    if (busy) return;
    if (start && nothing) {
      setError(t("torrent.confirm.noneChosen"));
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await onConfirm({
        file_name: null,
        dir: dir.trim() && dir.trim() !== task.dir ? dir.trim() : null,
        queue: queue !== task.queue ? queue : null,
        start,
        // All of them when they are all chosen; the app then keeps none.
        files: choices && !nothing ? choices.files.filter((f) => chosen.has(f.index)).map((f) => f.index) : null,
      });
      if (stopAsking) onStopAsking?.();
    } catch (err) {
      setError(errorMessage(err));
      setBusy(false);
    }
  };

  const close = () => void answer(false);
  const closeFromWindow = useEffectEvent(close);
  useEffect(() => {
    if (!inWindow) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void onConfirmClose(() => closeFromWindow()).then((fn) => (disposed ? fn() : (unlisten = fn)));
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [inWindow]);

  const browse = async () => {
    const picked = await api.pickFolder(dir || null).catch(() => null);
    if (picked) setDir(picked);
  };

  const toggle = (index: number) =>
    setChosen((now) => {
      const next = new Set(now);
      if (next.has(index)) next.delete(index);
      else next.add(index);
      return next;
    });
  const allChosen = choices !== null && choices.files.every((f) => chosen.has(f.index));
  const toggleAll = () => setChosen(allChosen || !choices ? new Set() : new Set(choices.files.map((f) => f.index)));

  const kind = t("torrent.kind");
  const from = task.from_app ? kind : t("confirm.fromBrowserWhat", { what: kind });
  const offerStop = !task.from_app && onStopAsking !== undefined;
  const name = choices?.name ?? displayName(task);
  const size = choices ? formatBytes(choices.total) : knownSize(task.total, undefined);
  const small = `${inputClass} h-8 text-[13px]`;

  return (
    <Dialog
      eyebrow={waiting > 0 ? t("confirm.moreWaiting", { from, count: waiting }) : from}
      title={t("torrent.confirm.title")}
      frame={inWindow ? "window" : "modal"}
      size="sm"
      compact
      onFit={onFit}
      onClose={close}
      footer={
        <>
          {offerStop && (
            <label className="mr-auto flex items-center gap-1.5 text-[12px] text-ink-secondary" title={t("confirm.stopAskingTitle")}>
              <input type="checkbox" checked={stopAsking} onChange={(e) => setStopAsking(e.target.checked)} className="size-3.5 accent-primary" />
              {t("confirm.stopAsking")}
            </label>
          )}
          <Button
            variant="ghost"
            size="sm"
            disabled={busy}
            onClick={() => {
              if (stopAsking) onStopAsking?.();
              onDiscard();
            }}
          >
            {t("common.cancel")}
          </Button>
          <Button size="sm" onClick={() => void answer(false)} disabled={busy}>
            {t("confirm.later")}
          </Button>
          <Button variant="primary" size="sm" onClick={() => void answer(true)} disabled={busy || nothing}>
            {t("confirm.start")}
          </Button>
        </>
      }
    >
      <form onSubmit={(e) => void answer(true, e)} className="flex flex-col gap-2.5">
        <div className="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-x-3 gap-y-2">
          <span className="label">{t("confirm.name")}</span>
          <div className="flex min-w-0 items-center gap-2">
            <span className="min-w-0 flex-1 truncate text-[13px] text-ink select-text" title={name}>
              {name}
            </span>
            {size && <span className="num shrink-0 font-mono text-[12px] text-ink-secondary">{size}</span>}
          </div>

          <label htmlFor={`${ids}-dir`} className="label">
            {t("confirm.saveTo")}
          </label>
          <div className="flex min-w-0 gap-2">
            <input
              id={`${ids}-dir`}
              value={dir}
              onChange={(e) => setDir(e.target.value)}
              spellCheck={false}
              title={dir}
              className={`${small} min-w-0 flex-1 font-mono text-[12px]`}
            />
            <Button size="sm" onClick={() => void browse()}>
              <FolderOpen size={14} strokeWidth={1.75} />
              {t("common.browse")}
            </Button>
          </div>

          {queues.length > 1 && (
            <>
              <label htmlFor={`${ids}-queue`} className="label">
                {t("confirm.queue")}
              </label>
              <select id={`${ids}-queue`} value={queue} onChange={(e) => setQueue(e.target.value)} className={small}>
                {queues.map((q) => (
                  <option key={q.id} value={q.id}>
                    {q.name}
                  </option>
                ))}
              </select>
            </>
          )}
        </div>

        <div className="flex flex-col rounded-md border border-line">
          <div className="flex items-center justify-between gap-3 border-b border-line px-3 py-1.5">
            <label className="flex items-center gap-2 text-[12px] font-medium text-ink">
              <input
                type="checkbox"
                checked={allChosen}
                disabled={!choices}
                ref={(el) => {
                  if (el) el.indeterminate = choices !== null && chosen.size > 0 && !allChosen;
                }}
                onChange={toggleAll}
                className="size-3.5 accent-primary"
              />
              {t("torrent.confirm.all")}
            </label>
            {choices && <span className="num font-mono text-[12px] text-ink-secondary">{chosenSummary(choices, chosen)}</span>}
          </div>
          {choices ? (
            <ul aria-label={t("torrent.confirm.files")} className="max-h-48 overflow-y-auto py-1">
              {choices.files.map((f) => (
                <li key={f.index}>
                  <label className="flex cursor-pointer items-center gap-2 px-3 py-1 hover:bg-surface">
                    <input type="checkbox" checked={chosen.has(f.index)} onChange={() => toggle(f.index)} className="size-3.5 shrink-0 accent-primary" />
                    <span className="min-w-0 flex-1 truncate text-[12px] text-ink" title={f.path}>
                      {f.path}
                    </span>
                    <span className="num shrink-0 font-mono text-[12px] text-ink-secondary">{formatBytes(f.size)}</span>
                  </label>
                </li>
              ))}
            </ul>
          ) : (
            <p className="px-3 py-3 text-[12px] text-ink-muted">
              {inspection.state === "failed"
                ? t("torrent.confirm.failed", { message: inspection.message })
                : isMagnet(task.url)
                  ? t("torrent.confirm.loadingMagnet")
                  : t("torrent.confirm.loading")}
            </p>
          )}
        </div>

        {choices?.folder && <p className="text-[12px] text-ink-muted">{t("torrent.confirm.folder", { name: choices.name })}</p>}
        {choices?.private && <Banner tone="neutral">{t("torrent.confirm.private")}</Banner>}
        <p className="text-[12px] text-ink-muted">{t("torrent.confirm.privacy")}</p>
        {error && <p className="text-[12px] text-danger-fg">{error}</p>}
      </form>
    </Dialog>
  );
}
