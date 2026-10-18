import { useState } from "react";

import { useI18n } from "../i18n";
import type { Task } from "../lib/api";
import { displayName } from "../lib/tasks";
import { Button, Dialog } from "./primitives";

/** "Download again" for a finished file, from the same link. */
export function RedownloadDialog({
  task,
  onClose,
  onConfirm,
}: {
  task: Task;
  onClose: () => void;
  onConfirm: (replace: boolean) => void;
}) {
  const { t } = useI18n();
  const [replace, setReplace] = useState(false);
  return (
    <Dialog
      eyebrow={t("dialogs.redownload.eyebrow")}
      title={displayName(task)}
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t("common.cancel")}
          </Button>
          <Button variant={replace ? "danger" : "primary"} onClick={() => onConfirm(replace)}>
            {replace ? t("dialogs.redownload.confirmReplace") : t("dialogs.redownload.confirm")}
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-4">
        <p className="text-[14px] text-ink-secondary">{t("dialogs.redownload.body")}</p>
        <label className="flex cursor-pointer items-start gap-2.5 rounded-md border border-line p-3">
          <input type="checkbox" checked={replace} onChange={(e) => setReplace(e.target.checked)} className="mt-0.5 size-4 accent-primary" />
          <span className="flex flex-col gap-1">
            <span className="text-[14px] text-ink">{t("dialogs.redownload.replace")}</span>
            <span className="text-[12px] text-ink-muted">
              {replace ? t("dialogs.redownload.replaceOn") : t("dialogs.redownload.replaceOff")}
            </span>
          </span>
        </label>
      </div>
    </Dialog>
  );
}
