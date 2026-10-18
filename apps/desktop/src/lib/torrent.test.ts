import { describe, expect, it } from "vitest";

import type { TorrentChoices } from "./api";
import { makeTask } from "./testing";
import {
  chosenFiles,
  chosenSummary,
  fromTorrentDraft,
  torrentDraft,
  initialChosen,
  isMagnet,
  isTorrentLink,
  isTorrentUrl,
  magnetName,
  ratio,
  torrentDetail,
} from "./torrent";

const HASH = "c9e15763f722f23e98a29decdfae341b98d53056";
const info = { info_hash: HASH, files: 3, uploaded: 0, private: false };

describe("torrent links", () => {
  it("knows magnet and .torrent links", () => {
    expect(isMagnet(` magnet:?xt=urn:btih:${HASH}`)).toBe(true);
    expect(isMagnet("https://example.org/a.torrent")).toBe(false);
    expect(isTorrentLink("https://example.org/files/Ubuntu.TORRENT?x=1")).toBe(true);
    expect(isTorrentLink("https://example.org/a.torrent.zip")).toBe(false);
    expect(isTorrentUrl(`magnet:?xt=urn:btih:${HASH}`)).toBe(true);
    expect(isTorrentUrl("https://example.org/a.iso")).toBe(false);
  });

  it("reads a magnet link's name", () => {
    expect(magnetName(`magnet:?xt=urn:btih:${HASH}&dn=Big+Buck%20Bunny`)).toBe("Big Buck Bunny");
    expect(magnetName(`magnet:?xt=urn:btih:${HASH}`)).toBeNull();
    expect(magnetName("https://example.org/?dn=x")).toBeNull();
  });
});

describe("the list's torrent details", () => {
  it("say what a magnet link waits for", () => {
    const task = makeTask("a", {
      torrent: info,
      swarm: { peers: 0, known_peers: 0, upload_speed: 0, seeding: false, fetching_metadata: true },
    });
    expect(torrentDetail(task)).toBe("Asking peers for the file list");
  });

  it("count peers, upload speed and the ratio while seeding", () => {
    const task = makeTask("a", {
      state: "completed",
      total: 1000,
      downloaded: 1000,
      torrent: { ...info, uploaded: 420 },
      swarm: { peers: 2, known_peers: 9, upload_speed: 2048, seeding: true, fetching_metadata: false },
    });
    expect(ratio(task)).toBeCloseTo(0.42);
    expect(torrentDetail(task)).toBe("2 peers · Up 2.0 KiB/s · Ratio 0.42");
  });

  it("show nothing special for other downloads", () => {
    expect(torrentDetail(makeTask("a"))).toBeNull();
    expect(torrentDetail(makeTask("b", { torrent: info }))).toBeNull();
    expect(ratio(makeTask("c", { torrent: info }))).toBeNull();
  });
});

describe("choosing files", () => {
  const choices: TorrentChoices = {
    info_hash: HASH,
    name: "Pack",
    files: [
      { index: 0, path: "movie.mkv", size: 1024 },
      // Index 1 is a padding file, left out by the core.
      { index: 2, path: "extras/notes.txt", size: 1024 },
    ],
    total: 2048,
    selection: null,
    folder: true,
    private: false,
  };

  it("starts with all, or with what was chosen before", () => {
    expect([...initialChosen(choices)]).toEqual([0, 2]);
    expect([...initialChosen({ ...choices, selection: [2] })]).toEqual([2]);
  });

  it("sends null for all and indices otherwise", () => {
    expect(chosenFiles(choices, new Set([0, 2]))).toBeNull();
    expect(chosenFiles(choices, new Set([2]))).toEqual([2]);
    expect(chosenFiles(choices, new Set())).toEqual([]);
  });

  it("sums up the choice", () => {
    expect(chosenSummary(choices, new Set([2]))).toBe("1 of 2 chosen · 1.0 KiB");
    expect(chosenSummary(choices, new Set([0, 2]))).toBe("2 of 2 chosen · 2.0 KiB");
  });
});

describe("the settings form", () => {
  const saved = {
    seed: true,
    seed_ratio_percent: 100,
    seed_minutes: 120,
    upload_limit: 512 * 1024,
    listen_port: 0,
    dht: true,
    lsd: true,
    upnp: false,
  };

  it("round-trips the saved values", () => {
    const draft = torrentDraft(saved);
    expect(draft).toMatchObject({ ratio: "1", minutes: "120", upload: "512", uploadUnit: "KiB", port: "0" });
    expect(fromTorrentDraft(draft, saved)).toEqual(saved);
  });

  it("reads what was typed and keeps the saved value for nonsense", () => {
    const draft = { ...torrentDraft(saved), ratio: "1,5", minutes: "x", upload: "", port: "70000", upnp: true };
    expect(fromTorrentDraft(draft, saved)).toEqual({
      ...saved,
      seed_ratio_percent: 150,
      upload_limit: 0,
      upnp: true,
    });
  });
});
