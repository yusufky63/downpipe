import { ArrowDown, ArrowUp, FolderOpen, Link2, MoreHorizontal, Pause, Play, RotateCw } from "lucide-react";
import type { DragEvent, MouseEvent } from "react";

import { useI18n, type MessageKey } from "../i18n";
import { longDate, shortDate } from "../i18n/format";
import { MAIN_QUEUE, type Task } from "../lib/api";
import { formatBytes, formatDuration, formatSpeed, hostOf, percent } from "../lib/format";
import { sizeIsEstimate, streamLabel } from "../lib/media";
import type { Modifiers } from "../lib/selection";
import type { Sort, SortKey } from "../lib/sort";
import { canPause, canResume, displayName, errorText, isRunning, lastActivity, needsNewLink, stateInfo, waitingText } from "../lib/tasks";
import { Badge, IconButton } from "./primitives";
import { SegmentBar } from "./SegmentBar";

// One template for header and rows keeps the columns aligned.
// The actions column has a fixed width (three 32px buttons) because every
// row is its own grid: an `auto` column would size differently per row.
const COLUMNS =
  "grid grid-cols-[16px_minmax(0,1fr)_auto] items-center gap-x-4 md:grid-cols-[16px_minmax(0,1fr)_88px_minmax(120px,220px)_96px_100px] lg:grid-cols-[16px_minmax(0,1fr)_96px_minmax(140px,240px)_104px_72px_104px_100px]";

const CHECKBOX = "size-4 cursor-pointer accent-primary";

export interface RowActions {
  /** Click: select, with Ctrl or Shift for several. */
  onSelect: (id: string, mods: Modifiers) => void;
  /** The row's checkbox: add or remove it, with Shift for a range. */
  onToggle: (id: string, shift: boolean) => void;
  /** Double click: the details dialog. */
  onProperties: (id: string) => void;
  /** Right click or the More button; `keyboard` starts on the first entry. */
  onMenu: (id: string, x: number, y: number, keyboard: boolean) => void;
  onPause: (id: string) => void;
  onResume: (id: string) => void;
  /** A failure only a fresh link fixes: the details dialog with its form open. */
  onRelink: (id: string) => void;
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
  onToggleAll,
  queueNames,
}: {
  tasks: Task[];
  selected: ReadonlySet<string>;
  actions: RowActions;
  sort: Sort;
  onSort: (key: SortKey) => void;
  /** The header checkbox: select every row shown, or none. */
  onToggleAll: () => void;
  /** Shown on rows outside the main queue. */
  queueNames: ReadonlyMap<string, string>;
}) {
  const { t } = useI18n();
  const header = (key: SortKey, label: MessageKey, className: string, align: "left" | "right" = "left") => (
    <SortHeader sortKey={key} label={t(label)} sort={sort} onSort={onSort} className={className} align={align} />
  );
  const shown = tasks.filter((task) => selected.has(task.id)).length;
  const all = shown > 0 && shown === tasks.length;
  const now = new Date();
  return (
    <div role="grid" aria-label={t("list.label")} aria-multiselectable="true" className="min-w-0">
      <div role="row" className={`${COLUMNS} sticky top-0 z-20 border-b border-line bg-canvas px-4 py-1.5`}>
        <span role="columnheader" className="flex items-center">
          <input
            type="checkbox"
            aria-label={t("list.selectAll")}
            title={t("list.selectAll")}
            checked={all}
            ref={(el) => {
              // Some, not all: a dash in the box.
              if (el) el.indeterminate = shown > 0 && !all;
            }}
            onChange={onToggleAll}
            className={CHECKBOX}
          />
        </span>
        {header("name", "list.columns.name", "")}
        {header("size", "list.columns.size", "hidden md:flex", "right")}
        {header("progress", "list.columns.progress", "hidden md:flex")}
        {header("speed", "list.columns.speed", "hidden md:flex", "right")}
        {header("left", "list.columns.left", "hidden lg:flex", "right")}
        {header("date", "list.columns.date", "hidden lg:flex", "right")}
        <span role="columnheader" className="label text-right">
          <span className="sr-only">{t("list.columns.actions")}</span>
        </span>
      </div>
      <div className="divide-y divide-line">
        {tasks.map((task) => (
          <DownloadRow
            key={task.id}
            task={task}
            now={now}
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
  now,
  selected,
  actions,
  queueName,
}: {
  task: Task;
  now: Date;
  selected: boolean;
  actions: RowActions;
  queueName: string | null;
}) {
  const { t } = useI18n();
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
    (task.state === "failed" ? errorText(task) : waitingText(task)) ??
    [hostOf(task.url), streamLabel(task), task.category, queueName].filter(Boolean).join(" · ");
  const done = task.state === "completed";
  const name = displayName(task);
  const dateTitle = [
    t("list.dateAdded", { date: longDate(task.created_at) }),
    task.completed_at !== null ? t("list.dateFinished", { date: longDate(task.completed_at) }) : t("list.dateLastTry", { date: longDate(task.updated_at) }),
  ].join("\n");

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
      aria-label={`${name}, ${info.label}`}
      title={done ? t("list.dragOut") : undefined}
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
      <div role="gridcell" className="flex items-center" onDoubleClick={(e) => e.stopPropagation()}>
        <input
          type="checkbox"
          aria-label={t("list.selectRow", { name })}
          checked={selected}
          readOnly
          onClick={(e) => {
            // The row's own click would select only this one.
            e.stopPropagation();
            actions.onToggle(task.id, e.shiftKey);
          }}
          className={CHECKBOX}
        />
      </div>
      <div role="gridcell" className="flex min-w-0 flex-col gap-1">
        <div className="flex min-w-0 items-center gap-2">
          <span className="truncate text-[14px] font-medium text-ink" title={name}>
            {name}
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
      <span role="gridcell" className="num hidden truncate text-right font-mono text-[12px] text-ink-secondary lg:block" title={dateTitle}>
        {shortDate(lastActivity(task), now)}
      </span>
      <div
        role="gridcell"
        className="flex items-center justify-end gap-0.5"
        onClick={(e) => e.stopPropagation()}
        onDoubleClick={(e) => e.stopPropagation()}
      >
        {canPause(task) && (
          <IconButton label={t("common.pause")} onClick={() => actions.onPause(task.id)}>
            <Pause size={16} strokeWidth={1.75} />
          </IconButton>
        )}
        {needsNewLink(task) ? (
          <IconButton label={t("details.relink.button")} onClick={() => actions.onRelink(task.id)}>
            <Link2 size={16} strokeWidth={1.75} />
          </IconButton>
        ) : canResume(task) && (
          <IconButton label={task.state === "paused" ? t("common.resume") : t("common.retry")} onClick={() => actions.onResume(task.id)}>
            {task.state === "paused" ? <Play size={16} strokeWidth={1.75} /> : <RotateCw size={16} strokeWidth={1.75} />}
          </IconButton>
        )}
        <IconButton label={t("common.showInFolder")} onClick={() => actions.onReveal(task.id)}>
          <FolderOpen size={16} strokeWidth={1.75} />
        </IconButton>
        <IconButton label={t("common.moreActions")} aria-haspopup="menu" onClick={onMore}>
          <MoreHorizontal size={16} strokeWidth={1.75} />
        </IconButton>
      </div>
    </div>
  );
}
