import { useState, type FormEvent } from "react";

import { useI18n } from "../i18n";
import type { Task } from "../lib/api";
import { bytesToRate, formatSpeed, rateToBytes, type RateUnit } from "../lib/format";
import { displayName } from "../lib/tasks";
import { Button, Dialog, inputClass } from "./primitives";

/** A rate as typed: the number stays a string until it is used. */
export interface RateDraft {
  value: string;
  unit: RateUnit;
}

export function rateDraft(bytes: number | null): RateDraft {
  const rate = bytesToRate(bytes ?? 0);
  return { value: rate.value > 0 ? String(rate.value) : "", unit: rate.unit };
}

/** Bytes per second, or null for an empty or zero field: no limit of its own. */
export function draftBytes(draft: RateDraft): number | null {
  const bytes = rateToBytes(Number.parseFloat(draft.value), draft.unit);
  return bytes > 0 ? bytes : null;
}

/** The number and unit fields of a speed limit, as in Settings. */
export function RateFields({
  id,
  draft,
  onChange,
  placeholder,
  unitLabel,
}: {
  id?: string;
  draft: RateDraft;
  onChange: (draft: RateDraft) => void;
  placeholder: string;
  unitLabel: string;
}) {
  return (
    <div className="flex gap-2">
      <input
        id={id}
        type="number"
        min={0}
        step="any"
        value={draft.value}
        placeholder={placeholder}
        onChange={(e) => onChange({ ...draft, value: e.target.value })}
        className={`${inputClass} num h-9 font-mono`}
      />
      <select
        aria-label={unitLabel}
        value={draft.unit}
        onChange={(e) => onChange({ ...draft, unit: e.target.value as RateUnit })}
        className={`${inputClass} h-9 w-28 font-mono`}
      >
        <option value="KiB">KiB/s</option>
        <option value="MiB">MiB/s</option>
      </select>
    </div>
  );
}

/** "Speed limit…" from the right-click menu, for one download or several. */
export function SpeedLimitDialog({
  tasks,
  globalLimit,
  onClose,
  onSave,
}: {
  tasks: Task[];
  /** Settings.speed_limit; 0 is none. */
  globalLimit: number;
  onClose: () => void;
  onSave: (limit: number | null) => void;
}) {
  const { t } = useI18n();
  const first = tasks[0];
  // Several downloads start from their shared limit, if they have one.
  const shared = tasks.every((x) => x.speed_limit === first?.speed_limit) ? (first?.speed_limit ?? null) : null;
  const [draft, setDraft] = useState(() => rateDraft(shared));
  const many = tasks.length > 1;
  const limit = formatSpeed(globalLimit);
  const body =
    globalLimit > 0
      ? t(many ? "dialogs.speedLimit.bodyMany" : "dialogs.speedLimit.body", { limit })
      : t(many ? "dialogs.speedLimit.bodyManyNoGlobal" : "dialogs.speedLimit.bodyNoGlobal");
  const submit = (e?: FormEvent) => {
    e?.preventDefault();
    onSave(draftBytes(draft));
  };

  return (
    <Dialog
      eyebrow={t("dialogs.speedLimit.eyebrow")}
      title={many || !first ? t("common.downloads", { count: tasks.length }) : displayName(first)}
      size="sm"
      onClose={onClose}
      footer={
        <>
          {tasks.some((x) => x.speed_limit !== null) && (
            <Button variant="ghost" className="mr-auto" onClick={() => onSave(null)}>
              {t("dialogs.speedLimit.remove")}
            </Button>
          )}
          <Button variant="ghost" onClick={onClose}>
            {t("common.cancel")}
          </Button>
          <Button variant="primary" onClick={() => submit()}>
            {t("dialogs.speedLimit.save")}
          </Button>
        </>
      }
    >
      <form onSubmit={submit} className="flex flex-col gap-3">
        <p className="text-[13px] text-ink-secondary">{body}</p>
        <label className="label" htmlFor="speed-limit-value">
          {t("dialogs.speedLimit.label")}
        </label>
        <RateFields
          id="speed-limit-value"
          draft={draft}
          onChange={setDraft}
          placeholder={t("dialogs.speedLimit.placeholder")}
          unitLabel={t("dialogs.speedLimit.unit")}
        />
        {tasks.some((x) => x.site !== null) && <p className="text-[12px] text-ink-muted">{t("dialogs.speedLimit.video")}</p>}
      </form>
    </Dialog>
  );
}
