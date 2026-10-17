import { describe, expect, it } from "vitest";

import type { Task } from "./api";
import { DEFAULT_SORT, formatSort, matchesSearch, nextSort, parseSort, sortTasks } from "./sort";
import { makeTask as task } from "./testing";

const ids = (list: Task[]) => list.map((t) => t.id);

describe("sortTasks", () => {
  const list = [
    task("b", { file_name: "file10.zip", total: 10, created_at: 2 }),
    task("a", { file_name: "file9.zip", total: 300, created_at: 1 }),
    task("c", { file_name: "Alpha.iso", total: null, created_at: 3 }),
  ];

  it("defaults to newest first", () => {
    expect(ids(sortTasks(list, DEFAULT_SORT))).toEqual(["c", "b", "a"]);
  });

  it("sorts names naturally and sizes with unknowns last", () => {
    expect(ids(sortTasks(list, { key: "name", dir: "asc" }))).toEqual(["c", "a", "b"]);
    expect(ids(sortTasks(list, { key: "size", dir: "desc" }))).toEqual(["a", "b", "c"]);
    expect(ids(sortTasks(list, { key: "size", dir: "asc" }))).toEqual(["b", "a", "c"]);
  });

  it("sorts by queue order, first to start on top", () => {
    const queue = [task("x", { position: 2 }), task("y", { position: 0 }), task("z", { position: 1 })];
    expect(ids(sortTasks(queue, nextSort(DEFAULT_SORT, "order")))).toEqual(["y", "z", "x"]);
    expect(parseSort("order:asc")).toEqual({ key: "order", dir: "asc" });
  });

  it("only running downloads have a speed", () => {
    const running = [
      task("x", { state: "downloading", speed: 5 }),
      task("y", { state: "paused", speed: 0 }),
      task("z", { state: "downloading", speed: 9 }),
    ];
    expect(ids(sortTasks(running, { key: "speed", dir: "desc" }))).toEqual(["z", "x", "y"]);
  });
});

describe("nextSort", () => {
  it("flips the same column and picks a natural direction for a new one", () => {
    expect(nextSort(DEFAULT_SORT, "added")).toEqual({ key: "added", dir: "asc" });
    expect(nextSort(DEFAULT_SORT, "name")).toEqual({ key: "name", dir: "asc" });
    expect(nextSort(DEFAULT_SORT, "size")).toEqual({ key: "size", dir: "desc" });
  });

  it("round-trips through storage and rejects junk", () => {
    expect(parseSort(formatSort({ key: "speed", dir: "asc" }))).toEqual({ key: "speed", dir: "asc" });
    expect(parseSort("nonsense")).toEqual(DEFAULT_SORT);
    expect(parseSort(null)).toEqual(DEFAULT_SORT);
  });
});

describe("matchesSearch", () => {
  const t = task("ubuntu", { file_name: "ubuntu-26.04.iso", category: "Disk images" });
  it("matches every word anywhere, case-insensitively", () => {
    expect(matchesSearch(t, "")).toBe(true);
    expect(matchesSearch(t, "UBUNTU iso")).toBe(true);
    expect(matchesSearch(t, "disk")).toBe(true);
    expect(matchesSearch(t, "example.org")).toBe(true);
    expect(matchesSearch(t, "ubuntu zip")).toBe(false);
  });
});
