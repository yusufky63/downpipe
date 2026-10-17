import { Check, Copy, ExternalLink, FolderOpen, Link2, Pause, Play, RefreshCcw, RotateCw } from "lucide-react";
import { useState, type FormEvent, type ReactNode } from "react";

import { errorMessage, type Task } from "../lib/api";
import { canRelink } from "../lib/menu";
import { formatBytes, formatDate, formatDuration, formatSpeed, percent, sentence } from "../lib/format";
import { kindName, sizeIsEstimate } from "../lib/media";
import { canCancel, canPause, canResume, displayName, isRunning, stateInfo } from "../lib/tasks";
import { Badge, Banner, Button, Dialog, IconButton, inputClass } from "./primitives";
import { SegmentBar } from "./SegmentBar";

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
}

export function DetailsDialog({
  task,
  actions,
  queueName,
  startRelink = false,
}: {
  task: Task;
  actions: DetailActions;
  /** Null for the main queue, which needs no mention. */
  queueName: string | null;
  /** Opened from "Use a new link…": the form is already open. */
  startRelink?: boolean;
}) {
  const info = stateInfo(task);
  const pct = percent(task.downloaded, task.total);
  const running = isRunning(task.state);
  const tone = task.state === "completed" ? "positive" : task.state === "failed" ? "danger" : running ? "primary" : "neutral";

  return (
    <Dialog
      size="lg"
      eyebrow="Download details"
      title={displayName(task)}
      aside={<Badge tone={info.tone}>{info.label}</Badge>}
      onClose={actions.onClose}
      focusFirstField={false}
      footer={
        <>
          <div className="mr-auto flex flex-wrap items-center gap-1">
            <Button size="sm" variant="ghost" onClick={() => actions.onReveal(task.id)}>
              <FolderOpen size={16} strokeWidth={1.75} />
              Show in folder
            </Button>
            {task.state === "completed" && (
              <Button size="sm" variant="ghost" onClick={() => actions.onOpen(task.id)}>
                <ExternalLink size={16} strokeWidth={1.75} />
                Open file
              </Button>
            )}
          </div>
          <Button size="sm" variant="ghost" onClick={() => actions.onRemove(task.id)}>
            Remove…
          </Button>
          {canCancel(task) && (
            <Button size="sm" variant="danger" onClick={() => actions.onCancel(task.id)} title="Stops the download and deletes the partial file">
              Cancel and delete
            </Button>
          )}
          <PrimaryAction task={task} actions={actions} />
        </>
      }
    >
      <div className="flex flex-col gap-5">
        <section className="flex flex-col gap-2.5" aria-label="Progress">
          <div className="flex items-baseline justify-between gap-3">
            <span className="label">Progress</span>
            <span className="num font-mono text-[12px] text-ink-secondary">
              {formatBytes(task.downloaded)}
              {task.total !== null ? ` of ${sizeIsEstimate(task) ? "about " : ""}${formatBytes(task.total)}` : ""}
              {pct !== null ? ` · ${pct.toFixed(1)}%` : ""}
            </span>
          </div>
          <SegmentBar segments={task.segments} total={task.total} downloaded={task.downloaded} height="h-3" tone={tone} />
          {running && task.segments.length > 0 && (
            <div className="flex flex-wrap items-center gap-x-4 gap-y-1 font-mono text-[11px] text-ink-muted" aria-hidden="true">
              <span className="inline-flex items-center gap-1.5">
                <span className="size-2 rounded-full bg-primary" /> Downloaded
              </span>
              <span className="inline-flex items-center gap-1.5">
                <span className="h-2.5 w-0.5 bg-primary-strong" /> Connection
              </span>
              <span className="inline-flex items-center gap-1.5">
                <span className="size-2 rounded-full bg-surface-muted ring-1 ring-line" /> Remaining
              </span>
            </div>
          )}
        </section>

        {(task.state === "failed" && task.error) || task.notices.length > 0 ? (
          <div className="flex flex-col gap-2">
            {task.state === "failed" && task.error && <Banner tone="danger">{sentence(task.error)}</Banner>}
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
            <Stat label="Speed" value={formatSpeed(task.speed)} />
            <Stat label="Time left" value={formatDuration(task.eta_secs)} />
            <Stat
              label="Connections"
              value={String(task.connections)}
              note={task.target_connections > 0 ? `target ${task.target_connections}` : undefined}
            />
            <Stat label="Resume" value={task.resumable ? "Supported" : "Not supported"} />
          </div>
        ) : task.state !== "completed" ? (
          <div className="grid grid-cols-3 gap-px overflow-hidden rounded-lg border border-line bg-line">
            <Stat label="Size" value={task.total !== null ? formatBytes(task.total) : "Unknown"} />
            <Stat label="Downloaded" value={formatBytes(task.downloaded)} />
            <Stat label="Resume" value={task.resumable ? "Supported" : "Unknown"} />
          </div>
        ) : null}

        {canRelink(task) && <ChangeLink task={task} startOpen={startRelink} onChangeUrl={actions.onChangeUrl} />}

        <dl className="divide-y divide-line rounded-lg border border-line">
          <Row label={task.path ? "Saved to" : "Folder"}>
            <CopyValue value={task.path ?? task.dir} />
          </Row>
          <Row label="Source">
            <CopyValue value={task.url} />
          </Row>
          {task.media && (
            <Row label="Stream">
              <span className="num font-mono text-[12px] text-ink-secondary">
                {[
                  kindName(task.media.kind),
                  task.media.quality ?? "Best quality",
                  task.fragments ? `${task.fragments.done} of ${task.fragments.total} fragments` : null,
                ]
                  .filter(Boolean)
                  .join(" · ")}
              </span>
            </Row>
          )}
          {queueName && (
            <Row label="Queue">
              <span className="text-[13px] text-ink-secondary">{queueName}</span>
            </Row>
          )}
          {task.checksum && (
            <Row label="Checksum">
              <CopyValue value={task.checksum} />
            </Row>
          )}
          <Row label="Added">
            <span className="num font-mono text-[12px] text-ink-secondary">{formatDate(task.created_at)}</span>
          </Row>
          {task.completed_at !== null && (
            <Row label="Finished">
              <span className="num font-mono text-[12px] text-ink-secondary">{formatDate(task.completed_at)}</span>
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
  const [open, setOpen] = useState(startOpen || task.error_kind === "forbidden" || task.error_kind === "not_found");
  const [url, setUrl] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  if (!open) {
    return (
      <div>
        <Button size="sm" variant="ghost" onClick={() => setOpen(true)}>
          <Link2 size={16} strokeWidth={1.75} />
          Use a new link…
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
        New link for this file
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
          Continue
        </Button>
      </div>
      {error ? (
        <p className="text-[13px] text-danger-fg">{error}</p>
      ) : (
        <p className="text-[12px] text-ink-muted">
          For an expired or broken link. The download continues where it stopped only if the new link serves the same file; otherwise nothing is
          overwritten.
        </p>
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
  if (canPause(task)) {
    return (
      <Button size="sm" onClick={() => actions.onPause(task.id)}>
        <Pause size={16} strokeWidth={1.75} />
        Pause
      </Button>
    );
  }
  if (canResume(task)) {
    return (
      <Button size="sm" variant="primary" onClick={() => actions.onResume(task.id)}>
        {task.state === "paused" ? <Play size={16} strokeWidth={1.75} /> : <RotateCw size={16} strokeWidth={1.75} />}
        {task.state === "paused" ? "Resume" : task.state === "canceled" ? "Download again" : "Retry"}
      </Button>
    );
  }
  if (task.state === "completed") {
    return (
      <Button size="sm" onClick={() => actions.onRedownload(task.id)}>
        <RefreshCcw size={16} strokeWidth={1.75} />
        Download again…
      </Button>
    );
  }
  return null;
}

function CopyValue({ value }: { value: string }) {
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
      <IconButton label={copied ? "Copied" : "Copy"} onClick={copy} className="size-7">
        {copied ? <Check size={14} strokeWidth={1.75} /> : <Copy size={14} strokeWidth={1.75} />}
      </IconButton>
    </div>
  );
}
