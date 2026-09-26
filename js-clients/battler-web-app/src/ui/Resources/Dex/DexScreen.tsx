import { useMemo, useState } from "react";
import { useDexCatalog } from "../../../hooks/useDexCatalog";
import {
  filterAbilities,
  filterItems,
  filterMoves,
  filterSpecies,
} from "../../../utils/dexFilter";
import Tabs from "../../Common/Tabs";
import DexCatalogGrid from "./DexCatalogGrid";
import type { DexTab } from "./DexCatalogItem";
import DexInspector from "./DexInspector";
import DexSearchBar from "./DexSearchBar";
import styles from "./DexScreen.module.scss";

export interface DexScreenProps {
  onBack: () => void;
}

const TAB_OPTIONS: { value: DexTab; label: string }[] = [
  { value: "species", label: "Species" },
  { value: "moves", label: "Moves" },
  { value: "abilities", label: "Abilities" },
  { value: "items", label: "Items" },
];

export default function DexScreen({ onBack }: DexScreenProps) {
  const { catalog } = useDexCatalog();

  const [tab, setTab] = useState<DexTab>("species");
  const [query, setQuery] = useState("");
  const [selectedTypes, setSelectedTypes] = useState<string[]>([]);
  const [selectedCategory, setSelectedCategory] = useState("all");
  const [page, setPage] = useState(1);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [selectedName, setSelectedName] = useState<string | null>(null);

  const handleTabChange = (newTab: DexTab) => {
    setTab(newTab);
    setQuery("");
    setSelectedTypes([]);
    setSelectedCategory("all");
    setPage(1);
    setSelectedId(null);
    setSelectedName(null);
  };

  const handleQueryChange = (newQuery: string) => {
    setQuery(newQuery);
    setPage(1);
  };

  const handleToggleType = (type: string) => {
    const norm = type.trim().toLowerCase();
    if (tab === "moves") {
      // Moves have 1 type: toggle on or off
      setSelectedTypes((prev) => (prev.includes(norm) ? [] : [norm]));
    } else {
      // Species can have up to 2 types for dual-type combinations
      setSelectedTypes((prev) => {
        if (prev.includes(norm)) {
          return prev.filter((t) => t !== norm);
        }
        if (prev.length >= 2) {
          return [prev[1], norm];
        }
        return [...prev, norm];
      });
    }
    setPage(1);
  };

  const handleClearTypes = () => {
    setSelectedTypes([]);
    setPage(1);
  };

  const handleCategoryChange = (newCategory: string) => {
    setSelectedCategory(newCategory);
    setPage(1);
  };

  const filteredItems = useMemo(() => {
    switch (tab) {
      case "species":
        return filterSpecies(catalog.species, {
          query,
          selectedTypes,
        });
      case "moves":
        return filterMoves(catalog.moves, {
          query,
          selectedTypes,
          category: selectedCategory,
        });
      case "abilities":
        return filterAbilities(catalog.abilities, query);
      case "items":
        return filterItems(catalog.items, query);
      default:
        return [];
    }
  }, [catalog, tab, query, selectedTypes, selectedCategory]);

  const totalCatalogCount = useMemo(() => {
    switch (tab) {
      case "species":
        return catalog.species.length;
      case "moves":
        return catalog.moves.length;
      case "abilities":
        return catalog.abilities.length;
      case "items":
        return catalog.items.length;
      default:
        return 0;
    }
  }, [catalog, tab]);

  const handleSelectItem = (id: string, name: string) => {
    if (selectedId === id) {
      setSelectedId(null);
      setSelectedName(null);
    } else {
      setSelectedId(id);
      setSelectedName(name);
    }
  };

  const handleCloseInspector = () => {
    setSelectedId(null);
    setSelectedName(null);
  };

  return (
    <div className="page-container scroll-y">
      <header className="screen-header flex-row justify-between align-center gap-m flex-tablet-col align-tablet-start">
        <div className="flex-row align-center gap-m">
          <button
            type="button"
            className="btn btn-secondary btn-sm"
            onClick={onBack}
            title="Back to Resources"
          >
            ← Resources
          </button>
          <div className="screen-header-title flex-col gap-xs">
            <h2>Dex</h2>
          </div>
        </div>

        <Tabs options={TAB_OPTIONS} active={tab} onChange={handleTabChange} />
      </header>

      <section className="flex-col gap-m flex-1 min-w-0 w-full">
        <DexSearchBar
          tab={tab}
          query={query}
          onQueryChange={handleQueryChange}
          selectedTypes={selectedTypes}
          onToggleType={handleToggleType}
          onClearTypes={handleClearTypes}
          selectedCategory={selectedCategory}
          onCategoryChange={handleCategoryChange}
          totalCount={totalCatalogCount}
          showingCount={filteredItems.length}
        />

        <div className={styles.dexLayout}>
          <DexCatalogGrid
            tab={tab}
            items={filteredItems}
            selectedId={selectedId}
            onSelectItem={handleSelectItem}
            page={page}
            onPageChange={setPage}
          />

          <DexInspector
            tab={tab}
            selectedName={selectedName}
            onClose={handleCloseInspector}
          />
        </div>
      </section>
    </div>
  );
}
