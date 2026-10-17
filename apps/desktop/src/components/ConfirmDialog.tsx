import { FolderOpen } from "lucide-react";
import { useEffect, useState, type FormEvent } from "react";

import { api, errorMessage, isCommandError, type ConfirmOptions, type MediaChoices, type Queue, type Task } from "../lib/api";
import { hostOf, sentence } from "../lib/format";
import { REFUSED_STREAM, audioName, audiosFor, initialChoice, kindName, needsMerge, qualityOption, selectionFor } from "../lib/media";
import { displayName } from "../lib/tasks";
import { Banner, Button, Dialog, Field, inputClass } from "./primitives";

/** What reading a stream's qualities came to. */
type Inspection =
  | { state: "loading" }
  | { state: "ready"; choices: MediaChoices }
  /** `refused`: the stream itself cannot be downloaded (DRM, live). */
  | { state: "failed"; message: string; refused: boolean };

/**
 * IDM's "Download File Info": a download caught in the browser waits here
 * for its name, folder and queue, and for a stream also its quality.
 * Nothing but a stream's manifest is fetched before the answer.
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
  const stream = task.media !== null;
  const [inspection, setInspection] = useState<Inspection>({ state: "loading" });
  const [videoId, setVideoId] = useState("");
  const [audioId, setAudioId] = useState("");

  useEffect(() => {
    if (!stream) return;
    let current = true;
    api
      .inspectMedia(task.id)
      .then((choices) => {
        if (!current) return;
        const initial = initialChoice(choices);
        setVideoId(initial.videoId);
        setAudioId(initial.audioId);
        setInspection({ state: "ready", choices });
      })
      .catch((err: unknown) => {
        if (!current) return;
        const refused = isCommandError(err) && REFUSED_STREAM.has(err.code);
        setInspection({ state: "failed", message: sentence(errorMessage(err)), refused });
      });
    return () => {
      current = false;
    };
  }, [stream, task.id]);

  const refused = inspection.state === "failed" && inspection.refused;
  const choices = inspection.state === "ready" ? inspection.choices : null;
  const video = choices?.options.videos.find((v) => v.id === videoId);
  const audios = choices ? audiosFor(choices.options, video) : [];

  const answer = async (start: boolean, e?: FormEvent) => {
    e?.preventDefault();
    if (refused) return;
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

  const from = stream && task.media ? `From the browser · ${kindName(task.media.kind)} stream` : "From the browser";

  return (
    <Dialog
      eyebrow={waiting > 0 ? `${from} · ${waiting} more waiting` : from}
      title={stream ? "Download this video?" : "Download this file?"}
      // Closing keeps it, paused, so a stray Escape loses nothing.
      onClose={() => (refused ? onDiscard() : void answer(false))}
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
            {refused ? "Remove" : "Cancel"}
          </Button>
          <Button onClick={() => void answer(false)} disabled={busy || refused}>
            Download later
          </Button>
          <Button variant="primary" onClick={() => void answer(true)} disabled={busy || refused}>
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
        {stream && (
          <StreamChoice
            inspection={inspection}
            videoId={videoId}
            audioId={audioId}
            audioCount={audios.length}
            onVideo={(id) => {
              setVideoId(id);
              // The audio list depends on the quality: keep the language if it is there.
              if (!choices) return;
              const next = audiosFor(choices.options, choices.options.videos.find((v) => v.id === id));
              const current = audios.find((a) => a.id === audioId);
              const same = next.find((a) => a.language === current?.language) ?? next.find((a) => a.default) ?? next[0];
              setAudioId(same?.id ?? "");
            }}
            onAudio={setAudioId}
          />
        )}
        <Field
          label="File name"
          error={error}
          hint={
            stream
              ? "The stream decides the extension: .mp4 in most cases, .ts or .webm for some."
              : "The server may suggest another name; this one is kept."
          }
        >
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

/** The quality and audio of a stream, once its manifest is read. */
function StreamChoice({
  inspection,
  videoId,
  audioId,
  audioCount,
  onVideo,
  onAudio,
}: {
  inspection: Inspection;
  videoId: string;
  audioId: string;
  audioCount: number;
  onVideo: (id: string) => void;
  onAudio: (id: string) => void;
}) {
  if (inspection.state === "failed") {
    return inspection.refused ? (
      <Banner tone="danger">{inspection.message}</Banner>
    ) : (
      <Banner tone="warning">Could not read the stream: {inspection.message} It is downloaded in the best quality when it starts.</Banner>
    );
  }
  const choices = inspection.state === "ready" ? inspection.choices : null;
  const options = choices?.options;
  const separate = choices !== null && needsMerge(choices, videoId);
  return (
    <div className="flex flex-col gap-3">
      <Field label="Quality">
        {(id) => (
          <select
            id={id}
            value={videoId}
            disabled={!options || options.videos.length < 2}
            onChange={(e) => onVideo(e.target.value)}
            className={`${inputClass} h-9 disabled:opacity-70`}
          >
            {!options && <option value="">Reading the stream…</option>}
            {options?.videos.map((v) => (
              <option key={v.id} value={v.id}>
                {qualityOption(v)}
              </option>
            ))}
            {options && options.videos.length === 0 && <option value="">Audio only</option>}
          </select>
        )}
      </Field>
      {options && audioCount > 1 && (
        <Field label="Audio">
          {(id) => (
            <select id={id} value={audioId} onChange={(e) => onAudio(e.target.value)} className={`${inputClass} h-9`}>
              {audiosFor(options, options.videos.find((v) => v.id === videoId)).map((a) => (
                <option key={a.id} value={a.id}>
                  {audioName(a)}
                </option>
              ))}
            </select>
          )}
        </Field>
      )}
      {separate && !choices.ffmpeg && (
        <Banner tone="warning">
          Video and sound come separately and FFmpeg was not found, so they are saved as two files. Settings, Media explains how to
          have them merged.
        </Banner>
      )}
    </div>
  );
}
