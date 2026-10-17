import { describe, expect, it } from "vitest";

import type { Task } from "./api";
import { applyEvent, canPause, canResume, countByFilter, displayName, sortForDisplay, totals } from "./tasks";

function task(id: string, patch: Partial<Task> = {}): Task {
  return {
    id,
    url: `https://example.org/${id}.bin`,
    dir: "C:/Downloads",
    file_name: null,
    path: null,
    state: "queued",
    total: null,
    downloaded: 0,
    error: null,
    error_kind: null,
    checksum: null,
    max_connections: null,
    category: null,
    auto_dir: true,
    created_at: 1,
    updated_at: 1,
    completed_at: null,
    speed: 0,
    connections: 0,
    target_connections: 0,
    resumable: false,
    segments: [],
    notices: [],
    eta_secs: null,
    ...patch,
  };
}

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
  });
});
