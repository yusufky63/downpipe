import { describe, expect, it } from "vitest";

import { isBrowserShortcut, listKey, type KeyLike } from "./shortcuts";

const key = (k: string, mods: Partial<KeyLike> = {}): KeyLike => ({
  key: k,
  ctrlKey: false,
  metaKey: false,
  altKey: false,
  shiftKey: false,
  ...mods,
});

describe("isBrowserShortcut", () => {
  it("blocks reload, print, save, find, history and navigation", () => {
    for (const e of [
      key("F5"),
      key("r", { ctrlKey: true }),
      key("R", { ctrlKey: true, shiftKey: true }),
      key("p", { ctrlKey: true }),
      key("s", { metaKey: true }),
      key("u", { ctrlKey: true }),
      key("f", { ctrlKey: true }),
      key("ArrowLeft", { altKey: true }),
      key("F7"),
      key("BrowserBack"),
    ]) {
      expect(isBrowserShortcut(e, false), JSON.stringify(e)).toBe(true);
    }
  });

  it("leaves editing keys and plain typing alone", () => {
    for (const e of [
      key("c", { ctrlKey: true }),
      key("v", { ctrlKey: true }),
      key("x", { ctrlKey: true }),
      key("z", { ctrlKey: true }),
      key("a", { ctrlKey: true }),
      key("r"),
      key("ArrowLeft"),
      key("ArrowLeft", { ctrlKey: true }),
    ]) {
      expect(isBrowserShortcut(e, false), JSON.stringify(e)).toBe(false);
    }
  });

  it("keeps the developer tools in development builds only", () => {
    expect(isBrowserShortcut(key("F12"), false)).toBe(true);
    expect(isBrowserShortcut(key("I", { ctrlKey: true, shiftKey: true }), false)).toBe(true);
    expect(isBrowserShortcut(key("F12"), true)).toBe(false);
    expect(isBrowserShortcut(key("I", { ctrlKey: true, shiftKey: true }), true)).toBe(false);
  });
});

describe("listKey", () => {
  it("maps the list's keys", () => {
    expect(listKey(key("Enter"))).toEqual({ kind: "properties" });
    expect(listKey(key("Delete"))).toEqual({ kind: "remove" });
    expect(listKey(key("a", { ctrlKey: true }))).toEqual({ kind: "select_all" });
    expect(listKey(key("C", { ctrlKey: true }))).toEqual({ kind: "copy_link" });
    expect(listKey(key("F10", { shiftKey: true }))).toEqual({ kind: "menu" });
    expect(listKey(key("ContextMenu"))).toEqual({ kind: "menu" });
  });

  it("moves the selection with arrows and the downloads with Ctrl", () => {
    expect(listKey(key("ArrowDown", { shiftKey: true }))).toEqual({ kind: "step", delta: 1, extend: true });
    expect(listKey(key("ArrowUp", { ctrlKey: true }))).toEqual({ kind: "move", to: "up" });
    expect(listKey(key("End", { ctrlKey: true }))).toEqual({ kind: "move", to: "bottom" });
    expect(listKey(key("Home"))).toEqual({ kind: "edge", end: false, extend: false });
  });

  it("ignores everything else", () => {
    expect(listKey(key("x"))).toBeNull();
    expect(listKey(key("F10"))).toBeNull();
    expect(listKey(key("Enter", { altKey: true }))).toBeNull();
  });
});
