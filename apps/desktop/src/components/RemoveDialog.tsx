import type { Task } from "../lib/api";
import { displayName } from "../lib/tasks";
import { Button, Dialog } from "./primitives";

export function RemoveDialog({
  tasks,
  onClose,
  onConfirm,
}: {
  tasks: Task[];
  onClose: () => void;
  onConfirm: (deleteFiles: boolean) => void;
}) {
  const one = tasks.length === 1 ? tasks[0] : undefined;
  const allDone = tasks.every((t) => t.state === "completed");
  const anyDone = tasks.some((t) => t.state === "completed");
  const deleteLabel = one
    ? allDone
      ? "Remove and delete file"
      : "Remove and delete partial data"
    : anyDone
      ? "Remove and delete files"
      : "Remove and delete partial data";
  return (
    <Dialog
      eyebrow="Remove"
      title={one ? displayName(one) : `${tasks.length} downloads`}
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Keep
          </Button>
          <Button variant="danger" onClick={() => onConfirm(true)}>
            {deleteLabel}
          </Button>
          <Button variant="primary" onClick={() => onConfirm(false)}>
            Remove from list
          </Button>
        </>
      }
    >
      <p className="text-[14px] text-ink-secondary">
        {allDone
          ? one
            ? "Removing it from the list keeps the file on disk. Deleting also removes the file itself."
            : "Removing them from the list keeps the files on disk. Deleting also removes the files themselves."
          : one
            ? "Removing it from the list keeps the partial file, so the same link can still resume later. Deleting removes the partial data too."
            : "Removing them from the list keeps finished files and partial data on disk. Deleting removes both."}
      </p>
      {!one && (
        <ul className="mt-4 flex max-h-40 flex-col gap-1 overflow-y-auto rounded-md border border-line px-3 py-2">
          {tasks.map((t) => (
            <li key={t.id} className="truncate font-mono text-[12px] text-ink-secondary" title={displayName(t)}>
              {displayName(t)}
            </li>
          ))}
        </ul>
      )}
    </Dialog>
  );
}
