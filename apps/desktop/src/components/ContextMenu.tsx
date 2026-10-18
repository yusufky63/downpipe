// Downpipe's own right-click menu, in place of the web view's (Back,
// Reload, Inspect). Opened by right click, the Menu key, Shift+F10 or a
// row's More button; arrows, Enter and Escape work as in system menus.
import {
  ArrowDown,
  ArrowUp,
  ArrowUpDown,
  Check,
  ChevronRight,
  ChevronsDown,
  ChevronsUp,
  Clock,
  Copy,
  ExternalLink,
  FolderOpen,
  Gauge,
  Info,
  Layers,
  Link2,
  Pause,
  Play,
  Plus,
  RefreshCcw,
  RotateCw,
  Trash2,
  type LucideIcon,
} from "lucide-react";
import { useEffect, useEffectEvent, useLayoutEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";

import type { MenuAction, MenuEntry, MenuIcon } from "../lib/menu";

const ICONS: Record<MenuIcon, LucideIcon> = {
  open: ExternalLink,
  folder: FolderOpen,
  play: Play,
  retry: RotateCw,
  pause: Pause,
  again: RefreshCcw,
  link: Link2,
  queue: Layers,
  order: ArrowUpDown,
  top: ChevronsUp,
  up: ArrowUp,
  down: ArrowDown,
  bottom: ChevronsDown,
  copy: Copy,
  info: Info,
  trash: Trash2,
  plus: Plus,
  clock: Clock,
  gauge: Gauge,
};

/** Keeps menus off the window edge. */
const MARGIN = 8;

export function ContextMenu({
  x,
  y,
  entries,
  label,
  onAction,
  onClose,
  focusFirst,
}: {
  x: number;
  y: number;
  entries: MenuEntry[];
  label: string;
  onAction: (action: MenuAction) => void;
  onClose: () => void;
  /** Opened from the keyboard: start on the first entry. */
  focusFirst: boolean;
}) {
  const close = useEffectEvent(() => onClose());
  // Read before the panel takes focus: child effects run before this one.
  const [previous] = useState(() => document.activeElement as HTMLElement | null);
  useEffect(() => {
    const outside = (e: Event) => {
      if (!(e.target instanceof Element) || !e.target.closest("[data-menu]")) close();
    };
    // Anything that moves the rows under the menu closes it.
    window.addEventListener("pointerdown", outside, true);
    window.addEventListener("wheel", outside, { passive: true });
    window.addEventListener("resize", close);
    window.addEventListener("blur", close);
    return () => {
      window.removeEventListener("pointerdown", outside, true);
      window.removeEventListener("wheel", outside);
      window.removeEventListener("resize", close);
      window.removeEventListener("blur", close);
      if (previous?.isConnected) previous.focus({ preventScroll: true });
    };
  }, [previous]);

  // Stable, so the menu is not placed again on every list update.
  const at = useMemo(() => ({ x, y }), [x, y]);
  return (
    <MenuPanel
      at={at}
      entries={entries}
      label={label}
      focusFirst={focusFirst}
      takeFocus
      onAction={(action) => {
        onClose();
        onAction(action);
      }}
      onClose={onClose}
    />
  );
}

type Place = { x: number; y: number } | { beside: DOMRect };

function MenuPanel({
  at,
  entries,
  label,
  focusFirst,
  takeFocus,
  onAction,
  onClose,
  onBack,
}: {
  at: Place;
  entries: MenuEntry[];
  label: string;
  focusFirst: boolean;
  /** The root menu, and submenus opened from the keyboard. */
  takeFocus: boolean;
  onAction: (action: MenuAction) => void;
  onClose: () => void;
  /** A submenu returns to its parent instead of closing everything. */
  onBack?: () => void;
}) {
  const panel = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState<{ left: number; top: number } | null>(null);
  const enabled = entries.flatMap((e, i) => (e.kind !== "separator" && !e.disabled ? [i] : []));
  const [active, setActive] = useState<number | null>(focusFirst ? (enabled[0] ?? null) : null);
  const [sub, setSub] = useState<{ index: number; rect: DOMRect; keyboard: boolean } | null>(null);

  useLayoutEffect(() => {
    const el = panel.current;
    if (!el) return;
    const { width, height } = el.getBoundingClientRect();
    const maxLeft = window.innerWidth - width - MARGIN;
    const maxTop = window.innerHeight - height - MARGIN;
    let left: number;
    let top: number;
    if ("beside" in at) {
      // To the right of the parent entry, or to its left when that does not
      // fit; in a narrow window, below it so the entry stays readable.
      left = at.beside.right - 2;
      top = at.beside.top - 5;
      if (left > maxLeft) left = at.beside.left - width + 2;
      if (left < MARGIN) {
        left = at.beside.left + 24;
        top = at.beside.bottom;
      }
    } else {
      left = at.x;
      // Opens upwards from the pointer when there is no room below.
      top = at.y > maxTop ? at.y - height : at.y;
    }
    setPos({ left: clamp(left, MARGIN, maxLeft), top: clamp(top, MARGIN, maxTop) });
  }, [at]);

  useEffect(() => {
    if (takeFocus) panel.current?.focus({ preventScroll: true });
  }, [takeFocus]);

  const openSub = (index: number, keyboard: boolean) => {
    const item = panel.current?.querySelector<HTMLElement>(`[data-index="${index}"]`);
    if (item) setSub({ index, rect: item.getBoundingClientRect(), keyboard });
  };

  const activate = (index: number, keyboard: boolean) => {
    const entry = entries[index];
    if (!entry || entry.kind === "separator" || entry.disabled) return;
    if (entry.kind === "submenu") openSub(index, keyboard);
    else onAction(entry.action);
  };

  const move = (delta: number) => {
    setSub(null);
    if (enabled.length === 0) return;
    const at = active === null ? -1 : enabled.indexOf(active);
    const next = at === -1 ? (delta > 0 ? 0 : enabled.length - 1) : (at + delta + enabled.length) % enabled.length;
    setActive(enabled[next] ?? null);
  };

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    // The list underneath has its own keys; none of them may fire here.
    e.stopPropagation();
    if (e.target !== e.currentTarget) return;
    const keys: Record<string, () => void> = {
      ArrowDown: () => move(1),
      ArrowUp: () => move(-1),
      Home: () => setActive(enabled[0] ?? null),
      End: () => setActive(enabled[enabled.length - 1] ?? null),
      Enter: () => active !== null && activate(active, true),
      " ": () => active !== null && activate(active, true),
      ArrowRight: () => active !== null && entries[active]?.kind === "submenu" && openSub(active, true),
      ArrowLeft: () => onBack?.(),
      Escape: () => (onBack ?? onClose)(),
      Tab: onClose,
    };
    const handler = keys[e.key];
    if (handler) {
      e.preventDefault();
      handler();
    }
  };

  const subEntry = sub ? entries[sub.index] : undefined;

  return (
    <>
      <div
        ref={panel}
        data-menu
        role="menu"
        aria-label={label}
        tabIndex={-1}
        onKeyDown={onKeyDown}
        onContextMenu={(e) => e.preventDefault()}
        // Transparent, not hidden, until placed: a hidden element cannot take focus.
        style={pos ? { left: pos.left, top: pos.top } : { left: 0, top: 0, opacity: 0, pointerEvents: "none" }}
        className="anim-fade fixed z-50 flex max-h-[calc(100vh-16px)] min-w-[232px] flex-col overflow-y-auto rounded-lg border border-line bg-canvas py-1 shadow-popover outline-none"
      >
        {entries.map((entry, i) => {
          if (entry.kind === "separator") return <div key={`sep-${i}`} role="separator" className="my-1 h-px shrink-0 bg-line" />;
          const Icon = entry.icon ? ICONS[entry.icon] : null;
          const isSub = entry.kind === "submenu";
          const radio = entry.kind === "item" && entry.checked !== undefined;
          const checked = entry.kind === "item" && entry.checked === true;
          const danger = entry.kind === "item" && entry.danger;
          const highlighted = active === i || sub?.index === i;
          return (
            <div
              key={`${entry.label}-${i}`}
              data-index={i}
              role={radio ? "menuitemradio" : "menuitem"}
              aria-checked={radio ? checked : undefined}
              aria-disabled={entry.disabled || undefined}
              aria-haspopup={isSub ? "menu" : undefined}
              aria-expanded={isSub ? sub?.index === i : undefined}
              onPointerMove={(e) => {
                // Only a real move: the browser also reports one when the menu
                // opens under a resting pointer, which must not move the highlight.
                if (e.movementX === 0 && e.movementY === 0) return;
                if (active === i && (!isSub || sub?.index === i)) return;
                if (entry.disabled) {
                  setActive(null);
                  setSub(null);
                  return;
                }
                setActive(i);
                if (isSub) openSub(i, false);
                else setSub(null);
              }}
              onClick={() => activate(i, false)}
              className={`flex h-8 shrink-0 cursor-default items-center gap-2.5 px-3 text-[13px] ${
                entry.disabled
                  ? "text-ink-muted opacity-60"
                  : highlighted
                    ? danger
                      ? "bg-danger-soft text-danger-fg"
                      : "bg-surface-muted text-ink"
                    : danger
                      ? "text-danger-fg"
                      : "text-ink"
              }`}
            >
              <span className="flex size-4 shrink-0 items-center justify-center text-ink-muted" aria-hidden="true">
                {checked ? <Check size={15} strokeWidth={2} className="text-primary" /> : Icon ? <Icon size={15} strokeWidth={1.75} /> : null}
              </span>
              <span className="min-w-0 flex-1 truncate">{entry.label}</span>
              {entry.kind === "item" && entry.shortcut && <span className="pl-6 font-mono text-[11px] text-ink-muted">{entry.shortcut}</span>}
              {isSub && <ChevronRight size={14} strokeWidth={1.75} className="text-ink-muted" aria-hidden="true" />}
            </div>
          );
        })}
      </div>
      {sub && subEntry?.kind === "submenu" && (
        <MenuPanel
          key={sub.index}
          at={{ beside: sub.rect }}
          entries={subEntry.items}
          label={subEntry.label}
          focusFirst={sub.keyboard}
          takeFocus={sub.keyboard}
          onAction={onAction}
          onClose={onClose}
          onBack={() => {
            setSub(null);
            setActive(sub.index);
            panel.current?.focus({ preventScroll: true });
          }}
        />
      )}
    </>
  );
}

function clamp(v: number, lo: number, hi: number): number {
  return Math.max(lo, Math.min(v, Math.max(lo, hi)));
}
