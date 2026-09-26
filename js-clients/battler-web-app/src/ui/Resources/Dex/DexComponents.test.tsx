import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import DexCatalogGrid from "./DexCatalogGrid";
import DexCatalogItem from "./DexCatalogItem";
import DexInspector from "./DexInspector";
import DexSearchBar from "./DexSearchBar";
import DexScreen from "./DexScreen";
import * as dexCatalogHook from "../../../hooks/useDexCatalog";
import * as dataStoreHook from "../../../hooks/useDataStore";

describe("Dex Components", () => {
  const mockSpecies = {
    id: "bulbasaur",
    name: "Bulbasaur",
    primary_type: "Grass",
    secondary_type: "Poison",
  };

  const mockMove = {
    id: "tackle",
    name: "Tackle",
    primary_type: "Normal",
    category: "Physical",
  };

  const mockAbility = {
    id: "overgrow",
    name: "Overgrow",
    description: "Powers up Grass-type moves when HP is low.",
  };

  const mockItem = {
    id: "leftovers",
    name: "Leftovers",
    description: "An item to be held by a Pokémon.",
  };

  describe("DexCatalogItem", () => {
    it("renders species item with icon, name, and type badges", () => {
      const html = renderToStaticMarkup(
        <DexCatalogItem
          tab="species"
          item={mockSpecies}
          isSelected={false}
          onClick={vi.fn()}
        />,
      );

      expect(html).toContain("Bulbasaur");
      expect(html).toContain("Grass");
      expect(html).toContain("Poison");
      expect(html).toContain("/assets/mons/icons/bulbasaur.png");
      expect(html).toContain('aria-selected="false"');
      expect(html).not.toContain('aria-selected="true"');
    });

    it("renders move item with category badge and type badge", () => {
      const html = renderToStaticMarkup(
        <DexCatalogItem
          tab="moves"
          item={mockMove}
          isSelected={true}
          onClick={vi.fn()}
        />,
      );

      expect(html).toContain("Tackle");
      expect(html).toContain("Normal");
      expect(html).toContain("Physical");
      expect(html).toContain("selected");
      expect(html).toContain('aria-selected="true"');
    });

    it("renders ability item with name and description", () => {
      const html = renderToStaticMarkup(
        <DexCatalogItem
          tab="abilities"
          item={mockAbility}
          isSelected={false}
          onClick={vi.fn()}
        />,
      );

      expect(html).toContain("Overgrow");
      expect(html).toContain("Powers up Grass-type moves when HP is low.");
    });

    it("renders item with icon, name, and description", () => {
      const html = renderToStaticMarkup(
        <DexCatalogItem
          tab="items"
          item={mockItem}
          isSelected={false}
          onClick={vi.fn()}
        />,
      );

      expect(html).toContain("Leftovers");
      expect(html).toContain("An item to be held by a Pokémon.");
      expect(html).toContain("/assets/items/leftovers.png");
    });
  });

  describe("DexSearchBar", () => {
    it("renders search input, result count, and type chips for species tab", () => {
      const html = renderToStaticMarkup(
        <DexSearchBar
          tab="species"
          query=""
          onQueryChange={vi.fn()}
          selectedTypes={["grass"]}
          onToggleType={vi.fn()}
          onClearTypes={vi.fn()}
          selectedCategory="all"
          onCategoryChange={vi.fn()}
          totalCount={100}
          showingCount={10}
        />,
      );

      expect(html).toContain('placeholder="Search species"');
      expect(html).toContain("10 of 100");
      expect(html).toContain("Filter by type (1/2):");
      expect(html).toContain("Grass");
      expect(html).toContain("Clear types");
      expect(html).not.toContain("Clear search");
    });

    it("renders category chips for moves tab and clear button when query is present", () => {
      const html = renderToStaticMarkup(
        <DexSearchBar
          tab="moves"
          query="tackle"
          onQueryChange={vi.fn()}
          selectedTypes={["normal"]}
          onToggleType={vi.fn()}
          onClearTypes={vi.fn()}
          selectedCategory="physical"
          onCategoryChange={vi.fn()}
          totalCount={500}
          showingCount={1}
        />,
      );

      expect(html).toContain('placeholder="Search moves"');
      expect(html).toContain("1 of 500");
      expect(html).toContain("Normal");
      expect(html).toContain("Physical");
      expect(html).toContain("Special");
      expect(html).toContain("Status");
      expect(html).toContain("Clear search");
      expect(html).toContain("✕");
    });

    it("hides type and category filters on abilities and items tabs", () => {
      const htmlAbilities = renderToStaticMarkup(
        <DexSearchBar
          tab="abilities"
          query=""
          onQueryChange={vi.fn()}
          selectedTypes={[]}
          onToggleType={vi.fn()}
          onClearTypes={vi.fn()}
          selectedCategory="all"
          onCategoryChange={vi.fn()}
          totalCount={300}
          showingCount={300}
        />,
      );

      expect(htmlAbilities).toContain('placeholder="Search abilities"');
      expect(htmlAbilities).not.toContain("Filter by type");
      expect(htmlAbilities).not.toContain("Category:");

      const htmlItems = renderToStaticMarkup(
        <DexSearchBar
          tab="items"
          query=""
          onQueryChange={vi.fn()}
          selectedTypes={[]}
          onToggleType={vi.fn()}
          onClearTypes={vi.fn()}
          selectedCategory="all"
          onCategoryChange={vi.fn()}
          totalCount={1500}
          showingCount={1500}
        />,
      );

      expect(htmlItems).toContain('placeholder="Search items"');
      expect(htmlItems).not.toContain("Filter by type");
      expect(htmlItems).not.toContain("Category:");
    });
  });

  describe("DexCatalogGrid", () => {
    it("renders empty state with sentence-cased 'None'", () => {
      const html = renderToStaticMarkup(
        <DexCatalogGrid
          tab="species"
          items={[]}
          selectedId={null}
          onSelectItem={vi.fn()}
          page={1}
          onPageChange={vi.fn()}
        />,
      );

      expect(html).toContain("None");
      expect(html).toContain('role="status"');
    });

    it("renders paginated items and mirrored top/bottom pagination when total items exceed page size", () => {
      const items = Array.from({ length: 60 }, (_, idx) => ({
        id: `item-${idx}`,
        name: `Item ${idx}`,
        description: `Description ${idx}`,
      }));

      const html = renderToStaticMarkup(
        <DexCatalogGrid
          tab="items"
          items={items}
          selectedId="item-0"
          onSelectItem={vi.fn()}
          page={1}
          onPageChange={vi.fn()}
          pageSize={50}
        />,
      );

      expect(html).toContain("Item 0");
      expect(html).toContain("Item 49");
      expect(html).not.toContain("Item 50");
      const pageIndicators = html.match(/Page 1 of 2/g);
      expect(pageIndicators).toHaveLength(2); // Mirrored top and bottom
      const prevButtons = html.match(/>Previous</g);
      expect(prevButtons).toHaveLength(2);
      const nextButtons = html.match(/>Next</g);
      expect(nextButtons).toHaveLength(2);
    });
  });

  describe("DexInspector", () => {
    it("renders placeholder when no item is selected", () => {
      const html = renderToStaticMarkup(
        <DexInspector
          tab="species"
          selectedName={null}
          onClose={vi.fn()}
        />,
      );

      expect(html).toContain("None");
    });

    it("renders details when species item is selected", () => {
      vi.spyOn(dataStoreHook, "useResourceData").mockReturnValue({
        data: {
          name: "Bulbasaur",
          primary_type: "Grass",
          secondary_type: "Poison",
          base_stats: { hp: 45, atk: 49, def: 49, spa: 65, spd: 65, spe: 45 },
          weight: 69,
          height: 7,
          abilities: ["Overgrow"],
          hidden_ability: "Chlorophyll",
        } as any,
        description: { description: "A strange seed was planted on its back at birth." } as any,
        loading: false,
      });

      const html = renderToStaticMarkup(
        <DexInspector
          tab="species"
          selectedName="Bulbasaur"
          onClose={vi.fn()}
        />,
      );

      expect(html).toContain("Bulbasaur");
      expect(html).toContain("A strange seed was planted on its back at birth.");
      expect(html).toContain("Chlorophyll");
      expect(html).toContain("Overgrow");
      expect(html).toContain("Close details");
      expect(html).toContain('role="dialog"');
    });
  });

  describe("DexScreen", () => {
    it("renders header with back button, Dex title, and tabs", () => {
      vi.spyOn(dexCatalogHook, "useDexCatalog").mockReturnValue({
        catalog: {
          species: [mockSpecies],
          moves: [mockMove],
          abilities: [mockAbility],
          items: [mockItem],
        },
        loading: false,
        error: null,
      });

      const html = renderToStaticMarkup(<DexScreen onBack={vi.fn()} />);

      expect(html).toContain("← Resources");
      expect(html).toContain("Dex");
      expect(html).toContain("Species");
      expect(html).toContain("Moves");
      expect(html).toContain("Abilities");
      expect(html).toContain("Items");
      expect(html).toContain("Bulbasaur");
      expect(html).toContain("None");
    });
  });
});
