import { Check, Copy, ExternalLink, FolderOpen, Link2, Pause, Play, RefreshCcw, RotateCw } from "lucide-react";
import { useState, type FormEvent, type ReactNode } from "react";

import { useI18n } from "../i18n";
import { longDate } from "../i18n/format";
import { errorMessage, type PostActions, type Settings, type Task } from "../lib/api";
import { canRelink } from "../lib/menu";
import { formatBytes, formatDuration, formatPercent, formatSpeed, percent } from "../lib/format";
import { engineQuality, kindName, sizeIsEstimate } from "../lib/media";
import { canCancel, canPause, canResume, displayName, errorDetail, errorText, isRunning, needsNewLink, stateInfo, waitingText } from "../lib/tasks";
import { PostSection } from "./PostSection";
import { Badge, Banner, Button, Dialog, IconButton, inputClass } from "./primitives";
import { SegmentBar } from "./SegmentBar";
import { RateFields, draftBytes, rateDraft } from "./SpeedLimitDialog";

export interface DetailActions {
  onClose: () => void;
  onPause: (id: string) => void;
  onResume: (id: string) => void;
  onCancel: (id: string) => void;
  onReveal: (id: string) => void;
  onOpen: (id: string) => void;
  onRemove: (id: string) => void;
  /** Rejects with the reason the link was refused. */
  onChangeUrl: (id: string, url: string) => Promise<void>;
  onRedownload: (id: string) => void;
  /** Bytes per second; null leaves only the global limit. */
  onSpeedLimit: (id: string, limit: number | null) => void;
  /** The folder its archive was extracted into. */
  onOpenExtracted: (id: string) => void;
  /** Rejects with the reason a command line was refused. */
  onSetPost: (id: string, actions: PostActions) => Promise<void>;
}

export function DetailsDialog({
  task,
  actions,
  queueName,
  settings,
  startRelink = false,
}: {
  task: Task;
  actions: DetailActions;
  /** For the command and extraction a download gets by default. */
  settings: Settings | null;
  /** Null for the main queue, which needs no mention. */
  queueName: string | null;
  /** Opened from "Use a new link…": the form is already open. */
  startRelink?: boolean;
}) {
  const { t } = useI18n();
  const info = stateInfo(task);
  const pct = percent(task.downloaded, task.total);
  const running = isRunning(task.state);
  const waiting = waitingText(task);
  const tone = task.state === "completed" ? "positive" : task.state === "failed" ? "danger" : running ? "primary" : "neutral";

  return (
    <Dialog
      size="lg"
      eyebrow={t("details.eyebrow")}
      title={displayName(task)}
      aside={<Badge tone={info.tone}>{info.label}</Badge>}
      onClose={actions.onClose}
      focusFirstField={false}
      footer={
        <>
          <div className="mr-auto flex flex-wrap items-center gap-1">
            <Button size="sm" variant="ghost" onClick={() => actions.onReveal(task.id)}>
              <FolderOpen size={16} strokeWidth={1.75} />
              {t("common.showInFolder")}
            </Button>
            {task.state === "completed" && (
              <Button size="sm" variant="ghost" onClick={() => actions.onOpen(task.id)}>
                <ExternalLink size={16} strokeWidth={1.75} />
                {t("details.openFile")}
              </Button>
            )}
          </div>
          <Button size="sm" variant="ghost" onClick={() => actions.onRemove(task.id)}>
            {t("details.remove")}
          </Button>
          {canCancel(task) && (
            <Button size="sm" variant="danger" onClick={() => actions.onCancel(task.id)} title={t("details.cancelAndDeleteTitle")}>
              {t("details.cancelAndDelete")}
            </Button>
          )}
          <PrimaryAction task={task} actions={actions} />
        </>
      }
    >
      <div className="flex flex-col gap-5">
        <section className="flex flex-col gap-2.5" aria-label={t("details.progress")}>
          <div className="flex items-baseline justify-between gap-3">
            <span className="label">{t("details.progress")}</span>
            <span className="num font-mono text-[12px] text-ink-secondary">
              {task.total !== null
                ? t(sizeIsEstimate(task) ? "details.progressOfAbout" : "details.progressOf", {
                    downloaded: formatBytes(task.downloaded),
                    total: formatBytes(task.total),
                  })
                : formatBytes(task.downloaded)}
              {pct !== null ? ` · ${formatPercent(pct, 1)}` : ""}
            </span>
          </div>
          <SegmentBar segments={task.segments} total={task.total} downloaded={task.downloaded} height="h-3" tone={tone} />
          {running && task.segments.length > 0 && (
            <div className="flex flex-wrap items-center gap-x-4 gap-y-1 font-mono text-[11px] text-ink-muted" aria-hidden="true">
              <span className="inline-flex items-center gap-1.5">
                <span className="size-2 rounded-full bg-primary" /> {t("details.legend.downloaded")}
              </span>
              <span className="inline-flex items-center gap-1.5">
                <span className="h-2.5 w-0.5 bg-primary-strong" /> {t("details.legend.connection")}
              </span>
              <span className="inline-flex items-center gap-1.5">
                <span className="size-2 rounded-full bg-surface-muted ring-1 ring-line" /> {t("details.legend.remaining")}
              </span>
            </div>
          )}
        </section>

        {(task.state === "failed" && task.error) || waiting || task.notices.length > 0 ? (
          <div className="flex flex-col gap-2">
            {task.state === "failed" && task.error && (
              <Banner tone="danger">
                <span>{errorText(task)}</span>
                {errorDetail(task) && (
                  // The engine's English text, for searching the web or reporting a bug.
                  <span className="mt-1.5 flex flex-col gap-0.5">
                    <span className="label">{t("errors.technical")}</span>
                    <span className="select-text break-words font-mono text-[12px] text-ink-secondary">{errorDetail(task)}</span>
                  </span>
                )}
              </Banner>
            )}
            {waiting && <Banner tone="warning">{waiting}</Banner>}
            {task.notices.map((n) => (
              <Banner key={n} tone="warning">
                {n}
              </Banner>
            ))}
          </div>
        ) : null}

        {/* 1px gaps over a line-coloured background draw the hairlines. Live
            numbers only while running; a finished download needs none. */}
        {running ? (
          <div className="grid grid-cols-2 gap-px overflow-hidden rounded-lg border border-line bg-line md:grid-cols-4">
            <Stat label={t("details.stats.speed")} value={formatSpeed(task.speed)} />
            <Stat label={t("details.stats.timeLeft")} value={formatDuration(task.eta_secs)} />
            <Stat
              label={t("details.stats.connections")}
              value={String(task.connections)}
              note={task.target_connections > 0 ? t("details.stats.target", { count: task.target_connections }) : undefined}
            />
            <Stat
              label={t("details.stats.resume")}
              value={task.resumable ? t("details.stats.supported") : t("details.stats.notSupported")}
            />
          </div>
        ) : task.state !== "completed" ? (
          <div className="grid grid-cols-3 gap-px overflow-hidden rounded-lg border border-line bg-line">
            <Stat label={t("details.stats.size")} value={task.total !== null ? formatBytes(task.total) : t("common.unknown")} />
            <Stat label={t("details.stats.downloaded")} value={formatBytes(task.downloaded)} />
            <Stat label={t("details.stats.resume")} value={task.resumable ? t("details.stats.supported") : t("common.unknown")} />
          </div>
        ) : null}

        <PostSection task={task} settings={settings} onOpenExtracted={actions.onOpenExtracted} onSave={actions.onSetPost} />

        {canRelink(task) && <ChangeLink task={task} startOpen={startRelink} onChangeUrl={actions.onChangeUrl} />}

        <dl className="divide-y divide-line rounded-lg border border-line">
          <Row label={task.path ? t("details.rows.savedTo") : t("details.rows.folder")}>
            <CopyValue value={task.path ?? task.dir} />
          </Row>
          <Row label={t("details.rows.source")}>
            <CopyValue value={task.url} />
          </Row>
          {task.site && (
            <Row label={t("details.rows.video")}>
              <span className="num font-mono text-[12px] text-ink-secondary">
                {[task.site.site, task.site.quality ? engineQuality(task.site.quality) : t("details.rows.bestQuality"), "yt-dlp"].join(" · ")}
              </span>
            </Row>
          )}
          {task.media && (
            <Row label={t("details.rows.stream")}>
              <span className="num font-mono text-[12px] text-ink-secondary">
                {[
                  kindName(task.media.kind),
                  task.media.quality ? engineQuality(task.media.quality) : t("details.rows.bestQuality"),
                  task.fragments ? t("details.rows.fragments", { done: task.fragments.done, count: task.fragments.total }) : null,
                ]
                  .filter(Boolean)
                  .join(" · ")}
              </span>
            </Row>
          )}
          {task.state !== "completed" && (
            <Row label={t("details.rows.speedLimit")}>
              <SpeedLimit task={task} onSpeedLimit={actions.onSpeedLimit} />
            </Row>
          )}
          {task.resume_at !== null && (task.state === "paused" || task.state === "failed") && (
            <Row label={t("details.rows.resumes")}>
              <span className="num font-mono text-[12px] text-ink-secondary">{longDate(task.resume_at)}</span>
            </Row>
          )}
          {queueName && (
            <Row label={t("details.rows.queue")}>
              <span className="text-[13px] text-ink-secondary">{queueName}</span>
            </Row>
          )}
          {task.site_rule && (
            <Row label={t("details.rows.siteRule")}>
              <span className="font-mono text-[12px] break-all text-ink-secondary">{task.site_rule}</span>
            </Row>
          )}
          {task.checksum && (
            <Row label={t("details.rows.checksum")}>
              <CopyValue value={task.checksum} />
            </Row>
          )}
          <Row label={t("details.rows.added")}>
            <span className="num font-mono text-[12px] text-ink-secondary">{longDate(task.created_at)}</span>
          </Row>
          {task.completed_at !== null && (
            <Row label={t("details.rows.finished")}>
              <span className="num font-mono text-[12px] text-ink-secondary">{longDate(task.completed_at)}</span>
            </Row>
          )}
        </dl>
      </div>
    </Dialog>
  );
}

/**
 * IDM's "refresh download address": links from file hosts and CDNs expire.
 * Paste a fresh one and the download continues if it is the same file.
 */
function ChangeLink({ task, startOpen, onChangeUrl }: { task: Task; startOpen: boolean; onChangeUrl: DetailActions["onChangeUrl"] }) {
  const { t } = useI18n();
  const [open, setOpen] = useState(startOpen || needsNewLink(task) || task.error_kind === "not_found");
  const [url, setUrl] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  if (!open) {
    return (
      <div>
        <Button size="sm" variant="ghost" onClick={() => setOpen(true)}>
          <Link2 size={16} strokeWidth={1.75} />
          {t("details.relink.button")}
        </Button>
      </div>
    );
  }

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!url.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await onChangeUrl(task.id, url.trim());
      setUrl("");
      setOpen(false);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <form onSubmit={submit} className="flex flex-col gap-2 rounded-lg border border-line p-4">
      <label htmlFor={`relink-${task.id}`} className="label">
        {t("details.relink.label")}
      </label>
      <div className="flex gap-2">
        <input
          id={`relink-${task.id}`}
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          placeholder="https://"
          spellCheck={false}
          autoFocus
          className={`${inputClass} h-9 font-mono text-[13px]`}
        />
        <Button type="submit" variant="primary" disabled={busy || !url.trim()}>
          {t("details.relink.submit")}
        </Button>
      </div>
      {error ? (
        <p className="text-[13px] text-danger-fg">{error}</p>
      ) : (
        <p className="text-[12px] text-ink-muted">{t("details.relink.hint")}</p>
      )}
    </form>
  );
}

/** The download's own limit, changed in place; a running download follows at once. */
function SpeedLimit({ task, onSpeedLimit }: { task: Task; onSpeedLimit: DetailActions["onSpeedLimit"] }) {
  const { t } = useI18n();
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(() => rateDraft(task.speed_limit));

  if (!editing) {
    return (
      <div className="flex min-w-0 items-center gap-2">
        <span className="num truncate font-mono text-[12px] text-ink-secondary">
          {task.speed_limit !== null ? formatSpeed(task.speed_limit) : t("details.speedLimit.none")}
        </span>
        <Button
          size="sm"
          variant="ghost"
          onClick={() => {
            setDraft(rateDraft(task.speed_limit));
            setEditing(true);
          }}
        >
          {t("details.speedLimit.change")}
        </Button>
      </div>
    );
  }

  const save = (limit: number | null) => {
    onSpeedLimit(task.id, limit);
    setEditing(false);
  };
  return (
    <form
      className="flex flex-wrap items-center gap-2"
      aria-label={t("details.speedLimit.label")}
      onSubmit={(e) => {
        e.preventDefault();
        save(draftBytes(draft));
      }}
    >
      <div className="min-w-0 flex-1">
        <RateFields
          draft={draft}
          onChange={setDraft}
          placeholder={t("dialogs.speedLimit.placeholder")}
          unitLabel={t("dialogs.speedLimit.unit")}
        />
      </div>
      <Button size="sm" type="submit" variant="primary">
        {t("details.speedLimit.set")}
      </Button>
      {task.speed_limit !== null && (
        <Button size="sm" variant="ghost" onClick={() => save(null)}>
          {t("details.speedLimit.remove")}
        </Button>
      )}
    </form>
  );
}

function Stat({ label, value, note }: { label: string; value: string; note?: string }) {
  return (
    <div className="flex min-w-0 flex-col gap-1 bg-canvas px-4 py-3">
      <span className="label">{label}</span>
      <span className="num truncate font-mono text-[14px] text-ink">
        {value}
        {note && <span className="text-[12px] text-ink-muted"> · {note}</span>}
      </span>
    </div>
  );
}

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="grid grid-cols-1 gap-1 px-4 py-2.5 sm:grid-cols-[96px_minmax(0,1fr)] sm:items-center sm:gap-3">
      <dt className="label">{label}</dt>
      <dd className="m-0 min-w-0">{children}</dd>
    </div>
  );
}

function PrimaryAction({ task, actions }: { task: Task; actions: DetailActions }) {
  const { t } = useI18n();
  if (canPause(task)) {
    return (
      <Button size="sm" onClick={() => actions.onPause(task.id)}>
        <Pause size={16} strokeWidth={1.75} />
        {t("common.pause")}
      </Button>
    );
  }
  if (canResume(task)) {
    return (
      <Button size="sm" variant="primary" onClick={() => actions.onResume(task.id)}>
        {task.state === "paused" ? <Play size={16} strokeWidth={1.75} /> : <RotateCw size={16} strokeWidth={1.75} />}
        {task.state === "paused" ? t("common.resume") : task.state === "canceled" ? t("details.downloadAgain") : t("common.retry")}
      </Button>
    );
  }
  if (task.state === "completed") {
    return (
      <Button size="sm" onClick={() => actions.onRedownload(task.id)}>
        <RefreshCcw size={16} strokeWidth={1.75} />
        {t("details.redownload")}
      </Button>
    );
  }
  return null;
}

function CopyValue({ value }: { value: string }) {
  const { t } = useI18n();
  const [copied, setCopied] = useState(false);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1200);
    } catch {
      // Clipboard can be unavailable; the value stays selectable.
    }
  };
  return (
    <div className="flex min-w-0 items-center gap-1">
      <span className="truncate font-mono text-[12px] text-ink-secondary select-text" title={value}>
        {value}
      </span>
      <IconButton label={copied ? t("details.copied") : t("details.copy")} onClick={copy} className="size-7">
        {copied ? <Check size={14} strokeWidth={1.75} /> : <Copy size={14} strokeWidth={1.75} />}
      </IconButton>
    </div>
  );
}
