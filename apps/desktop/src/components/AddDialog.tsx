import { FileText, FolderOpen } from "lucide-react";
import { useRef, useState, type DragEvent, type FormEvent } from "react";

import { MAIN_QUEUE, api, errorMessage, isCommandError, type Settings, type Task } from "../lib/api";
import { parseUrls } from "../lib/format";
import { MAX_EXPANDED, expandPatterns, linksFromList } from "../lib/patterns";
import { sitePage } from "../lib/sites";
import { Button, Dialog, Field, inputClass } from "./primitives";

/** The Links hint's word on the video pages among several links. */
function pagesNote(pages: number, files: number): string {
  if (pages === 0) return "";
  if (files === 0) return " You choose the quality of each video next.";
  return pages === 1 ? " One of them is a video: you choose its quality next." : ` ${pages} of them are videos: you choose their quality next.`;
}

export function AddDialog({
  text,
  onTextChange,
  settings,
  onClose,
  onAdded,
  onDuplicate,
}: {
  /** Kept by the app, so a link copied while the dialog is open joins the list. */
  text: string;
  onTextChange: (text: string) => void;
  settings: Settings;
  onClose: () => void;
  onAdded: (tasks: Task[]) => void;
  onDuplicate: (id: string) => void;
}) {
  // Empty means automatic: the default folder, or the category's folder.
  const [dir, setDir] = useState("");
  const [fileName, setFileName] = useState("");
  const [connections, setConnections] = useState(String(settings.max_connections_per_download));
  const [checksum, setChecksum] = useState("");
  const [paused, setPaused] = useState(false);
  const [queue, setQueue] = useState(MAIN_QUEUE);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const expansion = expandPatterns(text);
  const urls = parseUrls(expansion.urls.join("\n"));
  // Video pages (YouTube) are not started from here: each goes on to the
  // confirmation dialog, where yt-dlp lists its qualities.
  const pages = urls.filter((url) => sitePage(url) !== null).length;
  const onlyPages = pages > 0 && pages === urls.length;
  const fileInput = useRef<HTMLInputElement>(null);

  // A dropped or chosen list file is appended, so several lists can be combined.
  const appendList = async (file: File) => {
    if (file.size > 2 * 1024 * 1024) {
      setError("That file is too large for a list of links.");
      return;
    }
    const links = linksFromList(await file.text());
    onTextChange([text.trim(), links].filter(Boolean).join("\n"));
  };

  const onDropFile = (e: DragEvent<HTMLTextAreaElement>) => {
    const file = e.dataTransfer.files[0];
    if (!file) return;
    e.preventDefault();
    e.stopPropagation();
    void appendList(file);
  };
  // Per-file options make sense for zero (not typed yet) or one link.
  const single = urls.length <= 1;
  // yt-dlp builds a video's file itself: no connections or checksum to set.
  const firstPage = single && urls[0] !== undefined ? sitePage(urls[0]) : null;
  const video = firstPage !== null;

  const browse = async () => {
    const picked = await api.pickFolder(dir || settings.download_dir).catch(() => null);
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
      const page = sitePage(url) !== null;
      try {
        added.push(
          await api.add({
            url,
            dir: dir.trim() || null,
            file_name: single && fileName.trim() ? fileName.trim() : null,
            checksum: single && !page && checksum.trim() ? checksum.trim() : null,
            max_connections: Number.isFinite(conns) && !page ? conns : null,
            paused,
            queue,
            confirm: page,
            // A video page then asks for its quality here, not in the confirmation window.
            from_app: true,
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

  const files = urls.length - pages;
  const linksHint =
    expansion.dropped > 0
      ? `Only the first ${MAX_EXPANDED} links of the pattern are used.`
      : expansion.expanded
        ? `${urls.length} links from the pattern. One download each.`
        : urls.length > 1
          ? `${urls.length} links found. One download each.`
          : "One link per line, or a pattern like file[001-100].zip. Only http and https are supported.";

  return (
    <Dialog
      eyebrow="New download"
      title={urls.length > 1 ? `Add ${urls.length} links` : "Add a link"}
      onClose={onClose}
      footer={
        <>
          {/* Videos are held for their quality anyway, with Download later there. */}
          {!onlyPages && (
            <label className="mr-auto flex items-center gap-2 text-[13px] text-ink-secondary">
              <input type="checkbox" checked={paused} onChange={(e) => setPaused(e.target.checked)} className="size-4 accent-primary" />
              Add paused
            </label>
          )}
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" onClick={() => void submit()} disabled={busy || urls.length === 0}>
            {onlyPages ? "Next" : paused ? "Add to list" : files > 1 ? `Download ${files} files` : "Download"}
          </Button>
        </>
      }
    >
      <form onSubmit={submit} className="flex flex-col gap-5">
        <Field label="Links" error={error} hint={firstPage ? `A ${firstPage.site} video. You choose its quality next.` : linksHint + pagesNote(pages, files)}>
          {(id) => (
            <textarea
              id={id}
              value={text}
              onChange={(e) => onTextChange(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) void submit();
              }}
              onDrop={onDropFile}
              rows={3}
              spellCheck={false}
              placeholder="https://example.org/file.iso"
              className={`${inputClass} resize-y py-2 font-mono text-[13px]`}
            />
          )}
        </Field>

        <div className="-mt-3 flex items-center gap-2">
          <Button size="sm" variant="ghost" onClick={() => fileInput.current?.click()}>
            <FileText size={16} strokeWidth={1.75} />
            Import a list
          </Button>
          <span className="text-[12px] text-ink-muted">A .txt file with one link per line. You can also drop it here.</span>
          <input
            ref={fileInput}
            type="file"
            accept=".txt,text/plain"
            className="hidden"
            onChange={(e) => {
              const file = e.target.files?.[0];
              if (file) void appendList(file);
              e.target.value = "";
            }}
          />
        </div>

        <Field
          label="Save to"
          hint={dir.trim() ? undefined : settings.use_categories ? "Automatic: the folder of the file's category." : undefined}
        >
          {(id) => (
            <div className="flex gap-2">
              <input
                id={id}
                value={dir}
                placeholder={settings.download_dir}
                onChange={(e) => setDir(e.target.value)}
                spellCheck={false}
                className={`${inputClass} h-9 font-mono text-[13px]`}
              />
              <Button onClick={() => void browse()}>
                <FolderOpen size={16} strokeWidth={1.75} />
                Browse
              </Button>
            </div>
          )}
        </Field>

        <div className="grid grid-cols-1 gap-5 sm:grid-cols-[1fr_120px]">
          <Field
            label="File name"
            hint={video ? "Leave empty to use the video's title." : single ? "Leave empty to use the name the server sends." : "Only for a single link."}
          >
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
          <Field label="Connections" hint={video ? "Not for videos" : "Upper limit"}>
            {(id) => (
              <input
                id={id}
                type="number"
                min={1}
                max={32}
                value={connections}
                disabled={video}
                onChange={(e) => setConnections(e.target.value)}
                className={`${inputClass} num h-9 font-mono disabled:opacity-50`}
              />
            )}
          </Field>
        </div>

        {settings.queues.length > 1 && (
          <Field label="Queue" hint="Each queue runs its own downloads at once.">
            {(id) => (
              <select id={id} value={queue} onChange={(e) => setQueue(e.target.value)} className={`${inputClass} h-9`}>
                {settings.queues.map((q) => (
                  <option key={q.id} value={q.id}>
                    {q.name}
                  </option>
                ))}
              </select>
            )}
          </Field>
        )}

        <Field
          label="Checksum"
          hint={
            video
              ? "Only for files: yt-dlp puts a video's file together from the site's formats."
              : "Optional, e.g. sha256:9f86d0… The file is only renamed into place if it matches."
          }
        >
          {(id) => (
            <input
              id={id}
              value={checksum}
              disabled={!single || video}
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
