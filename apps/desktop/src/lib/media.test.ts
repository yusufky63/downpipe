import { describe, expect, it } from "vitest";
import type { MediaChoices, VideoOption } from "./api";
import {
  audioName,
  audiosFor,
  initialChoice,
  needsMerge,
  qualityDetail,
  qualityName,
  qualityOption,
  selectionFor,
  sizeIsEstimate,
  streamLabel,
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
