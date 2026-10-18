import { describe, expect, it } from "vitest";
// The Rust tests parse the same files (crates/native-host/src/protocol.rs),
// so a change on one side that the other does not accept fails a test.
import capturedFixture from "../../../../crates/native-host/tests/fixtures/download-captured.json";
import linkFixture from "../../../../crates/native-host/tests/fixtures/download-link.json";
import noStartFixture from "../../../../crates/native-host/tests/fixtures/download-no-start.json";
import mediaFixture from "../../../../crates/native-host/tests/fixtures/media-hls.json";
import playlistFixture from "../../../../crates/native-host/tests/fixtures/page-playlist.json";
import pageFixture from "../../../../crates/native-host/tests/fixtures/page-youtube.json";
import pingFixture from "../../../../crates/native-host/tests/fixtures/ping.json";
import tasksFixture from "../../../../crates/native-host/tests/fixtures/tasks.json";
import rustProtocol from "../../../../crates/native-host/src/protocol.rs?raw";
import { linkFields } from "./capture";
import {
  HOST_NAME,
  MAX_FILE_NAME_LENGTH,
  MAX_TASK_IDS,
  MAX_TITLE_LENGTH,
  PROTOCOL_VERSION,
  downloadMessage,
  fileNameHint,
  mediaMessage,
  mimeEssence,
  pageMessage,
  parseResponse,
  pingMessage,
  tasksMessage,
  titleHint,
  withStartApp,
} from "./protocol";
import { MAX_URL_LENGTH, referrerFor } from "./url";

describe("messages match the host's fixtures", () => {
  it("ping", () => {
    expect(pingMessage("p1")).toEqual(pingFixture);
  });

  it("link from the context menu", () => {
    const fields = linkFields({
      linkUrl: "https://cdn.example.net/files/report.pdf",
      pageUrl: "https://example.org/downloads",
    });
    expect(fields && downloadMessage(fields, "m1")).toEqual(linkFixture);
  });

  it("download with starting Downpipe turned off", () => {
    const message = downloadMessage(
      { resourceUrl: "https://example.org/files/report.pdf", fileName: "report.pdf" },
      "m3",
    );
    expect(message && withStartApp(message, false)).toEqual(noStartFixture);
    // On, the field is left out: the host's default starts the app.
    expect(message && withStartApp(message, true)).not.toHaveProperty("start_app");
  });

  it("captured browser download", () => {
    const message = downloadMessage(
      {
        resourceUrl: "https://downloads.example.org/release/app-1.2.3.tar.gz",
        referrer: "https://example.org/releases",
        mimeType: "application/gzip",
        fileName: "C:\\Users\\me\\Downloads\\app-1.2.3.tar.gz",
      },
      "m2",
    );
    expect(message).toEqual(capturedFixture);
  });

  it("video page from the popup or the overlay", () => {
    const message = pageMessage(
      { pageUrl: "https://www.youtube.com/watch?v=jNQXAC9IVRw&list=PL1&t=4s", title: "Me at the zoo" },
      "y1",
    );
    expect(message).toEqual(pageFixture);
  });

  it("playlist from the popup or the page menu", () => {
    const message = pageMessage({ pageUrl: "https://www.youtube.com/playlist?list=PLbpi6ZahtOH6Blw3RGYpWkSByi_T7Rygb&si=abc" }, "y2");
    expect(message).toEqual(playlistFixture);
    expect(pageMessage({ pageUrl: "https://www.youtube.com/@jawed/featured" }, "y3")?.page_url).toBe("https://www.youtube.com/@jawed/videos");
    expect(pageMessage({ pageUrl: "https://www.youtube.com/feed/history" }, "y4")).toBeNull();
  });

  it("stream from the media list", () => {
    const manifest = "https://video.example.net/streams/clip/master.m3u8#t=10";
    const page = "https://example.org/watch/clip";
    const message = mediaMessage(
      {
        manifestUrl: manifest,
        kind: "hls",
        pageUrl: page,
        referrer: referrerFor(page, manifest),
        origin: "https://example.org",
        title: " Clip of the week ",
      },
      "s1",
    );
    expect(message).toEqual(mediaFixture);
  });

  it("progress query", () => {
    expect(tasksMessage(tasksFixture.task_ids, "t1")).toEqual(tasksFixture);
  });

  it("uses the same limits as the host", () => {
    expect(rustProtocol).toContain(`pub const PROTOCOL_VERSION: u64 = ${PROTOCOL_VERSION};`);
    expect(rustProtocol).toContain("pub const MAX_URL_LEN: usize = 8 * 1024;");
    expect(MAX_URL_LENGTH).toBe(8 * 1024);
    expect(rustProtocol).toContain(`pub const MAX_FILE_NAME_LEN: usize = ${MAX_FILE_NAME_LENGTH};`);
    expect(rustProtocol).toContain(`pub const MAX_TASK_IDS: usize = ${MAX_TASK_IDS};`);
    expect(rustProtocol).toContain(`pub const MAX_TITLE_LEN: usize = ${MAX_TITLE_LENGTH};`);
    expect(HOST_NAME).toBe("dev.downpipe.native_host");
  });
});

describe("downloadMessage", () => {
  it("refuses resources the app cannot fetch", () => {
    for (const url of ["javascript:void(0)", "blob:https://a.org/1", "data:,x", "mailto:a@b.org", ""]) {
      expect(downloadMessage({ resourceUrl: url }, "x"), url).toBeNull();
    }
  });

  it("drops the fragment and keeps the query", () => {
    expect(downloadMessage({ resourceUrl: "https://a.org/f.zip?sig=1#x" }, "x")?.resource_url).toBe(
      "https://a.org/f.zip?sig=1",
    );
  });

  it("drops hints that do not fit instead of failing", () => {
    const message = downloadMessage(
      {
        resourceUrl: "https://a.org/f",
        pageUrl: "chrome://newtab",
        referrer: "file:///x",
        mimeType: "not a mime",
        fileName: "..",
      },
      "x",
    );
    expect(message).toEqual({ protocol_version: 1, type: "download", id: "x", resource_url: "https://a.org/f" });
  });

  it("never has a field for cookies or headers", () => {
    const message = downloadMessage(
      { resourceUrl: "https://a.org/f", pageUrl: "https://a.org/", referrer: "https://a.org/" },
      "x",
    );
    expect(Object.keys(message ?? {}).sort()).toEqual([
      "id",
      "page_url",
      "protocol_version",
      "referrer",
      "resource_url",
      "type",
    ]);
  });
});

describe("tasksMessage", () => {
  it("asks only about well-formed ids, once each, at most the host's limit", () => {
    expect(tasksMessage(["a", "a", "../x", "", "b"], "q")?.task_ids).toEqual(["a", "b"]);
    const many = Array.from({ length: MAX_TASK_IDS + 5 }, (_, i) => `t${i}`);
    expect(tasksMessage(many, "q")?.task_ids).toHaveLength(MAX_TASK_IDS);
    expect(tasksMessage([], "q")).toBeNull();
    expect(tasksMessage(["bad id"], "q")).toBeNull();
  });
});

describe("hints", () => {
  it("reduces MIME types to type/subtype", () => {
    expect(mimeEssence("Application/PDF; charset=binary")).toBe("application/pdf");
    expect(mimeEssence("application/vnd.ms-excel")).toBe("application/vnd.ms-excel");
    for (const bad of ["", "pdf", "a/b/c", "text/ html", undefined]) {
      expect(mimeEssence(bad), String(bad)).toBeUndefined();
    }
  });

  it("keeps bare file names only", () => {
    expect(fileNameHint("/tmp/x/report.pdf")).toBe("report.pdf");
    expect(fileNameHint("  notes.txt ")).toBe("notes.txt");
    expect(fileNameHint("a\u0000b")).toBeUndefined();
    expect(fileNameHint("n".repeat(MAX_FILE_NAME_LENGTH + 1))).toBeUndefined();
    expect(fileNameHint("C:\\")).toBeUndefined();
  });
});

describe("parseResponse", () => {
  it("reads each response type", () => {
    expect(
      parseResponse({ protocol_version: 1, id: "p", type: "pong", host_version: "0.2.0", app_running: true, app_version: "0.2.0" }),
    ).toEqual({ type: "pong", id: "p", hostVersion: "0.2.0", appRunning: true, appVersion: "0.2.0", features: [] });
    expect(parseResponse({ protocol_version: 1, type: "accepted", task_id: "t", file_name: "a.zip" })).toEqual({
      type: "accepted",
      id: undefined,
      taskId: "t",
      fileName: "a.zip",
    });
    expect(
      parseResponse({ protocol_version: 1, type: "error", code: "duplicate", message: "Already", task_id: "t" }),
    ).toEqual({ type: "error", id: undefined, code: "duplicate", message: "Already", taskId: "t" });
    expect(parseResponse({ protocol_version: 1, id: "y2", type: "list_opened", page_url: "https://www.youtube.com/@jawed/videos" })).toEqual({
      type: "list_opened",
      id: "y2",
      pageUrl: "https://www.youtube.com/@jawed/videos",
    });
    expect(parseResponse({ protocol_version: 1, type: "list_opened" })).toBeNull();
  });

  it("reads the confirmation flag, and only a literal true", () => {
    expect(parseResponse({ protocol_version: 1, type: "accepted", task_id: "t", awaiting_confirmation: true })).toEqual({
      type: "accepted",
      id: undefined,
      taskId: "t",
      fileName: undefined,
      awaitingConfirmation: true,
    });
    const notTrue = parseResponse({ protocol_version: 1, type: "accepted", task_id: "t", awaiting_confirmation: "true" });
    expect(notTrue?.type === "accepted" && notTrue.awaitingConfirmation).toBeUndefined();
    const tasks = parseResponse({
      protocol_version: 1,
      type: "tasks",
      tasks: [
        { task_id: "held", state: "paused", downloaded: 0, speed: 0, awaiting_confirmation: true },
        // From a Downpipe before the flag existed: still a plain paused entry.
        { task_id: "old", state: "paused", downloaded: 0, speed: 0 },
      ],
    });
    expect(tasks?.type === "tasks" && tasks.tasks.map((t) => [t.taskId, t.state, t.awaitingConfirmation])).toEqual([
      ["held", "paused", true],
      ["old", "paused", undefined],
    ]);
  });

  it("keeps a failure's kind only when it is a plain word", () => {
    const tasks = parseResponse({
      protocol_version: 1,
      type: "tasks",
      tasks: [
        { task_id: "a", state: "failed", downloaded: 0, speed: 0, error: "x", error_kind: "not_found" },
        { task_id: "b", state: "failed", downloaded: 0, speed: 0, error: "x", error_kind: "<b>bold</b>" },
        { task_id: "c", state: "failed", downloaded: 0, speed: 0, error: "x" },
      ],
    });
    expect(tasks?.type === "tasks" && tasks.tasks.map((t) => t.errorKind)).toEqual(["not_found", undefined, undefined]);
  });

  it("rejects anything else", () => {
    for (const bad of [
      null,
      "pong",
      [],
      { type: "pong", host_version: "1", app_running: true },
      { protocol_version: 2, type: "pong", host_version: "1", app_running: true },
      { protocol_version: 1, type: "pong", host_version: 1, app_running: true },
      { protocol_version: 1, type: "accepted" },
      { protocol_version: 1, type: "error", code: "x" },
      { protocol_version: 1, type: "surprise" },
    ]) {
      expect(parseResponse(bad), JSON.stringify(bad)).toBeNull();
    }
  });

  it("reads task updates and drops entries it does not understand", () => {
    expect(
      parseResponse({
        protocol_version: 1,
        id: "q",
        type: "tasks",
        tasks: [
          { task_id: "t1", state: "downloading", file_name: "a.zip", downloaded: 10, total: 20, speed: 5 },
          { task_id: "t2", state: "exploded", downloaded: 0, speed: 0 },
          { task_id: "t3", state: "completed", downloaded: -1, speed: 0 },
          { state: "completed", downloaded: 1, speed: 0 },
          { task_id: "t4", state: "failed", downloaded: 0, speed: 0, error: "the server refused access" },
        ],
      }),
    ).toEqual({
      type: "tasks",
      id: "q",
      tasks: [
        { taskId: "t1", state: "downloading", fileName: "a.zip", downloaded: 10, total: 20, speed: 5, error: undefined },
        {
          taskId: "t4",
          state: "failed",
          fileName: undefined,
          downloaded: 0,
          total: undefined,
          speed: 0,
          error: "the server refused access",
        },
      ],
    });
    expect(parseResponse({ protocol_version: 1, type: "tasks", tasks: "nope" })).toBeNull();
  });

  it("caps strings that end up in the popup", () => {
    const r = parseResponse({ protocol_version: 1, type: "error", code: "x", message: "m".repeat(5000) });
    expect(r?.type === "error" && r.message.length).toBe(300);
  });
});

describe("mediaMessage", () => {
  it("refuses manifests the host would refuse and drops unusable hints", () => {
    expect(mediaMessage({ manifestUrl: "blob:https://a.org/1", kind: "hls" }, "s")).toBeNull();
    expect(mediaMessage({ manifestUrl: `https://a.org/${"a".repeat(MAX_URL_LENGTH)}.mpd`, kind: "dash" }, "s")).toBeNull();
    expect(
      mediaMessage({ manifestUrl: "https://a.org/m.mpd", kind: "dash", title: "a\u0000b", pageUrl: "file:///x" }, "s"),
    ).toEqual({ protocol_version: 1, type: "media", id: "s", manifest_url: "https://a.org/m.mpd", kind: "dash" });
  });

  it("accepts titles the host accepts", () => {
    expect(titleHint("  Clip ")).toBe("Clip");
    expect(titleHint("x".repeat(MAX_TITLE_LENGTH))).toHaveLength(MAX_TITLE_LENGTH);
    expect(titleHint("x".repeat(MAX_TITLE_LENGTH + 1))).toBeUndefined();
    expect(titleHint("line\nbreak")).toBeUndefined();
    expect(titleHint("")).toBeUndefined();
  });

  it("reads what the bridge supports from a pong", () => {
    const pong = parseResponse({
      protocol_version: 1,
      type: "pong",
      host_version: "0.4.0",
      app_running: true,
      app_version: "0.4.0",
      features: ["media", 3, "x".repeat(40)],
    });
    expect(pong?.type === "pong" && pong.features).toEqual(["media"]);
  });
});

describe("pageMessage", () => {
  it("only for pages the site provider takes, as their plain address", () => {
    expect(pageMessage({ pageUrl: "https://example.org/watch?v=jNQXAC9IVRw" }, "x")).toBeNull();
    expect(pageMessage({ pageUrl: "https://www.youtube.com/playlist?list=WL" }, "x")).toBeNull();
    expect(pageMessage({ pageUrl: "https://youtu.be/jNQXAC9IVRw?si=a" }, "x")).toEqual({
      protocol_version: 1,
      type: "page",
      id: "x",
      page_url: "https://www.youtube.com/watch?v=jNQXAC9IVRw",
    });
  });

  it("drops a title the host would refuse", () => {
    const message = pageMessage({ pageUrl: "https://youtu.be/jNQXAC9IVRw", title: "a\u0007b" }, "x");
    expect(message?.title).toBeUndefined();
    expect(Object.keys(message ?? {})).not.toContain("title");
  });
});
