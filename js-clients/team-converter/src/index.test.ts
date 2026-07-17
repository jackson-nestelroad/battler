import { describe, it, expect } from "vitest";
import { parseShowdown, exportToShowdown } from "./index.js";
import type { MonData } from "battler-types";

describe("Showdown Format Team Converter", () => {
  describe("parseShowdown", () => {
    it("should parse a fully defined Pokémon with all properties", () => {
      const text = `
Sparky (Pikachu) (M) @ Light Ball
Ability: Static
Level: 50
Shiny: Yes
Friendship: 200
EVs: 4 HP / 252 SpA / 252 Spe
IVs: 0 Atk / 30 Def
Timid Nature
Gigantamax: Yes
Tera Type: Ghost
- Thunderbolt
- Grass Knot
- Volt Switch
- Protect
`;
      const result = parseShowdown(text);
      expect(result.length).toBe(1);

      const mon = result[0];
      expect(mon.name).toBe("Sparky");
      expect(mon.species).toBe("Pikachu");
      expect(mon.gender).toBe("M");
      expect(mon.item).toBe("Light Ball");
      expect(mon.ability).toBe("Static");
      expect(mon.level).toBe(50);
      expect(mon.shiny).toBe(true);
      expect(mon.friendship).toBe(200);
      expect(mon.evs).toEqual({ hp: 4, atk: 0, def: 0, spa: 252, spd: 0, spe: 252 });
      expect(mon.ivs).toEqual({ hp: 31, atk: 0, def: 30, spa: 31, spd: 31, spe: 31 });
      expect(mon.nature).toBe("Timid");
      expect(mon.gigantamax_factor).toBe(true);
      expect(mon.tera_type).toBe("Ghost");
      expect(mon.moves).toEqual(["Thunderbolt", "Grass Knot", "Volt Switch", "Protect"]);
    });

    it("should parse a minimal Pokémon and omit unspecified fields", () => {
      const text = `
Pikachu
- Tackle
`;
      const result = parseShowdown(text);
      expect(result.length).toBe(1);

      const mon = result[0];
      expect(mon.name).toBe("Pikachu");
      expect(mon.species).toBe("Pikachu");
      expect(mon.moves).toEqual(["Tackle"]);

      // Unspecified fields must be omitted (undefined)
      expect(mon.gender).toBeUndefined();
      expect(mon.item).toBeUndefined();
      expect(mon.ability).toBeUndefined();
      expect(mon.level).toBeUndefined();
      expect(mon.shiny).toBeUndefined();
      expect(mon.friendship).toBeUndefined();
      expect(mon.evs).toBeUndefined();
      expect(mon.ivs).toBeUndefined();
      expect(mon.nature).toBeUndefined();
      expect(mon.gigantamax_factor).toBeUndefined();
      expect(mon.tera_type).toBeUndefined();
    });

    it("should parse nickname and gender variants correctly", () => {
      const variants = [
        { text: "Sparky (Pikachu)", name: "Sparky", species: "Pikachu", gender: undefined },
        { text: "Sparky (Pikachu) (F)", name: "Sparky", species: "Pikachu", gender: "F" },
        { text: "Pikachu (M)", name: "Pikachu", species: "Pikachu", gender: "M" },
        { text: "Pikachu", name: "Pikachu", species: "Pikachu", gender: undefined },
      ];

      for (const variant of variants) {
        const result = parseShowdown(variant.text + "\n- Tackle");
        expect(result.length).toBe(1);
        expect(result[0].name).toBe(variant.name);
        expect(result[0].species).toBe(variant.species);
        expect(result[0].gender).toBe(variant.gender);
      }
    });

    it("should parse item syntaxes with or without spacing", () => {
      const items = [
        { text: "Pikachu @ Light Ball", item: "Light Ball" },
        { text: "Pikachu@Light Ball", item: "Light Ball" },
        { text: "Pikachu @Leftovers", item: "Leftovers" },
        { text: "Pikachu@ Leftovers", item: "Leftovers" },
      ];

      for (const itemCase of items) {
        const result = parseShowdown(itemCase.text + "\n- Tackle");
        expect(result.length).toBe(1);
        expect(result[0].item).toBe(itemCase.item);
      }
    });

    it("should handle nature name capitalization and format variations", () => {
      const texts = [
        "Pikachu\nJolly Nature\n- Tackle",
        "Pikachu\nNature: jolly\n- Tackle",
        "Pikachu\ntimid Nature\n- Tackle",
        "Pikachu\nNature: TIMID\n- Tackle",
      ];

      const expectedNatures = ["Jolly", "Jolly", "Timid", "Timid"];

      for (let i = 0; i < texts.length; i++) {
        const result = parseShowdown(texts[i]);
        expect(result.length).toBe(1);
        expect(result[0].nature).toBe(expectedNatures[i]);
      }
    });

    it("should fallback to Serious nature for invalid natures", () => {
      const result = parseShowdown("Pikachu\nSupercharged Nature\n- Tackle");
      expect(result.length).toBe(1);
      expect(result[0].nature).toBe("Serious");
    });

    it("should parse various EV/IV layouts and abbreviations", () => {
      const text = `
Pikachu
EVs: 120 HP/8 Def / 130 Sp. Atk/252 Speed
IVs: 0 Attack / 15 Def / 30 special defense
- Tackle
`;
      const result = parseShowdown(text);
      expect(result.length).toBe(1);
      const mon = result[0];

      expect(mon.evs).toEqual({ hp: 120, atk: 0, def: 8, spa: 130, spd: 0, spe: 252 });
      expect(mon.ivs).toEqual({ hp: 31, atk: 0, def: 15, spa: 31, spd: 30, spe: 31 });
    });

    it("should extract hidden power type from move name", () => {
      const text = `
Pikachu
- Hidden Power [Ice]
- Thunderbolt
`;
      const result = parseShowdown(text);
      expect(result.length).toBe(1);
      expect(result[0].hidden_power_type).toBe("Ice");
    });

    it("should handle Gmax factor alternative spelling", () => {
      const result = parseShowdown("Pikachu\nGmax: Yes\n- Tackle");
      expect(result.length).toBe(1);
      expect(result[0].gigantamax_factor).toBe(true);

      const result2 = parseShowdown("Pikachu\nGigantamax Factor: Yes\n- Tackle");
      expect(result2.length).toBe(1);
      expect(result2[0].gigantamax_factor).toBe(true);
    });

    it("should handle multiple newlines and CRLF formatting cleanly", () => {
      const text =
        "\r\n\r\nPikachu\r\nLevel: 50\r\n\r\n- Tackle\r\n\r\n\r\nRaichu\r\n- Thunderbolt\r\n\r\n";
      const result = parseShowdown(text);
      expect(result.length).toBe(2);
      expect(result[0].species).toBe("Pikachu");
      expect(result[0].level).toBe(50);
      expect(result[1].species).toBe("Raichu");
    });
  });

  describe("exportToShowdown", () => {
    it("should format and export a complete team to Showdown format", () => {
      const team: MonData[] = [
        {
          name: "Sparky",
          species: "Pikachu",
          ability: "Static",
          moves: ["Thunderbolt", "Quick Attack"],
          item: "Light Ball",
          pp_boosts: [],
          nature: "Jolly",
          true_nature: null,
          gender: "M",
          evs: { hp: 0, atk: 252, def: 4, spa: 0, spd: 0, spe: 252 },
          ivs: { hp: 31, atk: 31, def: 31, spa: 31, spd: 31, spe: 31 },
          level: 50,
          experience: 0,
          shiny: true,
          friendship: 200,
          ball: null,
          hidden_power_type: null,
          different_original_trainer: false,
          dynamax_level: 0,
          gigantamax_factor: true,
          tera_type: "Electric",
          persistent_battle_data: { hp: null, move_pp: [], status: null },
        },
      ];

      const exported = exportToShowdown(team);
      const lines = exported.split("\n");

      expect(lines[0]).toBe("Sparky (Pikachu) (M) @ Light Ball");
      expect(lines).toContain("Ability: Static");
      expect(lines).toContain("Level: 50");
      expect(lines).toContain("Shiny: Yes");
      expect(lines).toContain("Friendship: 200");
      expect(lines).toContain("EVs: 252 Atk / 4 Def / 252 Spe");
      // IVs should not be printed as they are all 31
      expect(exported.includes("IVs:")).toBe(false);
      expect(lines).toContain("Jolly Nature");
      expect(lines).toContain("Gigantamax: Yes");
      expect(lines).toContain("Tera Type: Electric");
      expect(lines).toContain("- Thunderbolt");
      expect(lines).toContain("- Quick Attack");
    });

    it("should format and export partial Pokémon data correctly without crashing", () => {
      const partialTeam: Partial<MonData>[] = [
        {
          species: "Pikachu",
          moves: ["Tackle", "Thunderbolt"],
          level: 50,
        },
      ];

      const exported = exportToShowdown(partialTeam as MonData[]);
      const lines = exported.split("\n");

      expect(lines[0]).toBe("Pikachu");
      expect(lines).toContain("Level: 50");
      expect(lines).toContain("- Tackle");
      expect(lines).toContain("- Thunderbolt");
      expect(lines.length).toBe(4);
    });
  });

  describe("Round-Trip Integrity", () => {
    it("should round-trip teams correctly preserving state", () => {
      const originalTeam: Partial<MonData>[] = [
        {
          name: "Bulbasaur",
          species: "Bulbasaur",
          ability: "Overgrow",
          moves: ["Tackle", "Vine Whip"],
          level: 50,
        },
        {
          name: "Charmander",
          species: "Charmander",
          ability: "Blaze",
          moves: ["Ember", "Growl"],
          item: "Eviolite",
          nature: "Adamant",
          gender: "F",
          evs: { hp: 120, atk: 252, def: 0, spa: 0, spd: 0, spe: 136 },
          ivs: { hp: 31, atk: 31, def: 30, spa: 31, spd: 31, spe: 31 },
          level: 50,
          shiny: true,
          friendship: 120,
          tera_type: "Fire",
        },
      ];

      const exported = exportToShowdown(originalTeam as MonData[]);
      const imported = parseShowdown(exported);

      expect(imported).toEqual(originalTeam);
    });
  });
});
