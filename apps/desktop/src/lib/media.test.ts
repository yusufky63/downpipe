import { describe, expect, it } from "vitest";
import type { AudioOption, MediaChoices, SiteChoices, VideoOption } from "./api";
import {
  audioName,
  audiosFor,
  codecName,
  initialChoice,
  needsMerge,
  qualityDetail,
  qualityName,
  qualityOption,
  selectionFor,
  sizeIsEstimate,
  sourceName,
  streamLabel,
  subtitleFromKey,
  subtitleGroups,
  subtitleKey,
  subtitleName,
} from "./media";
import { makeTask } from "./testing";

const video = (patch: Partial<VideoOption>): VideoOption => ({
  id: "v",
  has_audio: false,
  audio_only: false,
  ...patch,
});

const hls: MediaChoices = {
  options: {
    kind: "hls",
    videos: [
      video({ id: "v1", width: 1920, height: 1080, bandwidth: 5_000_000, frame_rate: "60", audio_group: "hi" }),
      video({ id: "v0", width: 640, height: 360, bandwidth: 800_000, audio_group: "lo" }),
    ],
    audios: [
      { id: "a0", name: "English", language: "en", default: true, group: "hi" },
      { id: "a1", name: "Deutsch", language: "de", default: false, group: "hi" },
      { id: "a2", name: "English", language: "en", default: true, group: "lo" },
    ],
  },
  selection: null,
  ffmpeg: false,
};

describe("quality names", () => {
  it("names qualities like the engine", () => {
    expect(qualityName(video({ height: 1080, frame_rate: "60" }))).toBe("1080p 60");
    expect(qualityName(video({ bandwidth: 2_500_000 }))).toBe("2.5 Mbit/s");
    expect(qualityName(video({ audio_only: true, bandwidth: 64_000 }))).toBe("Audio only");
    expect(qualityName(video({}))).toBe("Default");
  });

  it("adds resolution, bit rate and size", () => {
    expect(qualityDetail(video({ width: 1280, height: 720, bandwidth: 2_500_000, estimated_size: 10 * 1024 * 1024 }))).toBe(
      "1280×720 · 2.5 Mbit/s · about 10.0 MiB",
    );
    expect(qualityOption(video({ height: 720 }))).toBe("720p");
  });

  it("names audio tracks by name or language", () => {
    expect(audioName({ id: "a", name: "Director", default: false, bandwidth: 128_000 })).toBe("Director · 128 kbit/s");
    expect(audioName({ id: "a", language: "de", default: false })).toBe("German");
    expect(audioName({ id: "a", default: false })).toBe("Audio");
  });
});

describe("choosing tracks", () => {
  it("offers the audio of the chosen quality's group only", () => {
    expect(audiosFor(hls.options, hls.options.videos[0]).map((a) => a.id)).toEqual(["a0", "a1"]);
    expect(audiosFor(hls.options, hls.options.videos[1]).map((a) => a.id)).toEqual(["a2"]);
    expect(audiosFor(hls.options, video({ has_audio: true }))).toEqual([]);
  });

  it("starts with the best quality, or the earlier choice", () => {
    expect(initialChoice(hls)).toEqual({ videoId: "v1", audioId: "a0" });
    const earlier: MediaChoices = { ...hls, selection: { video: { id: "gone", height: 360 }, audio: { id: "a2" } } };
    expect(initialChoice(earlier)).toEqual({ videoId: "v0", audioId: "a2" });
  });

  it("builds the selection the app stores", () => {
    expect(selectionFor(hls, "v1", "a1")).toEqual({
      video: { id: "v1", height: 1080, bandwidth: 5_000_000 },
      audio: { id: "a1", bandwidth: undefined, language: "de" },
    });
    // An audio of another group is not sent.
    expect(selectionFor(hls, "v1", "a2").audio).toBeUndefined();
  });

  it("knows when FFmpeg is needed for one file", () => {
    expect(needsMerge(hls, "v1")).toBe(true);
    const muxed: MediaChoices = { ...hls, options: { ...hls.options, videos: [video({ id: "m", has_audio: true })] } };
    expect(needsMerge(muxed, "m")).toBe(false);
    expect(needsMerge(hls, "missing")).toBe(false);
  });
});

describe("stream rows", () => {
  it("labels streams and their estimated size", () => {
    const stream = makeTask("s", { media: { kind: "hls", quality: "720p" }, state: "downloading" });
    expect(streamLabel(stream)).toBe("HLS · 720p");
    expect(streamLabel(makeTask("d", { media: { kind: "dash" } }))).toBe("DASH");
    expect(streamLabel(makeTask("f"))).toBeNull();
    expect(sizeIsEstimate(stream)).toBe(true);
    expect(sizeIsEstimate({ ...stream, state: "completed" })).toBe(false);
    expect(sizeIsEstimate(makeTask("f"))).toBe(false);
  });
});

// What the app makes of `yt-dlp -J` for a YouTube video (crates/providers/src/info.rs).
const hd = video({ id: "137", width: 1920, height: 1080, codecs: "avc1.640028", bandwidth: 4_400_000, estimated_size: 50_000_000 });
const progressive = video({ id: "18", width: 640, height: 360, codecs: "avc1.42001E, mp4a.40.2", has_audio: true });
const soundOnly = video({ id: "140", codecs: "mp4a.40.2", bandwidth: 129_796, audio_only: true, has_audio: true });
const opus: AudioOption = { id: "251", codecs: "opus", bandwidth: 106_000, default: false };
const page: SiteChoices = {
  site: "YouTube",
  title: "Me at the zoo",
  name: "Me at the zoo.mp4",
  subtitles: [],
  chosen_subtitles: [],
  options: {
    videos: [hd, progressive, soundOnly],
    audios: [{ id: "140", codecs: "mp4a.40.2", bandwidth: 129_796, default: true }, opus],
    duration_ms: 19_000,
  },
  selection: null,
  ffmpeg: true,
};

describe("video pages", () => {
  it("names codecs in words", () => {
    expect(codecName("avc1.42001E, mp4a.40.2")).toBe("H.264 + AAC");
    expect(codecName("vp09.00.40.08")).toBe("VP9");
    expect(codecName("av01.0.08M.08, opus")).toBe("AV1 + Opus");
    // HLS lists the audio first; the video still leads.
    expect(codecName("mp4a.40.2,avc1.640020")).toBe("H.264 + AAC");
    expect(codecName("unknown")).toBe("");
    expect(codecName(undefined)).toBe("");
    expect(qualityOption(progressive)).toBe("360p · 640×360 · H.264 + AAC");
    expect(qualityOption(soundOnly)).toBe("Audio only · AAC · 130 kbit/s");
    expect(audioName(opus)).toBe("Audio · Opus · 106 kbit/s");
  });

  it("joins the best quality with the default sound", () => {
    expect(initialChoice(page)).toEqual({ videoId: "137", audioId: "140" });
    expect(needsMerge(page, "137")).toBe(true);
    expect(selectionFor(page, "137", "251")).toEqual({
      video: { id: "137", height: 1080, bandwidth: 4_400_000 },
      audio: { id: "251", bandwidth: 106_000, language: undefined },
    });
  });

  it("starts with a quality that has its own sound when nothing can join tracks", () => {
    const noFfmpeg = { ...page, ffmpeg: false };
    expect(initialChoice(noFfmpeg, { avoidMerge: true })).toEqual({ videoId: "18", audioId: "" });
    expect(needsMerge(noFfmpeg, "18")).toBe(false);
    // Audio only is downloaded as it comes.
    expect(needsMerge(noFfmpeg, "140")).toBe(false);
    expect(selectionFor(noFfmpeg, "140", "")).toEqual({
      video: { id: "140", height: undefined, bandwidth: 129_796 },
      audio: undefined,
    });
  });

  it("labels site downloads in the list", () => {
    const yt = makeTask("y", { site: { site: "YouTube", quality: "1080p" }, state: "downloading" });
    expect(streamLabel(yt)).toBe("YouTube · 1080p");
    expect(streamLabel(makeTask("y2", { site: { site: "YouTube" } }))).toBe("YouTube");
    expect(sourceName(yt)).toBe("YouTube");
    expect(sizeIsEstimate(yt)).toBe(true);
    expect(sizeIsEstimate({ ...yt, state: "completed" })).toBe(false);
  });
});

describe("subtitles", () => {
  it("names languages and the original captions", () => {
    expect(subtitleName({ language: "de", auto: false })).toBe("German");
    expect(subtitleName({ language: "en-orig", auto: true })).toBe("English (original)");
    expect(subtitleName({ language: "qqq-x", name: "Klingon", auto: false })).toBe("Klingon");
  });

  it("keys round-trip and groups put the original first", () => {
    expect(subtitleFromKey(subtitleKey({ language: "pt-BR", auto: true }))).toEqual({ language: "pt-BR", auto: true });
    expect(subtitleFromKey("")).toBeNull();
    const groups = subtitleGroups([
      { language: "tr", auto: true },
      { language: "en-orig", auto: true },
      { language: "de", auto: true },
      { language: "fr", auto: false },
    ]);
    expect(groups.uploaded.map((s) => s.language)).toEqual(["fr"]);
    expect(groups.auto.map((s) => s.language)).toEqual(["en-orig", "de", "tr"]);
  });
});
