import { describe, it, expect } from "vitest";
import { formatNoticeText, formatUiLogEntry } from "./logFormatter";
import { LogCategory } from "battler-log-formatter";
import type { BattleState, UiLogEntry } from "battler-state";

describe("logFormatter", () => {
  describe("formatNoticeText", () => {
    it("should format Ability notices with brackets", () => {
      expect(
        formatNoticeText({
          type: "Ability",
          name: "Drizzle",
          mon: "The opposing Pelipper's",
        }),
      ).toBe("[The opposing Pelipper's Drizzle]");

      expect(
        formatNoticeText({
          type: "Ability",
          name: "Neutralizing Gas",
        }),
      ).toBe("[Neutralizing Gas]");
    });

    it("should format Item notices with brackets", () => {
      expect(
        formatNoticeText({
          type: "Item",
          name: "Leftovers",
          mon: "The opposing Snorlax's",
        }),
      ).toBe("[The opposing Snorlax's Leftovers]");

      expect(
        formatNoticeText({
          type: "Item",
          name: "Life Orb",
        }),
      ).toBe("[Life Orb]");
    });

    it("should format Damage notices with parentheses", () => {
      expect(
        formatNoticeText({
          type: "Damage",
          name: "50/100",
          mon: "The opposing Charizard",
        }),
      ).toBe("(The opposing Charizard lost 50/100 HP)");

      expect(
        formatNoticeText({
          type: "Damage",
          name: "25%",
        }),
      ).toBe("(lost 25% HP)");
    });

    it("should format Heal notices with parentheses", () => {
      expect(
        formatNoticeText({
          type: "Heal",
          name: "15%",
          mon: "The opposing Squirtle",
        }),
      ).toBe("(The opposing Squirtle restored 15% HP)");
    });
  });

  describe("formatUiLogEntry", () => {
    it("should format primary category actions (move)", () => {
      const entry: Partial<UiLogEntry> = {
        title: "move",
        values: {
          mon: { Active: { side: 0, position: 0, name: "Pikachu", player: "p1" } },
          name: "Thunderbolt",
        },
      };

      const result = formatUiLogEntry(entry as UiLogEntry, undefined, "p1");
      expect(result.length).toBe(1);
      expect(result[0].kind).toBe("message");
      if (result[0].kind === "message") {
        expect(result[0].category).toBe(LogCategory.Primary);
      }
    });

    it("should format plain damage as a damage notice", () => {
      const entry: Partial<UiLogEntry> = {
        title: "damage",
        values: {
          mon: { Active: { position: 0, name: "Charmander", player: "p2", side: 1 } },
          health: [0n, 100n],
          damage: [100n, 100n],
        },
      };

      const result = formatUiLogEntry(entry as UiLogEntry, undefined, "p1");
      expect(result.length).toBe(1);
      expect(result[0].kind).toBe("notice");
      if (result[0].kind === "notice") {
        expect(result[0].notice.type).toBe("Damage");
        expect(formatNoticeText(result[0].notice)).toBe("(The opposing Charmander lost 100% HP)");
      }
    });

    it("should order ability notices before messages", () => {
      const entry: Partial<UiLogEntry> = {
        title: "ability",
        values: {
          mon: { Active: { position: 0, name: "Blastoise", player: "p2", side: 1 } },
          ability: "Neutralizing Gas",
        },
      };

      const result = formatUiLogEntry(entry as UiLogEntry, undefined, "p1");
      expect(result.length).toBe(2);
      expect(result[0].kind).toBe("notice");
      if (result[0].kind === "notice") {
        expect(result[0].notice.type).toBe("Ability");
        expect(formatNoticeText(result[0].notice)).toBe("[The opposing Blastoise's Neutralizing Gas]");
      }
      expect(result[1].kind).toBe("message");
      if (result[1].kind === "message") {
        expect(result[1].category).toBe(LogCategory.Secondary);
      }
    });

    it("should order damage notices after messages for damage with from", () => {
      const entry: Partial<UiLogEntry> = {
        title: "damage",
        values: {
          mon: { Active: { position: 0, name: "Charizard", player: "p2", side: 1 } },
          from: "move:Stealth Rock",
          health: [50n, 100n],
          damage: [50n, 100n],
        },
      };

      const result = formatUiLogEntry(entry as UiLogEntry, undefined, "p1");
      expect(result.length).toBe(2);
      expect(result[0].kind).toBe("message");
      if (result[0].kind === "message") {
        expect(result[0].category).toBe(LogCategory.Secondary);
      }
      expect(result[1].kind).toBe("notice");
      if (result[1].kind === "notice") {
        expect(result[1].notice.type).toBe("Damage");
        expect(formatNoticeText(result[1].notice)).toBe("(The opposing Charizard lost 50% HP)");
      }
    });

    it("should auto-capitalize you and preserve player names in switch logs", () => {
      const selfEntry: Partial<UiLogEntry> = {
        title: "switch",
        player: "p1",
        side: 0,
        values: {
          name: "Walking Wake",
          player: "p1",
        },
      };

      const foeEntry: Partial<UiLogEntry> = {
        title: "switch",
        player: "ai-random-1",
        side: 1,
        values: {
          name: "Walking Wake",
          player: "ai-random-1",
          mon: { Active: { position: 0, name: "Walking Wake", player: "ai-random-1", side: 1 } },
        },
      };

      const selfResult = formatUiLogEntry(selfEntry as UiLogEntry, undefined, "p1");
      expect(selfResult.length).toBe(1);
      if (selfResult[0].kind === "message") {
        expect(selfResult[0].message.context.__CAPITALIZED_PLAYER).toEqual({ text: "You" });
      }

      const foeResult = formatUiLogEntry(foeEntry as UiLogEntry, undefined, "p1");
      expect(foeResult.length).toBe(1);
      if (foeResult[0].kind === "message") {
        expect(foeResult[0].message.context.PLAYER).toEqual({ text: "ai-random-1" });
        expect(foeResult[0].message.context.__CAPITALIZED_PLAYER).toBeUndefined();
      }
    });

    it("should preserve proper noun side name in win logs without auto-capitalization", () => {
      const winEntry: Partial<UiLogEntry> = {
        title: "win",
        side: 1,
        values: {},
      };
      const state = {
        field: {
          sides: [
            { name: "jackson", players: { p1: { name: "jackson" } } },
            { name: "ai-random-1", players: { p2: { name: "ai-random-1" } } },
          ],
        },
      };

      const result = formatUiLogEntry(winEntry as UiLogEntry, state as unknown as BattleState, "p1");
      expect(result.length).toBe(1);
      if (result[0].kind === "message") {
        expect(result[0].message.context.SIDE_NAME).toEqual({ text: "ai-random-1" });
        expect(result[0].message.context.__CAPITALIZED_SIDE_NAME).toBeUndefined();
      }
    });

    it("should format turn logs as kind turn", () => {
      const turnEntry: Partial<UiLogEntry> = {
        title: "turn",
        values: {
          turn: 5n,
        },
      };

      const result = formatUiLogEntry(turnEntry as UiLogEntry, undefined, "p1");
      expect(result.length).toBe(1);
      expect(result[0]).toEqual({
        kind: "turn",
        turn: "5",
      });
    });

    it("should format continue and time logs as divider continue", () => {
      const continueEntry: Partial<UiLogEntry> = {
        title: "continue",
        values: {},
      };
      const timeEntry: Partial<UiLogEntry> = {
        title: "time",
        values: { value: "100" },
      };

      const contResult = formatUiLogEntry(continueEntry as UiLogEntry, undefined, "p1");
      expect(contResult).toEqual([{ kind: "divider", subtype: "continue" }]);

      const timeResult = formatUiLogEntry(timeEntry as UiLogEntry, undefined, "p1");
      expect(timeResult).toEqual([{ kind: "divider", subtype: "continue" }]);
    });

    it("should format residual logs as divider residual", () => {
      const residualEntry: Partial<UiLogEntry> = {
        title: "residual",
        values: {},
      };

      const result = formatUiLogEntry(residualEntry as UiLogEntry, undefined, "p1");
      expect(result).toEqual([{ kind: "divider", subtype: "residual" }]);
    });

    it("should format ability notices with correct unified capitalization in multi and single battles", () => {
      const multiState = {
        battle_type: "Multi",
        field: {
          sides: [
            { name: "Side 1", players: { p1: { name: "Player 1" } } } as any,
            { name: "Side 2", players: { "ai-random-1": { name: "ai-random-1" } } } as any,
          ],
        },
      } as unknown as BattleState;

      const multiEntry: Partial<UiLogEntry> = {
        title: "activate",
        values: {
          mon: { Active: { side: 1, position: 0, name: "Great Tusk", player: "ai-random-1" } },
          ability: "Protosynthesis",
        },
      };

      const multiResult = formatUiLogEntry(multiEntry as UiLogEntry, multiState as BattleState, "p1");
      expect(multiResult.length).toBe(2);
      expect(multiResult[0].kind).toBe("notice");
      if (multiResult[0].kind === "notice") {
        expect(formatNoticeText(multiResult[0].notice)).toBe(
          "[ai-random-1's Great Tusk's Protosynthesis]"
        );
      }

      const singleEntry: Partial<UiLogEntry> = {
        title: "activate",
        values: {
          mon: { Active: { side: 1, position: 0, name: "Ninetales", player: "p2" } },
          ability: "Drought",
        },
      };

      const singleResult = formatUiLogEntry(singleEntry as UiLogEntry, undefined, "p1");
      expect(singleResult.length).toBe(1);
      expect(singleResult[0].kind).toBe("notice");
      if (singleResult[0].kind === "notice") {
        expect(formatNoticeText(singleResult[0].notice)).toBe(
          "[The opposing Ninetales's Drought]"
        );
      }
    });

    it("should format timer logs as message with LogCategory.Hint", () => {
      const timerEntry: Partial<UiLogEntry> = {
        title: "timer",
        values: {
          source: "-battlerservice",
          player: "p1",
          warning: true,
          remainingsecs: 10n,
        },
      };

      const result = formatUiLogEntry(timerEntry as UiLogEntry, undefined, "p1");
      expect(result.length).toBe(1);
      expect(result[0].kind).toBe("message");
      if (result[0].kind === "message") {
        expect(result[0].category).toBe(LogCategory.Hint);
      }
    });
  });
});
