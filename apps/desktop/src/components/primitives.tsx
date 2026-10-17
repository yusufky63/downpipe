import { X } from "lucide-react";
import { useEffect, useEffectEvent, useId, useRef, type ButtonHTMLAttributes, type ReactNode } from "react";

import type { Tone } from "../lib/tasks";

type Variant = "primary" | "secondary" | "ghost" | "danger";
type Size = "sm" | "md";

const VARIANTS: Record<Variant, string> = {
  primary: "bg-primary-fill text-primary-contrast border border-primary-fill hover:bg-primary-strong hover:border-primary-strong",
  secondary: "border border-line-strong text-ink hover:bg-surface-muted",
  ghost: "border border-transparent text-ink-secondary hover:text-ink hover:bg-surface-muted",
  danger: "border border-line-strong text-danger-fg hover:bg-danger-soft",
};

const SIZES: Record<Size, string> = {
  sm: "h-8 px-3 text-[13px] gap-1.5",
  md: "h-9 px-3.5 text-[14px] gap-2",
};

export function Button({
  variant = "secondary",
  size = "md",
  className = "",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: Variant; size?: Size }) {
  return (
    <button
      type="button"
      className={`inline-flex shrink-0 items-center justify-center rounded-md font-medium whitespace-nowrap transition-colors duration-150 disabled:pointer-events-none disabled:opacity-50 ${VARIANTS[variant]} ${SIZES[size]} ${className}`}
      {...props}
    />
  );
}

export function IconButton({
  label,
  className = "",
  tone = "default",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { label: string; tone?: "default" | "danger" }) {
  const toneClass =
    tone === "danger" ? "text-ink-muted hover:text-danger-fg hover:bg-danger-soft" : "text-ink-muted hover:text-ink hover:bg-surface-muted";
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      className={`inline-flex size-8 shrink-0 items-center justify-center rounded-md transition-colors duration-150 disabled:pointer-events-none disabled:opacity-40 ${toneClass} ${className}`}
      {...props}
    />
  );
}

const BADGE_TONES: Record<Tone, string> = {
  primary: "bg-primary-soft text-primary",
  positive: "bg-positive-soft text-positive-fg",
  danger: "bg-danger-soft text-danger-fg",
  warning: "bg-warning-soft text-warning-fg",
  neutral: "bg-surface-muted text-ink-secondary",
};

export function Badge({ tone, children }: { tone: Tone; children: ReactNode }) {
  return (
    <span className={`inline-flex h-5 items-center rounded-sm px-1.5 font-mono text-[11px] tracking-wide uppercase ${BADGE_TONES[tone]}`}>
      {children}
    </span>
  );
}

export function Banner({ tone, children }: { tone: "danger" | "warning" | "neutral"; children: ReactNode }) {
  const cls =
    tone === "danger"
      ? "bg-danger-soft text-danger-fg"
      : tone === "warning"
        ? "bg-warning-soft text-warning-fg"
        : "bg-surface-muted text-ink-secondary";
  return <div className={`rounded-md px-3 py-2 text-[13px] ${cls}`}>{children}</div>;
}

export function Field({
  label,
  hint,
  error,
  children,
}: {
  label: string;
  hint?: ReactNode;
  error?: string | null;
  children: (id: string) => ReactNode;
}) {
  const id = useId();
  return (
    <div className="flex flex-col gap-1.5">
      <label htmlFor={id} className="label">
        {label}
      </label>
      {children(id)}
      {error ? (
        <p className="text-[13px] text-danger-fg">{error}</p>
      ) : hint ? (
        <p className="text-[12px] text-ink-muted">{hint}</p>
      ) : null}
    </div>
  );
}

/** An on/off setting: title and explanation on the left, the switch on the right. */
export function Switch({
  label,
  hint,
  checked,
  onChange,
}: {
  label: string;
  hint?: ReactNode;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  const id = useId();
  const hintId = useId();
  return (
    <label htmlFor={id} className="flex cursor-pointer items-start justify-between gap-4">
      <span className="flex min-w-0 flex-col gap-1">
        <span className="text-[14px] font-medium text-ink">{label}</span>
        {hint && (
          <span id={hintId} className="text-[12px] text-ink-muted">
            {hint}
          </span>
        )}
      </span>
      <input
        id={id}
        type="checkbox"
        role="switch"
        checked={checked}
        aria-describedby={hint ? hintId : undefined}
        onChange={(e) => onChange(e.target.checked)}
        className="switch mt-0.5"
      />
    </label>
  );
}

/** Shared look for inputs: the border shows focus, never a ring. */
export const inputClass =
  "w-full rounded-md border border-line-strong bg-canvas px-3 text-[14px] text-ink placeholder:text-ink-muted transition-colors duration-150 focus:border-primary";

export function KeyValue({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex min-w-0 flex-col gap-1 px-4 py-3">
      <span className="label">{label}</span>
      <div className="min-w-0 text-[13px] text-ink">{children}</div>
    </div>
  );
}

export function Dialog({
  title,
  eyebrow,
  onClose,
  children,
  footer,
  aside,
  size = "md",
  focusFirstField = true,
}: {
  title: string;
  eyebrow: string;
  onClose: () => void;
  children: ReactNode;
  footer: ReactNode;
  /** Shown next to the title, e.g. a status badge. */
  aside?: ReactNode;
  size?: "md" | "lg";
  /** Forms focus their first field; read-only dialogs focus themselves so no stray button looks selected. */
  focusFirstField?: boolean;
}) {
  const panel = useRef<HTMLDivElement>(null);
  const titleId = useId();
  // Callers pass a new onClose on every render (the list updates every
  // second); focus must be set once, not each time.
  const close = useEffectEvent(() => onClose());

  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    const first = focusFirstField
      ? panel.current?.querySelector<HTMLElement>("input, textarea, select, button:not([data-close])")
      : null;
    (first ?? panel.current)?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      if (previous?.isConnected) previous.focus();
    };
  }, [focusFirstField]);

  return (
    <div className="fixed inset-0 z-100 flex items-end justify-center md:items-center md:p-6">
      <div className="anim-fade absolute inset-0 bg-scrim" onClick={onClose} aria-hidden="true" />
      <div
        ref={panel}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
        className={`anim-rise relative flex max-h-[92vh] w-full flex-col rounded-t-xl border border-line bg-canvas shadow-overlay outline-none md:rounded-xl ${
          size === "lg" ? "md:max-w-[720px]" : "md:max-w-[560px]"
        }`}
      >
        <div className="flex items-start justify-between gap-4 border-b border-line px-5 py-4">
          <div className="flex min-w-0 flex-col gap-1">
            <span className="eyebrow">{eyebrow}</span>
            <div className="flex min-w-0 items-center gap-2">
              <h2 id={titleId} className="display-medium truncate text-[20px] leading-tight" title={title}>
                {title}
              </h2>
              {aside}
            </div>
          </div>
          <IconButton label="Close" data-close onClick={onClose}>
            <X size={18} strokeWidth={1.75} />
          </IconButton>
        </div>
        <div className="flex-1 overflow-y-auto px-5 py-5">{children}</div>
        <div className="flex flex-wrap items-center justify-end gap-2 border-t border-line px-5 py-4">{footer}</div>
      </div>
    </div>
  );
}
