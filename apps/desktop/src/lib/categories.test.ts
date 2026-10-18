import { afterEach, describe, expect, it } from "vitest";

import { CATALOGS } from "../i18n/catalogs";
import { activate } from "../i18n/core";
import { categoryLabel, storedCategoryName } from "./categories";

afterEach(() => activate("en", CATALOGS.en));

describe("built-in categories", () => {
  it("shows the built-in names in the active language and the user's own as typed", () => {
    expect(categoryLabel("Disk images")).toBe("Disk images");
    activate("tr", CATALOGS.tr);
    expect(categoryLabel("Compressed")).toBe("Sıkıştırılmış");
    expect(categoryLabel("Disk images")).toBe("Disk görüntüleri");
    expect(categoryLabel("Ebooks")).toBe("Ebooks");
    // Only own keys: an inherited property is not a category.
    expect(categoryLabel("toString")).toBe("toString");
  });

  it("keeps the stored English name, and so the folder, unless the user renames it", () => {
    activate("de", CATALOGS.de);
    expect(storedCategoryName("Music", "Musik")).toBe("Music");
    expect(storedCategoryName("Music", " Musik ")).toBe("Music");
    expect(storedCategoryName("Music", "Hörbücher")).toBe("Hörbücher");
    expect(storedCategoryName("", "Ebooks")).toBe("Ebooks");
  });
});
