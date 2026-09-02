import { describe, expect, it } from "vitest";
import { getMoveTargetInfo, getValidTargets, isAdjacent } from "./targeting";
import type { BattleState } from "battler-state";
import type { PlayerBattleData } from "battler-types";

describe("targeting utility", () => {
  describe("getMoveTargetInfo", () => {
    it("correctly identifies choosable target types", () => {
      expect(getMoveTargetInfo("Normal").isChoosable).toBe(true);
      expect(getMoveTargetInfo("AdjacentFoe").isChoosable).toBe(true);
      expect(getMoveTargetInfo("AdjacentAlly").isChoosable).toBe(true);
      expect(getMoveTargetInfo("AdjacentAllyOrUser").isChoosable).toBe(true);
      expect(getMoveTargetInfo("Any").isChoosable).toBe(true);

      expect(getMoveTargetInfo("User").isChoosable).toBe(false);
      expect(getMoveTargetInfo("AllAdjacentFoes").isChoosable).toBe(false);
    });

    it("correctly sets target scope flags", () => {
      const foeInfo = getMoveTargetInfo("AdjacentFoe");
      expect(foeInfo.canTargetFoe).toBe(true);
      expect(foeInfo.canTargetAlly).toBe(false);
      expect(foeInfo.canTargetSelf).toBe(false);
      expect(foeInfo.isAdjacentOnly).toBe(true);

      const allySelfInfo = getMoveTargetInfo("AdjacentAllyOrUser");
      expect(allySelfInfo.canTargetFoe).toBe(false);
      expect(allySelfInfo.canTargetAlly).toBe(true);
      expect(allySelfInfo.canTargetSelf).toBe(true);
      expect(allySelfInfo.isAdjacentOnly).toBe(true);

      const anyInfo = getMoveTargetInfo("Any");
      expect(anyInfo.canTargetFoe).toBe(true);
      expect(anyInfo.canTargetAlly).toBe(true);
      expect(anyInfo.canTargetSelf).toBe(false);
      expect(anyInfo.isAdjacentOnly).toBe(false);
    });
  });

  describe("isAdjacent", () => {
    it("checks ally adjacency", () => {
      expect(isAdjacent(0, 1, false, 2)).toBe(true);
      expect(isAdjacent(0, 2, false, 3)).toBe(false);
      expect(isAdjacent(1, 2, false, 3)).toBe(true);
    });

    it("checks foe adjacency in Doubles (2v2)", () => {
      expect(isAdjacent(0, 0, true, 2)).toBe(true);
      expect(isAdjacent(0, 1, true, 2)).toBe(true);
    });

    it("checks foe adjacency in Triples (3v3)", () => {
      // Foe 0 is opposite Player 2. Foe 2 is opposite Player 0.
      // User slot 0 (Left): Foe 1 and 2 are adjacent, Foe 0 is not
      expect(isAdjacent(0, 0, true, 3)).toBe(false);
      expect(isAdjacent(0, 1, true, 3)).toBe(true);
      expect(isAdjacent(0, 2, true, 3)).toBe(true);

      // User slot 1 (Center): All 3 foes are adjacent
      expect(isAdjacent(1, 0, true, 3)).toBe(true);
      expect(isAdjacent(1, 1, true, 3)).toBe(true);
      expect(isAdjacent(1, 2, true, 3)).toBe(true);

      // User slot 2 (Right): Foe 0 and 1 are adjacent, Foe 2 is not
      expect(isAdjacent(2, 0, true, 3)).toBe(true);
      expect(isAdjacent(2, 1, true, 3)).toBe(true);
      expect(isAdjacent(2, 2, true, 3)).toBe(false);
    });
  });

  describe("getValidTargets", () => {
    it("generates correct targets for AdjacentFoe in Doubles", () => {
      const targets = getValidTargets({
        moveTarget: "AdjacentFoe",
        currentSlotIndex: 0,
        battleType: "Doubles",
      });

      expect(targets.map((t) => t.value)).toEqual([1, 2]);
      expect(targets.every((t) => t.type === "foe")).toBe(true);
    });

    it("generates correct targets for AdjacentAllyOrUser in Doubles", () => {
      const targets = getValidTargets({
        moveTarget: "AdjacentAllyOrUser",
        currentSlotIndex: 0,
        battleType: "Doubles",
      });

      // Self at slot 0 -> value -1, Ally at slot 1 -> value -2
      expect(targets.map((t) => t.value)).toEqual([-1, -2]);
      expect(targets[0].type).toBe("self");
      expect(targets[1].type).toBe("ally");
    });

    it("generates correct targets for AdjacentFoe in Triples from slot 0 (Left)", () => {
      const targets = getValidTargets({
        moveTarget: "AdjacentFoe",
        currentSlotIndex: 0,
        battleType: "Triples",
      });

      // Left mon can reach Foe 1 (target 2) and Foe 2 (target 3), but not Foe 0 (target 1)
      expect(targets.map((t) => t.value)).toEqual([2, 3]);
    });

    it("generates correct targets for Any in Triples from slot 0 (Left)", () => {
      const targets = getValidTargets({
        moveTarget: "Any",
        currentSlotIndex: 0,
        battleType: "Triples",
      });

      // Any targets all foes [1, 2, 3] and allies except self [-2, -3]
      expect(targets.map((t) => t.value)).toEqual([1, 2, 3, -2, -3]);
    });

    it("excludes fainted foes from targeting options in Doubles", () => {
      const mockState = {
        field: {
          sides: [
            {
              id: 0,
              players: {
                "player-1": {
                  mons: [{ fainted: false, physical_appearance: { name: "Pikachu" } }],
                },
              },
              active: [{ player: "player-1", mon_index: 0, battle_appearance_index: 0 }],
            },
            {
              id: 1,
              players: {
                "player-2": {
                  mons: [
                    { fainted: false, physical_appearance: { name: "Gengar" } },
                    { fainted: true, physical_appearance: { name: "Dragonite" } },
                  ],
                },
              },
              active: [
                { player: "player-2", mon_index: 0, battle_appearance_index: 0 },
                { player: "player-2", mon_index: 1, battle_appearance_index: 0 },
              ],
            },
          ],
        },
      } as unknown as BattleState;

      const targets = getValidTargets({
        moveTarget: "AdjacentFoe",
        currentSlotIndex: 0,
        battleType: "Doubles",
        battleState: mockState,
      });

      // Dragonite (slot 1) is fainted, so only Gengar (slot 0 -> value 1) is a valid target
      expect(targets.map((t) => t.value)).toEqual([1]);
      expect(targets[0].monName).toBe("Gengar");
    });

    it("excludes fainted allies from targeting options in Doubles", () => {
      const playerData: PlayerBattleData = {
        name: "Ash",
        side: 0,
        mons: [
          {
            name: "Pikachu",
            species: "Pikachu",
            hp: 100,
            max_hp: 100,
            active: true,
            player_active_position: 0,
          },
          {
            name: "Charizard",
            species: "Charizard",
            hp: 0,
            max_hp: 100,
            active: true,
            player_active_position: 1,
          },
        ],
      } as unknown as PlayerBattleData;

      const targets = getValidTargets({
        moveTarget: "AdjacentAlly",
        currentSlotIndex: 0,
        battleType: "Doubles",
        playerData,
      });

      // Ally Charizard in slot 1 has hp === 0, so no valid allies remain
      expect(targets).toEqual([]);
    });

    it("excludes fainted foes when using Any in Triples", () => {
      const mockState = {
        field: {
          sides: [
            {
              id: 0,
              players: {
                "player-1": {
                  mons: [{ fainted: false, physical_appearance: { name: "Pikachu" } }],
                },
              },
              active: [{ player: "player-1", mon_index: 0, battle_appearance_index: 0 }],
            },
            {
              id: 1,
              players: {
                "player-2": {
                  mons: [
                    { fainted: false, physical_appearance: { name: "Gengar" } },
                    { fainted: true, physical_appearance: { name: "Alakazam" } },
                    { fainted: false, physical_appearance: { name: "Dragonite" } },
                  ],
                },
              },
              active: [
                { player: "player-2", mon_index: 0, battle_appearance_index: 0 },
                { player: "player-2", mon_index: 1, battle_appearance_index: 0 },
                { player: "player-2", mon_index: 2, battle_appearance_index: 0 },
              ],
            },
          ],
        },
      } as unknown as BattleState;

      const targets = getValidTargets({
        moveTarget: "Any",
        currentSlotIndex: 0,
        battleType: "Triples",
        battleState: mockState,
      });

      // Foe 0 (Gengar -> 1) and Foe 2 (Dragonite -> 3) are alive, Foe 1 (Alakazam -> 2) is fainted
      expect(targets.filter((t) => t.type === "foe").map((t) => t.value)).toEqual([1, 3]);
    });
  });
});
