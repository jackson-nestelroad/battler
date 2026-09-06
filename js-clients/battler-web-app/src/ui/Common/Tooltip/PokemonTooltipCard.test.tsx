import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import PokemonTooltipCard from "./PokemonTooltipCard";
import type { MonTooltipViewModel } from "../../../utils/monTooltipModel";

function createMockMon(overrides: Partial<MonTooltipViewModel> = {}): MonTooltipViewModel {
  return {
    species: "Charizard",
    name: "Charizard",
    level: 50,
    gender: "M",
    shiny: false,
    types: ["Fire", "Flying"],
    teraType: null,
    isTerastallized: false,
    ball: "pokeball",
    ownerLabel: "Your Mon",
    weightKg: 90.5,
    hp: 150,
    maxHp: 150,
    hpPercentage: 100,
    status: "ok",
    isFainted: false,
    boosts: [],
    conditions: [],
    ability: "Blaze",
    item: "Leftovers",
    moves: [],
    stats: null,
    baseSummary: null,
    ...overrides,
  };
}

describe("PokemonTooltipCard", () => {
  describe("Terastallization display", () => {
    it("renders Tera type as primary and base types as secondary when Terastallized", () => {
      const mon = createMockMon({
        types: ["Fire", "Flying"],
        teraType: "Water",
        isTerastallized: true,
      });

      const html = renderToStaticMarkup(<PokemonTooltipCard data={mon} />);

      // Primary type is Water
      expect(html).toContain("Water");
      expect(html).toContain("Terastallized");

      // Base types are preserved and indicated
      expect(html).toContain("Base:");
      expect(html).toContain("Fire");
      expect(html).toContain("Flying");
    });

    it("renders Stellar as primary Tera type with base types below when Terastallized", () => {
      const mon = createMockMon({
        types: ["Grass", "Poison"],
        teraType: "Stellar",
        isTerastallized: true,
      });

      const html = renderToStaticMarkup(<PokemonTooltipCard data={mon} />);

      expect(html).toContain("Stellar");
      expect(html).toContain("Terastallized");
      expect(html).toContain("Base:");
      expect(html).toContain("Grass");
      expect(html).toContain("Poison");
    });

    it("renders base types and Tera Type hint when not Terastallized", () => {
      const mon = createMockMon({
        types: ["Fire", "Flying"],
        teraType: "Water",
        isTerastallized: false,
      });

      const html = renderToStaticMarkup(<PokemonTooltipCard data={mon} />);

      expect(html).toContain("Fire");
      expect(html).toContain("Flying");
      expect(html).toContain("Tera Type:");
      expect(html).toContain("Water");
      expect(html).not.toContain("Terastallized");
      expect(html).not.toContain("Base:");
    });

    it("does not render Tera Type pill when teraType is null (e.g. disabled format)", () => {
      const mon = createMockMon({
        types: ["Fire", "Flying"],
        teraType: null,
        isTerastallized: false,
      });

      const html = renderToStaticMarkup(<PokemonTooltipCard data={mon} />);

      expect(html).toContain("Fire");
      expect(html).toContain("Flying");
      expect(html).not.toContain("Tera Type");
      expect(html).not.toContain("Terastallized");
      expect(html).not.toContain("Base:");
    });
  });

  describe("Volatiles section consolidation", () => {
    it("renders Dynamax only once in the volatile section without duplicating", () => {
      const mon = createMockMon({
        isDynamaxed: true,
        conditions: ["Dynamax"],
      });

      const html = renderToStaticMarkup(<PokemonTooltipCard data={mon} />);

      // Ensure "Dynamax" appears exactly once in the rendered HTML
      const matches = html.match(/Dynamax/g);
      expect(matches).not.toBeNull();
      expect(matches!.length).toBe(1);
    });

    it("renders Dynamax in volatile section even if omitted from conditions array", () => {
      const mon = createMockMon({
        isDynamaxed: true,
        conditions: [],
      });

      const html = renderToStaticMarkup(<PokemonTooltipCard data={mon} />);

      const matches = html.match(/Dynamax/g);
      expect(matches).not.toBeNull();
      expect(matches!.length).toBe(1);
    });

    it("renders Transformed in volatile section with original species", () => {
      const mon = createMockMon({
        species: "Mewtwo",
        name: "Mewtwo",
        isTransformed: true,
        originalSpecies: "Ditto",
      });

      const html = renderToStaticMarkup(<PokemonTooltipCard data={mon} />);

      expect(html).toContain("Transformed (Ditto)");
    });

    it("renders Transformed in volatile section without original species if unknown", () => {
      const mon = createMockMon({
        species: "Mewtwo",
        name: "Mewtwo",
        isTransformed: true,
        originalSpecies: null,
      });

      const html = renderToStaticMarkup(<PokemonTooltipCard data={mon} />);

      expect(html).toContain("Transformed");
    });

    it("renders multiple volatiles alongside stat boosts", () => {
      const mon = createMockMon({
        boosts: [{ stat: "atk", stage: 2, label: "+2 Atk" }],
        conditions: ["Taunt", "Dynamax"],
        isDynamaxed: true,
      });

      const html = renderToStaticMarkup(<PokemonTooltipCard data={mon} />);

      expect(html).toContain("+2 Atk");
      expect(html).toContain("Taunt");
      expect(html).toContain("Dynamax");
      expect(html.match(/Dynamax/g)!.length).toBe(1);
    });
  });
});
