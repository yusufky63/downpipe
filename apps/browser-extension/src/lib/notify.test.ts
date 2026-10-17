import { describe, expect, it } from "vitest";
import type { Activity } from "./messages";
import { notificationFor } from "./notify";

const base: Activity = { at: 1, label: "report.pdf", source: "link", outcome: "added", message: "Added to Downpipe" };

describe("notificationFor", () => {
  it("says the download started in Downpipe and clears itself", () => {
    expect(notificationFor(base)).toEqual({ title: "Downloading in Downpipe", message: "report.pdf", sticky: false });
    expect(notificationFor({ ...base, outcome: "duplicate" })).toEqual({
      title: "Already in Downpipe",
      message: "report.pdf",
      sticky: false,
    });
  });

  it("keeps problems on screen with the reason", () => {
    expect(
      notificationFor({ ...base, outcome: "kept", message: "Downpipe was not running, so the browser downloaded it." }),
    ).toEqual({
      title: "Downloaded by the browser",
      message: "report.pdf\nDownpipe was not running, so the browser downloaded it.",
      sticky: true,
    });
    expect(notificationFor({ ...base, outcome: "failed", message: "Downpipe is not running." })).toEqual({
      title: "Not sent to Downpipe",
      message: "report.pdf\nDownpipe is not running.",
      sticky: true,
    });
  });
});
