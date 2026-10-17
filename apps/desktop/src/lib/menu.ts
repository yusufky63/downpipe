// The right-click menu of the download list, as data: which entries the
// selected downloads get and whether they apply. The component draws it.
import type { Queue, Task } from "./api";
import { canPause, canResume } from "./tasks";

export type MenuAction =
  | "open"
  | "reveal"
  | "resume"
  | "pause"
  | "redownload"
  | "relink"
  | "move_top"
  | "move_up"
  | "move_down"
  | "move_bottom"
  | "copy_link"
  | "copy_path"
  | "properties"
  | "remove"
  | "new_queue"
  | `queue:${string}`;

export type MenuIcon =
  | "open"
  | "folder"
  | "play"
  | "retry"
  | "pause"
  | "again"
  | "link"
  | "queue"
  | "order"
  | "top"
  | "up"
  | "down"
  | "bottom"
  | "copy"
  | "info"
  | "trash"
  | "plus";

export type MenuEntry =
  | {
      kind: "item";
      action: MenuAction;
      label: string;
      icon?: MenuIcon;
      shortcut?: string;
      disabled?: boolean;
      danger?: boolean;
      checked?: boolean;
    }
  | { kind: "submenu"; label: string; icon?: MenuIcon; items: MenuEntry[]; disabled?: boolean }
  | { kind: "separator" };

/** Shown next to the entries; the list handles the same keys. */
export const SHORTCUTS = {
  properties: "Enter",
  remove: "Del",
  copy_link: "Ctrl+C",
  move_top: "Ctrl+Home",
  move_up: "Ctrl+Up",
  move_down: "Ctrl+Down",
  move_bottom: "Ctrl+End",
} as const;

const SEPARATOR: MenuEntry = { kind: "separator" };

export const canRelink = (t: Task) => t.state === "paused" || t.state === "failed" || t.state === "canceled" || t.state === "queued";

export function buildMenu(selected: readonly Task[], queues: readonly Queue[]): MenuEntry[] {
  if (selected.length === 0) return [];
  const one = selected.length === 1 ? (selected[0] ?? null) : null;
  const done = one?.state === "completed";
  const resumable = selected.filter(canResume);
  const retry = resumable.length > 0 && resumable.every((t) => t.state !== "paused");
  const many = selected.length > 1;

  const entries: MenuEntry[] = [];
  if (one) {
    if (done) entries.push({ kind: "item", action: "open", label: "Open", icon: "open" });
    entries.push({ kind: "item", action: "reveal", label: "Show in folder", icon: "folder" }, SEPARATOR);
  }
  entries.push(
    {
      kind: "item",
      action: "resume",
      label: retry ? "Retry" : "Resume",
      icon: retry ? "retry" : "play",
      disabled: resumable.length === 0,
    },
    { kind: "item", action: "pause", label: "Pause", icon: "pause", disabled: !selected.some(canPause) },
  );
  if (one && done) entries.push({ kind: "item", action: "redownload", label: "Download again…", icon: "again" });
  if (one && canRelink(one)) entries.push({ kind: "item", action: "relink", label: "Use a new link…", icon: "link" });
  entries.push(
    SEPARATOR,
    {
      kind: "submenu",
      label: "Move to queue",
      icon: "queue",
      items: [
        ...queues.map(
          (q): MenuEntry => ({
            kind: "item",
            action: `queue:${q.id}`,
            label: q.name,
            checked: selected.every((t) => t.queue === q.id),
          }),
        ),
        SEPARATOR,
        { kind: "item", action: "new_queue", label: "New queue…", icon: "plus" },
      ],
    },
    {
      kind: "submenu",
      label: "Queue order",
      icon: "order",
      items: [
        { kind: "item", action: "move_top", label: "Move to top", icon: "top", shortcut: SHORTCUTS.move_top },
        { kind: "item", action: "move_up", label: "Move up", icon: "up", shortcut: SHORTCUTS.move_up },
        { kind: "item", action: "move_down", label: "Move down", icon: "down", shortcut: SHORTCUTS.move_down },
        { kind: "item", action: "move_bottom", label: "Move to bottom", icon: "bottom", shortcut: SHORTCUTS.move_bottom },
      ],
    },
    SEPARATOR,
    { kind: "item", action: "copy_link", label: many ? "Copy links" : "Copy link", icon: "copy", shortcut: SHORTCUTS.copy_link },
  );
  if (one?.path) entries.push({ kind: "item", action: "copy_path", label: "Copy file path", icon: "copy" });
  entries.push(SEPARATOR);
  if (one) entries.push({ kind: "item", action: "properties", label: "Properties", icon: "info", shortcut: SHORTCUTS.properties });
  entries.push({
    kind: "item",
    action: "remove",
    label: many ? `Remove ${selected.length} downloads…` : "Remove…",
    icon: "trash",
    shortcut: SHORTCUTS.remove,
    danger: true,
  });
  return entries;
}

/** What gets copied: one link per line, in the order shown. */
export function linksText(selected: readonly Task[]): string {
  return selected.map((t) => t.url).join("\n");
}
