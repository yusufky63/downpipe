import { Check, Copy, ExternalLink, FolderOpen, Pause, Play, RotateCw, X } from "lucide-react";
import { useState } from "react";

import type { Task } from "../lib/api";
import { formatBytes, formatDuration, formatSpeed, percent, sentence } from "../lib/format";
import { canCancel, canPause, canResume, displayName, isRunning, stateInfo } from "../lib/tasks";
import { Badge, Banner, Button, IconButton, KeyValue } from "./primitives";
import { SegmentBar } from "./SegmentBar";

export interface DetailActions {
  onClose: () => void;
  onPause: (id: string) => void;
  onResume: (id: string) => void;
  onCancel: (id: string) => void;
  onReveal: (id: string) => void;
  onOpen: (id: string) => void;
  onRemove: (id: string) => void;
}

export function DetailPanel({ task, actions }: { task: Task; actions: DetailActions }) {
  const info = stateInfo(task);
  const pct = percent(task.downloaded, task.total);
  const running = isRunning(task.state);
  const location = task.path ?? task.dir;

  return (
    <section aria-label="Download details" className="ticks anim-fade mx-4 mb-4 rounded-lg border border-line bg-canvas">
      <header className="flex items-center gap-3 border-b border-line px-4 py-3">
        <div className="flex min-w-0 flex-1 flex-col gap-0.5">
          <span className="eyebrow">Details</span>
          <div className="flex min-w-0 items-center gap-2">
            <h2 className="display-medium truncate text-[16px]" title={displayName(task)}>
              {displayName(task)}
            </h2>
            <Badge tone={info.tone}>{info.label}</Badge>
          </div>
        </div>
        <div className="hidden items-center gap-2 sm:flex">
          <PrimaryActions task={task} actions={actions} />
        </div>
        <IconButton label="Close details" onClick={actions.onClose}>
          <X size={18} strokeWidth={1.75} />
        </IconButton>
      </header>

      <div className="flex flex-col gap-3 px-4 pt-4">
        <div className="flex items-baseline justify-between gap-3">
          <span className="label">Segments</span>
          <span className="num font-mono text-[12px] text-ink-secondary">
            {formatBytes(task.downloaded)}
            {task.total !== null ? ` of ${formatBytes(task.total)}` : ""}
            {pct !== null ? ` · ${pct.toFixed(1)}%` : ""}
          </span>
        </div>
        <SegmentBar
          segments={task.segments}
          total={task.total}
          downloaded={task.downloaded}
          height="h-4"
          tone={task.state === "completed" ? "positive" : task.state === "failed" ? "danger" : running ? "primary" : "neutral"}
        />
        {task.state === "failed" && task.error && <Banner tone="danger">{sentence(task.error)}</Banner>}
        {task.notices.map((n) => (
          <Banner key={n} tone="warning">
            {n}
          </Banner>
        ))}
      </div>

      <div className="mt-2 grid grid-cols-2 lg:grid-cols-4">
        <KeyValue label="Speed">
          <span className="num font-mono">{running ? formatSpeed(task.speed) : "–"}</span>
        </KeyValue>
        <KeyValue label="Time left">
          <span className="num font-mono">{running ? formatDuration(task.eta_secs) : "–"}</span>
        </KeyValue>
        <KeyValue label="Connections">
          <span className="num font-mono">
            {running ? `${task.connections} open` : "–"}
            {running && task.target_connections > 0 ? ` · target ${task.target_connections}` : ""}
          </span>
        </KeyValue>
        <KeyValue label="Resume">
          <span className="font-mono">{task.state === "completed" ? "–" : task.resumable ? "Supported" : running ? "Not supported" : "Unknown"}</span>
        </KeyValue>
      </div>

      <div className="grid grid-cols-1 border-t border-line md:grid-cols-2">
        <KeyValue label={task.path ? "Saved to" : "Folder"}>
          <CopyLine value={location} />
        </KeyValue>
        <KeyValue label="Source">
          <CopyLine value={task.url} />
        </KeyValue>
        {task.checksum && (
          <KeyValue label="Checksum">
            <CopyLine value={task.checksum} />
          </KeyValue>
        )}
      </div>

      <div className="flex flex-wrap items-center gap-2 border-t border-line px-4 py-3">
        <div className="flex items-center gap-2 sm:hidden">
          <PrimaryActions task={task} actions={actions} />
        </div>
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
        <div className="ml-auto flex items-center gap-2">
          {canCancel(task) && (
            <Button size="sm" variant="danger" onClick={() => actions.onCancel(task.id)}>
              Cancel and delete partial data
            </Button>
          )}
          <Button size="sm" variant="ghost" onClick={() => actions.onRemove(task.id)}>
            Remove…
          </Button>
        </div>
      </div>
    </section>
  );
}

function PrimaryActions({ task, actions }: { task: Task; actions: DetailActions }) {
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
  return null;
}

function CopyLine({ value }: { value: string }) {
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
