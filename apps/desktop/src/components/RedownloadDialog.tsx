import { useState } from "react";

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
  const [replace, setReplace] = useState(false);
  return (
    <Dialog
      eyebrow="Download again"
      title={displayName(task)}
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button variant={replace ? "danger" : "primary"} onClick={() => onConfirm(replace)}>
            {replace ? "Delete and download again" : "Download again"}
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-4">
        <p className="text-[14px] text-ink-secondary">
          Downloads the file again from the same link into the same folder, for example when it changed on the server or the copy on disk is damaged.
        </p>
        <label className="flex cursor-pointer items-start gap-2.5 rounded-md border border-line p-3">
          <input type="checkbox" checked={replace} onChange={(e) => setReplace(e.target.checked)} className="mt-0.5 size-4 accent-primary" />
          <span className="flex flex-col gap-1">
            <span className="text-[14px] text-ink">Replace the existing file</span>
            <span className="text-[12px] text-ink-muted">
              {replace
                ? "The current file is deleted first, and the new one takes its name."
                : "Off: the current file stays, and the new copy is saved next to it under a free name."}
            </span>
          </span>
        </label>
      </div>
    </Dialog>
  );
}
