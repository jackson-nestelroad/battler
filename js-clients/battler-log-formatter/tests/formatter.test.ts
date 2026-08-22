import { describe, it, expect } from "vitest";
import { LogFormatter, stringifyLog } from "../src/formatter.js";
import { LogCategory } from "../src/types.js";

describe("LogFormatter", () => {
  it("should format string log correctly", () => {
    const formatter = new LogFormatter();
    // Simulate UiLogEntry enum using any
    const entry: any = "Tie";
    const result = formatter.format(entry);
    expect(result).not.toBeNull();
    const log = result!.message!;
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
    expect(result).not.toBeNull();
    const log = result!.message!;
    expect(log.category).toBe(LogCategory.Primary);
    
    // Because localPlayerId is p1, and the mon belongs to p2, it should format as a foe.
    // wait, we mocked the state to have player id 'p2', which is not 'p1'.
    // so it should use mon.foe -> "The opposing Pikachu"
    expect(log.context.__CAPITALIZED_MON).toEqual({ text: "The opposing Pikachu", monRef: { Active: { player: "p2", position: 0, name: "Pikachu", side: 0 } } });
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

    const singlesLog = singlesResult!.message!;
    const doublesLog = doublesResult!.message!;
    const multiLog = multiResult!.message!;

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

  it("should format boost magnitudes correctly", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const mockState: any = {
      settings: { battle_type: "Singles" },
      field: {
        sides: [
          { players: { p1: { name: "Player 1" } } },
          { players: { p2: { name: "Player 2" } } }
        ]
      }
    };
    
    const getBoostLog = (by: number, max?: boolean) => {
      const entry: any = {
        Boost: {
          mon: { Active: { position: 0, name: "Snorlax", player: "p1" } },
          stat: "atk",
          by: by
        }
      };
      if (max) entry.Boost.max = true;
      const result = formatter.format(entry, mockState);
      return stringifyLog(result!.message!);
    };

    expect(getBoostLog(0)).toBe("Snorlax's Attack can't go any higher!");
    expect(getBoostLog(1)).toBe("Snorlax's Attack rose!");
    expect(getBoostLog(2)).toBe("Snorlax's Attack rose sharply!");
    expect(getBoostLog(3)).toBe("Snorlax's Attack rose drastically!");
    expect(getBoostLog(12)).toBe("Snorlax's Attack rose drastically!");
    expect(getBoostLog(12, true)).toBe("Snorlax maximized its Attack!");
  });

  it("should format unboost magnitudes correctly", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const mockState: any = {
      settings: { battle_type: "Singles" },
      field: {
        sides: [
          { players: { p1: { name: "Player 1" } } },
          { players: { p2: { name: "Player 2" } } }
        ]
      }
    };
    
    const getUnboostLog = (by: number, min?: boolean) => {
      const entry: any = {
        Unboost: {
          mon: { Active: { position: 0, name: "Snorlax", player: "p1" } },
          stat: "def",
          by: by
        }
      };
      if (min) entry.Unboost.min = true;
      const result = formatter.format(entry, mockState);
      return stringifyLog(result!.message!);
    };

    expect(getUnboostLog(0)).toBe("Snorlax's Defense won't go any lower!");
    expect(getUnboostLog(1)).toBe("Snorlax's Defense fell!");
    expect(getUnboostLog(2)).toBe("Snorlax's Defense harshly fell!");
    expect(getUnboostLog(3)).toBe("Snorlax's Defense severely fell!");
    expect(getUnboostLog(4)).toBe("Snorlax's Defense severely fell!");
    expect(getUnboostLog(12, true)).toBe("Snorlax minimized its Defense!");
  });
});
