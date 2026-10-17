import { FolderOpen, Pause, Play, RotateCw, Trash2 } from "lucide-react";
import type { KeyboardEvent } from "react";

import type { Task } from "../lib/api";
import { formatBytes, formatDuration, formatSpeed, hostOf, percent, sentence } from "../lib/format";
import { canPause, canResume, displayName, isRunning, stateInfo } from "../lib/tasks";
import { Badge, IconButton } from "./primitives";
import { SegmentBar } from "./SegmentBar";

// One template for header and rows keeps the columns aligned.
// The actions column has a fixed width (three 32px buttons) because every
// row is its own grid: an `auto` column would size differently per row.
const COLUMNS =
  "grid grid-cols-[minmax(0,1fr)_auto] items-center gap-x-4 md:grid-cols-[minmax(0,1fr)_88px_minmax(120px,220px)_96px_100px] lg:grid-cols-[minmax(0,1fr)_96px_minmax(160px,260px)_104px_72px_100px]";

export interface RowActions {
  onSelect: (id: string) => void;
  onPause: (id: string) => void;
  onResume: (id: string) => void;
  onReveal: (id: string) => void;
  onRemove: (id: string) => void;
  onOpen: (id: string) => void;
}

export function DownloadList({ tasks, selectedId, actions }: { tasks: Task[]; selectedId: string | null; actions: RowActions }) {
  return (
    <div role="table" aria-label="Downloads" className="min-w-0">
      <div role="row" className={`${COLUMNS} sticky top-0 z-20 border-b border-line bg-canvas px-4 py-2`}>
        <span role="columnheader" className="label">
          Name
        </span>
        <span role="columnheader" className="label hidden text-right md:block">
          Size
        </span>
        <span role="columnheader" className="label hidden md:block">
          Progress
        </span>
        <span role="columnheader" className="label hidden text-right md:block">
          Speed
        </span>
        <span role="columnheader" className="label hidden text-right lg:block">
          Left
        </span>
        <span role="columnheader" className="label text-right">
          <span className="sr-only">Actions</span>
        </span>
      </div>
      <div className="divide-y divide-line">
        {tasks.map((task) => (
          <DownloadRow key={task.id} task={task} selected={task.id === selectedId} actions={actions} />
        ))}
      </div>
    </div>
  );
}

function DownloadRow({ task, selected, actions }: { task: Task; selected: boolean; actions: RowActions }) {
  const info = stateInfo(task);
  const pct = percent(task.downloaded, task.total);
  const running = isRunning(task.state);
  const barTone = task.state === "completed" ? "positive" : task.state === "failed" ? "danger" : running ? "primary" : "neutral";
  const sizeText = task.total !== null ? formatBytes(task.total) : task.downloaded > 0 ? formatBytes(task.downloaded) : "–";
  const detail = task.state === "failed" && task.error ? sentence(task.error) : hostOf(task.url);

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.target !== e.currentTarget) return;
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      actions.onSelect(task.id);
    }
  };

  return (
    <div
      role="row"
      tabIndex={0}
      aria-selected={selected}
      aria-label={`${displayName(task)}, ${info.label}. Open details`}
      onClick={() => actions.onSelect(task.id)}
      onKeyDown={onKeyDown}
      className={`${COLUMNS} rail cursor-pointer px-4 py-3 ${selected ? "rail-active bg-surface" : "hover:bg-surface"}`}
    >
      <div role="cell" className="flex min-w-0 flex-col gap-1">
        <div className="flex min-w-0 items-center gap-2">
          <span className="truncate text-[14px] font-medium text-ink" title={displayName(task)}>
            {displayName(task)}
          </span>
          <Badge tone={info.tone}>{info.label}</Badge>
        </div>
        <span className={`truncate font-mono text-[12px] ${task.state === "failed" ? "text-danger-fg" : "text-ink-muted"}`} title={detail}>
          {detail}
        </span>
        {/* Narrow windows fold progress into the name cell. */}
        <div className="flex items-center gap-2 pt-1 md:hidden">
          <SegmentBar segments={task.segments} total={task.total} downloaded={task.downloaded} tone={barTone} />
          <span className="num shrink-0 font-mono text-[12px] text-ink-secondary">
            {pct !== null ? `${Math.floor(pct)}%` : task.downloaded > 0 ? formatBytes(task.downloaded) : "–"}
            {running && task.speed > 0 ? ` · ${formatSpeed(task.speed)}` : ""}
          </span>
        </div>
      </div>
      <span role="cell" className="num hidden text-right font-mono text-[13px] text-ink-secondary md:block">
        {sizeText}
      </span>
      <div role="cell" className="hidden items-center gap-2 md:flex">
        <SegmentBar segments={task.segments} total={task.total} downloaded={task.downloaded} tone={barTone} />
        <span className="num w-10 shrink-0 text-right font-mono text-[12px] text-ink-secondary">
          {pct === null ? "–" : `${Math.floor(pct)}%`}
        </span>
      </div>
      <span role="cell" className="num hidden text-right font-mono text-[13px] text-ink-secondary md:block">
        {running ? formatSpeed(task.speed) : "–"}
      </span>
      <span role="cell" className="num hidden text-right font-mono text-[13px] text-ink-secondary lg:block">
        {running ? formatDuration(task.eta_secs) : "–"}
      </span>
      <div role="cell" className="flex items-center justify-end gap-0.5" onClick={(e) => e.stopPropagation()}>
        {canPause(task) && (
          <IconButton label="Pause" onClick={() => actions.onPause(task.id)}>
            <Pause size={16} strokeWidth={1.75} />
          </IconButton>
        )}
        {canResume(task) && (
          <IconButton label={task.state === "paused" ? "Resume" : "Retry"} onClick={() => actions.onResume(task.id)}>
            {task.state === "paused" ? <Play size={16} strokeWidth={1.75} /> : <RotateCw size={16} strokeWidth={1.75} />}
          </IconButton>
        )}
        <IconButton label="Show in folder" onClick={() => actions.onReveal(task.id)}>
          <FolderOpen size={16} strokeWidth={1.75} />
        </IconButton>
        <IconButton label="Remove from list" tone="danger" onClick={() => actions.onRemove(task.id)}>
          <Trash2 size={16} strokeWidth={1.75} />
        </IconButton>
      </div>
    </div>
  );
}
