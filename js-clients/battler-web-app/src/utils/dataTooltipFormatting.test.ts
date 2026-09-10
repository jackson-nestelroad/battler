import { describe, expect, it } from "vitest";
import {
  extractResourceName,
  formatAccuracy,
  formatBasePower,
  formatDeciMetric,
  formatPp,
  formatPriority,
  formatSpeciesClass,
  parseGenderRatio,
  toId,
} from "./dataTooltipFormatting";

describe("dataTooltipFormatting", () => {
  describe("toId", () => {
    it("normalizes mixed case and spaces to lowercase alphanumeric", () => {
      expect(toId("Thunder Wave")).toBe("thunderwave");
      expect(toId("U-turn")).toBe("uturn");
      expect(toId("Zoroark-Hisui")).toBe("zoroarkhisui");
      expect(toId("10,000,000 Volt Volt")).toBe("10000000voltvolt");
    });

    it("handles empty strings", () => {
      expect(toId("")).toBe("");
      expect(toId("!@#$%^&*()")).toBe("");
    });
  });

  describe("formatBasePower", () => {
    it("formats positive base power as string", () => {
      expect(formatBasePower(90)).toBe("90");
      expect(formatBasePower(120)).toBe("120");
    });

    it("returns em dash for zero, negative, or omitted base power", () => {
      expect(formatBasePower(0)).toBe("—");
      expect(formatBasePower(-1)).toBe("—");
      expect(formatBasePower(null)).toBe("—");
      expect(formatBasePower(undefined)).toBe("—");
    });
  });
  describe("formatAccuracy", () => {
    it("formats numeric accuracy", () => {
      expect(formatAccuracy(100)).toBe("100%");
      expect(formatAccuracy(85)).toBe("85%");
    });

    it("formats exempt / bypass accuracy as em dash", () => {
      expect(formatAccuracy("exempt")).toBe("—");
      expect(formatAccuracy(null)).toBe("—");
      expect(formatAccuracy(undefined)).toBe("—");
    });
  });

  describe("formatPp", () => {
    it("formats normal move PP with maximum boosted value", () => {
      expect(formatPp(15)).toBe("15 (max 24)");
      expect(formatPp(10)).toBe("10 (max 16)");
      expect(formatPp(5)).toBe("5 (max 8)");
    });

    it("formats boost-exempt move PP without redundant max suffix", () => {
      expect(formatPp(1, true)).toBe("1");
      expect(formatPp(5, true)).toBe("5");
    });

    it("omits redundant max suffix when calculated max equals base PP", () => {
      expect(formatPp(1)).toBe("1");
    });

    it("returns em dash for empty or zero PP", () => {
      expect(formatPp(0)).toBe("—");
      expect(formatPp(null)).toBe("—");
      expect(formatPp(undefined)).toBe("—");
    });
  });

  describe("formatPriority", () => {
    it("formats positive priority with leading plus", () => {
      expect(formatPriority(1)).toBe("+1");
      expect(formatPriority(3)).toBe("+3");
    });

    it("formats negative priority with negative sign", () => {
      expect(formatPriority(-1)).toBe("-1");
      expect(formatPriority(-6)).toBe("-6");
    });

    it("returns null for priority 0, null, or undefined to omit from display", () => {
      expect(formatPriority(0)).toBeNull();
      expect(formatPriority(null)).toBeNull();
      expect(formatPriority(undefined)).toBeNull();
    });
  });

  describe("formatSpeciesClass", () => {
    it("replaces Pokémon with Mon in species class", () => {
      expect(formatSpeciesClass("Mach Pokémon")).toBe("Mach Mon");
      expect(formatSpeciesClass("Flame Pokémon")).toBe("Flame Mon");
      expect(formatSpeciesClass("Tiny Turtle Pokémon")).toBe("Tiny Turtle Mon");
    });

    it("appends Mon when species class lacks Mon suffix", () => {
      expect(formatSpeciesClass("Tricky Fox")).toBe("Tricky Fox Mon");
      expect(formatSpeciesClass("Disaster")).toBe("Disaster Mon");
    });

    it("preserves classes already ending in Mon", () => {
      expect(formatSpeciesClass("Mach Mon")).toBe("Mach Mon");
    });

    it("returns default 'Mon' for empty or null raw class", () => {
      expect(formatSpeciesClass(null)).toBe("Mon");
      expect(formatSpeciesClass(undefined)).toBe("Mon");
      expect(formatSpeciesClass("")).toBe("Mon");
      expect(formatSpeciesClass("Pokémon")).toBe("Mon");
    });
  });

  describe("parseGenderRatio", () => {
    it("returns genderless for ratio 255, negative, null, or undefined", () => {
      expect(parseGenderRatio(255)).toEqual({ type: "genderless" });
      expect(parseGenderRatio(-1)).toEqual({ type: "genderless" });
      expect(parseGenderRatio(null)).toEqual({ type: "genderless" });
      expect(parseGenderRatio(undefined)).toEqual({ type: "genderless" });
    });

    it("returns male-only for ratio 0", () => {
      expect(parseGenderRatio(0)).toEqual({
        type: "male-only",
        malePercent: 100,
        femalePercent: 0,
      });
    });

    it("returns female-only for ratio 254", () => {
      expect(parseGenderRatio(254)).toEqual({
        type: "female-only",
        malePercent: 0,
        femalePercent: 100,
      });
    });

    it("returns 50/50 split for ratio 127", () => {
      expect(parseGenderRatio(127)).toEqual({
        type: "split",
        malePercent: 50,
        femalePercent: 50,
      });
    });

    it("returns standard split ratios for 31, 63, 191, and 223", () => {
      expect(parseGenderRatio(31)).toEqual({
        type: "split",
        malePercent: 87.5,
        femalePercent: 12.5,
      });
      expect(parseGenderRatio(63)).toEqual({
        type: "split",
        malePercent: 75,
        femalePercent: 25,
      });
      expect(parseGenderRatio(191)).toEqual({
        type: "split",
        malePercent: 25,
        femalePercent: 75,
      });
      expect(parseGenderRatio(223)).toEqual({
        type: "split",
        malePercent: 12.5,
        femalePercent: 87.5,
      });
    });

    it("calculates proportional percentages for custom ratios", () => {
      const custom = parseGenderRatio(100);
      expect(custom.type).toBe("split");
      expect(custom.femalePercent).toBe(39.7);
      expect(custom.malePercent).toBe(60.3);
    });
  });

  describe("formatDeciMetric", () => {
    it("converts tenths-based value to single-decimal metric string with unit", () => {
      expect(formatDeciMetric(950, "kg")).toBe("95.0 kg");
      expect(formatDeciMetric(19, "m")).toBe("1.9 m");
      expect(formatDeciMetric(5, "kg")).toBe("0.5 kg");
    });

    it("formats without unit when unit is empty", () => {
      expect(formatDeciMetric(123)).toBe("12.3");
    });

    it("returns null for null, undefined, zero, or negative values", () => {
      expect(formatDeciMetric(null, "kg")).toBeNull();
      expect(formatDeciMetric(undefined, "m")).toBeNull();
      expect(formatDeciMetric(0, "kg")).toBeNull();
      expect(formatDeciMetric(-10, "kg")).toBeNull();
    });
  });

  describe("extractResourceName", () => {
    it("extracts name property when present and non-empty string", () => {
      expect(extractResourceName({ name: "Thunderbolt" })).toBe("Thunderbolt");
      expect(extractResourceName({ name: "Rain", type: "weather" })).toBe("Rain");
    });

    it("returns undefined for null, undefined, non-objects, or missing name", () => {
      expect(extractResourceName(null)).toBeUndefined();
      expect(extractResourceName(undefined)).toBeUndefined();
      expect(extractResourceName("Thunderbolt")).toBeUndefined();
      expect(extractResourceName(123)).toBeUndefined();
      expect(extractResourceName({})).toBeUndefined();
      expect(extractResourceName({ name: "" })).toBeUndefined();
      expect(extractResourceName({ name: 123 })).toBeUndefined();
    });
  });
});
