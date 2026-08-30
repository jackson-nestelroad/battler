import { describe, it, expect } from "vitest";
import { LogFormatter, stringifyLog } from "../src/formatter.js";
import { BattleState, UiLogEntry } from "battler-state";
import { LogCategory } from "../src/types.js";

describe("LogFormatter", () => {
  it("should format string log correctly", () => {
    const formatter = new LogFormatter();
    const entry: Partial<UiLogEntry> = {
      title: "tie",
      values: {}
    };
    const result = formatter.format(entry as UiLogEntry);
    expect(result).not.toBeNull();
    expect(result!.messages.length).toBe(1);
    const log = result!.messages[0];
    expect(log.category).toBe(LogCategory.Primary);
    expect(stringifyLog(log)).toBe("The battle resulted in a tie!");
  });

  it("should format complex Move log", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const entry: Partial<UiLogEntry> = {
      title: "move",
      values: {
        mon: { Active: { side: 0, position: 0, name: "Pikachu", player: "p2" } },
        name: "Thunderbolt"
      }
    };
    
    const state = {
      field: {
        sides: [
          {
            active: [{ player: "p2", mon_index: 0 }],
            players: { p2: { id: "p2", mons: [{ physical_appearance: { name: "Pikachu" } }] } }
          }
        ]
      }
    };
    
    const result = formatter.format(entry as unknown as UiLogEntry, state as unknown as BattleState);
    expect(result).not.toBeNull();
    expect(result!.messages.length).toBe(1);
    const log = result!.messages[0];
    expect(log.category).toBe(LogCategory.Primary);
    expect(log.context.__CAPITALIZED_MON).toEqual({ text: "The opposing Pikachu", monRef: { Active: { player: "p2", position: 0, name: "Pikachu", side: 0 } } });
    expect(log.context.MOVE).toBe("Thunderbolt");
    expect(stringifyLog(log)).toBe("The opposing Pikachu used Thunderbolt!");
  });

  it("should handle battle type disambiguation for critical hits", () => {
    const formatter = new LogFormatter();
    const entry: Partial<UiLogEntry> = {
      title: "crit",
      values: {
        mon: { Active: { position: 0, name: "Pikachu", player: "p2", side: 0 } }
      }
    };
    
    const singlesState = {
      battle_type: "Singles",
      field: {
        sides: [
          { players: { p1: { name: "Player 1" } } },
          { players: { p2: { name: "Player 2" } } }
        ]
      }
    };
    
    const multiState = {
      battle_type: "Multi",
      field: {
        sides: [
          { players: { p1: { name: "Player 1" }, p3: { name: "Player 3" } } },
          { players: { p2: { name: "Player 2" }, p4: { name: "Player 4" } } }
        ]
      }
    };
    
    const doublesState = {
      battle_type: "Doubles",
      field: {
        sides: [
          { players: { p1: { name: "Player 1" } } },
          { players: { p2: { name: "Player 2" } } }
        ]
      }
    };

    const singlesResult = formatter.format(entry as unknown as UiLogEntry, singlesState as unknown as BattleState);
    const doublesResult = formatter.format(entry as unknown as UiLogEntry, doublesState as unknown as BattleState);
    const multiResult = formatter.format(entry as unknown as UiLogEntry, multiState as unknown as BattleState);

    const singlesLog = singlesResult!.messages[0];
    const doublesLog = doublesResult!.messages[0];
    const multiLog = multiResult!.messages[0];

    expect(singlesLog.key).toBe("crit__battletype_singles");
    expect(stringifyLog(singlesLog)).toBe("A critical hit!");
    
    expect(doublesLog.key).toBe("crit");
    expect(stringifyLog(doublesLog)).toBe("A critical hit on the opposing Pikachu!");
    
    expect(multiLog.key).toBe("crit");
    expect(stringifyLog(multiLog)).toBe("A critical hit on Player 2's Pikachu!");
  });

  it("should format boost magnitudes correctly", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const mockState = {
      battle_type: "Singles",
      field: {
        sides: [
          { players: { p1: { name: "Player 1" } } },
          { players: { p2: { name: "Player 2" } } }
        ]
      }
    };
    
    const getBoostLog = (by: number, max?: boolean) => {
      const entry: Partial<UiLogEntry> = {
        title: "boost",
        values: {
          mon: { Active: { position: 0, name: "Snorlax", player: "p1", side: 0 } },
          stat: "atk",
          by: by
        }
      };
      if (max) entry.values!["max"] = true;
      const result = formatter.format(entry as unknown as UiLogEntry, mockState as unknown as BattleState);
      return stringifyLog(result!.messages[0]);
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
    const mockState = {
      battle_type: "Singles",
      field: {
        sides: [
          { players: { p1: { name: "Player 1" } } },
          { players: { p2: { name: "Player 2" } } }
        ]
      }
    };
    
    const getUnboostLog = (by: number, min?: boolean) => {
      const entry: Partial<UiLogEntry> = {
        title: "unboost",
        values: {
          mon: { Active: { position: 0, name: "Snorlax", player: "p1", side: 0 } },
          stat: "def",
          by: by
        }
      };
      if (min) entry.values!["min"] = true;
      const result = formatter.format(entry as unknown as UiLogEntry, mockState as unknown as BattleState);
      return stringifyLog(result!.messages[0]);
    };

    expect(getUnboostLog(0)).toBe("Snorlax's Defense won't go any lower!");
    expect(getUnboostLog(1)).toBe("Snorlax's Defense fell!");
    expect(getUnboostLog(2)).toBe("Snorlax's Defense harshly fell!");
    expect(getUnboostLog(3)).toBe("Snorlax's Defense severely fell!");
    expect(getUnboostLog(4)).toBe("Snorlax's Defense severely fell!");
    expect(getUnboostLog(12, true)).toBe("Snorlax minimized its Defense!");
  });

  it("should categorize plain damage as Hint and damage with from as Secondary", () => {
    const formatter = new LogFormatter();
    const plainDamageEntry: Partial<UiLogEntry> = {
      title: "damage",
      values: {
        mon: { Active: { position: 0, name: "Charmander", player: "p2", side: 1 } },
        health: [0, 100]
      }
    };

    const plainResult = formatter.format(plainDamageEntry as UiLogEntry);
    expect(plainResult).toBeNull();

    const stealthRockDamageEntry: Partial<UiLogEntry> = {
      title: "damage",
      values: {
        mon: { Active: { position: 0, name: "Charizard", player: "p2", side: 1 } },
        from: "move:Stealth Rock",
        health: [50, 100]
      }
    };

    const srResult = formatter.format(stealthRockDamageEntry as UiLogEntry);
    expect(srResult).not.toBeNull();
    expect(srResult!.messages.length).toBe(1);
    expect(srResult!.messages[0].category).toBe(LogCategory.Secondary);
    expect(stringifyLog(srResult!.messages[0])).toBe("Pointed stones dug into the opposing Charizard!");
  });

  it("should disambiguate single vs multiple stats for fail what:unboost", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const singleStatEntry: Partial<UiLogEntry> = {
      title: "fail",
      values: {
        mon: { Active: { position: 0, name: "Beldum", player: "p1", side: 0 } },
        what: "unboost",
        boosts: "atk",
        from: "ability:Clear Body"
      }
    };

    const singleResult = formatter.format(singleStatEntry as UiLogEntry);
    expect(singleResult).not.toBeNull();
    expect(stringifyLog(singleResult!.messages[0])).toBe("Beldum's Clear Body prevents its Attack from being lowered!");

    const multiStatEntry: Partial<UiLogEntry> = {
      title: "fail",
      values: {
        mon: { Active: { position: 0, name: "Beldum", player: "p1", side: 0 } },
        what: "unboost",
        boosts: "atk,def",
        from: "ability:Clear Body"
      }
    };

    const multiResult = formatter.format(multiStatEntry as UiLogEntry);
    expect(multiResult).not.toBeNull();
    expect(stringifyLog(multiResult!.messages[0])).toBe("Beldum's Clear Body prevents its stats from being lowered!");
  });

  it("should preserve species and forme information in formechange", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const entry: Partial<UiLogEntry> = {
      title: "formechange",
      values: {
        mon: { Active: { position: 0, name: "Aegislash", player: "p1", side: 0 } },
        species: "Aegislash-Blade",
        from: "ability:Stance Change"
      }
    };

    const result = formatter.format(entry as UiLogEntry);
    expect(result).not.toBeNull();
    expect(result!.messages.length).toBe(1);
    expect(stringifyLog(result!.messages[0])).toBe("Aegislash transformed!");
  });

  it("should emit two messages for Mega Evolution", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const entry: Partial<UiLogEntry> = {
      title: "mega",
      values: {
        mon: { Active: { position: 0, name: "Venusaur", player: "p1", side: 0 } },
        species: "Venusaur-Mega",
        from: "item:Venusaurite"
      }
    };

    const result = formatter.format(entry as UiLogEntry);
    expect(result).not.toBeNull();
    expect(result!.messages.length).toBe(2);
    expect(stringifyLog(result!.messages[0])).toBe("Venusaur's Venusaurite is reacting to Your Mega Bracelet!");
    expect(stringifyLog(result!.messages[1])).toBe("Venusaur Mega Evolved!");
  });

  it("should emit two messages for Primal Reversion", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const entry: Partial<UiLogEntry> = {
      title: "primal",
      values: {
        mon: { Active: { position: 0, name: "Kyogre", player: "p1", side: 0 } },
        species: "Kyogre-Primal"
      }
    };

    const result = formatter.format(entry as UiLogEntry);
    expect(result).not.toBeNull();
    expect(result!.messages.length).toBe(2);
    expect(stringifyLog(result!.messages[0])).toBe("Primal Reversion! It regained its true power!");
    expect(stringifyLog(result!.messages[1])).toBe("Kyogre underwent Primal Reversion!");
  });

  it("should format debug logs correctly with Hint category", () => {
    const formatter = new LogFormatter();
    const entry: Partial<UiLogEntry> = {
      title: "debug",
      values: {
        event: "ModifyDamage",
        error: "Unexpected state connector"
      }
    };

    const result = formatter.format(entry as UiLogEntry);
    expect(result).not.toBeNull();
    expect(result!.messages[0].category).toBe(LogCategory.Hint);
    expect(stringifyLog(result!.messages[0])).toBe("[Debug] ModifyDamage: Unexpected state connector");
  });

  it("should format wild Pokémon as 'the wild' instead of 'the opposing'", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const wildState = {
      field: {
        sides: [
          { players: { p1: { id: "p1", name: "Jackson" } } },
          { players: { "wild-1": { id: "wild-1", name: "Wild", player_type: "wild" } } }
        ]
      }
    };

    const entry: Partial<UiLogEntry> = {
      title: "move",
      values: {
        mon: { Active: { position: 0, name: "Pikachu", player: "wild-1", side: 1 } },
        name: "Thunderbolt"
      }
    };

    const result = formatter.format(entry as UiLogEntry, wildState as unknown as BattleState);
    expect(result).not.toBeNull();
    expect(stringifyLog(result!.messages[0])).toBe("The wild Pikachu used Thunderbolt!");
  });

  it("should format Z-status move names with Z- prefix", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const entry: Partial<UiLogEntry> = {
      title: "move",
      values: {
        mon: { Active: { position: 0, name: "Pikachu", player: "p1", side: 0 } },
        name: "Thunder Wave",
        zpower: true
      }
    };

    const result = formatter.format(entry as UiLogEntry);
    expect(result).not.toBeNull();
    expect(stringifyLog(result!.messages[0])).toBe("Pikachu used Z-Thunder Wave!");
  });

  it("should emit two messages for Magic Bounce reflection", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const entry: Partial<UiLogEntry> = {
      title: "move",
      values: {
        mon: { Active: { position: 0, name: "Espeon", player: "p2", side: 1 } },
        name: "Toxic",
        from: "ability:Magic Bounce"
      }
    };

    const result = formatter.format(entry as UiLogEntry);
    expect(result).not.toBeNull();
    expect(result!.messages.length).toBe(2);
    expect(stringifyLog(result!.messages[0])).toBe("The opposing Espeon bounced the Toxic back!");
    expect(stringifyLog(result!.messages[1])).toBe("The opposing Espeon used Toxic!");
  });

  it("should emit two messages for switch when replacing an active mon", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const state = {
      field: {
        sides: [
          {
            active: [{ position: 0, name: "Pikachu", player: "p1", side: 0 }],
            players: { p1: { id: "p1", name: "Jackson" } }
          }
        ]
      }
    };

    const entry: Partial<UiLogEntry> = {
      title: "switch",
      player: "p1",
      side: 0,
      values: {
        name: "Charmander",
        position: 0,
        player: "p1"
      }
    };

    const result = formatter.format(entry as UiLogEntry, state as unknown as BattleState);
    expect(result).not.toBeNull();
    expect(result!.messages.length).toBe(2);
    expect(stringifyLog(result!.messages[0])).toBe("You withdrew Pikachu!");
    expect(stringifyLog(result!.messages[1])).toBe("You sent out Charmander!");
  });

  it("should generalize magnitude formatting dynamically", () => {
    const formatter = new LogFormatter();
    const getMagnitudeLog = (mag: number) => {
      const entry: Partial<UiLogEntry> = {
        title: "activate",
        values: {
          move: "Magnitude",
          magnitude: mag
        }
      };
      const result = formatter.format(entry as UiLogEntry);
      return stringifyLog(result!.messages[0]);
    };

    expect(getMagnitudeLog(4)).toBe("Magnitude 4!");
    expect(getMagnitudeLog(7)).toBe("Magnitude 7!");
    expect(getMagnitudeLog(10)).toBe("Magnitude 10!");
  });
});
