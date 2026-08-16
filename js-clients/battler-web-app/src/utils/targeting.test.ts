import { describe, expect, it } from "vitest";
import { getMoveTargetInfo, getValidTargets, isAdjacent } from "./targeting";

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
  });
});
