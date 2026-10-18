// The right-click menu of the download list, as data: which entries the
// selected downloads get and whether they apply. The component draws it.
import { t } from "../i18n/core";
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

/** A video page's address does not expire, so only file and stream links can be replaced. */
export const canRelink = (t: Task) =>
  t.site === null && (t.state === "paused" || t.state === "failed" || t.state === "canceled" || t.state === "queued");

export function buildMenu(selected: readonly Task[], queues: readonly Queue[]): MenuEntry[] {
  if (selected.length === 0) return [];
  const one = selected.length === 1 ? (selected[0] ?? null) : null;
  const done = one?.state === "completed";
  const resumable = selected.filter(canResume);
  const retry = resumable.length > 0 && resumable.every((t) => t.state !== "paused");
  const many = selected.length > 1;

  const entries: MenuEntry[] = [];
  if (one) {
    if (done) entries.push({ kind: "item", action: "open", label: t("list.menu.open"), icon: "open" });
    entries.push({ kind: "item", action: "reveal", label: t("list.menu.reveal"), icon: "folder" }, SEPARATOR);
  }
  entries.push(
    {
      kind: "item",
      action: "resume",
      label: retry ? t("list.menu.retry") : t("list.menu.resume"),
      icon: retry ? "retry" : "play",
      disabled: resumable.length === 0,
    },
    { kind: "item", action: "pause", label: t("list.menu.pause"), icon: "pause", disabled: !selected.some(canPause) },
  );
  if (one && done) entries.push({ kind: "item", action: "redownload", label: t("list.menu.redownload"), icon: "again" });
  if (one && canRelink(one)) entries.push({ kind: "item", action: "relink", label: t("list.menu.relink"), icon: "link" });
  entries.push(
    SEPARATOR,
    {
      kind: "submenu",
      label: t("list.menu.moveToQueue"),
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
        { kind: "item", action: "new_queue", label: t("list.menu.newQueue"), icon: "plus" },
      ],
    },
    {
      kind: "submenu",
      label: t("list.menu.order"),
      icon: "order",
      items: [
        { kind: "item", action: "move_top", label: t("list.menu.moveTop"), icon: "top", shortcut: SHORTCUTS.move_top },
        { kind: "item", action: "move_up", label: t("list.menu.moveUp"), icon: "up", shortcut: SHORTCUTS.move_up },
        { kind: "item", action: "move_down", label: t("list.menu.moveDown"), icon: "down", shortcut: SHORTCUTS.move_down },
        { kind: "item", action: "move_bottom", label: t("list.menu.moveBottom"), icon: "bottom", shortcut: SHORTCUTS.move_bottom },
      ],
    },
    SEPARATOR,
    { kind: "item", action: "copy_link", label: many ? t("list.menu.copyLinks") : t("list.menu.copyLink"), icon: "copy", shortcut: SHORTCUTS.copy_link },
  );
  if (one?.path) entries.push({ kind: "item", action: "copy_path", label: t("list.menu.copyPath"), icon: "copy" });
  entries.push(SEPARATOR);
  if (one) entries.push({ kind: "item", action: "properties", label: t("list.menu.properties"), icon: "info", shortcut: SHORTCUTS.properties });
  entries.push({
    kind: "item",
    action: "remove",
    label: many ? t("list.menu.removeMany", { count: selected.length }) : t("list.menu.remove"),
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
