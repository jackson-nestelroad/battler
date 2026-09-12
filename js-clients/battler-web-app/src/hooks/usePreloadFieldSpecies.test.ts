import { describe, expect, it } from "vitest";
import type { BattleState } from "battler-state";
import type { PlayerBattleData } from "battler-types";
import { extractActiveFieldSpecies } from "./usePreloadFieldSpecies";

describe("usePreloadFieldSpecies", () => {
  describe("extractActiveFieldSpecies", () => {
    it("returns empty array if no active mons on field", () => {
      expect(extractActiveFieldSpecies(null, null)).toEqual([]);
    });

    it("extracts unique species from both playerData and battleState sides", () => {
      const mockState = {
        field: {
          sides: [
            {
              active: [{ player: "p1", mon_index: 0, battle_appearance_index: 0 }],
              players: {
                p1: {
                  mons: [
                    {
                      physical_appearance: { species: "Charizard" },
                    },
                  ],
                },
              },
            },
            {
              active: [{ player: "p2", mon_index: 0, battle_appearance_index: 0 }],
              players: {
                p2: {
                  mons: [
                    {
                      physical_appearance: { species: "Blastoise" },
                    },
                  ],
                },
              },
            },
          ],
        },
      } as unknown as BattleState;

      const mockPlayer: Partial<PlayerBattleData> = {
        mons: [
          {
            active: true,
            species: "Venusaur",
          } as any,
          {
            active: false,
            species: "Pikachu",
          } as any,
        ],
      };

      const species = extractActiveFieldSpecies(mockState, mockPlayer as PlayerBattleData);
      expect(species).toContain("Venusaur");
      expect(species).toContain("Charizard");
      expect(species).toContain("Blastoise");
      expect(species).not.toContain("Pikachu");
    });
  });
});
