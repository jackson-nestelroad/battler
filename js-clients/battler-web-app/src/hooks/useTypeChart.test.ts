import { describe, expect, it } from "vitest";
import type { TypeChartData } from "battler-types";
import {
  ALL_POKEMON_TYPES,
  formatFraction,
  formatMultiplier,
  getDefensiveMultipliers,
  getEffectiveness,
  getOffensiveMultipliers,
} from "./useTypeChart";

const MOCK_TYPE_CHART: TypeChartData = {
  types: {
    Water: {
      Fire: 2,
      Water: 0.5,
      Grass: 0.5,
      Ground: 2,
      Rock: 2,
      Dragon: 0.5,
    },
    Electric: {
      Water: 2,
      Electric: 0.5,
      Grass: 0.5,
      Ground: 0,
      Flying: 2,
      Dragon: 0.5,
    },
    Grass: {
      Fire: 0.5,
      Water: 2,
      Grass: 0.5,
      Poison: 0.5,
      Ground: 2,
      Flying: 0.5,
      Bug: 0.5,
      Rock: 2,
      Dragon: 0.5,
      Steel: 0.5,
    },
    Fire: {
      Fire: 0.5,
      Water: 0.5,
      Grass: 2,
      Ice: 2,
      Bug: 2,
      Rock: 0.5,
      Dragon: 0.5,
      Steel: 2,
    },
  },
};

describe("useTypeChart helpers", () => {
  it("evaluates single-type effectiveness correctly", () => {
    expect(getEffectiveness(MOCK_TYPE_CHART, "Water", "Fire")).toBe(2);
    expect(getEffectiveness(MOCK_TYPE_CHART, "Fire", "Water")).toBe(0.5);
    expect(getEffectiveness(MOCK_TYPE_CHART, "Electric", "Ground")).toBe(0);
    // Unlisted matchup is neutral 1x
    expect(getEffectiveness(MOCK_TYPE_CHART, "Normal", "Normal")).toBe(1);
  });

  it("calculates compound defensive multipliers for dual-types", () => {
    // Defender: Water / Ground
    const multipliers = getDefensiveMultipliers(MOCK_TYPE_CHART, ["Water", "Ground"]);

    // Grass hits Water (2x) and Ground (2x) -> 4x
    expect(multipliers["Grass"]).toBe(4);

    // Electric hits Water (2x) and Ground (0x) -> 0x
    expect(multipliers["Electric"]).toBe(0);

    // Fire hits Water (0.5x) and Ground (1x) -> 0.5x
    expect(multipliers["Fire"]).toBe(0.5);
  });

  it("calculates offensive multipliers across all defending types", () => {
    const offensive = getOffensiveMultipliers(MOCK_TYPE_CHART, "Water");
    expect(offensive["Fire"]).toBe(2);
    expect(offensive["Water"]).toBe(0.5);
    expect(offensive["Normal"]).toBe(1);
    expect(Object.keys(offensive)).toHaveLength(ALL_POKEMON_TYPES.length);
  });

  it("formats multipliers into concise symbols including higher multipliers and fractions", () => {
    expect(formatMultiplier(32)).toBe("32×");
    expect(formatMultiplier(16)).toBe("16×");
    expect(formatMultiplier(8)).toBe("8×");
    expect(formatMultiplier(4)).toBe("4×");
    expect(formatMultiplier(2)).toBe("2×");
    expect(formatMultiplier(1)).toBe("1×");
    expect(formatMultiplier(0.5)).toBe("½×");
    expect(formatMultiplier(0.25)).toBe("¼×");
    expect(formatMultiplier(0.125)).toBe("⅛×");
    expect(formatMultiplier(0.0625)).toBe("¹⁄₁₆×");
    expect(formatMultiplier(0.03125)).toBe("¹⁄₃₂×");
    expect(formatMultiplier(0.015625)).toBe("¹⁄₆₄×");
    expect(formatMultiplier(0.0078125)).toBe("¹⁄₁₂₈×");
    expect(formatMultiplier(0)).toBe("0×");
  });

  it("formats fraction variants correctly down to 1/128 and arbitrary denominators", () => {
    expect(formatFraction(0.5)).toBe("½");
    expect(formatFraction(0.25)).toBe("¼");
    expect(formatFraction(0.125)).toBe("⅛");
    expect(formatFraction(0.0625)).toBe("¹⁄₁₆");
    expect(formatFraction(0.03125)).toBe("¹⁄₃₂");
    expect(formatFraction(0.015625)).toBe("¹⁄₆₄");
    expect(formatFraction(0.0078125)).toBe("¹⁄₁₂₈");
    // Arbitrary denominator fallback
    expect(formatFraction(1 / 256)).toBe("¹⁄₂₅₆");
    // Non-fractions
    expect(formatFraction(2)).toBe("2");
    expect(formatFraction(1)).toBe("1");
  });
});
