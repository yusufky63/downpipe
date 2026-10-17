// Streams (HLS, DASH) in the UI: how qualities are named, which audio goes
// with which quality, and what a row says about a stream download.
import type { AudioOption, MediaChoices, MediaKind, MediaSelection, Task, VideoOption } from "./api";
import { formatBytes } from "./format";

const KIND_NAMES: Record<MediaKind, string> = { hls: "HLS", dash: "DASH" };

export function kindName(kind: MediaKind): string {
  return KIND_NAMES[kind];
}

/** "1080p", "720p 60", "Audio only" or "2.5 Mbit/s". Same rules as the engine's labels. */
export function qualityName(v: VideoOption): string {
  if (v.audio_only) return "Audio only";
  const fps = Number.parseFloat(v.frame_rate ?? "");
  const rate = Number.isFinite(fps) && fps > 30.5 ? ` ${Math.round(fps)}` : "";
  if (v.height) return `${v.height}p${rate}`;
  if (v.bandwidth) return `${(v.bandwidth / 1_000_000).toFixed(1)} Mbit/s`;
  return "Default";
}

/** The second line of a quality: resolution, bit rate and estimated size. */
export function qualityDetail(v: VideoOption): string {
  const parts: string[] = [];
  if (v.width && v.height) parts.push(`${v.width}×${v.height}`);
  if (v.bandwidth) parts.push(`${(v.bandwidth / 1_000_000).toFixed(1)} Mbit/s`);
  if (v.estimated_size) parts.push(`about ${formatBytes(v.estimated_size)}`);
  return parts.join(" · ");
}

/** One line for a select's option: name and details. */
export function qualityOption(v: VideoOption): string {
  const detail = qualityDetail(v);
  return detail ? `${qualityName(v)} · ${detail}` : qualityName(v);
}

const LANGUAGES = new Intl.DisplayNames(["en"], { type: "language" });

function languageName(code: string): string {
  try {
    return LANGUAGES.of(code) ?? code;
  } catch {
    return code;
  }
}

export function audioName(a: AudioOption): string {
  const name = a.name ?? (a.language ? languageName(a.language) : "Audio");
  const rate = a.bandwidth ? ` · ${Math.round(a.bandwidth / 1000)} kbit/s` : "";
  return `${name}${rate}`;
}

/** The audio tracks that can go with a quality; none when its sound is inside. */
export function audiosFor(options: MediaChoices["options"], video: VideoOption | undefined): AudioOption[] {
  if (video?.has_audio || video?.audio_only) return [];
  if (!video) return options.audios;
  return options.audios.filter((a) => !video.audio_group || a.group === video.audio_group);
}

/** The default audio of a list: the one the manifest marks, else the first. */
export function defaultAudio(audios: AudioOption[]): AudioOption | undefined {
  return audios.find((a) => a.default) ?? audios[0];
}

/**
 * What the dialog starts with: an earlier choice when it still exists
 * (same id, or the same height), else the best quality and the default audio.
 */
export function initialChoice(choices: MediaChoices): { videoId: string; audioId: string } {
  const { videos } = choices.options;
  const wanted = choices.selection?.video;
  const video =
    (wanted && (videos.find((v) => v.id === wanted.id) ?? videos.find((v) => wanted.height && v.height === wanted.height))) ??
    videos[0];
  const audios = audiosFor(choices.options, video);
  const wantedAudio = choices.selection?.audio;
  const audio = (wantedAudio && audios.find((a) => a.id === wantedAudio.id)) ?? defaultAudio(audios);
  return { videoId: video?.id ?? "", audioId: audio?.id ?? "" };
}

/** The selection to send for the ids picked in the dialog. */
export function selectionFor(choices: MediaChoices, videoId: string, audioId: string): MediaSelection {
  const video = choices.options.videos.find((v) => v.id === videoId);
  const audio = audiosFor(choices.options, video).find((a) => a.id === audioId);
  return {
    video: video ? { id: video.id, height: video.height, bandwidth: video.bandwidth } : undefined,
    audio: audio ? { id: audio.id, bandwidth: audio.bandwidth, language: audio.language } : undefined,
  };
}

/** Video and sound come as two tracks, so FFmpeg is needed for one file. */
export function needsMerge(choices: MediaChoices, videoId: string): boolean {
  const video = choices.options.videos.find((v) => v.id === videoId);
  return video !== undefined && audiosFor(choices.options, video).length > 0;
}

/** "HLS · 720p" for a list row, or null for a plain file. */
export function streamLabel(task: Task): string | null {
  if (!task.media) return null;
  const kind = kindName(task.media.kind);
  return task.media.quality ? `${kind} · ${task.media.quality}` : kind;
}

/**
 * A stream's size is an estimate until it finishes: fragments have no size
 * before they arrive.
 */
export function sizeIsEstimate(task: Task): boolean {
  return task.media !== null && task.state !== "completed";
}

/** Error codes the engine uses for streams it will not download. */
export const REFUSED_STREAM = new Set(["drm_protected", "live_stream", "unsupported_media", "invalid_manifest"]);
