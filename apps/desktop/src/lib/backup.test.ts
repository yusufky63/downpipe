import { describe, expect, it } from "vitest";
import { errorMessage, type BackupPreview, type IdmRow } from "./api";
import { backupOrigin, idmRowName, importErrorText, importReportText, importableRows, initialIdmSelection } from "./backup";

const row = (patch: Partial<IdmRow>): IdmRow => ({
  url: "https://example.org/files/a.zip",
  file_name: null,
  dir: null,
  size: null,
  finished: false,
  listed: false,
  ...patch,
});

const preview = (patch: Partial<BackupPreview>): BackupPreview => ({
  file: "downpipe-backup.json",
  app_version: "",
  exported_at: 0,
  downloads: 0,
  new_downloads: 0,
  invalid: 0,
  queues: 0,
  categories: 0,
  site_rules: 0,
  feeds: 0,
  new_feeds: 0,
  ...patch,
});

describe("import results", () => {
  it("say what was taken and what was left", () => {
    expect(importReportText({ settings: true, added: 3, duplicates: 1, invalid: 0, feeds: 0, feeds_failed: 0 })).toBe(
      "The settings were imported. Added 3 downloads. 1 was already in the list.",
    );
    expect(importReportText({ settings: false, added: 1, duplicates: 0, invalid: 2, feeds: 0, feeds_failed: 0 })).toBe(
      "Added 1 download. 2 damaged entries were left out.",
    );
    expect(importReportText({ settings: false, added: 0, duplicates: 0, invalid: 0, feeds: 2, feeds_failed: 1 })).toBe(
      "Added 0 downloads. Subscribed to 2 feeds. 1 feed could not be read and was left out.",
    );
  });

  it("explain why a file cannot be imported", () => {
    expect(importErrorText({ code: "not_a_backup", message: "x" })).toBe("This file is not a Downpipe backup.");
    expect(importErrorText({ code: "backup_too_new", message: "x" })).toContain("newer Downpipe");
    expect(importErrorText({ code: "file_too_large", message: "x" })).toBe("This file is too large to import.");
    // Anything else is worded like any other failure.
    const io = { code: "io", message: "Access is denied." };
    expect(importErrorText(io)).toBe(errorMessage(io));
  });

  it("name where a backup came from as far as it says", () => {
    expect(backupOrigin(preview({}))).toBe("The file does not say when it was made.");
    expect(backupOrigin(preview({ app_version: "0.5.0", exported_at: 1_760_000_000 }))).toMatch(/^Made by Downpipe 0\.5\.0 on .+\.$/);
    expect(backupOrigin(preview({ exported_at: 1_760_000_000 }))).toMatch(/^Made on .+\.$/);
  });
});

describe("IDM's list", () => {
  it("starts with what is neither finished nor in Downpipe ticked", () => {
    const rows = [row({}), row({ finished: true }), row({ listed: true }), row({ url: "https://example.org/b" })];
    expect([...initialIdmSelection(rows)]).toEqual([0, 3]);
    expect(importableRows(rows)).toEqual([0, 1, 3]);
  });

  it("names rows by file name, else by the link", () => {
    expect(idmRowName(row({ file_name: "setup.exe" }))).toBe("setup.exe");
    expect(idmRowName(row({}))).toBe("a.zip");
  });
});
