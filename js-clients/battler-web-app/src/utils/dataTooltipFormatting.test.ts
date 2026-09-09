import { describe, expect, it } from "vitest";
import {
  formatAccuracy,
  formatDrain,
  formatFraction,
  formatHitEffect,
  formatMoveFlags,
  formatMoveTarget,
  formatMultihit,
  formatRecoil,
  formatSecondaryEffect,
} from "./dataTooltipFormatting";

describe("dataTooltipFormatting", () => {
  describe("formatAccuracy", () => {
    it("formats numeric accuracy", () => {
      expect(formatAccuracy(100)).toBe("100%");
      expect(formatAccuracy(85)).toBe("85%");
    });

    it("formats exempt / bypass accuracy as em dash", () => {
      expect(formatAccuracy("exempt")).toBe("—");
    });
  });

  describe("formatMoveTarget", () => {
    it("formats various targets properly", () => {
      expect(formatMoveTarget("Normal")).toBe("Normal");
      expect(formatMoveTarget("AdjacentFoe")).toBe("AdjacentFoe");
      expect(formatMoveTarget("User")).toBe("User");
      expect(formatMoveTarget("AllAdjacentFoes")).toBe("AllAdjacentFoes");
      expect(formatMoveTarget("Field")).toBe("Field");
    });
  });

  describe("formatFraction", () => {
    it("formats numbers to percentages", () => {
      expect(formatFraction(0.5)).toBe("50%");
      expect(formatFraction(1)).toBe("100%");
      expect(formatFraction(0.333)).toBe("33%");
    });

    it("formats strings and fractional strings", () => {
      expect(formatFraction("1/2")).toBe("50%");
      expect(formatFraction("1/3")).toBe("33%");
      expect(formatFraction("75%")).toBe("75%");
      expect(formatFraction(null)).toBe("");
    });
  });

  describe("formatMultihit", () => {
    it("formats single number or ranges", () => {
      expect(formatMultihit(null)).toBeNull();
      expect(formatMultihit(2)).toBe("Hits 2");
      expect(formatMultihit([2, 5])).toBe("Hits 2–5");
    });
  });

  describe("formatRecoil", () => {
    it("formats struggle recoil", () => {
      expect(formatRecoil({ base: "Damage", percent: "1/4", struggle: true })).toBe(
        "25% struggle recoil",
      );
    });

    it("formats regular recoil", () => {
      expect(formatRecoil({ base: "Damage", percent: "1/3", struggle: false })).toBe(
        "33% recoil",
      );
    });
  });

  describe("formatDrain", () => {
    it("formats drain percentages", () => {
      expect(formatDrain(null)).toBeNull();
      expect(formatDrain("1/2")).toBe("Recovers 50%");
    });
  });

  describe("formatMoveFlags", () => {
    it("alphabetizes raw flags", () => {
      expect(formatMoveFlags(["Protect", "Punch", "Contact"])).toEqual([
        "Contact",
        "Protect",
        "Punch",
      ]);
    });
  });

  describe("formatHitEffect", () => {
    it("formats status and stat boosts directly", () => {
      const effect = {
        boosts: {
          atk: -1,
          def: 0,
          spa: 0,
          spd: 0,
          spe: 2,
          acc: 0,
          eva: 0,
        },
        heal_percent: null,
        status: "brn",
        volatile_status: null,
        side_condition: null,
        slot_condition: null,
        weather: null,
        pseudo_weather: null,
        terrain: null,
        force_switch: false,
      };
      const lines = formatHitEffect(effect, "target");
      expect(lines).toContain("brn (target)");
      expect(lines).toContain("-1 Atk (target)");
      expect(lines).toContain("+2 Spe (target)");
    });
  });

  describe("formatSecondaryEffect", () => {
    it("formats secondary effect with chance", () => {
      const sec = {
        chance: "1/10",
        apply_once: false,
        target: {
          boosts: null,
          heal_percent: null,
          status: "par",
          volatile_status: null,
          side_condition: null,
          slot_condition: null,
          weather: null,
          pseudo_weather: null,
          terrain: null,
          force_switch: false,
        },
        user: null,
        source_effect: null,
        effect: null,
      };
      expect(formatSecondaryEffect(sec)).toBe("10%: par (target)");
    });

    it("formats all-stat omniboost compactly", () => {
      const omni = {
        chance: null,
        apply_once: true,
        target: null,
        user: {
          boosts: {
            atk: 1,
            def: 1,
            spa: 1,
            spd: 1,
            spe: 1,
            acc: 0,
            eva: 0,
          },
          heal_percent: null,
          status: null,
          volatile_status: null,
          side_condition: null,
          slot_condition: null,
          weather: null,
          pseudo_weather: null,
          terrain: null,
          force_switch: false,
        },
        source_effect: null,
        effect: null,
      };
      expect(formatSecondaryEffect(omni)).toBe("+1 Atk, Def, SpA, SpD, Spe (user)");
    });
  });
});
