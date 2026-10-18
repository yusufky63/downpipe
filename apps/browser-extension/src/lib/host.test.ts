import { describe, expect, it, vi } from "vitest";
import { callHost, classifyNativeError, type NativeSend } from "./host";
import { HOST_NAME, downloadMessage, pingMessage, withStartApp } from "./protocol";

describe("callHost", () => {
  it("sends to the Downpipe host and parses the answer", async () => {
    const send = vi.fn<NativeSend>().mockResolvedValue({
      protocol_version: 1,
      id: "p1",
      type: "pong",
      host_version: "0.2.0",
      app_running: true,
      app_version: "0.2.0",
    });
    const result = await callHost(send, pingMessage("p1"), 1000);
    expect(send).toHaveBeenCalledWith(HOST_NAME, pingMessage("p1"));
    expect(result).toEqual({
      ok: true,
      response: { type: "pong", id: "p1", hostVersion: "0.2.0", appRunning: true, appVersion: "0.2.0", features: [] },
    });
  });

  it("times out when the host never answers", async () => {
    vi.useFakeTimers();
    try {
      const pending = callHost(() => new Promise(() => undefined), pingMessage("p"), 5000);
      await vi.advanceTimersByTimeAsync(5000);
      expect(await pending).toEqual({ ok: false, failure: "timeout", detail: "No answer within 5 s" });
    } finally {
      vi.useRealTimers();
    }
  });

  it("treats malformed or mismatched answers as a broken bridge", async () => {
    const garbage = await callHost(async () => ({ hello: "world" }), pingMessage("p"), 1000);
    expect(garbage.ok === false && garbage.failure).toBe("bad_response");
    const otherId = await callHost(
      async () => ({ protocol_version: 1, id: "q", type: "pong", host_version: "1", app_running: false }),
      pingMessage("p"),
      1000,
    );
    expect(otherId.ok === false && otherId.failure).toBe("bad_response");
  });

  it("sends start_app again without it to a host that predates it", async () => {
    const download = downloadMessage({ resourceUrl: "https://example.org/a.zip" }, "d1");
    if (!download) throw new Error("no message");
    const message = withStartApp(download, false);
    expect(message.start_app).toBe(false);
    const send = vi
      .fn<NativeSend>()
      .mockResolvedValueOnce({
        protocol_version: 1,
        id: "d1",
        type: "error",
        code: "invalid_message",
        message: "unknown field `start_app`",
      })
      .mockResolvedValueOnce({ protocol_version: 1, id: "d1", type: "error", code: "app_not_running", message: "closed" });
    const result = await callHost(send, message, 1000);
    expect(send).toHaveBeenCalledTimes(2);
    expect(send.mock.calls[1]?.[1]).not.toHaveProperty("start_app");
    expect(result.ok && result.response.type === "error" && result.response.code).toBe("app_not_running");
  });

  it("does not send other refused messages twice", async () => {
    const send = vi
      .fn<NativeSend>()
      .mockResolvedValue({ protocol_version: 1, id: "d1", type: "error", code: "invalid_message", message: "bad" });
    const download = downloadMessage({ resourceUrl: "https://example.org/a.zip" }, "d1");
    if (!download) throw new Error("no message");
    await callHost(send, download, 1000);
    expect(send).toHaveBeenCalledTimes(1);
  });

  it("maps browser errors to failure kinds", async () => {
    const missing = await callHost(
      async () => {
        throw new Error("Specified native messaging host not found.");
      },
      pingMessage("p"),
      1000,
    );
    expect(missing).toEqual({ ok: false, failure: "host_missing", detail: "Specified native messaging host not found." });
  });
});

describe("classifyNativeError", () => {
  it("knows Chrome's and Firefox's wording", () => {
    expect(classifyNativeError("Specified native messaging host not found.")).toBe("host_missing");
    expect(classifyNativeError("No such native application dev.downpipe.native_host")).toBe("host_missing");
    expect(classifyNativeError("Access to the specified native messaging host is forbidden.")).toBe("host_forbidden");
    expect(classifyNativeError("Native host has exited.")).toBe("host_error");
    expect(classifyNativeError("Error when communicating with the native messaging host.")).toBe("host_error");
  });
});
