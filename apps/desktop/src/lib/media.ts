// Streams (HLS, DASH) and video pages (YouTube, through yt-dlp) in the UI:
// how qualities are named, which audio goes with which quality, and what a
// row says about such a download.
import type { AudioOption, ChoiceOptions, Choices, MediaKind, MediaSelection, Task, VideoOption } from "./api";
import { formatBytes } from "./format";

const KIND_NAMES: Record<MediaKind, string> = { hls: "HLS", dash: "DASH" };

export function kindName(kind: MediaKind): string {
  return KIND_NAMES[kind];
}

/** "HLS", "DASH" or the site ("YouTube"); null for a plain file. */
export function sourceName(task: Task): string | null {
  if (task.site) return task.site.site;
  if (task.media) return kindName(task.media.kind);
  return null;
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

const CODEC_NAMES: [prefix: string, name: string][] = [
  ["avc", "H.264"],
  ["h264", "H.264"],
  ["hev1", "H.265"],
  ["hvc1", "H.265"],
  ["av01", "AV1"],
  ["vp09", "VP9"],
  ["vp9", "VP9"],
  ["vp8", "VP8"],
  ["mp4a.40.34", "MP3"],
  ["mp4a", "AAC"],
  ["aac", "AAC"],
  ["opus", "Opus"],
  ["vorbis", "Vorbis"],
  ["ac-3", "AC-3"],
  ["ec-3", "E-AC-3"],
  ["flac", "FLAC"],
  ["mp3", "MP3"],
];

/**
 * "H.264 + AAC" from "avc1.4d401e, mp4a.40.2": the formats in words, so a
 * quality with its own sound can be told from one that joins a separate
 * track. Codecs it does not know are left out.
 */
export function codecName(codecs: string | undefined): string {
  const names = (codecs ?? "")
    .split(",")
    .map((c) => c.trim().toLowerCase())
    .flatMap((c) => {
      const found = CODEC_NAMES.find(([prefix]) => c.startsWith(prefix));
      return found ? [found[1]] : [];
    });
  return [...new Set(names)].join(" + ");
}

/** The second line of a quality: resolution, codecs, bit rate and estimated size. */
export function qualityDetail(v: VideoOption): string {
  const parts: string[] = [];
  if (v.width && v.height) parts.push(`${v.width}×${v.height}`);
  const codec = codecName(v.codecs);
  if (codec) parts.push(codec);
  if (v.bandwidth) {
    parts.push(v.audio_only ? `${Math.round(v.bandwidth / 1000)} kbit/s` : `${(v.bandwidth / 1_000_000).toFixed(1)} Mbit/s`);
  }
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
  const codec = codecName(a.codecs);
  const rate = a.bandwidth ? ` · ${Math.round(a.bandwidth / 1000)} kbit/s` : "";
  return `${name}${codec ? ` · ${codec}` : ""}${rate}`;
}

/** The audio tracks that can go with a quality; none when its sound is inside. */
export function audiosFor(options: ChoiceOptions, video: VideoOption | undefined): AudioOption[] {
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
 * (same id, or the same height), else the best quality and the default
 * audio. With `avoidMerge` (a video page without FFmpeg, which cannot join
 * separate tracks) the best quality that needs no joining, if there is one.
 */
export function initialChoice(choices: Choices, { avoidMerge = false } = {}): { videoId: string; audioId: string } {
  const { videos } = choices.options;
  const wanted = choices.selection?.video;
  const whole = avoidMerge ? videos.find((v) => audiosFor(choices.options, v).length === 0) : undefined;
  const video =
    (wanted && (videos.find((v) => v.id === wanted.id) ?? videos.find((v) => wanted.height && v.height === wanted.height))) ??
    whole ??
    videos[0];
  const audios = audiosFor(choices.options, video);
  const wantedAudio = choices.selection?.audio;
  const audio = (wantedAudio && audios.find((a) => a.id === wantedAudio.id)) ?? defaultAudio(audios);
  return { videoId: video?.id ?? "", audioId: audio?.id ?? "" };
}

/** The selection to send for the ids picked in the dialog. */
export function selectionFor(choices: Choices, videoId: string, audioId: string): MediaSelection {
  const video = choices.options.videos.find((v) => v.id === videoId);
  const audio = audiosFor(choices.options, video).find((a) => a.id === audioId);
  return {
    video: video ? { id: video.id, height: video.height, bandwidth: video.bandwidth } : undefined,
    audio: audio ? { id: audio.id, bandwidth: audio.bandwidth, language: audio.language } : undefined,
  };
}

/** Video and sound come as two tracks, so FFmpeg is needed for one file. */
export function needsMerge(choices: Choices, videoId: string): boolean {
  const video = choices.options.videos.find((v) => v.id === videoId);
  return video !== undefined && audiosFor(choices.options, video).length > 0;
}

/** "HLS · 720p" or "YouTube · 1080p" for a list row, or null for a plain file. */
export function streamLabel(task: Task): string | null {
  const source = sourceName(task);
  if (!source) return null;
  const quality = task.site?.quality ?? task.media?.quality;
  return quality ? `${source} · ${quality}` : source;
}

/**
 * A stream's or a video page's size is an estimate until it finishes:
 * fragments have no size before they arrive, and sites give approximate
 * sizes for their formats.
 */
export function sizeIsEstimate(task: Task): boolean {
  return (task.media !== null || task.site !== null) && task.state !== "completed";
}

/**
 * Error codes for streams and video pages that will not be downloaded
 * (protected, live, unavailable without an account). Other failures, such
 * as yt-dlp not being set up, can be fixed.
 */
export const REFUSED_STREAM = new Set(["drm_protected", "live_stream", "unsupported_media", "invalid_manifest", "unavailable"]);
