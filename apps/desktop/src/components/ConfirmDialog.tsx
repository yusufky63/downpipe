import { FolderOpen } from "lucide-react";
import { useState, type FormEvent } from "react";

import { api, errorMessage, type ConfirmOptions, type Queue, type Task } from "../lib/api";
import { hostOf } from "../lib/format";
import { displayName } from "../lib/tasks";
import { Button, Dialog, Field, inputClass } from "./primitives";

/**
 * IDM's "Download File Info": a download caught in the browser waits here
 * for its name, folder and queue. Nothing is fetched before the answer.
 */
export function ConfirmDialog({
  task,
  waiting,
  queues,
  onConfirm,
  onDiscard,
  onStopAsking,
}: {
  task: Task;
  /** More downloads waiting after this one. */
  waiting: number;
  queues: Queue[];
  onConfirm: (options: ConfirmOptions) => Promise<void>;
  /** Cancel: remove it from the list. */
  onDiscard: () => void;
  onStopAsking: () => void;
}) {
  const [name, setName] = useState(displayName(task));
  const [dir, setDir] = useState(task.dir);
  const [queue, setQueue] = useState(task.queue);
  const [stopAsking, setStopAsking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const answer = async (start: boolean, e?: FormEvent) => {
    e?.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const trimmed = name.trim();
      await onConfirm({
        file_name: trimmed && trimmed !== displayName(task) ? trimmed : null,
        // Unchanged keeps the automatic folder, which may still follow the category.
        dir: dir.trim() && dir.trim() !== task.dir ? dir.trim() : null,
        queue: queue !== task.queue ? queue : null,
        start,
      });
      if (stopAsking) onStopAsking();
    } catch (err) {
      setError(errorMessage(err));
      setBusy(false);
    }
  };

  const browse = async () => {
    const picked = await api.pickFolder(dir || null).catch(() => null);
    if (picked) setDir(picked);
  };

  return (
    <Dialog
      eyebrow={waiting > 0 ? `From the browser · ${waiting} more waiting` : "From the browser"}
      title="Download this file?"
      // Closing keeps it, paused, so a stray Escape loses nothing.
      onClose={() => void answer(false)}
      footer={
        <>
          <label className="mr-auto flex items-center gap-2 text-[13px] text-ink-secondary">
            <input type="checkbox" checked={stopAsking} onChange={(e) => setStopAsking(e.target.checked)} className="size-4 accent-primary" />
            Don&apos;t ask again
          </label>
          <Button
            variant="ghost"
            disabled={busy}
            onClick={() => {
              if (stopAsking) onStopAsking();
              onDiscard();
            }}
          >
            Cancel
          </Button>
          <Button onClick={() => void answer(false)} disabled={busy}>
            Download later
          </Button>
          <Button variant="primary" onClick={() => void answer(true)} disabled={busy}>
            Start download
          </Button>
        </>
      }
    >
      <form onSubmit={(e) => void answer(true, e)} className="flex flex-col gap-5">
        <div className="flex min-w-0 flex-col gap-1 rounded-md border border-line bg-surface px-3 py-2">
          <span className="label">{hostOf(task.url)}</span>
          <span className="truncate font-mono text-[12px] text-ink-secondary select-text" title={task.url}>
            {task.url}
          </span>
        </div>
        <Field label="File name" error={error} hint="The server may suggest another name; this one is kept.">
          {(id) => (
            <input
              id={id}
              value={name}
              onChange={(e) => setName(e.target.value)}
              spellCheck={false}
              className={`${inputClass} h-9`}
            />
          )}
        </Field>
        <Field label="Save to" hint={dir === task.dir && task.category ? `The ${task.category} folder.` : undefined}>
          {(id) => (
            <div className="flex gap-2">
              <input id={id} value={dir} onChange={(e) => setDir(e.target.value)} spellCheck={false} className={`${inputClass} h-9 font-mono text-[13px]`} />
              <Button onClick={() => void browse()}>
                <FolderOpen size={16} strokeWidth={1.75} />
                Browse
              </Button>
            </div>
          )}
        </Field>
        {queues.length > 1 && (
          <Field label="Queue">
            {(id) => (
              <select id={id} value={queue} onChange={(e) => setQueue(e.target.value)} className={`${inputClass} h-9`}>
                {queues.map((q) => (
                  <option key={q.id} value={q.id}>
                    {q.name}
                  </option>
                ))}
              </select>
            )}
          </Field>
        )}
      </form>
    </Dialog>
  );
}
