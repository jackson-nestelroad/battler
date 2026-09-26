import { describe, expect, it } from "vitest";
import {
  filterAbilities,
  filterItems,
  filterMoves,
  filterSpecies,
  paginateList,
} from "./dexFilter";
import type {
  AbilitySummary,
  ItemSummary,
  MoveSummary,
  SpeciesSummary,
} from "../hooks/useDexCatalog";

describe("dexFilter", () => {
  const speciesList: SpeciesSummary[] = [
    { id: "bulbasaur", name: "Bulbasaur", primary_type: "Grass", secondary_type: "Poison" },
    { id: "charmander", name: "Charmander", primary_type: "Fire", secondary_type: null },
    { id: "squirtle", name: "Squirtle", primary_type: "Water", secondary_type: null },
    { id: "mrmime", name: "Mr. Mime", primary_type: "Psychic", secondary_type: "Fairy" },
    { id: "hooh", name: "Ho-Oh", primary_type: "Fire", secondary_type: "Flying" },
  ];

  const moveList: MoveSummary[] = [
    { id: "tackle", name: "Tackle", primary_type: "Normal", category: "Physical" },
    { id: "flamethrower", name: "Flamethrower", primary_type: "Fire", category: "Special" },
    { id: "willowisp", name: "Will-O-Wisp", primary_type: "Fire", category: "Status" },
    { id: "surf", name: "Surf", primary_type: "Water", category: "Special" },
  ];

  const abilityList: AbilitySummary[] = [
    { id: "overgrow", name: "Overgrow", description: "Powers up Grass-type moves." },
    { id: "blaze", name: "Blaze", description: "Powers up Fire-type moves." },
    { id: "torrent", name: "Torrent", description: "Powers up Water-type moves." },
  ];

  const itemList: ItemSummary[] = [
    { id: "choiceband", name: "Choice Band", description: "Boosts Attack." },
    { id: "leftovers", name: "Leftovers", description: "Restores HP." },
    { id: "lifeorb", name: "Life Orb", description: "Boosts damage but hurts holder." },
  ];

  describe("filterSpecies", () => {
    it("returns all species when no query or type filter is provided", () => {
      expect(filterSpecies(speciesList, {})).toEqual(speciesList);
    });

    it("filters species by query substring case-insensitively with punctuation tolerance", () => {
      expect(filterSpecies(speciesList, { query: "bulb" })).toEqual([speciesList[0]]);
      expect(filterSpecies(speciesList, { query: "mr mime" })).toEqual([speciesList[3]]);
      expect(filterSpecies(speciesList, { query: "ho oh" })).toEqual([speciesList[4]]);
      expect(filterSpecies(speciesList, { query: "HO-OH" })).toEqual([speciesList[4]]);
    });

    it("filters species by primary or secondary type", () => {
      const grass = filterSpecies(speciesList, { type: "Grass" });
      expect(grass.map((s) => s.id)).toEqual(["bulbasaur"]);

      const poison = filterSpecies(speciesList, { type: "Poison" });
      expect(poison.map((s) => s.id)).toEqual(["bulbasaur"]);

      const fire = filterSpecies(speciesList, { type: "Fire" });
      expect(fire.map((s) => s.id)).toEqual(["charmander", "hooh"]);

      const flying = filterSpecies(speciesList, { type: "Flying" });
      expect(flying.map((s) => s.id)).toEqual(["hooh"]);
    });

    it("filters species by dual-type combination using selectedTypes", () => {
      const fireFlying = filterSpecies(speciesList, { selectedTypes: ["Fire", "Flying"] });
      expect(fireFlying.map((s) => s.id)).toEqual(["hooh"]);

      // Reverse order should match the same dual-type combination
      const flyingFire = filterSpecies(speciesList, { selectedTypes: ["Flying", "Fire"] });
      expect(flyingFire.map((s) => s.id)).toEqual(["hooh"]);

      // Pure fire should not match dual fire/flying
      expect(fireFlying.map((s) => s.id)).not.toContain("charmander");

      // Non-existent combination returns empty
      const grassFire = filterSpecies(speciesList, { selectedTypes: ["Grass", "Fire"] });
      expect(grassFire).toEqual([]);
    });

    it("ignores 'all' as type filter", () => {
      expect(filterSpecies(speciesList, { type: "all" })).toEqual(speciesList);
    });

    it("combines query and type filter", () => {
      const res = filterSpecies(speciesList, { query: "ho", type: "Flying" });
      expect(res.map((s) => s.id)).toEqual(["hooh"]);

      const empty = filterSpecies(speciesList, { query: "bulb", type: "Fire" });
      expect(empty).toEqual([]);
    });
  });

  describe("filterMoves", () => {
    it("returns all moves when no filter is provided", () => {
      expect(filterMoves(moveList, {})).toEqual(moveList);
    });

    it("filters moves by query with punctuation tolerance", () => {
      expect(filterMoves(moveList, { query: "will o wisp" })).toEqual([moveList[2]]);
      expect(filterMoves(moveList, { query: "surf" })).toEqual([moveList[3]]);
    });

    it("filters moves by type", () => {
      const fireMoves = filterMoves(moveList, { type: "Fire" });
      expect(fireMoves.map((m) => m.id)).toEqual(["flamethrower", "willowisp"]);
    });

    it("filters moves by category", () => {
      const statusMoves = filterMoves(moveList, { category: "Status" });
      expect(statusMoves.map((m) => m.id)).toEqual(["willowisp"]);

      const specialMoves = filterMoves(moveList, { category: "Special" });
      expect(specialMoves.map((m) => m.id)).toEqual(["flamethrower", "surf"]);
    });

    it("combines type, category, and query filters", () => {
      const res = filterMoves(moveList, { query: "flame", type: "Fire", category: "Special" });
      expect(res.map((m) => m.id)).toEqual(["flamethrower"]);

      const empty = filterMoves(moveList, { query: "flame", type: "Fire", category: "Physical" });
      expect(empty).toEqual([]);
    });
  });

  describe("filterAbilities", () => {
    it("returns all abilities when query is empty or undefined", () => {
      expect(filterAbilities(abilityList)).toEqual(abilityList);
      expect(filterAbilities(abilityList, "")).toEqual(abilityList);
    });

    it("filters abilities by name or id", () => {
      expect(filterAbilities(abilityList, "blaze")).toEqual([abilityList[1]]);
      expect(filterAbilities(abilityList, "grow")).toEqual([abilityList[0]]);
      expect(filterAbilities(abilityList, "nonexistent")).toEqual([]);
    });
  });

  describe("filterItems", () => {
    it("returns all items when query is empty or undefined", () => {
      expect(filterItems(itemList)).toEqual(itemList);
      expect(filterItems(itemList, "")).toEqual(itemList);
    });

    it("filters items by name or id", () => {
      expect(filterItems(itemList, "band")).toEqual([itemList[0]]);
      expect(filterItems(itemList, "left")).toEqual([itemList[1]]);
      expect(filterItems(itemList, "life orb")).toEqual([itemList[2]]);
    });

    it("excludes dynamax crystals by default, but allows searching for them", () => {
      const itemsWithCrystal: ItemSummary[] = [
        ...itemList,
        { id: "dynamaxcrystaland15", name: "★And15", description: null },
      ];

      expect(filterItems(itemsWithCrystal)).toEqual(itemList);
      expect(filterItems(itemsWithCrystal, "and15")).toEqual([
        { id: "dynamaxcrystaland15", name: "★And15", description: null },
      ]);
    });

    it("sorts items starting with symbols to the end", () => {
      const mixedItems: ItemSummary[] = [
        { id: "percentitem", name: "%Special Item", description: null },
        { id: "choiceband", name: "Choice Band", description: null },
        { id: "apple", name: "Apple", description: null },
      ];

      const res = filterItems(mixedItems);
      expect(res.map((i) => i.name)).toEqual(["Apple", "Choice Band", "%Special Item"]);
    });
  });

  describe("paginateList", () => {
    const numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    it("correctly slices first page", () => {
      const res = paginateList(numbers, 1, 4);
      expect(res.pageItems).toEqual([1, 2, 3, 4]);
      expect(res.totalPages).toBe(3);
      expect(res.totalItems).toBe(10);
    });

    it("correctly slices middle and last pages", () => {
      const page2 = paginateList(numbers, 2, 4);
      expect(page2.pageItems).toEqual([5, 6, 7, 8]);

      const page3 = paginateList(numbers, 3, 4);
      expect(page3.pageItems).toEqual([9, 10]);
    });

    it("clamps page when out of bounds", () => {
      const highPage = paginateList(numbers, 999, 4);
      expect(highPage.pageItems).toEqual([9, 10]);

      const lowPage = paginateList(numbers, -5, 4);
      expect(lowPage.pageItems).toEqual([1, 2, 3, 4]);
    });

    it("handles empty lists gracefully", () => {
      const empty = paginateList([], 1, 20);
      expect(empty.pageItems).toEqual([]);
      expect(empty.totalPages).toBe(1);
      expect(empty.totalItems).toBe(0);
    });
  });
});
