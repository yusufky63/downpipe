import { FolderOpen } from "lucide-react";
import { useEffect, useEffectEvent, useId, useState, type FormEvent } from "react";

import { useI18n } from "../i18n";
import { api, errorMessage, isCommandError, onConfirmClose, type Choices, type ConfirmOptions, type SubtitleOption, type Queue, type Task } from "../lib/api";
import { describeError } from "../lib/errors";
import { withStop } from "../lib/format";
import {
  REFUSED_STREAM,
  audioName,
  audiosFor,
  initialChoice,
  kindName,
  needsMerge,
  qualityOption,
  selectionFor,
  subtitleFromKey,
  subtitleGroups,
  subtitleKey,
  subtitleName,
} from "../lib/media";
import { SITE_SETUP_KINDS, siteFailureText } from "../lib/siteErrors";
import { displayName, knownSize } from "../lib/tasks";
import { Banner, Button, Dialog, inputClass } from "./primitives";

/** What reading a stream's or a video page's qualities came to. */
type Inspection =
  | { state: "loading" }
  | { state: "ready"; choices: Choices }
  /**
   * `refused`: it cannot be downloaded at all (DRM, live, unavailable).
   * `setup`: yt-dlp is missing, which Settings can fix.
   * `detail`: yt-dlp's or the engine's own words (the site's reason, which
   * tool), kept beside the translated message.
   * `explained`: the message is the app's own text for a known yt-dlp
   * failure, which says what to do.
   */
  | { state: "failed"; message: string; detail: string | null; refused: boolean; setup: boolean; explained: boolean };

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
  const { t } = useI18n();
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
  // A subtitleKey, or empty for none.
  const [subtitle, setSubtitle] = useState("");
  const [subtitleOptions, setSubtitleOptions] = useState<SubtitleOption[]>([]);
  const ids = useId();

  useEffect(() => {
    if (!stream) return;
    let current = true;
    const read: Promise<Choices> = site
      ? api.inspectSite(task.id).then((choices) => {
          // The media name template, filled from the page, names the file
          // unless the user typed a name; the tab's title the browser sent
          // can be stale on a page that changes videos without loading.
          const named = choices.name ?? "";
          if (current && named) setName((now) => (now === initialName ? named : now));
          if (current) setSubtitleOptions(choices.subtitles);
          const chosen = choices.chosen_subtitles[0];
          if (current && chosen) setSubtitle(subtitleKey(chosen));
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
        const code = isCommandError(err) ? err.code : null;
        const refused = code !== null && REFUSED_STREAM.has(code);
        const setup = code !== null && SITE_SETUP_KINDS.has(code);
        // yt-dlp's common failures in the app's own words, with what to do.
        const explained = siteFailureText(code);
        const { text, detail } = describeError(err);
        setInspection({ state: "failed", message: explained ?? withStop(text), detail, refused, setup, explained: explained !== null });
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
        subtitles: site && choices ? [subtitleFromKey(subtitle)].filter((s) => s !== null) : null,
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

  const what = task.site ? task.site.site : task.media ? t("confirm.stream", { kind: kindName(task.media.kind) }) : null;
  const from = fromApp ? (what ?? t("confirm.newDownload")) : what ? t("confirm.fromBrowserWhat", { what }) : t("confirm.fromBrowser");
  // The setting is about the browser; a video added here always needs its quality chosen.
  const offerStop = !fromApp && onStopAsking !== undefined;
  const options = choices?.options;
  const size = knownSize(task.total, video?.estimated_size);
  const subtitles = site && choices ? subtitleGroups(subtitleOptions) : null;
  // Embedded in a video when FFmpeg is there; next to an audio file always.
  const embedded = choices?.ffmpeg === true && !(video?.audio_only ?? false);
  const pickQueue = queues.length > 1;
  const small = `${inputClass} h-8 text-[13px]`;

  return (
    <Dialog
      eyebrow={waiting > 0 ? t("confirm.moreWaiting", { from, count: waiting }) : from}
      title={stream ? t("confirm.titleVideo") : t("confirm.titleFile")}
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
              title={t("confirm.stopAskingTitle")}
            >
              <input type="checkbox" checked={stopAsking} onChange={(e) => setStopAsking(e.target.checked)} className="size-3.5 accent-primary" />
              {t("confirm.stopAsking")}
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
            {refused ? t("common.remove") : t("common.cancel")}
          </Button>
          <Button size="sm" onClick={() => void answer(false)} disabled={busy || refused}>
            {t("confirm.later")}
          </Button>
          <Button variant="primary" size="sm" onClick={() => void answer(true)} disabled={busy || refused || blocked}>
            {t("confirm.start")}
          </Button>
        </>
      }
    >
      <form onSubmit={(e) => void answer(true, e)} className="flex flex-col gap-2.5">
        {/* IDM's "Download File Info": one line per value, labels on the left. */}
        <div className="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-x-3 gap-y-2">
          <span className="label">{t("confirm.url")}</span>
          <span className="truncate font-mono text-[12px] text-ink-secondary select-text" title={task.url}>
            {task.url}
          </span>

          <label htmlFor={`${ids}-name`} className="label">
            {t("confirm.name")}
          </label>
          <div className="flex min-w-0 items-center gap-2">
            <input
              id={`${ids}-name`}
              value={name}
              onChange={(e) => setName(e.target.value)}
              spellCheck={false}
              title={
                site ? t("confirm.nameHint.site") : stream ? t("confirm.nameHint.stream") : t("confirm.nameHint.file")
              }
              className={`${small} min-w-0 flex-1`}
            />
            {size && <span className="num shrink-0 font-mono text-[12px] text-ink-secondary">{size}</span>}
          </div>

          <label htmlFor={`${ids}-dir`} className="label">
            {t("confirm.saveTo")}
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
              {t("common.browse")}
            </Button>
          </div>

          {(stream || pickQueue) && (
            <>
              <label htmlFor={stream ? `${ids}-quality` : `${ids}-queue`} className="label">
                {stream ? t("confirm.quality") : t("confirm.queue")}
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
                        <option value="">
                          {inspection.state === "failed" ? t("confirm.qualityBest") : site ? t("confirm.askingYtDlp") : t("confirm.readingStream")}
                        </option>
                      )}
                      {options?.videos.map((v) => (
                        <option key={v.id} value={v.id}>
                          {qualityOption(v)}
                        </option>
                      ))}
                      {options && options.videos.length === 0 && <option value="">{t("media.audioOnly")}</option>}
                    </select>
                  </div>
                )}
                {pickQueue && (
                  <div className={stream ? "w-28 shrink-0" : "min-w-0 flex-1"}>
                    <select
                      id={`${ids}-queue`}
                      aria-label={t("confirm.queue")}
                      title={t("confirm.queue")}
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

          {/* Its own row: next to quality and queue it would be cut to a word. */}
          {options && audios.length > 1 && (
            <>
              <label htmlFor={`${ids}-audio`} className="label">
                {t("confirm.audio")}
              </label>
              <select id={`${ids}-audio`} value={audioId} onChange={(e) => setAudioId(e.target.value)} className={small}>
                {audios.map((a) => (
                  <option key={a.id} value={a.id}>
                    {audioName(a)}
                  </option>
                ))}
              </select>
            </>
          )}
          {subtitles && subtitles.uploaded.length + subtitles.auto.length > 0 && (
            <>
              <label htmlFor={`${ids}-subtitles`} className="label">
                {t("media.subtitles.label")}
              </label>
              <select
                id={`${ids}-subtitles`}
                value={subtitle}
                onChange={(e) => setSubtitle(e.target.value)}
                title={subtitle ? t(embedded ? "media.subtitles.embedded" : "media.subtitles.separate") : undefined}
                className={small}
              >
                <option value="">{t("media.subtitles.none")}</option>
                {subtitles.uploaded.length > 0 && (
                  <optgroup label={t("media.subtitles.uploaded")}>
                    {subtitles.uploaded.map((s) => (
                      <option key={subtitleKey(s)} value={subtitleKey(s)}>
                        {subtitleName(s)}
                      </option>
                    ))}
                  </optgroup>
                )}
                {subtitles.auto.length > 0 && (
                  <optgroup label={t("media.subtitles.auto")}>
                    {subtitles.auto.map((s) => (
                      <option key={subtitleKey(s)} value={subtitleKey(s)}>
                        {subtitleName(s)}
                      </option>
                    ))}
                  </optgroup>
                )}
              </select>
            </>
          )}
        </div>
        {subtitle && (
          <p className="text-[12px] text-ink-secondary">{t(embedded ? "media.subtitles.embedded" : "media.subtitles.separate")}</p>
        )}
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
  const { t } = useI18n();
  if (inspection.state === "failed") {
    const detail = inspection.detail && (
      <span className="mt-1 block select-text break-words font-mono text-[12px] text-ink-secondary">{inspection.detail}</span>
    );
    if (inspection.refused) {
      return (
        <Banner tone="danger">
          {inspection.message}
          {detail}
        </Banner>
      );
    }
    if (inspection.setup) {
      return (
        <Banner tone="warning">
          <span>{inspection.message} </span>
          <button type="button" className={linkClass} onClick={onOpenSettings}>
            {t("confirm.openMediaSettings")}
          </button>
          {detail}
        </Banner>
      );
    }
    // A known failure says what to do itself.
    if (inspection.explained) {
      return (
        <Banner tone="warning">
          {inspection.message}
          {detail}
        </Banner>
      );
    }
    return (
      <Banner tone="warning">
        {t(site ? "confirm.readFailedPage" : "confirm.readFailedStream", { message: inspection.message })}
        {detail}
      </Banner>
    );
  }
  const choices = inspection.state === "ready" ? inspection.choices : null;
  if (!choices || choices.ffmpeg || !needsMerge(choices, videoId)) return null;
  if (!site) {
    return (
      <Banner tone="warning">
        <span>{t("confirm.noFfmpegStream")} </span>
        <button type="button" className={linkClass} onClick={onOpenSettings}>
          {t("confirm.setUpFfmpeg")}
        </button>
      </Banner>
    );
  }
  // The link sits wherever the language puts {link} in the sentence.
  const [before = "", after = ""] = t("confirm.noFfmpegSite").split("{link}");
  return (
    <Banner tone="danger">
      <span>{before}</span>
      <button type="button" className={linkClass} onClick={onOpenSettings}>
        {t("confirm.setUpFfmpegLink")}
      </button>
      <span>{after}</span>
    </Banner>
  );
}
