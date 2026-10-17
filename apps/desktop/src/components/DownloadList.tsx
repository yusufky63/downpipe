import { ArrowDown, ArrowUp, FolderOpen, MoreHorizontal, Pause, Play, RotateCw } from "lucide-react";
import type { DragEvent, MouseEvent } from "react";

import { MAIN_QUEUE, type Task } from "../lib/api";
import { formatBytes, formatDuration, formatSpeed, hostOf, percent, sentence } from "../lib/format";
import { sizeIsEstimate, streamLabel } from "../lib/media";
import type { Modifiers } from "../lib/selection";
import type { Sort, SortKey } from "../lib/sort";
import { canPause, canResume, displayName, isRunning, stateInfo } from "../lib/tasks";
import { Badge, IconButton } from "./primitives";
import { SegmentBar } from "./SegmentBar";

// One template for header and rows keeps the columns aligned.
// The actions column has a fixed width (three 32px buttons) because every
// row is its own grid: an `auto` column would size differently per row.
const COLUMNS =
  "grid grid-cols-[minmax(0,1fr)_auto] items-center gap-x-4 md:grid-cols-[minmax(0,1fr)_88px_minmax(120px,220px)_96px_100px] lg:grid-cols-[minmax(0,1fr)_96px_minmax(160px,260px)_104px_72px_100px]";

export interface RowActions {
  /** Click: select, with Ctrl or Shift for several. */
  onSelect: (id: string, mods: Modifiers) => void;
  /** Double click: the details dialog. */
  onProperties: (id: string) => void;
  /** Right click or the More button; `keyboard` starts on the first entry. */
  onMenu: (id: string, x: number, y: number, keyboard: boolean) => void;
  onPause: (id: string) => void;
  onResume: (id: string) => void;
  onReveal: (id: string) => void;
  /** Start dragging a finished file out of the window. */
  onDragOut: (id: string) => void;
}

/** For focusing a row from the keyboard handler. */
export const rowSelector = (id: string) => `[data-row-id="${CSS.escape(id)}"]`;

export function DownloadList({
  tasks,
  selected,
  actions,
  sort,
  onSort,
  queueNames,
}: {
  tasks: Task[];
  selected: ReadonlySet<string>;
  actions: RowActions;
  sort: Sort;
  onSort: (key: SortKey) => void;
  /** Shown on rows outside the main queue. */
  queueNames: ReadonlyMap<string, string>;
}) {
  const header = (key: SortKey, label: string, className: string, align: "left" | "right" = "left") => (
    <SortHeader sortKey={key} label={label} sort={sort} onSort={onSort} className={className} align={align} />
  );
  return (
    <div role="grid" aria-label="Downloads" aria-multiselectable="true" className="min-w-0">
      <div role="row" className={`${COLUMNS} sticky top-0 z-20 border-b border-line bg-canvas px-4 py-1.5`}>
        {header("name", "Name", "")}
        {header("size", "Size", "hidden md:flex", "right")}
        {header("progress", "Progress", "hidden md:flex")}
        {header("speed", "Speed", "hidden md:flex", "right")}
        {header("left", "Left", "hidden lg:flex", "right")}
        <span role="columnheader" className="label text-right">
          <span className="sr-only">Actions</span>
        </span>
      </div>
      <div className="divide-y divide-line">
        {tasks.map((task) => (
          <DownloadRow
            key={task.id}
            task={task}
            selected={selected.has(task.id)}
            actions={actions}
            queueName={task.queue === MAIN_QUEUE ? null : (queueNames.get(task.queue) ?? null)}
          />
        ))}
      </div>
    </div>
  );
}

function SortHeader({
  sortKey,
  label,
  sort,
  onSort,
  className,
  align,
}: {
  sortKey: SortKey;
  label: string;
  sort: Sort;
  onSort: (key: SortKey) => void;
  className: string;
  align: "left" | "right";
}) {
  const active = sort.key === sortKey;
  const Arrow = sort.dir === "asc" ? ArrowUp : ArrowDown;
  return (
    <span
      role="columnheader"
      aria-sort={active ? (sort.dir === "asc" ? "ascending" : "descending") : "none"}
      className={`${className} ${align === "right" ? "justify-end" : ""}`}
    >
      <button
        type="button"
        onClick={() => onSort(sortKey)}
        className={`label -mx-1 inline-flex h-7 items-center gap-1 rounded-sm px-1 transition-colors duration-150 hover:text-ink ${active ? "text-ink" : ""}`}
      >
        {label}
        {active && <Arrow size={12} strokeWidth={2} aria-hidden="true" />}
      </button>
    </span>
  );
}

function DownloadRow({
  task,
  selected,
  actions,
  queueName,
}: {
  task: Task;
  selected: boolean;
  actions: RowActions;
  queueName: string | null;
}) {
  const info = stateInfo(task);
  const pct = percent(task.downloaded, task.total);
  const running = isRunning(task.state);
  const barTone = task.state === "completed" ? "positive" : task.state === "failed" ? "danger" : running ? "primary" : "neutral";
  // A stream's size is known only when its last fragment is in.
  const sizeText =
    task.total !== null
      ? `${sizeIsEstimate(task) ? "~" : ""}${formatBytes(task.total)}`
      : task.downloaded > 0
        ? formatBytes(task.downloaded)
        : "–";
  const detail =
    task.state === "failed" && task.error
      ? sentence(task.error)
      : [hostOf(task.url), streamLabel(task), task.category, queueName].filter(Boolean).join(" · ");
  const done = task.state === "completed";

  // A native drag needs the mouse button still down, so it starts from the
  // HTML drag event and hands over to the system right away.
  const onDragStart = (e: DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    actions.onDragOut(task.id);
  };

  const onMore = (e: MouseEvent<HTMLButtonElement>) => {
    const r = e.currentTarget.getBoundingClientRect();
    // detail is 0 when Enter or Space pressed the button.
    actions.onMenu(task.id, r.right - 232, r.bottom + 4, e.detail === 0);
  };

  return (
    <div
      role="row"
      tabIndex={0}
      data-row-id={task.id}
      aria-selected={selected}
      aria-label={`${displayName(task)}, ${info.label}`}
      title={done ? "Drag the file into another app" : undefined}
      draggable={done}
      onDragStart={done ? onDragStart : undefined}
      onClick={(e) => actions.onSelect(task.id, { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey })}
      onDoubleClick={() => actions.onProperties(task.id)}
      onContextMenu={(e) => {
        e.preventDefault();
        actions.onMenu(task.id, e.clientX, e.clientY, false);
      }}
      className={`${COLUMNS} rail cursor-default px-4 py-3 ${selected ? "rail-active bg-primary-soft" : "hover:bg-surface"}`}
    >
      <div role="gridcell" className="flex min-w-0 flex-col gap-1">
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
      <span role="gridcell" className="num hidden text-right font-mono text-[13px] text-ink-secondary md:block">
        {sizeText}
      </span>
      <div role="gridcell" className="hidden items-center gap-2 md:flex">
        <SegmentBar segments={task.segments} total={task.total} downloaded={task.downloaded} tone={barTone} />
        <span className="num w-10 shrink-0 text-right font-mono text-[12px] text-ink-secondary">
          {pct === null ? "–" : `${Math.floor(pct)}%`}
        </span>
      </div>
      <span role="gridcell" className="num hidden text-right font-mono text-[13px] text-ink-secondary md:block">
        {running ? formatSpeed(task.speed) : "–"}
      </span>
      <span role="gridcell" className="num hidden text-right font-mono text-[13px] text-ink-secondary lg:block">
        {running ? formatDuration(task.eta_secs) : "–"}
      </span>
      <div
        role="gridcell"
        className="flex items-center justify-end gap-0.5"
        onClick={(e) => e.stopPropagation()}
        onDoubleClick={(e) => e.stopPropagation()}
      >
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
        <IconButton label="More actions" aria-haspopup="menu" onClick={onMore}>
          <MoreHorizontal size={16} strokeWidth={1.75} />
        </IconButton>
      </div>
    </div>
  );
}
