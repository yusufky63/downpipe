import { FolderOpen } from "lucide-react";
import { useState, type FormEvent } from "react";

import { api, errorMessage, isCommandError, type Settings, type Task } from "../lib/api";
import { parseUrls } from "../lib/format";
import { Button, Dialog, Field, inputClass } from "./primitives";

export function AddDialog({
  initialText,
  settings,
  onClose,
  onAdded,
  onDuplicate,
}: {
  initialText: string;
  settings: Settings;
  onClose: () => void;
  onAdded: (tasks: Task[]) => void;
  onDuplicate: (id: string) => void;
}) {
  const [text, setText] = useState(initialText);
  const [dir, setDir] = useState(settings.download_dir);
  const [fileName, setFileName] = useState("");
  const [connections, setConnections] = useState(String(settings.max_connections_per_download));
  const [checksum, setChecksum] = useState("");
  const [paused, setPaused] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const urls = parseUrls(text);
  // Per-file options make sense for zero (not typed yet) or one link.
  const single = urls.length <= 1;

  const browse = async () => {
    const picked = await api.pickFolder(dir || null).catch(() => null);
    if (picked) setDir(picked);
  };

  const submit = async (e?: FormEvent) => {
    e?.preventDefault();
    if (urls.length === 0) {
      setError("Paste at least one http or https link.");
      return;
    }
    const conns = Number.parseInt(connections, 10);
    setBusy(true);
    setError(null);
    const added: Task[] = [];
    const problems: string[] = [];
    for (const url of urls) {
      try {
        added.push(
          await api.add({
            url,
            dir: dir.trim() || null,
            file_name: single && fileName.trim() ? fileName.trim() : null,
            checksum: single && checksum.trim() ? checksum.trim() : null,
            max_connections: Number.isFinite(conns) ? conns : null,
            paused,
          }),
        );
      } catch (err) {
        if (isCommandError(err) && err.code === "duplicate" && err.id && urls.length === 1) {
          onDuplicate(err.id);
          return;
        }
        problems.push(urls.length > 1 ? `${url}: ${errorMessage(err)}` : errorMessage(err));
      }
    }
    setBusy(false);
    if (added.length > 0) onAdded(added);
    if (problems.length > 0) setError(problems.join("\n"));
    else onClose();
  };

  return (
    <Dialog
      eyebrow="New download"
      title={urls.length > 1 ? `Add ${urls.length} links` : "Add a link"}
      onClose={onClose}
      footer={
        <>
          <label className="mr-auto flex items-center gap-2 text-[13px] text-ink-secondary">
            <input type="checkbox" checked={paused} onChange={(e) => setPaused(e.target.checked)} className="size-4 accent-primary" />
            Add paused
          </label>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" onClick={() => void submit()} disabled={busy || urls.length === 0}>
            {paused ? "Add to list" : urls.length > 1 ? `Download ${urls.length} files` : "Download"}
          </Button>
        </>
      }
    >
      <form onSubmit={submit} className="flex flex-col gap-5">
        <Field
          label="Links"
          error={error}
          hint={urls.length > 1 ? `${urls.length} links found. One download each.` : "One link per line. Only http and https are supported."}
        >
          {(id) => (
            <textarea
              id={id}
              value={text}
              onChange={(e) => setText(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) void submit();
              }}
              rows={3}
              spellCheck={false}
              placeholder="https://example.org/file.iso"
              className={`${inputClass} resize-y py-2 font-mono text-[13px]`}
            />
          )}
        </Field>

        <Field label="Save to">
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

        <div className="grid grid-cols-1 gap-5 sm:grid-cols-[1fr_120px]">
          <Field label="File name" hint={single ? "Leave empty to use the name the server sends." : "Only for a single link."}>
            {(id) => (
              <input
                id={id}
                value={fileName}
                disabled={!single}
                onChange={(e) => setFileName(e.target.value)}
                spellCheck={false}
                placeholder="Automatic"
                className={`${inputClass} h-9 disabled:opacity-50`}
              />
            )}
          </Field>
          <Field label="Connections" hint="Upper limit">
            {(id) => (
              <input
                id={id}
                type="number"
                min={1}
                max={32}
                value={connections}
                onChange={(e) => setConnections(e.target.value)}
                className={`${inputClass} num h-9 font-mono`}
              />
            )}
          </Field>
        </div>

        <Field label="Checksum" hint="Optional, e.g. sha256:9f86d0… The file is only renamed into place if it matches.">
          {(id) => (
            <input
              id={id}
              value={checksum}
              disabled={!single}
              onChange={(e) => setChecksum(e.target.value)}
              spellCheck={false}
              placeholder="sha256:…"
              className={`${inputClass} h-9 font-mono text-[13px] disabled:opacity-50`}
            />
          )}
        </Field>
      </form>
    </Dialog>
  );
}
