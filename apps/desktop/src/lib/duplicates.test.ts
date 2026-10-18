import { describe, expect, it } from "vitest";

import type { DuplicateAnswer } from "./api";
import { rememberedAnswer } from "./duplicates";

describe("rememberedAnswer", () => {
  it("answers only what was chosen with Don't ask again", () => {
    expect(rememberedAnswer({ duplicate_answer: "ask" })).toBeNull();
    expect(rememberedAnswer({ duplicate_answer: "copy" })).toBe("copy");
    expect(rememberedAnswer({ duplicate_answer: "overwrite" })).toBe("overwrite");
    expect(rememberedAnswer({ duplicate_answer: "existing" })).toBe("existing");
  });

  it("asks when the stored answer is unknown", () => {
    expect(rememberedAnswer({ duplicate_answer: "rename" as DuplicateAnswer })).toBeNull();
  });
});
