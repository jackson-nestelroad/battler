import { describe, expect, it } from "vitest";
import { formatFractionPercent } from "../../src/moves/fraction.js";

describe("formatFractionPercent", () => {
  it("formats percentage strings", () => {
    expect(formatFractionPercent("10%")).toBe("10%");
    expect(formatFractionPercent("30%")).toBe("30%");
    expect(formatFractionPercent("100%")).toBe("100%");
    expect(formatFractionPercent("0%")).toBe("0%");
    expect(formatFractionPercent("25%")).toBe("25%");
    expect(formatFractionPercent("33.3%")).toBe("33%");
    expect(formatFractionPercent("  50%  ")).toBe("50%");
  });

  it("formats fraction / ratio strings", () => {
    expect(formatFractionPercent("1/10")).toBe("10%");
    expect(formatFractionPercent("3/10")).toBe("30%");
    expect(formatFractionPercent("1/2")).toBe("50%");
    expect(formatFractionPercent("2/5")).toBe("40%");
    expect(formatFractionPercent("1/1")).toBe("100%");
    expect(formatFractionPercent("1/3")).toBe("33%");
    expect(formatFractionPercent(" 1 / 4 ")).toBe("25%");
  });

  it("formats decimal numbers", () => {
    expect(formatFractionPercent(0.1)).toBe("10%");
    expect(formatFractionPercent(0.3)).toBe("30%");
    expect(formatFractionPercent(0.5)).toBe("50%");
    expect(formatFractionPercent(1)).toBe("100%");
    expect(formatFractionPercent(0)).toBe("0%");
    expect(formatFractionPercent(-0.5)).toBe("0%");
  });

  it("formats integer numbers > 1", () => {
    expect(formatFractionPercent(10)).toBe("10%");
    expect(formatFractionPercent(30)).toBe("30%");
    expect(formatFractionPercent(100)).toBe("100%");
  });

  it("formats 2-element arrays [num, den]", () => {
    expect(formatFractionPercent([1, 10])).toBe("10%");
    expect(formatFractionPercent([1, 2])).toBe("50%");
  });

  it("returns null for invalid or empty inputs", () => {
    expect(formatFractionPercent(null)).toBeNull();
    expect(formatFractionPercent(undefined)).toBeNull();
    expect(formatFractionPercent("")).toBeNull();
    expect(formatFractionPercent("   ")).toBeNull();
    expect(formatFractionPercent("invalid")).toBeNull();
    expect(formatFractionPercent("1/0")).toBeNull();
    expect(formatFractionPercent(NaN)).toBeNull();
  });
});
