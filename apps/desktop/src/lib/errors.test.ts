import { afterEach, describe, expect, it } from "vitest";

import { CATALOGS } from "../i18n/catalogs";
import { activate } from "../i18n/core";
import { en } from "../i18n/en";
import { COMMAND_CODES, FAILURE_KINDS, describeError, describeFailure, failureText } from "./errors";

afterEach(() => activate("en", CATALOGS.en));

describe("failure kinds", () => {
  it("has a text for every kind the engine reports and every command code", () => {
    expect(Object.keys(en.errors.kind).sort()).toEqual([...FAILURE_KINDS].sort());
    expect(Object.keys(en.errors.command).sort()).toEqual([...COMMAND_CODES].sort());
  });

  it("words a kind and keeps the engine's text as the detail", () => {
    expect(describeFailure("not_found", "the file was not found on the server (HTTP 404)")).toEqual({
      text: "The file was not found on the server. Check the link or use a new one.",
      detail: "The file was not found on the server (HTTP 404)",
    });
    expect(failureText("http_status", "unexpected HTTP status 502")).toBe("The server answered with an error (HTTP 502).");
    expect(failureText("http_status", null)).toBe("The server answered with an error.");
    expect(failureText(null, "anything")).toBeNull();
    expect(describeFailure("not_a_kind", "the engine's words")).toEqual({ text: "The engine's words", detail: null });
    expect(describeFailure(null, null)).toBeNull();
  });

  it("words yt-dlp's common failures with what to do", () => {
    const signIn = describeFailure("site_sign_in", "YouTube wants a signed-in browser (yt-dlp: Sign in to confirm you're not a bot)");
    expect(signIn?.text).toMatch(/^YouTube wants a signed-in browser/);
    expect(signIn?.detail).toMatch(/not a bot/);
    activate("tr", CATALOGS.tr);
    expect(failureText("site_js_runtime", null)).toMatch(/^yt-dlp, YouTube'un JavaScript/);
  });

  it("speaks the active language", () => {
    activate("tr", CATALOGS.tr);
    expect(failureText("network", "network error: timed out")).toBe("Sunucuyla bağlantı kurulamadı. Ağınızı denetleyip yeniden deneyin.");
    expect(failureText("http_status", "unexpected HTTP status 503")).toBe("Sunucu bir hatayla yanıt verdi (HTTP 503).");
    activate("de", CATALOGS.de);
    expect(failureText("io", "disk error: no space")).toMatch(/^Die Datei konnte nicht geschrieben werden/);
  });
});

describe("command errors", () => {
  it("tells a download that left the list from a missing file", () => {
    expect(describeError({ code: "gone", message: "That download no longer exists" }).text).toBe("That download is no longer in the list.");
    expect(describeError({ code: "not_found", message: "the file was not found on the server (HTTP 404)" }).text).toMatch(/^The file was not found/);
  });

  it("falls back to the message, then to a general text", () => {
    expect(describeError({ code: "something_new", message: "a new problem" })).toEqual({ text: "A new problem", detail: null });
    expect(describeError(new Error("boom")).text).toBe("boom");
    expect(describeError(undefined).text).toBe("Something went wrong");
  });
});
