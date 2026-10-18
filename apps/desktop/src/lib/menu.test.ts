import { describe, expect, it } from "vitest";

import type { Queue } from "./api";
import { buildMenu, linksText, type MenuEntry } from "./menu";
import { makeTask as task } from "./testing";

const queues: Queue[] = [
  { id: "main", name: "Main", max_concurrent: 3, command: "" },
  { id: "night", name: "Night", max_concurrent: 1, command: "" },
];

/** Top-level items by action, with submenus flattened. */
function items(entries: MenuEntry[]) {
  const out = new Map<string, Extract<MenuEntry, { kind: "item" }>>();
  const walk = (list: MenuEntry[]) => {
    for (const e of list) {
      if (e.kind === "item") out.set(e.action, e);
      if (e.kind === "submenu") walk(e.items);
    }
  };
  walk(entries);
  return out;
}

describe("buildMenu", () => {
  it("offers file actions for one finished download", () => {
    const menu = items(buildMenu([task("a", { state: "completed", path: "C:/D/a.bin" })], queues));
    expect([...menu.keys()]).toEqual(expect.arrayContaining(["open", "reveal", "redownload", "copy_path", "properties", "remove"]));
    expect(menu.has("relink")).toBe(false);
    expect(menu.get("resume")?.disabled).toBe(true);
    expect(menu.get("pause")?.disabled).toBe(true);
  });

  it("offers no new link for a video page, whose address does not expire", () => {
    const menu = items(buildMenu([task("y", { state: "failed", site: { site: "YouTube" } })], queues));
    expect(menu.has("relink")).toBe(false);
    expect(menu.get("resume")).toMatchObject({ label: "Retry", disabled: false });
  });

  it("offers a new link for a failed download and calls resuming a retry", () => {
    const menu = items(buildMenu([task("a", { state: "failed" })], queues));
    expect(menu.has("relink")).toBe(true);
    expect(menu.has("open")).toBe(false);
    expect(menu.has("copy_path")).toBe(false);
    expect(menu.get("resume")).toMatchObject({ label: "Retry", disabled: false });
  });

  it("acts on every selected download at once", () => {
    const menu = items(
      buildMenu([task("a", { state: "paused" }), task("b", { state: "downloading" }), task("c", { state: "failed" })], queues),
    );
    expect(menu.get("resume")).toMatchObject({ label: "Resume", disabled: false });
    expect(menu.get("pause")?.disabled).toBe(false);
    expect(menu.get("remove")?.label).toBe("Remove 3 downloads…");
    expect(menu.get("copy_link")?.label).toBe("Copy links");
    for (const single of ["open", "reveal", "properties", "redownload", "relink", "copy_path"]) {
      expect(menu.has(single), single).toBe(false);
    }
  });

  it("ticks the queue only when every selected download is in it", () => {
    const one = items(buildMenu([task("a", { queue: "night" })], queues));
    expect(one.get("queue:night")?.checked).toBe(true);
    expect(one.get("queue:main")?.checked).toBe(false);
    const mixed = items(buildMenu([task("a", { queue: "night" }), task("b")], queues));
    expect(mixed.get("queue:night")?.checked).toBe(false);
    expect(mixed.has("new_queue")).toBe(true);
  });

  it("is empty without a selection", () => {
    expect(buildMenu([], queues)).toEqual([]);
  });
});

describe("linksText", () => {
  it("puts one link per line", () => {
    expect(linksText([task("a"), task("b")])).toBe("https://example.org/a.bin\nhttps://example.org/b.bin");
  });
});
