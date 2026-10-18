import { describe, expect, it } from "vitest";

import {
  RemoteError,
  cleanToken,
  groupToken,
  linkWithToken,
  qrPath,
  remoteCanPause,
  remoteCanResume,
  remoteClient,
  remoteErrorText,
  remoteStateInfo,
  tokenFromHash,
  type RemoteDownload,
} from "./remote";

const download = (patch: Partial<RemoteDownload> = {}): RemoteDownload => ({
  id: "a",
  name: "movie.mkv",
  host: "cdn.example",
  state: "downloading",
  total: 100,
  downloaded: 40,
  speed: 10,
  eta_secs: 6,
  error_kind: null,
  waiting: null,
  queue: "main",
  category: "Video",
  awaiting_confirmation: false,
  quality: null,
  created_at: 1,
  completed_at: null,
  ...patch,
});

describe("tokens", () => {
  it("reads the token from the link's fragment", () => {
    expect(tokenFromHash("#token=ABCD-efgh")).toBe("abcdefgh");
    expect(tokenFromHash("#other=1")).toBeNull();
    expect(tokenFromHash("")).toBeNull();
    expect(tokenFromHash("#token=")).toBeNull();
  });

  it("cleans and groups typed tokens", () => {
    expect(cleanToken(" abcd-EFGH ijkl ")).toBe("abcdefghijkl");
    expect(groupToken("abcdefghij")).toBe("abcd-efgh-ij");
  });

  it("puts the token in the fragment, never the query", () => {
    expect(linkWithToken("http://192.168.1.5:9420/", "abc")).toBe("http://192.168.1.5:9420/#token=abc");
    expect(linkWithToken("http://h/#old", "abc")).toBe("http://h/#token=abc");
  });
});

describe("qrPath", () => {
  it("draws one square per dark module", () => {
    expect(
      qrPath([
        [true, false],
        [false, true],
      ]),
    ).toBe("M0 0h1v1h-1zM1 1h1v1h-1z");
  });
});

describe("row actions", () => {
  it("pauses running and queued downloads, resumes the rest", () => {
    expect(remoteCanPause(download())).toBe(true);
    expect(remoteCanPause(download({ state: "queued" }))).toBe(true);
    expect(remoteCanResume(download())).toBe(false);
    expect(remoteCanResume(download({ state: "failed" }))).toBe(true);
    expect(remoteCanPause(download({ state: "completed" }))).toBe(false);
    expect(remoteCanResume(download({ state: "completed" }))).toBe(false);
  });

  it("starts a download waiting for confirmation as it is", () => {
    const waiting = download({ state: "paused", awaiting_confirmation: true });
    expect(remoteCanResume(waiting)).toBe(true);
    expect(remoteCanPause(waiting)).toBe(false);
    expect(remoteStateInfo(waiting).tone).toBe("warning");
  });

  it("labels states", () => {
    expect(remoteStateInfo(download({ state: "completed" }))).toEqual({ label: "Done", tone: "positive" });
  });
});

describe("errors", () => {
  it("has a sentence for every code the API sends", () => {
    for (const code of ["unauthorized", "too_many_attempts", "duplicate", "invalid_url", "invalid_folder", "unknown_queue", "not_found"]) {
      expect(remoteErrorText(code)).not.toBe(remoteErrorText("something_new"));
    }
    expect(remoteErrorText("something_new")).toBe("Something went wrong. Try again.");
  });
});

describe("remoteClient", () => {
  type Call = { input: string; init: RequestInit | undefined };

  const fake = (status: number, body: unknown) => {
    const calls: Call[] = [];
    const fetchImpl = (input: string, init?: RequestInit) => {
      calls.push({ input, init });
      return Promise.resolve(new Response(status === 204 ? null : JSON.stringify(body), { status }));
    };
    return { calls, fetchImpl };
  };

  it("sends the token on every request and the body as JSON", async () => {
    const { calls, fetchImpl } = fake(201, { download: download() });
    const client = remoteClient("tok", fetchImpl);
    const added = await client.add({ url: "https://cdn.example/movie.mkv", folder: "Movies" });
    expect(added.name).toBe("movie.mkv");
    const call = calls[0];
    expect(call?.input).toBe("/api/v1/downloads");
    expect(call?.init?.method).toBe("POST");
    expect(call?.init?.credentials).toBe("omit");
    expect((call?.init?.headers as Record<string, string>).Authorization).toBe("Bearer tok");
    expect(JSON.parse(String(call?.init?.body))).toEqual({ url: "https://cdn.example/movie.mkv", folder: "Movies" });
  });

  it("escapes ids in paths and accepts empty answers", async () => {
    const { calls, fetchImpl } = fake(204, null);
    const client = remoteClient("tok", fetchImpl);
    await client.remove("a/b");
    expect(calls[0]?.input).toBe("/api/v1/downloads/a%2Fb");
    expect(calls[0]?.init?.method).toBe("DELETE");
  });

  it("turns error answers into codes", async () => {
    const { fetchImpl } = fake(409, { error: "duplicate", id: "x" });
    const err = await remoteClient("tok", fetchImpl)
      .add({ url: "https://a" })
      .catch((e: unknown) => e);
    expect(err).toBeInstanceOf(RemoteError);
    expect((err as RemoteError).code).toBe("duplicate");
    expect((err as RemoteError).id).toBe("x");
  });

  it("reports an unreachable app as offline", async () => {
    const client = remoteClient("tok", () => Promise.reject(new TypeError("Failed to fetch")));
    const err = await client.list().catch((e: unknown) => e);
    expect((err as RemoteError).code).toBe("offline");
  });
});
