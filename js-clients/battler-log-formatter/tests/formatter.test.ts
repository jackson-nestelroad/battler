import { describe, it, expect } from "vitest";
import { LogFormatter, stringifyLog } from "../src/formatter.js";
import { LogCategory } from "../src/types.js";

describe("LogFormatter", () => {
  it("should format string log correctly", () => {
    const formatter = new LogFormatter();
    // Simulate UiLogEntry enum using any
    const entry: any = "Tie";
    const result = formatter.format(entry);
    
    expect(result.length).toBeGreaterThan(0);
    const log = result[result.length - 1];
    expect(log.category).toBe(LogCategory.Primary);
    expect(stringifyLog(log)).toBe("The battle resulted in a tie!");
  });

  it("should format complex Move log", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const entry: any = {
      Move: {
        name: "Thunderbolt",
        mon: {
          Active: { side: 0, position: 0, name: "Pikachu", player: "p2" }
        }
      }
    };
    
    // Mock state
    const state: any = {
      field: {
        sides: [
          {
            active: [{ player: "p2", mon_index: 0 }],
            players: { p2: { id: "p2", mons: [{ physical_appearance: { name: "Pikachu" } }] } }
          }
        ]
      }
    };
    
    const result = formatter.format(entry, state);
    expect(result.length).toBeGreaterThan(0);
    const log = result[result.length - 1];
    expect(log.category).toBe(LogCategory.Primary);
    
    // Because localPlayerId is p1, and the mon belongs to p2, it should format as a foe.
    // wait, we mocked the state to have player id 'p2', which is not 'p1'.
    // so it should use mon.foe -> "The opposing Pikachu"
    expect(log.context.MON).toEqual({ text: "the opposing Pikachu", id: "p2-active-0", noAutoCapitalize: false });
    expect(log.context.MOVE).toBe("Thunderbolt");
    
    expect(stringifyLog(log)).toBe("The opposing Pikachu used Thunderbolt!");
  });

  it("should handle battle type disambiguation for critical hits", () => {
    const formatter = new LogFormatter();
    const entry: any = {
      Crit: {
        mon: {
          Active: { position: 0, name: "Pikachu", player: "p2" }
        }
      }
    };
    
    // Singles State
    const singlesState: any = {
      settings: { battle_type: "Singles" },
      field: {
        sides: [
          { players: { p1: { name: "Player 1" } } },
          { players: { p2: { name: "Player 2" } } }
        ]
      }
    };
    
    // Multi State
    const multiState: any = {
      settings: { battle_type: "Multi" },
      field: {
        sides: [
          { players: { p1: { name: "Player 1" }, p3: { name: "Player 3" } } },
          { players: { p2: { name: "Player 2" }, p4: { name: "Player 4" } } }
        ]
      }
    };
    
    // Doubles State
    const doublesState: any = {
      settings: { battle_type: "Doubles" },
      field: {
        sides: [
          { players: { p1: { name: "Player 1" } } },
          { players: { p2: { name: "Player 2" } } }
        ]
      }
    };

    const singlesResult = formatter.format(entry, singlesState);
    const doublesResult = formatter.format(entry, doublesState);
    const multiResult = formatter.format(entry, multiState);

    const singlesLog = singlesResult[singlesResult.length - 1];
    const doublesLog = doublesResult[doublesResult.length - 1];
    const multiLog = multiResult[multiResult.length - 1];

    // Singles omitting the mon
    expect(singlesLog.key).toBe("crit__battletype_singles");
    expect(stringifyLog(singlesLog)).toBe("A critical hit!");
    
    // Doubles requiring the mon, but using "the opposing" since there's only one opponent
    expect(doublesLog.key).toBe("crit");
    expect(stringifyLog(doublesLog)).toBe("A critical hit on the opposing Pikachu!");
    
    // Multi requiring the mon (since there are multiple opponents)
    expect(multiLog.key).toBe("crit");
    expect(stringifyLog(multiLog)).toBe("A critical hit on Player 2's Pikachu!");
  });
});
