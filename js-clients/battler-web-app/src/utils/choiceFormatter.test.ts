import { describe, expect, it } from "vitest";
import { formatTurnChoice, parseChoiceString, resolveTargetName } from "./choiceFormatter";

describe("choiceFormatter utility", () => {
  describe("parseChoiceString", () => {
    it("parses move action without target or modifiers", () => {
      const parsed = parseChoiceString("move 0");
      expect(parsed).toEqual({
        type: "move",
        moveIndex: 0,
        targetVal: null,
        mega: false,
        zmove: false,
        ultra: false,
        dyna: false,
        tera: false,
      });
    });

    it("parses move action with target and mega modifier", () => {
      const parsed = parseChoiceString("move 1, 2, mega");
      expect(parsed).toEqual({
        type: "move",
        moveIndex: 1,
        targetVal: 2,
        mega: true,
        zmove: false,
        ultra: false,
        dyna: false,
        tera: false,
      });
    });

    it("parses move action with target and multiple modifiers", () => {
      const parsed = parseChoiceString("move 0, -1, tera, mega");
      expect(parsed).toEqual({
        type: "move",
        moveIndex: 0,
        targetVal: -1,
        mega: true,
        zmove: false,
        ultra: false,
        dyna: false,
        tera: true,
      });
    });

    it("parses switch action", () => {
      const parsed = parseChoiceString("switch 3");
      expect(parsed).toEqual({
        type: "switch",
        switchPosition: 3,
      });
    });

    it("parses pass action", () => {
      const parsed = parseChoiceString("pass");
      expect(parsed).toEqual({
        type: "pass",
      });
    });

    it("parses shift action", () => {
      const parsed = parseChoiceString("shift");
      expect(parsed).toEqual({
        type: "shift",
      });
    });
  });

  describe("resolveTargetName", () => {
    it("returns null for null target", () => {
      expect(resolveTargetName(null, 0)).toBeNull();
    });

    it("formats foe target correctly", () => {
      const targetName = resolveTargetName(1, 0);
      expect(targetName).toContain("Foe 1");
    });

    it("formats self target correctly", () => {
      const targetName = resolveTargetName(-1, 0);
      expect(targetName).toContain("Self");
    });
  });

  describe("formatTurnChoice", () => {
    it("formats move choice summary cleanly", () => {
      const formatted = formatTurnChoice("move 0, 1, mega", 0, {
        type: "turn",
        active: [
          {
            team_position: 0,
            moves: [{ id: "flamethrower", name: "Flamethrower", type: "Fire", pp: 15, max_pp: 15, disabled: false, target: "Normal" }],
            z_moves: [],
            max_moves: [],
            trapped: false,
            can_mega_evolve: true,
            can_z_move: false,
            can_ultra_burst: false,
            can_dynamax: false,
            can_terastallize: false,
            locked_into_move: false,
          },
        ],
        allies: [],
      });

      expect(formatted.actionName).toBe("Flamethrower");
      expect(formatted.modifiers).toContain("Mega");
      expect(formatted.summaryText).toContain("Flamethrower");
      expect(formatted.summaryText).toContain("Mega");
    });

    it("formats forced switch pass choice summary cleanly", () => {
      const formatted = formatTurnChoice("pass", 2, {
        type: "switch",
        needs_switch: [0, 1, 2],
      });

      expect(formatted.actionType).toBe("pass");
      expect(formatted.summaryText).toBe("Leave Empty (Pass)");
    });
  });
});
