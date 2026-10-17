import { describe, expect, it } from "vitest";

import type { Task } from "./api";
import { applyEvent, awaitingIn, canPause, canResume, countByFilter, displayName, knownSize, sortForDisplay, totals } from "./tasks";
import { makeTask as task } from "./testing";

describe("applyEvent", () => {
  it("inserts, replaces and removes", () => {
    let list: Task[] = [];
    list = applyEvent(list, { type: "updated", task: task("a") });
    list = applyEvent(list, { type: "updated", task: task("b") });
    list = applyEvent(list, { type: "updated", task: task("a", { state: "downloading" }) });
    expect(list.map((t) => [t.id, t.state])).toEqual([
      ["a", "downloading"],
      ["b", "queued"],
    ]);
    list = applyEvent(list, { type: "removed", id: "a" });
    expect(list.map((t) => t.id)).toEqual(["b"]);
  });
});

describe("summaries", () => {
  const list = [
    task("a", { state: "downloading", speed: 100 }),
    task("b", { state: "waiting", speed: 0 }),
    task("c", { state: "queued" }),
    task("d", { state: "paused" }),
    task("e", { state: "completed" }),
    task("f", { state: "failed" }),
  ];

  it("counts filters", () => {
    expect(countByFilter(list)).toEqual({ all: 6, active: 2, queued: 2, completed: 1, failed: 1 });
  });

  it("totals running work", () => {
    expect(totals(list)).toEqual({ speed: 100, active: 2, queued: 1 });
  });

  it("knows which actions apply", () => {
    const byId = (id: string) => {
      const found = list.find((t) => t.id === id);
      if (!found) throw new Error(`no task ${id}`);
      return found;
    };
    expect(canPause(byId("a"))).toBe(true);
    expect(canPause(byId("e"))).toBe(false);
    expect(canResume(byId("d"))).toBe(true);
    expect(canResume(byId("a"))).toBe(false);
  });
});

describe("display", () => {
  it("sorts newest first and names from the URL until the server answers", () => {
    const sorted = sortForDisplay([task("old", { created_at: 1 }), task("new", { created_at: 2 })]);
    expect(sorted.map((t) => t.id)).toEqual(["new", "old"]);
    expect(displayName(task("x"))).toBe("x.bin");
    expect(displayName(task("x", { file_name: "real.iso" }))).toBe("real.iso");
    // A page's address says nothing; its title replaces this once known.
    expect(displayName(task("y", { url: "https://www.youtube.com/watch?v=jNQXAC9IVRw", site: { site: "YouTube" } }))).toBe(
      "YouTube video",
    );
  });
});

describe("awaitingIn", () => {
  const list = [
    task("browser-new", { state: "paused", awaiting_confirmation: true, created_at: 3, position: 1 }),
    task("page-here", { state: "paused", awaiting_confirmation: true, from_app: true, created_at: 2 }),
    // Sent in the same second as the next one, but after it.
    task("browser-second", { state: "paused", awaiting_confirmation: true, created_at: 1, position: 5 }),
    task("browser-first", { state: "paused", awaiting_confirmation: true, created_at: 1, position: 4 }),
    task("answered", { state: "paused", from_app: true }),
    task("running", { state: "downloading" }),
  ];

  it("asks for what the browser sent in the confirmation window, in the order it came", () => {
    expect(awaitingIn(list, "window").map((t) => t.id)).toEqual(["browser-first", "browser-second", "browser-new"]);
  });

  it("asks for what was added in the app in the main window", () => {
    expect(awaitingIn(list, "main").map((t) => t.id)).toEqual(["page-here"]);
  });

  it("asks nowhere once nothing waits", () => {
    const answered = list.map((t) => ({ ...t, awaiting_confirmation: false }));
    expect(awaitingIn(answered, "main")).toEqual([]);
    expect(awaitingIn(answered, "window")).toEqual([]);
  });
});

describe("knownSize", () => {
  it("shows the real size once known, else the estimate", () => {
    expect(knownSize(1536, 99)).toBe("1.5 KiB");
    expect(knownSize(null, 10 * 1024 * 1024)).toBe("~10.0 MiB");
  });

  it("shows nothing for a file nobody asked yet", () => {
    expect(knownSize(null, undefined)).toBeNull();
    expect(knownSize(0, 0)).toBeNull();
  });
});
