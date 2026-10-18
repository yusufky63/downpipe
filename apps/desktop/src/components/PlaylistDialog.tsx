import { FolderOpen, Search } from "lucide-react";
import { useEffect, useMemo, useRef, useState, type KeyboardEvent, type MouseEvent } from "react";

import { useI18n } from "../i18n";
import { MAIN_QUEUE, api, errorMessage, isCommandError, type AddListResult, type ListChoices, type Preset, type Settings, type SubtitleRef } from "../lib/api";
import { formatDuration, sentence, withStop } from "../lib/format";
import { languageName } from "../lib/media";
import { SITE_SETUP_KINDS, siteFailureText } from "../lib/siteErrors";
import { Banner, Button, Dialog, Field, inputClass } from "./primitives";

/** What reading the list came to. */
type Reading =
  | { state: "loading" }
  | { state: "ready"; list: ListChoices }
  /** `code` is the app's error code; `message` its own words. */
  | { state: "failed"; code: string | null; message: string };

const PRESETS: Preset[] = ["best", "1080p", "720p", "audio"];

/**
 * Subtitle languages offered for a whole list, whose videos' languages are
 * not read: YouTube's codes for widely used languages.
 */
const SUBTITLE_LANGUAGES = ["ar", "de", "en", "es", "fr", "hi", "id", "it", "ja", "ko", "nl", "pl", "pt", "pt-BR", "ru", "tr", "uk", "vi", "zh-Hans"];

/** Rows drawn at once: a list of thousands stays quick to scroll and tick. */
const ROW_HEIGHT = 32;
const VISIBLE_ROWS = 9;
const OVERSCAN = 6;

/**
 * A YouTube playlist or channel: its videos (read by yt-dlp from the list
 * pages only) with check boxes, one quality and subtitle choice for all, and
 * a folder named after the list. The chosen videos go to a queue, each as a
 * video page of its own.
 */
export function PlaylistDialog({
  url,
  settings,
  onClose,
  onAdded,
  onOpenSettings,
}: {
  url: string;
  settings: Settings;
  onClose: () => void;
  onAdded: (result: AddListResult) => void;
  onOpenSettings: () => void;
}) {
  const { t } = useI18n();
  const [reading, setReading] = useState<Reading>({ state: "loading" });
  const [selected, setSelected] = useState<ReadonlySet<string>>(new Set());
  const [filter, setFilter] = useState("");
  const [preset, setPreset] = useState<Preset>("best");
  const [subtitle, setSubtitle] = useState("");
  const [autoCaptions, setAutoCaptions] = useState(true);
  const [folder, setFolder] = useState("");
  const [dir, setDir] = useState("");
  const [queue, setQueue] = useState(MAIN_QUEUE);
  const [paused, setPaused] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const anchor = useRef<number | null>(null);

  useEffect(() => {
    let current = true;
    api
      .inspectList(url)
      .then((list) => {
        if (!current) return;
        setSelected(new Set(list.entries.filter((e) => e.available).map((e) => e.id)));
        setFolder(list.title ?? "");
        setDir(list.dir);
        setReading({ state: "ready", list });
      })
      .catch((err: unknown) => {
        if (!current) return;
        setReading({ state: "failed", code: isCommandError(err) ? err.code : null, message: withStop(sentence(errorMessage(err))) });
      });
    return () => {
      current = false;
    };
  }, [url]);

  const list = reading.state === "ready" ? reading.list : null;
  const entries = useMemo(() => list?.entries ?? [], [list]);
  const needle = filter.trim().toLocaleLowerCase();
  const shown = useMemo(
    () => (needle ? entries.filter((e) => (e.title ?? "").toLocaleLowerCase().includes(needle)) : entries),
    [entries, needle],
  );
  const available = useMemo(() => shown.filter((e) => e.available), [shown]);
  const chosen = useMemo(() => entries.filter((e) => selected.has(e.id)), [entries, selected]);

  const first = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN);
  const last = Math.min(shown.length, Math.ceil((scrollTop + VISIBLE_ROWS * ROW_HEIGHT) / ROW_HEIGHT) + OVERSCAN);

  const setMany = (videoIds: string[], on: boolean) =>
    setSelected((now) => {
      const next = new Set(now);
      for (const id of videoIds) {
        if (on) next.add(id);
        else next.delete(id);
      }
      return next;
    });

  // Shift-click ticks or unticks everything between the last row and this one.
  const toggle = (index: number, e: MouseEvent | KeyboardEvent) => {
    const entry = shown[index];
    if (!entry?.available) return;
    const on = !selected.has(entry.id);
    if (e.shiftKey && anchor.current !== null) {
      const [from, to] = anchor.current < index ? [anchor.current, index] : [index, anchor.current];
      setMany(
        shown
          .slice(from, to + 1)
          .filter((v) => v.available)
          .map((v) => v.id),
        on,
      );
    } else {
      setMany([entry.id], on);
    }
    anchor.current = index;
  };

  const browse = async () => {
    const picked = await api.pickFolder(dir || settings.download_dir).catch(() => null);
    if (picked) setDir(picked);
  };

  const submit = async () => {
    if (!list || chosen.length === 0 || busy) return;
    setBusy(true);
    setError(null);
    const subtitles: SubtitleRef[] = subtitle
      ? [{ language: subtitle, auto: false }, ...(autoCaptions ? [{ language: subtitle, auto: true }] : [])]
      : [];
    try {
      const result = await api.addList({
        url: list.url,
        title: folder.trim() || null,
        entries: chosen.map((e) => ({ id: e.id, title: e.title, index: e.index })),
        count: entries.length,
        preset,
        subtitles,
        dir: dir.trim() && dir.trim() !== list.dir ? dir.trim() : null,
        queue,
        paused,
      });
      onAdded(result);
      onClose();
    } catch (err) {
      setError(errorMessage(err));
      setBusy(false);
    }
  };

  const eyebrow = list ? t(list.kind === "channel" ? "playlist.eyebrow.channel" : "playlist.eyebrow.playlist", { site: list.site }) : "YouTube";
  // Named in the language the window shows.
  const languages = SUBTITLE_LANGUAGES.map((code) => ({ code, name: languageName(code) })).sort((a, b) => a.name.localeCompare(b.name));
  const failure =
    reading.state === "failed"
      ? {
          // yt-dlp's common failures in the user's language, with what to do.
          message:
            reading.code === "unavailable" ? t("playlist.unavailable") : (siteFailureText(reading.code) ?? reading.message),
          setup: reading.code !== null && SITE_SETUP_KINDS.has(reading.code),
        }
      : null;
  const small = `${inputClass} h-8 text-[13px]`;

  return (
    <Dialog
      eyebrow={eyebrow}
      title={list?.title ?? t("playlist.untitled")}
      size="lg"
      onClose={onClose}
      footer={
        <>
          <label className="mr-auto flex items-center gap-2 text-[13px] text-ink-secondary">
            <input type="checkbox" checked={paused} onChange={(e) => setPaused(e.target.checked)} className="size-4 accent-primary" />
            {t("add.addPaused")}
          </label>
          <Button variant="ghost" onClick={onClose}>
            {t("common.cancel")}
          </Button>
          <Button variant="primary" onClick={() => void submit()} disabled={busy || chosen.length === 0}>
            {paused ? t("playlist.addToList", { count: chosen.length }) : t("playlist.submit", { count: chosen.length })}
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-4">
        {reading.state === "loading" && <p className="text-[13px] text-ink-secondary">{t("playlist.reading")}</p>}
        {failure && (
          <Banner tone={failure.setup ? "warning" : "danger"}>
            <span>{failure.message} </span>
            {failure.setup && (
              <button type="button" className="font-medium text-primary underline-offset-2 hover:underline" onClick={onOpenSettings}>
                {t("confirm.openMediaSettings")}
              </button>
            )}
          </Banner>
        )}

        {list && entries.length === 0 && <Banner tone="warning">{t("playlist.empty")}</Banner>}
        {list && entries.length > 0 && (
          <>
            <div className="flex flex-wrap items-center gap-2">
              {/* A row of its own when the dialog is narrow. */}
              <div className="relative min-w-0 flex-1 basis-full sm:basis-0">
                <Search size={14} strokeWidth={1.75} className="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-ink-muted" />
                <input
                  type="search"
                  value={filter}
                  onChange={(e) => {
                    setFilter(e.target.value);
                    setScrollTop(0);
                    anchor.current = null;
                  }}
                  placeholder={t("playlist.filter")}
                  aria-label={t("playlist.filter")}
                  spellCheck={false}
                  className={`${small} pl-8`}
                />
              </div>
              <Button size="sm" variant="ghost" onClick={() => setMany(available.map((e) => e.id), true)}>
                {t("playlist.selectAll")}
              </Button>
              <Button size="sm" variant="ghost" onClick={() => setMany(shown.map((e) => e.id), false)}>
                {t("playlist.selectNone")}
              </Button>
              <span className="num font-mono text-[12px] text-ink-secondary">
                {t("playlist.selected", { count: chosen.length, total: entries.length })}
              </span>
            </div>

            {/* Only the rows in view are drawn; the spacer keeps the scroll bar true. */}
            <div
              role="listbox"
              aria-multiselectable="true"
              aria-label={t("playlist.rowsLabel")}
              tabIndex={-1}
              onScroll={(e) => setScrollTop(e.currentTarget.scrollTop)}
              className="relative overflow-y-auto rounded-md border border-line"
              style={{ height: VISIBLE_ROWS * ROW_HEIGHT }}
            >
              <div style={{ height: shown.length * ROW_HEIGHT }}>
                {shown.slice(first, last).map((entry, i) => {
                  const index = first + i;
                  const on = selected.has(entry.id);
                  return (
                    <div
                      key={entry.id}
                      role="option"
                      aria-selected={on}
                      aria-disabled={!entry.available}
                      tabIndex={entry.available ? 0 : -1}
                      onClick={(e) => toggle(index, e)}
                      onKeyDown={(e) => {
                        if (e.key === " " || e.key === "Enter") {
                          e.preventDefault();
                          toggle(index, e);
                        }
                      }}
                      title={entry.title ?? undefined}
                      className={`absolute inset-x-0 flex items-center gap-2.5 border-b border-line px-3 text-[13px] select-none ${
                        entry.available ? "cursor-pointer hover:bg-surface-muted" : "text-ink-muted"
                      }`}
                      style={{ top: index * ROW_HEIGHT, height: ROW_HEIGHT }}
                    >
                      <input
                        type="checkbox"
                        checked={on}
                        disabled={!entry.available}
                        readOnly
                        tabIndex={-1}
                        aria-hidden="true"
                        className="pointer-events-none size-4 shrink-0 accent-primary"
                      />
                      <span className="num w-10 shrink-0 text-right font-mono text-[12px] text-ink-muted">{entry.index}</span>
                      <span className="min-w-0 flex-1 truncate">{entry.title ?? t("playlist.untitledVideo")}</span>
                      {!entry.available && <span className="shrink-0 text-[12px]">{t("playlist.notAvailable")}</span>}
                      {entry.duration_ms !== null && (
                        <span className="num shrink-0 font-mono text-[12px] text-ink-secondary">{formatDuration(Math.round(entry.duration_ms / 1000))}</span>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
            {list.truncated && <p className="text-[12px] text-ink-secondary">{t("playlist.truncated", { count: entries.length })}</p>}

            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
              <Field label={t("playlist.quality")} hint={t("playlist.qualityHint")}>
                {(id) => (
                  <select id={id} value={preset} onChange={(e) => setPreset(e.target.value as Preset)} className={`${inputClass} h-9`}>
                    {PRESETS.map((p) => (
                      <option key={p} value={p}>
                        {t(`playlist.presets.${p}`)}
                      </option>
                    ))}
                  </select>
                )}
              </Field>
              <Field
                label={t("media.subtitles.label")}
                hint={subtitle ? t(list.ffmpeg && preset !== "audio" ? "media.subtitles.embedded" : "media.subtitles.separate") : undefined}
              >
                {(id) => (
                  <select id={id} value={subtitle} onChange={(e) => setSubtitle(e.target.value)} className={`${inputClass} h-9`}>
                    <option value="">{t("media.subtitles.none")}</option>
                    {languages.map((l) => (
                      <option key={l.code} value={l.code}>
                        {l.name}
                      </option>
                    ))}
                  </select>
                )}
              </Field>
            </div>
            {subtitle && (
              <label className="-mt-2 flex items-center gap-2 text-[13px] text-ink-secondary">
                <input type="checkbox" checked={autoCaptions} onChange={(e) => setAutoCaptions(e.target.checked)} className="size-4 accent-primary" />
                {t("playlist.autoCaptions")}
              </label>
            )}
            {!list.ffmpeg && preset !== "audio" && (
              <Banner tone="warning">
                <span>{t("playlist.noFfmpeg")} </span>
                <button type="button" className="font-medium text-primary underline-offset-2 hover:underline" onClick={onOpenSettings}>
                  {t("confirm.setUpFfmpeg")}
                </button>
              </Banner>
            )}

            <div className="grid grid-cols-1 gap-4 sm:grid-cols-[1fr_1.4fr]">
              <Field label={t("playlist.folder")} hint={t("playlist.folderHint")}>
                {(id) => (
                  <input id={id} value={folder} onChange={(e) => setFolder(e.target.value)} spellCheck={false} className={`${inputClass} h-9`} />
                )}
              </Field>
              <Field label={t("playlist.saveIn")}>
                {(id) => (
                  <div className="flex gap-2">
                    <input
                      id={id}
                      value={dir}
                      onChange={(e) => setDir(e.target.value)}
                      spellCheck={false}
                      title={dir}
                      className={`${inputClass} h-9 min-w-0 font-mono text-[13px]`}
                    />
                    <Button onClick={() => void browse()}>
                      <FolderOpen size={16} strokeWidth={1.75} />
                      {t("common.browse")}
                    </Button>
                  </div>
                )}
              </Field>
            </div>

            {settings.queues.length > 1 && (
              <Field label={t("add.queue.label")}>
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
          </>
        )}
        {error && <p className="text-[13px] text-danger-fg">{error}</p>}
      </div>
    </Dialog>
  );
}
