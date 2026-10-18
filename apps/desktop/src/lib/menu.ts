// The right-click menu of the download list, as data: which entries the
// selected downloads get and whether they apply. The component draws it.
import { currentLocale, t } from "../i18n/core";
import type { Queue, Task } from "./api";
import { canPause, canResume } from "./tasks";
import { TONIGHT, UNTIL_PRESETS, canPauseUntil, type UntilPreset } from "./until";

export type MenuAction =
  | "open"
  | "reveal"
  | "resume"
  | "pause"
  | `pause_until:${UntilPreset}`
  | "pause_until_custom"
  | "speed_limit"
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
  | "plus"
  | "clock"
  | "gauge";

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

/** A video page's address and a torrent's hash do not expire, so only file and stream links can be replaced. */
export const canRelink = (t: Task) =>
  t.site === null && t.torrent === null && (t.state === "paused" || t.state === "failed" || t.state === "canceled" || t.state === "queued");

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
    {
      kind: "item",
      action: "pause",
      // For finished torrents that still upload, pausing stops the seeding.
      label: selected.every((t) => t.swarm?.seeding) ? t("torrent.stopSeeding") : t("list.menu.pause"),
      icon: "pause",
      disabled: !selected.some(canPause),
    },
    {
      kind: "submenu",
      label: t("list.menu.pauseUntil"),
      icon: "clock",
      disabled: !selected.some(canPauseUntil),
      items: [
        ...UNTIL_PRESETS.map((preset): MenuEntry => ({ kind: "item", action: `pause_until:${preset}`, label: untilLabel(preset) })),
        SEPARATOR,
        { kind: "item", action: "pause_until_custom", label: t("list.menu.untilCustom") },
      ],
    },
  );
  if (one && done) entries.push({ kind: "item", action: "redownload", label: t("list.menu.redownload"), icon: "again" });
  if (one && canRelink(one)) entries.push({ kind: "item", action: "relink", label: t("list.menu.relink"), icon: "link" });
  entries.push({
    kind: "item",
    action: "speed_limit",
    label: t("list.menu.speedLimit"),
    icon: "gauge",
    // Torrents follow the BitTorrent limits in Settings instead.
    disabled: selected.every((t) => t.state === "completed" || t.torrent !== null),
  });
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

function untilLabel(preset: UntilPreset): string {
  switch (preset) {
    case "30m":
      return t("list.menu.until30m");
    case "1h":
      return t("list.menu.until1h");
    case "tonight": {
      const at = new Date(2000, 0, 1, TONIGHT.hour, TONIGHT.minute);
      const time = new Intl.DateTimeFormat(currentLocale(), { hour: "2-digit", minute: "2-digit" }).format(at);
      return t("list.menu.untilTonight", { time });
    }
  }
}

/** What gets copied: one link per line, in the order shown. */
export function linksText(selected: readonly Task[]): string {
  return selected.map((t) => t.url).join("\n");
}
