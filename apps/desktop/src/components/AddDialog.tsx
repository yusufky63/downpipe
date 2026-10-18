import { FileText, FolderOpen } from "lucide-react";
import { useRef, useState, type DragEvent, type FormEvent } from "react";

import { useI18n } from "../i18n";
import type { Translate } from "../i18n/core";
import { MAIN_QUEUE, api, errorMessage, isCommandError, type AddOptions, type Settings, type Task } from "../lib/api";

/** A link that was already in the list, and how it was to be added. */
export interface Duplicate {
  options: AddOptions;
  /** The download already in the list. */
  existing: string;
}
import { nameFromUrl, parseUrls } from "../lib/format";
import { MAX_EXPANDED, expandPatterns, linksFromList } from "../lib/patterns";
import { categoryExtracts, inheritedCommand, isArchive } from "../lib/post";
import { sitePage } from "../lib/sites";
import { PostFields } from "./PostFields";
import { Button, Dialog, Field, inputClass } from "./primitives";

/** The Links hint's word on the video pages among several links. */
function pagesNote(t: Translate, pages: number, files: number): string {
  if (pages === 0) return "";
  if (files === 0) return t("add.links.allVideos");
  return t("add.links.someVideos", { count: pages });
}

export function AddDialog({
  text,
  onTextChange,
  settings,
  onClose,
  onAdded,
  onDuplicates,
}: {
  /** Kept by the app, so a link copied while the dialog is open joins the list. */
  text: string;
  onTextChange: (text: string) => void;
  settings: Settings;
  onClose: () => void;
  onAdded: (tasks: Task[]) => void;
  /** Links already in the list, with what was asked, so the app can ask what to do. */
  onDuplicates: (items: Duplicate[]) => void;
}) {
  const { t } = useI18n();
  // Empty means automatic: the default folder, or the category's folder.
  const [dir, setDir] = useState("");
  const [fileName, setFileName] = useState("");
  const [connections, setConnections] = useState(String(settings.max_connections_per_download));
  const [checksum, setChecksum] = useState("");
  const [paused, setPaused] = useState(false);
  const [queue, setQueue] = useState(MAIN_QUEUE);
  const [command, setCommand] = useState("");
  // Null until the box is touched: the category then decides, also later.
  const [extract, setExtract] = useState<boolean | null>(null);
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
      setError(t("add.errors.listTooLarge"));
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
  // The names known now; the server may still name the file differently.
  const names = single && fileName.trim() ? [fileName.trim()] : urls.map(nameFromUrl);
  const archives = names.filter(isArchive);
  const extractShown = extract ?? archives.some((name) => categoryExtracts(settings, name));

  const browse = async () => {
    const picked = await api.pickFolder(dir || settings.download_dir).catch(() => null);
    if (picked) setDir(picked);
  };

  const submit = async (e?: FormEvent) => {
    e?.preventDefault();
    if (urls.length === 0) {
      setError(t("add.errors.noLinks"));
      return;
    }
    const conns = Number.parseInt(connections, 10);
    setBusy(true);
    setError(null);
    const added: Task[] = [];
    const problems: string[] = [];
    const duplicates: Duplicate[] = [];
    for (const url of urls) {
      const page = sitePage(url) !== null;
      const options: AddOptions = {
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
        post: { command: command.trim(), extract },
      };
      try {
        added.push(await api.add(options));
      } catch (err) {
        if (isCommandError(err) && err.code === "duplicate" && err.id) {
          duplicates.push({ options, existing: err.id });
          continue;
        }
        problems.push(urls.length > 1 ? t("add.errors.forLink", { url, error: errorMessage(err) }) : errorMessage(err));
      }
    }
    setBusy(false);
    if (added.length > 0) onAdded(added);
    if (duplicates.length > 0) onDuplicates(duplicates);
    if (problems.length > 0) setError(problems.join("\n"));
    else onClose();
  };

  const files = urls.length - pages;
  const linksHint =
    expansion.dropped > 0
      ? t("add.links.patternCapped", { count: MAX_EXPANDED })
      : expansion.expanded
        ? t("add.links.fromPattern", { count: urls.length })
        : urls.length > 1
          ? t("add.links.found", { count: urls.length })
          : t("add.links.hint");
  // A sentence of its own, joined here so neither text has to carry the space.
  const note = pagesNote(t, pages, files);

  return (
    <Dialog
      eyebrow={t("add.eyebrow")}
      title={urls.length > 1 ? t("add.titleMany", { count: urls.length }) : t("add.title")}
      onClose={onClose}
      footer={
        <>
          {/* Videos are held for their quality anyway, with Download later there. */}
          {!onlyPages && (
            <label className="mr-auto flex items-center gap-2 text-[13px] text-ink-secondary">
              <input type="checkbox" checked={paused} onChange={(e) => setPaused(e.target.checked)} className="size-4 accent-primary" />
              {t("add.addPaused")}
            </label>
          )}
          <Button variant="ghost" onClick={onClose}>
            {t("common.cancel")}
          </Button>
          <Button variant="primary" onClick={() => void submit()} disabled={busy || urls.length === 0}>
            {onlyPages
              ? t("add.submit.next")
              : paused
                ? t("add.submit.addToList")
                : files > 1
                  ? t("add.submit.downloadMany", { count: files })
                  : t("add.submit.download")}
          </Button>
        </>
      }
    >
      <form onSubmit={submit} className="flex flex-col gap-5">
        <Field
          label={t("add.links.label")}
          error={error}
          hint={firstPage ? t("add.links.video", { site: firstPage.site }) : note ? `${linksHint} ${note}` : linksHint}
        >
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
            {t("add.links.importList")}
          </Button>
          <span className="text-[12px] text-ink-muted">{t("add.links.importHint")}</span>
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
          label={t("add.saveTo.label")}
          hint={dir.trim() ? undefined : settings.use_categories ? t("add.saveTo.automatic") : undefined}
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
                {t("common.browse")}
              </Button>
            </div>
          )}
        </Field>

        <div className="grid grid-cols-1 gap-5 sm:grid-cols-[1fr_120px]">
          <Field
            label={t("add.fileName.label")}
            hint={video ? t("add.fileName.videoHint") : single ? t("add.fileName.hint") : t("add.fileName.singleOnly")}
          >
            {(id) => (
              <input
                id={id}
                value={fileName}
                disabled={!single}
                onChange={(e) => setFileName(e.target.value)}
                spellCheck={false}
                placeholder={t("add.fileName.placeholder")}
                className={`${inputClass} h-9 disabled:opacity-50`}
              />
            )}
          </Field>
          <Field label={t("add.connections.label")} hint={video ? t("add.connections.videoHint") : t("add.connections.hint")}>
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
          <Field label={t("add.queue.label")} hint={t("add.queue.hint")}>
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
          label={t("add.checksum.label")}
          hint={video ? t("add.checksum.videoHint") : t("add.checksum.hint")}
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

        <PostFields
          command={command}
          onCommand={setCommand}
          inherited={inheritedCommand(settings, queue)}
          archive={archives.length > 0}
          extract={extractShown}
          onExtract={setExtract}
        />
      </form>
    </Dialog>
  );
}
