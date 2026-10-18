import { describe, expect, it } from "vitest";

import type { Settings } from "./api";
import {
  categoryExtracts,
  commandPlaceholder,
  commandText,
  extractText,
  hasArchiveTypes,
  inheritedCommand,
  isArchive,
  postBadge,
  postProblem,
  scanText,
} from "./post";
import { stateInfo } from "./tasks";
import { makeTask as task } from "./testing";

function settings(patch: Partial<Settings> = {}): Settings {
  return {
    queues: [
      { id: "main", name: "Main", max_concurrent: 3, command: "" },
      { id: "night", name: "Night", max_concurrent: 1, command: "notify-send {name}" },
    ],
    categories: [
      { name: "Compressed", extensions: ["zip", "7z", "gz"], folder: "", extract: true },
      { name: "Video", extensions: ["mp4"], folder: "", extract: false },
    ],
    use_categories: false,
    post: { command: "global {path}", timeout_secs: 600, scan: false, scanner: "", delete_archive: false },
    ...patch,
  } as Settings;
}

describe("archives", () => {
  it("knows the formats the core extracts", () => {
    expect(["a.zip", "a.TAR.GZ", "a.tgz", "a.tar.xz", "a.txz", "a.tar", "a.7z"].every(isArchive)).toBe(true);
    expect(["a.rar", "a.gz", ".zip", "zip", "a.zip.part"].some(isArchive)).toBe(false);
    expect(hasArchiveTypes(["pdf", "Zip"])).toBe(true);
    expect(hasArchiveTypes(["mp4"])).toBe(false);
  });

  it("follows the category's switch, sorting on or off", () => {
    const s = settings();
    expect(categoryExtracts(s, "pack.zip")).toBe(true);
    expect(categoryExtracts(s, "clip.mp4")).toBe(false);
    expect(categoryExtracts(s, "README")).toBe(false);
  });
});

describe("commands", () => {
  it("the queue's command comes before the one in Settings", () => {
    const s = settings();
    expect(inheritedCommand(s, "night")).toBe("notify-send {name}");
    expect(inheritedCommand(s, "main")).toBe("global {path}");
    expect(inheritedCommand(s, "gone")).toBe("global {path}");
    expect(commandPlaceholder("global {path}")).toBe("Default: global {path}");
    expect(commandPlaceholder("")).toBe("Nothing");
  });
});

describe("reports", () => {
  it("names the scanner and what it found", () => {
    expect(scanText({ result: "clean", scanner: "Microsoft Defender" })).toBe("Microsoft Defender found nothing.");
    expect(scanText({ result: "threat", scanner: "clamscan", detail: "Eicar" })).toMatch(/^clamscan found Eicar\. /);
    expect(scanText({ result: "threat", scanner: "clamscan" })).toMatch(/^clamscan found a threat\. /);
    expect(scanText({ result: "missing", scanner: "" })).toMatch(/no scanner is set up/);
    expect(scanText({ result: "missing", scanner: "Microsoft Defender" })).toBe("Not scanned: Microsoft Defender was not found.");
  });

  it("explains extraction and commands", () => {
    expect(extractText({ result: "done", files: 3, bytes: 30, skipped: 0, archive_deleted: true })).toBe(
      "3 files into a new folder. The archive was deleted afterwards.",
    );
    expect(extractText({ result: "done", files: 1, bytes: 1, skipped: 2, archive_deleted: false })).toMatch(/^1 file into a new folder\. 2 entries/);
    expect(extractText({ result: "failed", files: 0, bytes: 0, skipped: 0, archive_deleted: false, error: "encrypted" })).toBe(
      "Not extracted: the archive is password-protected.",
    );
    expect(commandText({ result: "failed", program: "7z.exe", exit_code: 2, output: "" })).toBe("7z.exe ended with exit code 2.");
    expect(commandText({ result: "failed", program: "7z", output: "" })).toBe("7z was ended before it finished.");
    expect(commandText({ result: "timed_out", program: "x", output: "" })).toMatch(/time limit/);
  });

  it("marks threats and running steps in the list", () => {
    const threat = task("a", {
      state: "completed",
      post: { scan: { result: "threat", scanner: "Microsoft Defender", detail: "EICAR" }, stopped_by_threat: true },
    });
    expect(postBadge(threat)).toEqual({ label: "Threat", tone: "danger" });
    expect(stateInfo(threat)).toEqual({ label: "Threat", tone: "danger" });
    expect(postProblem(threat)).toBe("Threat found: EICAR");

    const scanning = task("b", { state: "completed", post_step: "scanning" });
    expect(stateInfo(scanning)).toEqual({ label: "Scanning", tone: "primary" });

    const fine = task("c", { state: "completed", post: { scan: { result: "clean", scanner: "x" }, stopped_by_threat: false } });
    expect(postBadge(fine)).toBeNull();
    expect(postProblem(fine)).toBeNull();
    expect(stateInfo(fine).tone).toBe("positive");

    const failed = task("d", {
      state: "completed",
      post: { command: { result: "not_started", program: "x", output: "" }, stopped_by_threat: false },
    });
    expect(postProblem(failed)).toBe("The command after download failed");
  });
});
