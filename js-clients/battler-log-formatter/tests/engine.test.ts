import { describe, it, expect } from "vitest";
import { parseTemplateToTokens } from "../src/engine.js";

describe("engine", () => {
  it("should parse text and variables correctly", () => {
    const tokens = parseTemplateToTokens("{{MON}} used {{MOVE}}!");
    expect(tokens).toEqual([
      { type: "variable", value: "MON" },
      { type: "text", value: " used " },
      { type: "variable", value: "MOVE" },
      { type: "text", value: "!" },
    ]);
  });

  it("should handle strings without variables", () => {
    const tokens = parseTemplateToTokens("The turn limit has been reached.");
    expect(tokens).toEqual([
      { type: "text", value: "The turn limit has been reached." },
    ]);
  });

  it("should cache results", () => {
    const tokens1 = parseTemplateToTokens("{{TEST}}");
    const tokens2 = parseTemplateToTokens("{{TEST}}");
    expect(tokens1).toBe(tokens2); // Exact same reference
  });
});
