import { useI18n } from "../i18n";
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
  const { t } = useI18n();
  const one = tasks.length === 1 ? tasks[0] : undefined;
  const allDone = tasks.every((task) => task.state === "completed");
  const anyDone = tasks.some((task) => task.state === "completed");
  const deleteLabel = one
    ? allDone
      ? t("dialogs.remove.deleteFile")
      : t("dialogs.remove.deletePartial")
    : anyDone
      ? t("dialogs.remove.deleteFiles")
      : t("dialogs.remove.deletePartial");
  return (
    <Dialog
      eyebrow={t("common.remove")}
      title={one ? displayName(one) : t("common.downloads", { count: tasks.length })}
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t("common.keep")}
          </Button>
          <Button variant="danger" onClick={() => onConfirm(true)}>
            {deleteLabel}
          </Button>
          <Button variant="primary" onClick={() => onConfirm(false)}>
            {t("dialogs.remove.fromList")}
          </Button>
        </>
      }
    >
      <p className="text-[14px] text-ink-secondary">
        {allDone
          ? one
            ? t("dialogs.remove.doneOne")
            : t("dialogs.remove.doneMany")
          : one
            ? t("dialogs.remove.partialOne")
            : t("dialogs.remove.partialMany")}
      </p>
      {!one && (
        <ul className="mt-4 flex max-h-40 flex-col gap-1 overflow-y-auto rounded-md border border-line px-3 py-2">
          {tasks.map((task) => (
            <li key={task.id} className="truncate font-mono text-[12px] text-ink-secondary" title={displayName(task)}>
              {displayName(task)}
            </li>
          ))}
        </ul>
      )}
    </Dialog>
  );
}
