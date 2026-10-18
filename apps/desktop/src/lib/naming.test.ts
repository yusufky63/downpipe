import { describe, expect, it } from "vitest";
// crates/providers/src/naming.rs reads the same cases, so the preview in
// Settings shows the name the app gives.
import cases from "../../../../crates/providers/tests/fixtures/naming.json";
import { namesAlike, renderTemplate } from "./naming";

describe("renderTemplate", () => {
  it.each(cases)("$template", ({ template, fields, expected }) => {
    expect(renderTemplate(template, fields)).toBe(expected);
  });

  it("leaves a template without placeholders as it is", () => {
    expect(renderTemplate("clip", { title: "x" })).toBe("clip");
    expect(renderTemplate("", { title: "x" })).toBe("");
  });
});

describe("namesAlike", () => {
  it("is true only without a title or id", () => {
    expect(namesAlike("{channel} {date}")).toBe(true);
    expect(namesAlike("{index} - {title}")).toBe(false);
    expect(namesAlike("{id}")).toBe(false);
  });
});
