import { useState, type FormEvent } from "react";

import { useI18n } from "../i18n";
import type { Task } from "../lib/api";
import { displayName } from "../lib/tasks";
import { fromLocalInput, toLocalInput } from "../lib/until";
import { Button, Dialog, Field, inputClass } from "./primitives";

/** "Pause until" with a time of the user's choice; the presets need no dialog. */
export function PauseUntilDialog({
  tasks,
  onClose,
  onPause,
}: {
  tasks: Task[];
  onClose: () => void;
  /** The chosen time as a date in the future. */
  onPause: (at: Date) => void;
}) {
  const { t } = useI18n();
  // An hour from now, on the minute: a sensible start to adjust.
  const [value, setValue] = useState(() => toLocalInput(new Date(Date.now() + 60 * 60_000)));
  const [error, setError] = useState<string | null>(null);
  const first = tasks[0];

  const submit = (e?: FormEvent) => {
    e?.preventDefault();
    const at = fromLocalInput(value);
    if (at === null) {
      setError(t("dialogs.pauseUntil.invalid"));
      return;
    }
    if (at.getTime() <= Date.now()) {
      setError(t("dialogs.pauseUntil.past"));
      return;
    }
    onPause(at);
  };

  return (
    <Dialog
      eyebrow={t("dialogs.pauseUntil.eyebrow")}
      title={tasks.length > 1 || !first ? t("common.downloads", { count: tasks.length }) : displayName(first)}
      size="sm"
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t("common.cancel")}
          </Button>
          <Button variant="primary" onClick={() => submit()}>
            {t("dialogs.pauseUntil.submit")}
          </Button>
        </>
      }
    >
      <form onSubmit={submit}>
        <Field label={t("dialogs.pauseUntil.label")} hint={t("dialogs.pauseUntil.hint")} error={error}>
          {(id) => (
            <input
              id={id}
              type="datetime-local"
              value={value}
              onChange={(e) => {
                setValue(e.target.value);
                setError(null);
              }}
              className={`${inputClass} num h-9 font-mono`}
            />
          )}
        </Field>
      </form>
    </Dialog>
  );
}
