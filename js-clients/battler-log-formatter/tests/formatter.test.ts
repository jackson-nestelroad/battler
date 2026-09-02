import { describe, it, expect } from "vitest";
import { LogFormatter, stringifyLog } from "../src/formatter.js";
import { mapUiLogEntry } from "../src/mapper.js";
import { BattleState, UiLogEntry } from "battler-state";
import { LogCategory } from "../src/types.js";

describe("LogFormatter", () => {
  it("should format tie log correctly", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const entry: Partial<UiLogEntry> = {
      title: "tie",
      values: {}
    };
    const state = {
      field: {
        sides: [
          { name: "Team Rocket", players: { p1: { name: "Player 1" } } },
          { name: "Team Magma", players: { p2: { name: "Player 2" } } },
        ]
      }
    };
    const result = formatter.format(entry as unknown as UiLogEntry, state as unknown as BattleState);
    expect(result).not.toBeNull();
    expect(result!.messages.length).toBe(1);
    const log = result!.messages[0];
    expect(log.category).toBe(LogCategory.Primary);
    expect(stringifyLog(log)).toBe("You battled to a draw against Team Magma!");
  });

  it("should format win log correctly with side name", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const entry: Partial<UiLogEntry> = {
      title: "win",
      side: 0,
      values: {}
    };
    const state = {
      field: {
        sides: [
          { name: "Team Rocket", players: { p1: { name: "Player 1" } } },
          { name: "Team Magma", players: { p2: { name: "Player 2" } } },
        ]
      }
    };
    const result = formatter.format(entry as unknown as UiLogEntry, state as unknown as BattleState);
    expect(result).not.toBeNull();
    expect(result!.messages.length).toBe(1);
    const log = result!.messages[0];
    expect(log.category).toBe(LogCategory.Primary);
    expect(stringifyLog(log)).toBe("Team Rocket won the battle!");
  });
  it("should format tie log without state using fallback side name", () => {
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
    expect(stringifyLog(log)).toBe("You battled to a draw against Side 2!");
  });

  it("should format win log without state using fallback side name", () => {
    const formatter = new LogFormatter();
    const entry: Partial<UiLogEntry> = {
      title: "win",
      side: 1,
      values: {}
    };
    const result = formatter.format(entry as UiLogEntry);
    expect(result).not.toBeNull();
    expect(result!.messages.length).toBe(1);
    const log = result!.messages[0];
    expect(log.category).toBe(LogCategory.Primary);
    expect(stringifyLog(log)).toBe("Side 2 won the battle!");
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

    const multiStateLowercase = {
      battle_type: "multi",
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
    const multiLowercaseResult = formatter.format(entry as unknown as UiLogEntry, multiStateLowercase as unknown as BattleState);

    const singlesLog = singlesResult!.messages[0];
    const doublesLog = doublesResult!.messages[0];
    const multiLog = multiResult!.messages[0];
    const multiLowercaseLog = multiLowercaseResult!.messages[0];

    expect(singlesLog.key).toBe("crit__battletype_singles");
    expect(stringifyLog(singlesLog)).toBe("A critical hit!");
    
    expect(doublesLog.key).toBe("crit");
    expect(stringifyLog(doublesLog)).toBe("A critical hit on the opposing Pikachu!");
    
    expect(multiLog.key).toBe("crit");
    expect(stringifyLog(multiLog)).toBe("A critical hit on Player 2's Pikachu!");

    expect(multiLowercaseLog.key).toBe("crit");
    expect(stringifyLog(multiLowercaseLog)).toBe("A critical hit on Player 2's Pikachu!");
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
          by: BigInt(by)
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
          by: BigInt(by)
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

  it("should format plain damage with empty messages and damage with from as Secondary", () => {
    const formatter = new LogFormatter();
    const plainDamageEntry: Partial<UiLogEntry> = {
      title: "damage",
      values: {
        mon: { Active: { position: 0, name: "Charmander", player: "p2", side: 1 } },
        health: [0n, 100n],
        damage: [100n, 100n]
      }
    };

    const plainResult = formatter.format(plainDamageEntry as UiLogEntry);
    expect(plainResult).not.toBeNull();
    expect(plainResult!.messages.length).toBe(0);
    expect(plainResult!.notices.length).toBe(1);
    expect(plainResult!.notices[0]).toEqual({
      type: "Damage",
      name: "100/100",
      mon: "The opposing Charmander",
      monRef: { Active: { position: 0, name: "Charmander", player: "p2", side: 1 } }
    });

    const stealthRockDamageEntry: Partial<UiLogEntry> = {
      title: "damage",
      values: {
        mon: { Active: { position: 0, name: "Charizard", player: "p2", side: 1 } },
        from: "move:Stealth Rock",
        health: [50n, 100n],
        damage: [50n, 100n]
      }
    };

    const srResult = formatter.format(stealthRockDamageEntry as UiLogEntry);
    expect(srResult).not.toBeNull();
    expect(srResult!.messages.length).toBe(1);
    expect(srResult!.messages[0].category).toBe(LogCategory.Secondary);
    expect(stringifyLog(srResult!.messages[0])).toBe("Pointed stones dug into the opposing Charizard!");
    expect(srResult!.notices.length).toBe(1);
    expect(srResult!.notices[0]).toEqual({
      type: "Damage",
      name: "50/100",
      mon: "The opposing Charizard",
      monRef: { Active: { position: 0, name: "Charizard", player: "p2", side: 1 } }
    });
  });

  it("should format Damage and Heal notices with non-possessive mon and percentage format", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1", healthFormat: "percentage" });

    const damageEntry: Partial<UiLogEntry> = {
      title: "damage",
      values: {
        mon: { Active: { position: 0, name: "Charmander", player: "p1", side: 0 } },
        health: [75n, 100n],
        damage: [25n, 100n]
      }
    };

    const damageResult = formatter.format(damageEntry as UiLogEntry);
    expect(damageResult).not.toBeNull();
    expect(damageResult!.notices.length).toBe(1);
    expect(damageResult!.notices[0]).toEqual({
      type: "Damage",
      name: "25%",
      mon: "Charmander",
      monRef: { Active: { position: 0, name: "Charmander", player: "p1", side: 0 } }
    });

    const healEntry: Partial<UiLogEntry> = {
      title: "heal",
      values: {
        mon: { Active: { position: 0, name: "Squirtle", player: "p2", side: 1 } },
        health: [90n, 100n],
        heal: [15n, 100n]
      }
    };

    const healResult = formatter.format(healEntry as UiLogEntry);
    expect(healResult).not.toBeNull();
    expect(healResult!.notices.length).toBe(1);
    expect(healResult!.notices[0]).toEqual({
      type: "Heal",
      name: "15%",
      mon: "The opposing Squirtle",
      monRef: { Active: { position: 0, name: "Squirtle", player: "p2", side: 1 } }
    });
  });

  it("should disambiguate single vs multiple stats for fail what:unboost and emit ability notice", () => {
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
    expect(singleResult!.messages[0].key).toBe("fail__what_unboost");
    expect(stringifyLog(singleResult!.messages[0])).toBe("Beldum's Attack was not lowered!");
    expect(singleResult!.notices.length).toBe(1);
    expect(singleResult!.notices[0]).toEqual({
      type: "Ability",
      name: "Clear Body",
      mon: "Beldum's",
      monRef: { Active: { position: 0, name: "Beldum", player: "p1", side: 0 } }
    });

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
    expect(multiResult!.messages[0].key).toBe("fail__what_unboost");
    expect(stringifyLog(multiResult!.messages[0])).toBe("Beldum's stats were not lowered!");
    expect(multiResult!.notices.length).toBe(1);
    expect(multiResult!.notices[0]).toEqual({
      type: "Ability",
      name: "Clear Body",
      mon: "Beldum's",
      monRef: { Active: { position: 0, name: "Beldum", player: "p1", side: 0 } }
    });
  });

  it("should emit ability notice for formechange", () => {
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
    expect(result!.notices.length).toBe(1);
    expect(result!.notices[0]).toEqual({
      type: "Ability",
      name: "Stance Change",
      mon: "Aegislash's",
      monRef: { Active: { position: 0, name: "Aegislash", player: "p1", side: 0 } }
    });
  });

  it("should emit two messages and item notice for Mega Evolution", () => {
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
    expect(stringifyLog(result!.messages[0])).toBe("Venusaur's Venusaurite is reacting to your Mega Bracelet!");
    expect(stringifyLog(result!.messages[1])).toBe("Venusaur has Mega Evolved into Venusaur-Mega!");
    expect(result!.notices.length).toBe(1);
    expect(result!.notices[0]).toEqual({
      type: "Item",
      name: "Venusaurite",
      mon: "Venusaur's",
      monRef: { Active: { position: 0, name: "Venusaur", player: "p1", side: 0 } }
    });
  });

  it("should format Primal Reversion", () => {
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
    expect(result!.messages.length).toBe(1);
    expect(stringifyLog(result!.messages[0])).toBe("Kyogre's Primal Reversion! It reverted to its primal state!");
  });

  it("should format debug logs correctly with Secondary category", () => {
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
    expect(result!.messages[0].category).toBe(LogCategory.Secondary);
    expect(stringifyLog(result!.messages[0])).toBe("DEBUG: ModifyDamage: Unexpected state connector");
  });

  it("should format wild Pokémon as 'the wild' instead of 'the opposing'", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const wildState = {
      field: {
        sides: [
          { players: { p1: { id: "p1", name: "Jackson", wild: false } } },
          { players: { "wild-1": { id: "wild-1", name: "Wild", wild: true } } }
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

  it("should format Battle Bond differently for trainer vs wild Mon", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const wildState = {
      field: {
        sides: [
          { players: { p1: { id: "p1", name: "Jackson", wild: false } } },
          { players: { "wild-1": { id: "wild-1", name: "Wild", wild: true } } }
        ]
      }
    };

    // Trainer-owned Greninja (self)
    const trainerSelfEntry: Partial<UiLogEntry> = {
      title: "activate",
      values: {
        mon: { Active: { position: 0, name: "Greninja", player: "p1", side: 0 } },
        ability: "Battle Bond"
      }
    };
    const trainerSelfResult = formatter.format(trainerSelfEntry as UiLogEntry, wildState as unknown as BattleState);
    expect(trainerSelfResult).not.toBeNull();
    expect(trainerSelfResult!.messages[0].key).toBe("activate__ability_battlebond");
    expect(stringifyLog(trainerSelfResult!.messages[0])).toBe("Greninja became fully charged due to its bond with its Trainer!");

    // Wild Greninja
    const wildEntry: Partial<UiLogEntry> = {
      title: "activate",
      values: {
        mon: { Active: { position: 0, name: "Greninja", player: "wild-1", side: 1 } },
        ability: "Battle Bond"
      }
    };
    const wildResult = formatter.format(wildEntry as UiLogEntry, wildState as unknown as BattleState);
    expect(wildResult).not.toBeNull();
    expect(wildResult!.messages[0].key).toBe("activate__ability_battlebond__wild");
    expect(stringifyLog(wildResult!.messages[0])).toBe("The wild Greninja became fully charged due to its Battle Bond!");
  });
  it("should format move with zpower flag", () => {
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

  it("should format Magic Bounce reflection message and ability notice", () => {
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
    expect(result!.messages.length).toBe(1);
    expect(stringifyLog(result!.messages[0])).toBe("The opposing Espeon bounced the Toxic back!");
    expect(result!.notices.length).toBe(1);
    expect(result!.notices[0]).toEqual({
      type: "Ability",
      name: "Magic Bounce",
      mon: "The opposing Espeon's",
      monRef: { Active: { position: 0, name: "Espeon", player: "p2", side: 1 } }
    });
  });

  it("should format Disguise damage with ability and damage notices", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const state = {
      field: {
        sides: [
          { players: { p1: { name: "Alice" } } },
          { players: { p2: { name: "Bob" } } }
        ]
      }
    };

    const disguiseEntry: Partial<UiLogEntry> = {
      title: "damage",
      values: {
        mon: { Active: { side: 0, position: 0, name: "Mimikyu", player: "p1" } },
        from: "ability:Disguise",
        damage: [12n, 100n],
      }
    };

    const result = formatter.format(disguiseEntry as UiLogEntry, state as unknown as BattleState);
    expect(result).not.toBeNull();
    expect(result!.messages).toEqual([]);
    expect(result!.notices.length).toBe(2);
    expect(result!.notices[0]).toEqual({
      type: "Ability",
      name: "Disguise",
      mon: "Mimikyu's",
      monRef: { Active: { side: 0, position: 0, name: "Mimikyu", player: "p1" } }
    });
    expect(result!.notices[1]).toEqual({
      type: "Damage",
      name: "12/100",
      mon: "Mimikyu",
      monRef: { Active: { side: 0, position: 0, name: "Mimikyu", player: "p1" } }
    });
  });

  it("should format Powder damage with empty messages and damage notice", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const state = {
      field: {
        sides: [
          { players: { p1: { name: "Alice" } } },
          { players: { p2: { name: "Bob" } } }
        ]
      }
    };

    const powderEntry: Partial<UiLogEntry> = {
      title: "damage",
      values: {
        mon: { Active: { side: 1, position: 0, name: "Charizard", player: "p2" } },
        from: "move:Powder",
        damage: [25n, 100n],
      }
    };

    const result = formatter.format(powderEntry as UiLogEntry, state as unknown as BattleState);
    expect(result).not.toBeNull();
    expect(result!.messages).toEqual([]);
    expect(result!.notices.length).toBe(1);
    expect(result!.notices[0]).toEqual({
      type: "Damage",
      name: "25/100",
      mon: "The opposing Charizard",
      monRef: { Active: { side: 1, position: 0, name: "Charizard", player: "p2" } }
    });
  });

  it("should format ability notice with of (source) on weather log", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const entry: Partial<UiLogEntry> = {
      title: "weather",
      values: {
        weather: "Harsh Sunlight",
        from: "ability:Drought",
        of: { Active: { side: 0, position: 0, name: "Ninetales", player: "p1" } }
      }
    };

    const result = formatter.format(entry as UiLogEntry);
    expect(result).not.toBeNull();
    expect(result!.notices.length).toBe(1);
    expect(result!.notices[0]).toEqual({
      type: "Ability",
      name: "Drought",
      mon: "Ninetales's",
      monRef: { Active: { side: 0, position: 0, name: "Ninetales", player: "p1" } }
    });
    expect(stringifyLog(result!.messages[0])).toBe("The sunlight turned harsh!");
  });

  it("should format ability notice on unboost with of (source)", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const entry: Partial<UiLogEntry> = {
      title: "unboost",
      values: {
        mon: { Active: { side: 0, position: 0, name: "Walking Wake", player: "p1" } },
        stat: "atk",
        by: 1n,
        from: "ability:Intimidate",
        of: { Active: { side: 1, position: 0, name: "Gyarados", player: "p2" } }
      }
    };

    const result = formatter.format(entry as UiLogEntry);
    expect(result).not.toBeNull();
    expect(result!.notices.length).toBe(1);
    expect(result!.notices[0]).toEqual({
      type: "Ability",
      name: "Intimidate",
      mon: "The opposing Gyarados's",
      monRef: { Active: { side: 1, position: 0, name: "Gyarados", player: "p2" } }
    });
    expect(stringifyLog(result!.messages[0])).toBe("Walking Wake's Attack fell!");
  });

  it("should format item notice and heal notice on heal with Leftovers", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1", healthFormat: "percentage" });
    const entry: Partial<UiLogEntry> = {
      title: "heal",
      values: {
        mon: { Active: { side: 0, position: 0, name: "Zapdos", player: "p1" } },
        health: [100n, 100n],
        heal: [6n, 100n],
        from: "item:Leftovers"
      }
    };

    const result = formatter.format(entry as UiLogEntry);
    expect(result).not.toBeNull();
    expect(result!.notices.length).toBe(2);
    expect(result!.notices[0]).toEqual({
      type: "Item",
      name: "Leftovers",
      mon: "Zapdos's",
      monRef: { Active: { side: 0, position: 0, name: "Zapdos", player: "p1" } }
    });
    expect(result!.notices[1]).toEqual({
      type: "Heal",
      name: "6%",
      mon: "Zapdos",
      monRef: { Active: { side: 0, position: 0, name: "Zapdos", player: "p1" } }
    });
    expect(stringifyLog(result!.messages[0])).toBe("Zapdos restored a little HP using its Leftovers!");
  });

  it("should format item notice and damage notice on damage with Life Orb", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1", healthFormat: "percentage" });
    const entry: Partial<UiLogEntry> = {
      title: "damage",
      values: {
        mon: { Active: { side: 1, position: 0, name: "Walking Wake", player: "p2" } },
        health: [90n, 100n],
        damage: [10n, 100n],
        from: "item:Life Orb"
      }
    };

    const result = formatter.format(entry as UiLogEntry);
    expect(result).not.toBeNull();
    expect(result!.notices.length).toBe(2);
    expect(result!.notices[0]).toEqual({
      type: "Item",
      name: "Life Orb",
      mon: "The opposing Walking Wake's",
      monRef: { Active: { side: 1, position: 0, name: "Walking Wake", player: "p2" } }
    });
    expect(result!.notices[1]).toEqual({
      type: "Damage",
      name: "10%",
      mon: "The opposing Walking Wake",
      monRef: { Active: { side: 1, position: 0, name: "Walking Wake", player: "p2" } }
    });
  });

  it("should format both item and ability notices for Booster Energy activating Protosynthesis", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const entry: Partial<UiLogEntry> = {
      title: "activate",
      values: {
        mon: { Active: { side: 0, position: 0, name: "Walking Wake", player: "p1" } },
        ability: "Protosynthesis",
        from: "item:Booster Energy"
      }
    };

    const result = formatter.format(entry as UiLogEntry);
    expect(result).not.toBeNull();
    expect(result!.notices.length).toBe(2);
    expect(result!.notices[0]).toEqual({
      type: "Item",
      name: "Booster Energy",
      mon: "Walking Wake's",
      monRef: { Active: { side: 0, position: 0, name: "Walking Wake", player: "p1" } }
    });
    expect(result!.notices[1]).toEqual({
      type: "Ability",
      name: "Protosynthesis",
      mon: "Walking Wake's",
      monRef: { Active: { side: 0, position: 0, name: "Walking Wake", player: "p1" } }
    });
    expect(stringifyLog(result!.messages[0])).toBe("Walking Wake used its Booster Energy to activate Protosynthesis!");
  });

  it("should format both source ability notice and target damage notice simultaneously on contact ability damage", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1", healthFormat: "percentage" });
    const entry: Partial<UiLogEntry> = {
      title: "damage",
      values: {
        mon: { Active: { side: 0, position: 0, name: "Walking Wake", player: "p1" } },
        of: { Active: { side: 1, position: 0, name: "Garchomp", player: "p2" } },
        health: [88n, 100n],
        damage: [12n, 100n],
        from: "ability:Rough Skin"
      }
    };

    const result = formatter.format(entry as UiLogEntry);
    expect(result).not.toBeNull();
    expect(result!.notices.length).toBe(2);
    // Source notice (Rough Skin belongs to Garchomp)
    expect(result!.notices[0]).toEqual({
      type: "Ability",
      name: "Rough Skin",
      mon: "The opposing Garchomp's",
      monRef: { Active: { side: 1, position: 0, name: "Garchomp", player: "p2" } }
    });
    // Target notice (Damage taken by Walking Wake)
    expect(result!.notices[1]).toEqual({
      type: "Damage",
      name: "12%",
      mon: "Walking Wake",
      monRef: { Active: { side: 0, position: 0, name: "Walking Wake", player: "p1" } }
    });
    expect(stringifyLog(result!.messages[0])).toBe("Walking Wake was hurt!");
  });

  it("should format both source item notice and target damage notice simultaneously on contact item damage", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1", healthFormat: "percentage" });
    const entry: Partial<UiLogEntry> = {
      title: "damage",
      values: {
        mon: { Active: { side: 0, position: 0, name: "Zapdos", player: "p1" } },
        of: { Active: { side: 1, position: 0, name: "Ferrothorn", player: "p2" } },
        health: [84n, 100n],
        damage: [16n, 100n],
        from: "item:Rocky Helmet"
      }
    };

    const result = formatter.format(entry as UiLogEntry);
    expect(result).not.toBeNull();
    expect(result!.notices.length).toBe(2);
    // Source notice (Rocky Helmet belongs to Ferrothorn)
    expect(result!.notices[0]).toEqual({
      type: "Item",
      name: "Rocky Helmet",
      mon: "The opposing Ferrothorn's",
      monRef: { Active: { side: 1, position: 0, name: "Ferrothorn", player: "p2" } }
    });
    // Target notice (Damage taken by Zapdos)
    expect(result!.notices[1]).toEqual({
      type: "Damage",
      name: "16%",
      mon: "Zapdos",
      monRef: { Active: { side: 0, position: 0, name: "Zapdos", player: "p1" } }
    });
    expect(stringifyLog(result!.messages[0])).toBe("Zapdos was hurt by the opposing Ferrothorn's Rocky Helmet!");
  });

  it("should format Skill Swap flow with ability notices on each swap log entry", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });

    // 1. Plusle (p1) gains Drizzle from Minun (p2)
    const plusleGainEntry: Partial<UiLogEntry> = {
      title: "abilitystart",
      values: {
        mon: { Active: { side: 0, position: 0, name: "Plusle", player: "p1" } },
        source: { Active: { side: 1, position: 0, name: "Minun", player: "p2" } },
        ability: "Drizzle",
        from: "move:Skill Swap"
      }
    };

    const plusleResult = formatter.format(plusleGainEntry as UiLogEntry);
    expect(plusleResult).not.toBeNull();
    expect(plusleResult!.notices.length).toBe(1);
    expect(plusleResult!.notices[0]).toEqual({
      type: "Ability",
      name: "Drizzle",
      mon: "Plusle's",
      monRef: { Active: { side: 0, position: 0, name: "Plusle", player: "p1" } }
    });
    expect(stringifyLog(plusleResult!.messages[0])).toBe("Plusle swapped Abilities with its target!");

    // 2. Minun (p2) gains Soundproof from Plusle (p1)
    const minunGainEntry: Partial<UiLogEntry> = {
      title: "abilitystart",
      values: {
        mon: { Active: { side: 1, position: 0, name: "Minun", player: "p2" } },
        source: { Active: { side: 0, position: 0, name: "Plusle", player: "p1" } },
        ability: "Soundproof",
        from: "move:Skill Swap",
        of: { Active: { side: 0, position: 0, name: "Plusle", player: "p1" } }
      }
    };

    const minunResult = formatter.format(minunGainEntry as UiLogEntry);
    expect(minunResult).not.toBeNull();
    expect(minunResult!.notices.length).toBe(1);
    expect(minunResult!.notices[0]).toEqual({
      type: "Ability",
      name: "Soundproof",
      mon: "The opposing Minun's",
      monRef: { Active: { side: 1, position: 0, name: "Minun", player: "p2" } }
    });
    expect(stringifyLog(minunResult!.messages[0])).toBe("The opposing Minun swapped Abilities with its target!");
  });

  it("should format two ability notices simultaneously on abilityend with from:ability (Mummy)", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const entry: Partial<UiLogEntry> = {
      title: "abilityend",
      values: {
        mon: { Active: { side: 0, position: 0, name: "Conkeldurr", player: "p1" } },
        ability: "Sheer Force",
        from: "ability:Mummy",
        of: { Active: { side: 1, position: 0, name: "Cofagrigus", player: "p2" } }
      }
    };

    const result = formatter.format(entry as UiLogEntry);
    expect(result).not.toBeNull();
    expect(result!.notices.length).toBe(2);
    // 1. Source Ability notice (Mummy from the opposing Cofagrigus)
    expect(result!.notices[0]).toEqual({
      type: "Ability",
      name: "Mummy",
      mon: "The opposing Cofagrigus's",
      monRef: { Active: { side: 1, position: 0, name: "Cofagrigus", player: "p2" } }
    });
    // 2. Target Ability notice (Sheer Force from Conkeldurr)
    expect(result!.notices[1]).toEqual({
      type: "Ability",
      name: "Sheer Force",
      mon: "Conkeldurr's",
      monRef: { Active: { side: 0, position: 0, name: "Conkeldurr", player: "p1" } }
    });
  });

  it("should format two ability notices simultaneously on abilitystart with from:ability (Lingering Aroma)", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const entry: Partial<UiLogEntry> = {
      title: "abilitystart",
      values: {
        mon: { Active: { side: 0, position: 0, name: "Conkeldurr", player: "p1" } },
        ability: "Lingering Aroma",
        from: "ability:Lingering Aroma",
        of: { Active: { side: 1, position: 0, name: "Oinkologne", player: "p2" } }
      }
    };

    const result = formatter.format(entry as UiLogEntry);
    expect(result).not.toBeNull();
    expect(result!.notices.length).toBe(2);
    // 1. Source Ability notice (Lingering Aroma on the opposing Oinkologne)
    expect(result!.notices[0]).toEqual({
      type: "Ability",
      name: "Lingering Aroma",
      mon: "The opposing Oinkologne's",
      monRef: { Active: { side: 1, position: 0, name: "Oinkologne", player: "p2" } }
    });
    // 2. Target Ability notice (Lingering Aroma acquired by Conkeldurr)
    expect(result!.notices[1]).toEqual({
      type: "Ability",
      name: "Lingering Aroma",
      mon: "Conkeldurr's",
      monRef: { Active: { side: 0, position: 0, name: "Conkeldurr", player: "p1" } }
    });
  });

  it("should format two ability notices simultaneously on activate with from:ability (Mirror Armor / Synchronize)", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const entry: Partial<UiLogEntry> = {
      title: "activate",
      values: {
        mon: { Active: { side: 0, position: 0, name: "Corviknight", player: "p1" } },
        ability: "Mirror Armor",
        from: "ability:Intimidate",
        of: { Active: { side: 1, position: 0, name: "Gyarados", player: "p2" } }
      }
    };

    const result = formatter.format(entry as UiLogEntry);
    expect(result).not.toBeNull();
    expect(result!.notices.length).toBe(2);
    // 1. Source Ability notice (Intimidate from the opposing Gyarados via sourceFirst)
    expect(result!.notices[0]).toEqual({
      type: "Ability",
      name: "Intimidate",
      mon: "The opposing Gyarados's",
      monRef: { Active: { side: 1, position: 0, name: "Gyarados", player: "p2" } }
    });
    // 2. Target Ability notice (Mirror Armor on Corviknight via targetFirst)
    expect(result!.notices[1]).toEqual({
      type: "Ability",
      name: "Mirror Armor",
      mon: "Corviknight's",
      monRef: { Active: { side: 0, position: 0, name: "Corviknight", player: "p1" } }
    });
  });

  it("should preserve lowercase player handles in multi-battle ability notices and messages", () => {
    const multiState = {
      battle_type: "Multi",
      field: {
        sides: [
          { name: "Side 1", players: { p1: { name: "Player 1" } } },
          { name: "Side 2", players: { "ai-random-1": { name: "ai-random-1" } } },
        ]
      }
    };
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const entry: Partial<UiLogEntry> = {
      title: "activate",
      values: {
        mon: { Active: { side: 1, position: 0, name: "Great Tusk", player: "ai-random-1" } },
        ability: "Protosynthesis",
      }
    };

    const result = formatter.format(entry as UiLogEntry, multiState as unknown as BattleState);
    expect(result).not.toBeNull();
    expect(result!.notices.length).toBe(1);
    expect(result!.notices[0]).toEqual({
      type: "Ability",
      name: "Protosynthesis",
      mon: "ai-random-1's Great Tusk's",
      monRef: { Active: { side: 1, position: 0, name: "Great Tusk", player: "ai-random-1" } }
    });
    expect(stringifyLog(result!.messages[0])).toBe(
      "The harsh sunlight activated ai-random-1's Great Tusk's Protosynthesis!"
    );
  });

  it("should capitalize the opposing Mon in single-battle ability notices", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const entry: Partial<UiLogEntry> = {
      title: "activate",
      values: {
        mon: { Active: { side: 1, position: 0, name: "Ninetales", player: "p2" } },
        ability: "Drought",
      }
    };

    const result = formatter.format(entry as UiLogEntry);
    expect(result).not.toBeNull();
    expect(result!.notices.length).toBe(1);
    expect(result!.notices[0]).toEqual({
      type: "Ability",
      name: "Drought",
      mon: "The opposing Ninetales's",
      monRef: { Active: { side: 1, position: 0, name: "Ninetales", player: "p2" } }
    });
  });

  describe("Extension logs", () => {
    it("should format turn timer warning for player with Hint category", () => {
      const formatter = new LogFormatter({ localPlayerId: "p1" });
      const entry: Partial<UiLogEntry> = {
        title: "timer",
        values: {
          source: "-battlerservice",
          action: "p1",
          warning: true,
          remainingsecs: 10n,
        }
      };
      const state = {
        field: {
          sides: [
            { name: "Side 1", players: { p1: { name: "Alice" } } },
            { name: "Side 2", players: { p2: { name: "Bob" } } },
          ]
        }
      };

      const result = formatter.format(entry as UiLogEntry, state as unknown as BattleState);
      expect(result).not.toBeNull();
      expect(result!.messages.length).toBe(1);
      const msg = result!.messages[0];
      expect(msg.category).toBe(LogCategory.Hint);
      expect(msg.key).toBe("timer__player_any__turn__warning");
      expect(stringifyLog(msg)).toBe("10 seconds left for you to make a decision!");
    });

    it("should format turn timer expiration for player", () => {
      const formatter = new LogFormatter({ localPlayerId: "p1" });
      const entry: Partial<UiLogEntry> = {
        title: "timer",
        values: {
          source: "-battlerservice",
          action: "p2",
          done: true,
          remainingsecs: 0n,
        }
      };
      const state = {
        field: {
          sides: [
            { name: "Side 1", players: { p1: { name: "Alice" } } },
            { name: "Side 2", players: { p2: { name: "Bob" } } },
          ]
        }
      };

      const result = formatter.format(entry as UiLogEntry, state as unknown as BattleState);
      expect(result).not.toBeNull();
      expect(result!.messages.length).toBe(1);
      const msg = result!.messages[0];
      expect(msg.category).toBe(LogCategory.Hint);
      expect(msg.key).toBe("timer__player_any__done__turn");
      expect(stringifyLog(msg)).toBe("Bob did not make a move in time.");
    });

    it("should format player bank timer warning", () => {
      const formatter = new LogFormatter({ localPlayerId: "p1" });
      const entry: Partial<UiLogEntry> = {
        title: "timer",
        values: {
          source: "-battlerservice",
          player: "p2",
          warning: true,
          remainingsecs: 5n,
        }
      };
      const state = {
        field: {
          sides: [
            { name: "Side 1", players: { p1: { name: "Alice" } } },
            { name: "Side 2", players: { p2: { name: "Bob" } } },
          ]
        }
      };

      const result = formatter.format(entry as UiLogEntry, state as unknown as BattleState);
      expect(result).not.toBeNull();
      expect(result!.messages.length).toBe(1);
      const msg = result!.messages[0];
      expect(msg.category).toBe(LogCategory.Hint);
      expect(msg.key).toBe("timer__player_any__warning");
      expect(stringifyLog(msg)).toBe("5 seconds left for Bob.");
    });

    it("should format player bank timer expiration (done)", () => {
      const formatter = new LogFormatter({ localPlayerId: "p1" });
      const entry: Partial<UiLogEntry> = {
        title: "timer",
        values: {
          source: "-battlerservice",
          player: "p2",
          done: true,
          remainingsecs: 0n,
        }
      };
      const state = {
        field: {
          sides: [
            { name: "Side 1", players: { p1: { name: "Alice" } } },
            { name: "Side 2", players: { p2: { name: "Bob" } } },
          ]
        }
      };

      const result = formatter.format(entry as UiLogEntry, state as unknown as BattleState);
      expect(result).not.toBeNull();
      expect(result!.messages.length).toBe(1);
      const msg = result!.messages[0];
      expect(msg.category).toBe(LogCategory.Hint);
      expect(msg.key).toBe("timer__player_any__done");
      expect(stringifyLog(msg)).toBe("Bob ran out of time!");
    });

    it("should format teampreview timer warning and done", () => {
      const formatter = new LogFormatter({ localPlayerId: "p1" });
      const entry: Partial<UiLogEntry> = {
        title: "timer",
        values: {
          source: "-battlerservice",
          teampreview: true,
          warning: true,
          remainingsecs: 15n,
        }
      };

      const result = formatter.format(entry as UiLogEntry);
      expect(result).not.toBeNull();
      expect(result!.messages.length).toBe(1);
      const msg = result!.messages[0];
      expect(msg.category).toBe(LogCategory.Hint);
      expect(stringifyLog(msg)).toBe("15 seconds left for Team Preview!");

      const doneEntry: Partial<UiLogEntry> = {
        title: "timer",
        values: {
          source: "-battlerservice",
          teampreview: true,
          done: true,
          remainingsecs: 0n,
        }
      };
      const doneResult = formatter.format(doneEntry as UiLogEntry);
      expect(doneResult).not.toBeNull();
      expect(stringifyLog(doneResult!.messages[0])).toBe("Team Preview time has expired!");
    });

    it("should format battle timer warning and done", () => {
      const formatter = new LogFormatter({ localPlayerId: "p1" });
      const warningEntry: Partial<UiLogEntry> = {
        title: "timer",
        values: {
          source: "-battlerservice",
          battle: true,
          warning: true,
          remainingsecs: 60n,
        }
      };
      const warningResult = formatter.format(warningEntry as UiLogEntry);
      expect(warningResult).not.toBeNull();
      expect(stringifyLog(warningResult!.messages[0])).toBe("1 minute left in the battle!");

      const doneEntry: Partial<UiLogEntry> = {
        title: "timer",
        values: {
          source: "-battlerservice",
          battle: true,
          done: true,
          remainingsecs: 0n,
        }
      };
      const doneResult = formatter.format(doneEntry as UiLogEntry);
      expect(doneResult).not.toBeNull();
      expect(stringifyLog(doneResult!.messages[0])).toBe("The battle ran out of time!");
    });

    it("should return null (silent) for ordinary timer countdown ticks", () => {
      const formatter = new LogFormatter({ localPlayerId: "p1" });
      const entry: Partial<UiLogEntry> = {
        title: "timer",
        values: {
          source: "-battlerservice",
          player: "p1",
          remainingsecs: 47n,
        }
      };

      const result = formatter.format(entry as UiLogEntry);
      expect(result).toBeNull();
    });

    it("should format non-timer extension entries like started and done as Hint category", () => {
      const formatter = new LogFormatter({ localPlayerId: "p1" });
      const startedEntry: Partial<UiLogEntry> = {
        title: "-battlerservice:started",
        values: {
          source: "-battlerservice",
        }
      };
      const startedResult = formatter.format(startedEntry as UiLogEntry);
      expect(startedResult).not.toBeNull();
      expect(startedResult!.messages[0].category).toBe(LogCategory.Hint);
      expect(stringifyLog(startedResult!.messages[0])).toBe("The battle has started!");

      const doneEntry: Partial<UiLogEntry> = {
        title: "-battlerservice:done",
        values: {
          source: "-battlerservice",
        }
      };
      const doneResult = formatter.format(doneEntry as UiLogEntry);
      expect(doneResult).not.toBeNull();
      expect(doneResult!.messages[0].category).toBe(LogCategory.Hint);
      expect(stringifyLog(doneResult!.messages[0])).toBe("The battle is over!");
    });
  });

  describe("MON_NAME context variable usage", () => {
    const singleState: BattleState = {
      battle_type: "singles",
      field: {
        sides: [
          { name: "Side 1", players: { p1: { name: "Jackson" } } },
          { name: "Side 2", players: { p2: { name: "ai-random-1" } } },
        ],
      },
    } as unknown as BattleState;

    const multiState: BattleState = {
      battle_type: "multi",
      field: {
        sides: [
          { name: "Side 1", players: { p1: { name: "Jackson" }, p3: { name: "Alice" } } },
          { name: "Side 2", players: { p2: { name: "Bob" }, p4: { name: "Charlie" } } },
        ],
      },
    } as unknown as BattleState;

    it("should format switch logs using MON_NAME without redundant prefixes", () => {
      const formatter = new LogFormatter({ localPlayerId: "p1" });

      // Local player switch
      const selfSwitch: Partial<UiLogEntry> = {
        title: "switch",
        player: "p1",
        values: {
          name: "Ninetales",
          player: "p1",
          position: 0n,
        },
      };
      const selfRes = formatter.format(selfSwitch as UiLogEntry, singleState);
      expect(selfRes).not.toBeNull();
      expect(stringifyLog(selfRes!.messages[0])).toBe("You sent out Ninetales!");

      // Foe single battle switch
      const foeSwitch: Partial<UiLogEntry> = {
        title: "switch",
        player: "p2",
        values: {
          name: "Ninetales",
          player: "p2",
          position: 0n,
        },
      };
      const foeRes = formatter.format(foeSwitch as UiLogEntry, singleState);
      expect(foeRes).not.toBeNull();
      expect(stringifyLog(foeRes!.messages[0])).toBe("ai-random-1 sent out Ninetales!");

      // Foe multi battle switch (should not say Bob sent out Bob's Ninetales)
      const foeMultiSwitch: Partial<UiLogEntry> = {
        title: "switch",
        player: "p2",
        values: {
          name: "Ninetales",
          player: "p2",
          position: 0n,
        },
      };
      const foeMultiRes = formatter.format(foeMultiSwitch as UiLogEntry, multiState);
      expect(foeMultiRes).not.toBeNull();
      expect(stringifyLog(foeMultiRes!.messages[0])).toBe("Bob sent out Ninetales!");

      // Ally multi battle switch (should not say Alice sent out Alice's Ninetales)
      const allyMultiSwitch: Partial<UiLogEntry> = {
        title: "switch",
        player: "p3",
        values: {
          name: "Ninetales",
          player: "p3",
          position: 1n,
        },
      };
      const allyMultiRes = formatter.format(allyMultiSwitch as UiLogEntry, multiState);
      expect(allyMultiRes).not.toBeNull();
      expect(stringifyLog(allyMultiRes!.messages[0])).toBe("Alice sent out Ninetales!");
    });

    it("should format switchout logs using MON_NAME when switching out previous Mon", () => {
      const formatter = new LogFormatter({ localPlayerId: "p1" });

      const foeSwitchWithPrev: Partial<UiLogEntry> = {
        title: "switch",
        player: "p2",
        values: {
          name: "Vulpix",
          player: "p2",
          position: 0n,
          prev_mon: {
            Active: {
              name: "Ninetales",
              player: "p2",
              position: 0,
              side: 1,
            },
          },
        },
      };

      const foeRes = formatter.format(foeSwitchWithPrev as UiLogEntry, singleState);
      expect(foeRes).not.toBeNull();
      expect(foeRes!.messages.length).toBe(2);
      expect(stringifyLog(foeRes!.messages[0])).toBe("ai-random-1 withdrew Ninetales!");
      expect(stringifyLog(foeRes!.messages[1])).toBe("ai-random-1 sent out Vulpix!");
    });

    it("should format mega evolution logs using MON_POSSESSIVE and MON", () => {
      const formatter = new LogFormatter({ localPlayerId: "p1" });

      const megaEntry: Partial<UiLogEntry> = {
        title: "mega",
        target: {
          Active: {
            name: "Charizard",
            player: "p2",
            position: 0,
            side: 1,
          },
        },
        values: {
          from_item: "Charizardite X",
          species: "Mega Charizard X",
        },
      };

      const megaRes = formatter.format(megaEntry as UiLogEntry, singleState);
      expect(megaRes).not.toBeNull();
      expect(megaRes!.messages.length).toBe(2);
      expect(stringifyLog(megaRes!.messages[0])).toBe("The opposing Charizard's Charizardite X is reacting to ai-random-1's Mega Bracelet!");
      expect(stringifyLog(megaRes!.messages[1])).toBe("The opposing Charizard has Mega Evolved into Mega Charizard X!");

      const dragonAscentMega: Partial<UiLogEntry> = {
        title: "mega",
        target: {
          Active: {
            name: "Rayquaza",
            player: "p2",
            position: 0,
            side: 1,
          },
        },
        source_effect: {
          name: "Dragon Ascent",
          effect_type: "Move",
        },
        values: {
          species: "Mega Rayquaza",
        },
      };

      const dragonRes = formatter.format(dragonAscentMega as UiLogEntry, singleState);
      expect(dragonRes).not.toBeNull();
      expect(dragonRes!.messages.length).toBe(2);
      expect(stringifyLog(dragonRes!.messages[0])).toBe("ai-random-1's fervent wish has reached the opposing Rayquaza!");
      expect(stringifyLog(dragonRes!.messages[1])).toBe("The opposing Rayquaza has Mega Evolved into Mega Rayquaza!");
    });
  });
});
