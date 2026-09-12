import { describe, expect, it } from "vitest";
import type { BattleState } from "battler-state";
import type { PlayerBattleData, SpeciesData, TypeChartData } from "battler-types";
import {
  calculateTargetEffectiveness,
  canCalculateEffectiveness,
  getEffectivenessTitle,
  getMultiplierClass,
  getSingleOpposingTargetTypes,
  getSpeciesTypesFromData,
  resolveTargetTypes,
} from "./typeEffectiveness";

const mockTypeChart: TypeChartData = {
  types: {
    Fire: {
      Grass: 2,
      Bug: 2,
      Fire: 0.5,
      Water: 0.5,
      Rock: 0.5,
      Dragon: 0.5,
    },
    Water: {
      Fire: 2,
      Ground: 2,
      Rock: 2,
      Water: 0.5,
      Grass: 0.5,
      Dragon: 0.5,
    },
    Electric: {
      Water: 2,
      Flying: 2,
      Electric: 0.5,
      Grass: 0.5,
      Dragon: 0.5,
      Ground: 0,
    },
    Normal: {
      Ghost: 0,
      Rock: 0.5,
      Steel: 0.5,
    },
  },
};

describe("typeEffectiveness", () => {
  describe("getSpeciesTypesFromData", () => {
    it("returns primary type only when secondary type is null", () => {
      const data: Partial<SpeciesData> = {
        primary_type: "Fire",
        secondary_type: null,
      };
      expect(getSpeciesTypesFromData(data as SpeciesData)).toEqual(["Fire"]);
    });

    it("returns dual types when secondary type exists", () => {
      const data: Partial<SpeciesData> = {
        primary_type: "Water",
        secondary_type: "Ground",
      };
      expect(getSpeciesTypesFromData(data as SpeciesData)).toEqual(["Water", "Ground"]);
    });

    it("returns empty array when data is null or undefined", () => {
      expect(getSpeciesTypesFromData(null)).toEqual([]);
      expect(getSpeciesTypesFromData(undefined)).toEqual([]);
    });
  });

  describe("canCalculateEffectiveness", () => {
    it("returns true for damaging move with type", () => {
      expect(canCalculateEffectiveness({ category: "Physical", type: "Fire" } as any)).toBe(true);
      expect(canCalculateEffectiveness({ category: "Special", type: "Water" } as any)).toBe(true);
    });

    it("returns false for Status moves", () => {
      expect(canCalculateEffectiveness({ category: "Status", type: "Fire" } as any)).toBe(false);
    });

    it("returns false for null, undefined, or moves without type", () => {
      expect(canCalculateEffectiveness(null)).toBe(false);
      expect(canCalculateEffectiveness(undefined)).toBe(false);
      expect(canCalculateEffectiveness({ category: "Physical", type: "" } as any)).toBe(false);
    });
  });

  describe("calculateTargetEffectiveness", () => {
    it("calculates 4x super effective correctly", () => {
      // Fire against Grass/Bug = 2 * 2 = 4
      const mult = calculateTargetEffectiveness(mockTypeChart, "Fire", ["Grass", "Bug"]);
      expect(mult).toBe(4);
    });

    it("calculates 2x super effective correctly", () => {
      // Fire against Grass = 2
      const mult = calculateTargetEffectiveness(mockTypeChart, "Fire", ["Grass"]);
      expect(mult).toBe(2);
    });

    it("calculates 1x neutral effectiveness correctly", () => {
      // Water against Fire/Water = 2 * 0.5 = 1
      const mult = calculateTargetEffectiveness(mockTypeChart, "Water", ["Fire", "Water"]);
      expect(mult).toBe(1);
    });

    it("calculates 0.5x resistance correctly", () => {
      // Fire against Water = 0.5
      const mult = calculateTargetEffectiveness(mockTypeChart, "Fire", ["Water"]);
      expect(mult).toBe(0.5);
    });

    it("calculates 0.25x double resistance correctly", () => {
      // Water against Grass/Dragon = 0.5 * 0.5 = 0.25
      const mult = calculateTargetEffectiveness(mockTypeChart, "Water", ["Grass", "Dragon"]);
      expect(mult).toBe(0.25);
    });

    it("calculates 0x immunity correctly", () => {
      // Electric against Ground/Water = 0 * 2 = 0
      const mult = calculateTargetEffectiveness(mockTypeChart, "Electric", ["Ground", "Water"]);
      expect(mult).toBe(0);
    });

    it("returns 1 for unknown or empty input", () => {
      expect(calculateTargetEffectiveness(null, "Fire", ["Grass"])).toBe(1);
      expect(calculateTargetEffectiveness(mockTypeChart, "", ["Grass"])).toBe(1);
      expect(calculateTargetEffectiveness(mockTypeChart, "Fire", [])).toBe(1);
    });
  });

  describe("getMultiplierClass", () => {
    it("maps multipliers to appropriate style classes", () => {
      expect(getMultiplierClass(4)).toBe("multQuad");
      expect(getMultiplierClass(2)).toBe("multSuper");
      expect(getMultiplierClass(1)).toBe("multNeutral");
      expect(getMultiplierClass(0.5)).toBe("multResist");
      expect(getMultiplierClass(0.25)).toBe("multQuadResist");
      expect(getMultiplierClass(0)).toBe("multImmune");
    });
  });

  describe("getEffectivenessTitle", () => {
    it("returns helpful labels for tooltips", () => {
      expect(getEffectivenessTitle(4)).toContain("4×");
      expect(getEffectivenessTitle(2)).toContain("2×");
      expect(getEffectivenessTitle(1)).toContain("1×");
      expect(getEffectivenessTitle(0.5)).toContain("½×");
      expect(getEffectivenessTitle(0.25)).toContain("¼×");
      expect(getEffectivenessTitle(0)).toContain("0×");
    });
  });

  describe("resolveTargetTypes", () => {
    it("returns tera type when mon is terastallized", () => {
      const mockState = {
        field: {
          sides: [
            {
              active: [{ player: "p1", mon_index: 0, battle_appearance_index: 0 }],
              players: {
                p1: {
                  mons: [
                    {
                      battle_appearances: [
                        {
                          terastallization: { known: "Fairy" },
                        },
                      ],
                    },
                  ],
                },
              },
            },
          ],
        },
      } as unknown as BattleState;

      const types = resolveTargetTypes(mockState, null, 0, 0);
      expect(types).toEqual(["Fairy"]);
    });

    it("falls back to species types callback when not terastallized", () => {
      const mockState = {
        field: {
          sides: [
            {
              active: [{ player: "p1", mon_index: 0, battle_appearance_index: 0 }],
              players: {
                p1: {
                  mons: [
                    {
                      battle_appearances: [
                        {
                          terastallization: {},
                        },
                      ],
                      physical_appearance: {
                        species: "Charizard",
                      },
                    },
                  ],
                },
              },
            },
          ],
        },
      } as unknown as BattleState;

      const types = resolveTargetTypes(mockState, null, 0, 0, (species) => {
        if (species === "Charizard") return ["Fire", "Flying"];
        return [];
      });
      expect(types).toEqual(["Fire", "Flying"]);
    });

    it("falls back to playerData if on player side and state is missing", () => {
      const playerData: Partial<PlayerBattleData> = {
        side: 0,
        mons: [
          {
            active: true,
            player_active_position: 0,
            types: ["Electric"],
            species: "Pikachu",
          } as unknown as PlayerBattleData["mons"][number],
        ],
      };

      const types = resolveTargetTypes(null, playerData as PlayerBattleData, 0, 0);
      expect(types).toEqual(["Electric"]);
    });
  });

  describe("getSingleOpposingTargetTypes", () => {
    it("returns types for single opposing mon in Singles battle", () => {
      const mockState = {
        field: {
          sides: [
            { active: [] },
            {
              active: [{ player: "p2", mon_index: 0, battle_appearance_index: 0 }],
              players: {
                p2: {
                  mons: [
                    {
                      fainted: false,
                      types: ["Water"],
                      battle_appearances: [{}],
                    },
                  ],
                },
              },
            },
          ],
        },
      } as unknown as BattleState;

      const playerData = { side: 0 } as PlayerBattleData;
      const types = getSingleOpposingTargetTypes(mockState, playerData, "Singles", 1);
      expect(types).toEqual(["Water"]);
    });

    it("returns null in Doubles when both opposing mons are active", () => {
      const mockState = {
        field: {
          sides: [
            { active: [] },
            {
              active: [
                { player: "p2", mon_index: 0, battle_appearance_index: 0 },
                { player: "p2", mon_index: 1, battle_appearance_index: 0 },
              ],
              players: {
                p2: {
                  mons: [
                    { fainted: false, types: ["Water"], battle_appearances: [{}] },
                    { fainted: false, types: ["Grass"], battle_appearances: [{}] },
                  ],
                },
              },
            },
          ],
        },
      } as unknown as BattleState;

      const playerData = { side: 0 } as PlayerBattleData;
      const types = getSingleOpposingTargetTypes(mockState, playerData, "Doubles", 2);
      expect(types).toBeNull();
    });

    it("returns types in Doubles when one opponent is fainted and only one remains active", () => {
      const mockState = {
        field: {
          sides: [
            { active: [] },
            {
              active: [
                { player: "p2", mon_index: 0, battle_appearance_index: 0 },
                { player: "p2", mon_index: 1, battle_appearance_index: 0 },
              ],
              players: {
                p2: {
                  mons: [
                    { fainted: true, types: ["Water"], battle_appearances: [{}] },
                    { fainted: false, types: ["Grass"], battle_appearances: [{}] },
                  ],
                },
              },
            },
          ],
        },
      } as unknown as BattleState;

      const playerData = { side: 0 } as PlayerBattleData;
      const types = getSingleOpposingTargetTypes(mockState, playerData, "Doubles", 2);
      expect(types).toEqual(["Grass"]);
    });

    it("returns null when all opponents are fainted", () => {
      const mockState = {
        field: {
          sides: [
            { active: [] },
            {
              active: [{ player: "p2", mon_index: 0, battle_appearance_index: 0 }],
              players: {
                p2: {
                  mons: [{ fainted: true, types: ["Water"], battle_appearances: [{}] }],
                },
              },
            },
          ],
        },
      } as unknown as BattleState;

      const playerData = { side: 0 } as PlayerBattleData;
      const types = getSingleOpposingTargetTypes(mockState, playerData, "Singles", 1);
      expect(types).toBeNull();
    });
  });
});
