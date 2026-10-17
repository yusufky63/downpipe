import { FolderOpen } from "lucide-react";
import { useEffect, useEffectEvent, useId, useState, type FormEvent } from "react";

import { api, errorMessage, isCommandError, onConfirmClose, type Choices, type ConfirmOptions, type Queue, type Task } from "../lib/api";
import { sentence } from "../lib/format";
import { REFUSED_STREAM, audioName, audiosFor, initialChoice, kindName, needsMerge, qualityOption, selectionFor } from "../lib/media";
import { displayName, knownSize } from "../lib/tasks";
import { Banner, Button, Dialog, inputClass } from "./primitives";

/** What reading a stream's or a video page's qualities came to. */
type Inspection =
  | { state: "loading" }
  | { state: "ready"; choices: Choices }
  /**
   * `refused`: it cannot be downloaded at all (DRM, live, unavailable).
   * `setup`: yt-dlp is missing, which Settings can fix.
   */
  | { state: "failed"; message: string; refused: boolean; setup: boolean };

const UNSAFE_IN_NAMES = '<>:"/\\|?*';

/**
 * A page title as a file name, made safe the way the app does it (the app
 * checks it again); yt-dlp replaces the extension with the real one.
 */
function titleName(title: string): string {
  const safe = [...title]
    .map((ch) => ((ch.codePointAt(0) ?? 0) < 0x20 || UNSAFE_IN_NAMES.includes(ch) ? "_" : ch))
    .join("")
    .trim();
  return safe ? `${safe}.mp4` : "";
}

/**
 * IDM's "Download File Info": a download caught in the browser waits here
 * for its name, folder and queue, and for a stream also its quality, in the
 * confirmation window over the browser. A video page added in Downpipe
 * itself waits here too, for its quality, in the main window.
 * Nothing but a stream's manifest (or yt-dlp's reading of the page) is
 * fetched before the answer.
 */
export function ConfirmDialog({
  task,
  waiting,
  queues,
  inWindow = false,
  onFit,
  onConfirm,
  onDiscard,
  onStopAsking,
  onOpenSettings,
}: {
  task: Task;
  /** More downloads waiting after this one. */
  waiting: number;
  queues: Queue[];
  /** The whole confirmation window rather than a dialog in the main window. */
  inWindow?: boolean;
  /** In the window: the height it needs. */
  onFit?: (height: number) => void;
  onConfirm: (options: ConfirmOptions) => Promise<void>;
  /** Cancel: remove it from the list. */
  onDiscard: () => void;
  /** "Don't ask again", offered for downloads from the browser. */
  onStopAsking?: () => void;
  /** Settings, Media: where yt-dlp and FFmpeg are set up. */
  onOpenSettings: () => void;
}) {
  // Added in Downpipe (the Add dialog and the like), not sent by the browser.
  const fromApp = task.from_app;
  const [name, setName] = useState(displayName(task));
  const [dir, setDir] = useState(task.dir);
  const [queue, setQueue] = useState(task.queue);
  const [stopAsking, setStopAsking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  // A video page (YouTube) is chosen like a stream, with yt-dlp's formats.
  const site = task.site !== null;
  const stream = task.media !== null || site;
  const initialName = displayName(task);
  const [inspection, setInspection] = useState<Inspection>({ state: "loading" });
  const [videoId, setVideoId] = useState("");
  const [audioId, setAudioId] = useState("");
  const ids = useId();

  useEffect(() => {
    if (!stream) return;
    let current = true;
    const read: Promise<Choices> = site
      ? api.inspectSite(task.id).then((choices) => {
          // The page's real title names the file unless the user typed one;
          // the tab's title the browser sent can be stale on a page that
          // changes videos without loading.
          const title = choices.title ? titleName(choices.title) : "";
          if (current && title) setName((now) => (now === initialName ? title : now));
          return choices;
        })
      : api.inspectMedia(task.id);
    read
      .then((choices) => {
        if (!current) return;
        // Without FFmpeg a page's separate video and sound cannot be joined.
        const initial = initialChoice(choices, { avoidMerge: site && !choices.ffmpeg });
        setVideoId(initial.videoId);
        setAudioId(initial.audioId);
        setInspection({ state: "ready", choices });
      })
      .catch((err: unknown) => {
        if (!current) return;
        const refused = isCommandError(err) && REFUSED_STREAM.has(err.code);
        const setup = isCommandError(err) && err.code === "tool_missing";
        setInspection({ state: "failed", message: sentence(errorMessage(err)), refused, setup });
      });
    return () => {
      current = false;
    };
  }, [stream, site, task.id, initialName]);

  const refused = inspection.state === "failed" && inspection.refused;
  const choices = inspection.state === "ready" ? inspection.choices : null;
  const video = choices?.options.videos.find((v) => v.id === videoId);
  const audios = choices ? audiosFor(choices.options, video) : [];
  // yt-dlp cannot save separate video and sound without FFmpeg; a stream's
  // tracks are then saved as two files instead.
  // Started before yt-dlp answers, a page would download in its best
  // quality, which can be a very large 4K file: Start waits for the list
  // (or for yt-dlp to fail, when the best quality is all there is).
  const blocked =
    (site && inspection.state === "loading") || (site && choices !== null && !choices.ffmpeg && needsMerge(choices, videoId));

  const answer = async (start: boolean, e?: FormEvent) => {
    e?.preventDefault();
    if (busy || refused || (start && blocked)) return;
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
        // Without the stream's qualities the best one is taken when it starts.
        media: choices ? selectionFor(choices, videoId, audioId) : null,
      });
      if (stopAsking) onStopAsking?.();
    } catch (err) {
      setError(errorMessage(err));
      setBusy(false);
    }
  };

  const chooseVideo = (id: string) => {
    setVideoId(id);
    // The audio list depends on the quality: keep the language if it is there.
    if (!choices) return;
    const next = audiosFor(choices.options, choices.options.videos.find((v) => v.id === id));
    const current = audios.find((a) => a.id === audioId);
    const same = next.find((a) => a.language === current?.language) ?? next.find((a) => a.default) ?? next[0];
    setAudioId(same?.id ?? "");
  };

  const browse = async () => {
    const picked = await api.pickFolder(dir || null).catch(() => null);
    if (picked) setDir(picked);
  };

  // Closing keeps it, paused, so a stray Escape loses nothing.
  const close = () => (refused ? onDiscard() : void answer(false));
  // The confirmation window's own close button means the same as Escape.
  const closeFromWindow = useEffectEvent(close);
  useEffect(() => {
    if (!inWindow) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void onConfirmClose(() => closeFromWindow()).then((fn) => (disposed ? fn() : (unlisten = fn)));
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [inWindow]);

  const what = task.site ? task.site.site : task.media ? `${kindName(task.media.kind)} stream` : null;
  const from = fromApp ? (what ?? "New download") : what ? `From the browser · ${what}` : "From the browser";
  // The setting is about the browser; a video added here always needs its quality chosen.
  const offerStop = !fromApp && onStopAsking !== undefined;
  const options = choices?.options;
  const size = knownSize(task.total, video?.estimated_size);
  const pickQueue = queues.length > 1;
  const small = `${inputClass} h-8 text-[13px]`;

  return (
    <Dialog
      eyebrow={waiting > 0 ? `${from} · ${waiting} more waiting` : from}
      title={stream ? "Download this video?" : "Download this file?"}
      frame={inWindow ? "window" : "modal"}
      size="sm"
      compact
      onFit={onFit}
      onClose={close}
      footer={
        <>
          {offerStop && (
            <label
              className="mr-auto flex items-center gap-1.5 text-[12px] text-ink-secondary"
              title="Downloads from the browser then start at once. Settings, App turns the question back on."
            >
              <input type="checkbox" checked={stopAsking} onChange={(e) => setStopAsking(e.target.checked)} className="size-3.5 accent-primary" />
              Don&apos;t ask again
            </label>
          )}
          <Button
            variant="ghost"
            size="sm"
            disabled={busy}
            onClick={() => {
              if (stopAsking) onStopAsking?.();
              onDiscard();
            }}
          >
            {refused ? "Remove" : "Cancel"}
          </Button>
          <Button size="sm" onClick={() => void answer(false)} disabled={busy || refused}>
            Download later
          </Button>
          <Button variant="primary" size="sm" onClick={() => void answer(true)} disabled={busy || refused || blocked}>
            Start download
          </Button>
        </>
      }
    >
      <form onSubmit={(e) => void answer(true, e)} className="flex flex-col gap-2.5">
        {/* IDM's "Download File Info": one line per value, labels on the left. */}
        <div className="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-x-3 gap-y-2">
          <span className="label">URL</span>
          <span className="truncate font-mono text-[12px] text-ink-secondary select-text" title={task.url}>
            {task.url}
          </span>

          <label htmlFor={`${ids}-name`} className="label">
            Name
          </label>
          <div className="flex min-w-0 items-center gap-2">
            <input
              id={`${ids}-name`}
              value={name}
              onChange={(e) => setName(e.target.value)}
              spellCheck={false}
              title={
                site
                  ? "yt-dlp decides the extension: .mp4 in most cases, .mkv or .webm for some formats, .m4a for audio only."
                  : stream
                    ? "The stream decides the extension: .mp4 in most cases, .ts or .webm for some."
                    : "The server may suggest another name; this one is kept."
              }
              className={`${small} min-w-0 flex-1`}
            />
            {size && <span className="num shrink-0 font-mono text-[12px] text-ink-secondary">{size}</span>}
          </div>

          <label htmlFor={`${ids}-dir`} className="label">
            Save to
          </label>
          <div className="flex min-w-0 gap-2">
            <input
              id={`${ids}-dir`}
              value={dir}
              onChange={(e) => setDir(e.target.value)}
              spellCheck={false}
              title={dir}
              className={`${small} min-w-0 flex-1 font-mono text-[12px]`}
            />
            <Button size="sm" onClick={() => void browse()}>
              <FolderOpen size={14} strokeWidth={1.75} />
              Browse
            </Button>
          </div>

          {(stream || pickQueue) && (
            <>
              <label htmlFor={stream ? `${ids}-quality` : `${ids}-queue`} className="label">
                {stream ? "Quality" : "Queue"}
              </label>
              {/* Widths on wrappers: the shared input class sets its own. */}
              <div className="flex min-w-0 items-center gap-2">
                {stream && (
                  <div className="min-w-0 flex-[2]">
                    <select
                      id={`${ids}-quality`}
                      value={videoId}
                      disabled={!options || options.videos.length < 2}
                      onChange={(e) => chooseVideo(e.target.value)}
                      className={`${small} disabled:opacity-70`}
                    >
                      {!options && (
                        <option value="">{inspection.state === "failed" ? "Best" : site ? "Asking yt-dlp…" : "Reading the stream…"}</option>
                      )}
                      {options?.videos.map((v) => (
                        <option key={v.id} value={v.id}>
                          {qualityOption(v)}
                        </option>
                      ))}
                      {options && options.videos.length === 0 && <option value="">Audio only</option>}
                    </select>
                  </div>
                )}
                {options && audios.length > 1 && (
                  <div className="min-w-0 flex-1">
                    <select
                      aria-label="Audio"
                      title="Audio"
                      value={audioId}
                      onChange={(e) => setAudioId(e.target.value)}
                      className={small}
                    >
                      {audios.map((a) => (
                        <option key={a.id} value={a.id}>
                          {audioName(a)}
                        </option>
                      ))}
                    </select>
                  </div>
                )}
                {pickQueue && (
                  <div className={stream ? "w-28 shrink-0" : "min-w-0 flex-1"}>
                    <select
                      id={`${ids}-queue`}
                      aria-label="Queue"
                      title="Queue"
                      value={queue}
                      onChange={(e) => setQueue(e.target.value)}
                      className={small}
                    >
                      {queues.map((q) => (
                        <option key={q.id} value={q.id}>
                          {q.name}
                        </option>
                      ))}
                    </select>
                  </div>
                )}
              </div>
            </>
          )}
        </div>
        {stream && <StreamNotice inspection={inspection} site={site} videoId={videoId} onOpenSettings={onOpenSettings} />}
        {error && <p className="text-[12px] text-danger-fg">{error}</p>}
      </form>
    </Dialog>
  );
}

const linkClass = "font-medium text-primary underline-offset-2 hover:underline";

/** What keeps a stream or a video page from downloading as chosen, if anything. */
function StreamNotice({
  inspection,
  site,
  videoId,
  onOpenSettings,
}: {
  inspection: Inspection;
  site: boolean;
  videoId: string;
  onOpenSettings: () => void;
}) {
  if (inspection.state === "failed") {
    if (inspection.refused) return <Banner tone="danger">{inspection.message}</Banner>;
    if (inspection.setup) {
      return (
        <Banner tone="warning">
          <span>{inspection.message} </span>
          <button type="button" className={linkClass} onClick={onOpenSettings}>
            Open Settings, Media
          </button>
        </Banner>
      );
    }
    return (
      <Banner tone="warning">
        Could not read the {site ? "page" : "stream"}: {inspection.message} It is downloaded in the best quality when it starts.
      </Banner>
    );
  }
  const choices = inspection.state === "ready" ? inspection.choices : null;
  if (!choices || choices.ffmpeg || !needsMerge(choices, videoId)) return null;
  if (!site) {
    return (
      <Banner tone="warning">
        <span>No FFmpeg found, so video and sound are saved as two files. </span>
        <button type="button" className={linkClass} onClick={onOpenSettings}>
          Set up FFmpeg
        </button>
      </Banner>
    );
  }
  return (
    <Banner tone="danger">
      <span>This quality needs FFmpeg to join video and sound, and none was found. Pick Audio only or an H.264 + AAC quality, or </span>
      <button type="button" className={linkClass} onClick={onOpenSettings}>
        set up FFmpeg
      </button>
      <span>.</span>
    </Banner>
  );
}
