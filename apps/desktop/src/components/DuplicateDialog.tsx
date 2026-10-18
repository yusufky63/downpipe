import { useId, useState } from "react";

import { useI18n } from "../i18n";
import type { Task } from "../lib/api";
import type { DuplicateChoice } from "../lib/duplicates";
import { displayName, stateInfo } from "../lib/tasks";
import { Button, Dialog } from "./primitives";

/**
 * Asked when a link that is already in the list is added again, from the
 * Add dialog or from the browser. `existing` is the download already there
 * (the first one when several links were added at once).
 */
export function DuplicateDialog({
  existing,
  count = 1,
  inWindow = false,
  onFit,
  onChoose,
  onCancel,
}: {
  existing: Task;
  /** Links that were already in the list; the answer applies to all of them. */
  count?: number;
  /** The whole confirmation window rather than a dialog in the main window. */
  inWindow?: boolean;
  onFit?: (height: number) => void;
  /** `remember`: "Don't ask again" was ticked, so the choice becomes the setting. */
  onChoose: (choice: DuplicateChoice, remember: boolean) => void;
  onCancel: () => void;
}) {
  const { t } = useI18n();
  const [choice, setChoice] = useState<DuplicateChoice>("copy");
  const [remember, setRemember] = useState(false);
  const group = useId();
  // Replacing a file only makes sense once there is one.
  const canOverwrite = existing.state === "completed";
  const options: { id: DuplicateChoice; label: string; disabled?: boolean }[] = [
    { id: "copy", label: t("dialogs.duplicate.copy") },
    { id: "overwrite", label: t("dialogs.duplicate.overwrite"), disabled: !canOverwrite },
    { id: "existing", label: t("dialogs.duplicate.existing") },
  ];
  return (
    <Dialog
      eyebrow={t("dialogs.duplicate.eyebrow")}
      title={count > 1 ? t("dialogs.duplicate.titleMany", { count }) : displayName(existing)}
      frame={inWindow ? "window" : "modal"}
      size="sm"
      compact
      onFit={onFit}
      onClose={onCancel}
      footer={
        <>
          <Button variant="ghost" size="sm" onClick={onCancel}>
            {t("common.cancel")}
          </Button>
          <Button variant="primary" size="sm" onClick={() => onChoose(choice, remember)}>
            {t("dialogs.duplicate.ok")}
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-3">
        <p className="text-[13px] text-ink-secondary">
          {count > 1 ? t("dialogs.duplicate.bodyMany", { count }) : t("dialogs.duplicate.body", { state: stateInfo(existing).label })}
        </p>
        <fieldset className="flex flex-col gap-2">
          <legend className="sr-only">{t("dialogs.duplicate.question")}</legend>
          {options.map((o) => (
            <label key={o.id} className={`flex items-start gap-2 text-[13px] ${o.disabled ? "opacity-50" : "cursor-pointer text-ink"}`}>
              <input
                type="radio"
                name={group}
                value={o.id}
                checked={choice === o.id}
                disabled={o.disabled}
                onChange={() => setChoice(o.id)}
                className="mt-0.5 size-4 accent-primary"
              />
              {o.label}
            </label>
          ))}
        </fieldset>
        <label className="flex cursor-pointer items-start gap-2 border-t border-line pt-3 text-[13px] text-ink-secondary">
          <input
            type="checkbox"
            checked={remember}
            onChange={(e) => setRemember(e.target.checked)}
            className="mt-0.5 size-4 accent-primary"
          />
          <span className="flex flex-col gap-0.5">
            {t("dialogs.duplicate.remember")}
            <span className="text-[12px] text-ink-muted">{t("dialogs.duplicate.rememberHint")}</span>
          </span>
        </label>
      </div>
    </Dialog>
  );
}
