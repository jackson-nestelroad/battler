import { describe, expect, it } from "vitest";
import type { TypeChartData } from "battler-types";
import {
  ALL_POKEMON_TYPES,
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

  it("formats multipliers into concise symbols", () => {
    expect(formatMultiplier(4)).toBe("4×");
    expect(formatMultiplier(2)).toBe("2×");
    expect(formatMultiplier(1)).toBe("1×");
    expect(formatMultiplier(0.5)).toBe("½×");
    expect(formatMultiplier(0.25)).toBe("¼×");
    expect(formatMultiplier(0)).toBe("0×");
  });
});
