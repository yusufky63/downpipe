import type { Task } from "../lib/api";
import { displayName } from "../lib/tasks";
import { Button, Dialog } from "./primitives";

export function RemoveDialog({
  task,
  onClose,
  onConfirm,
}: {
  task: Task;
  onClose: () => void;
  onConfirm: (deleteFiles: boolean) => void;
}) {
  const done = task.state === "completed";
  return (
    <Dialog
      eyebrow="Remove"
      title={displayName(task)}
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Keep
          </Button>
          <Button variant="danger" onClick={() => onConfirm(true)}>
            {done ? "Remove and delete file" : "Remove and delete partial data"}
          </Button>
          <Button variant="primary" onClick={() => onConfirm(false)}>
            Remove from list
          </Button>
        </>
      }
    >
      <p className="text-[14px] text-ink-secondary">
        {done
          ? "Removing it from the list keeps the file on disk. Deleting also removes the file itself."
          : "Removing it from the list keeps the partial file, so the same link can still resume later. Deleting removes the partial data too."}
      </p>
    </Dialog>
  );
}
