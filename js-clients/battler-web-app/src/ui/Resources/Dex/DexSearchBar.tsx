import { ALL_POKEMON_TYPES } from "../../../hooks/useTypeChart";
import CategoryBadge from "../../Common/CategoryBadge";
import TypeBadge from "../../Common/TypeBadge";
import type { DexTab } from "./DexCatalogItem";
import styles from "./DexSearchBar.module.scss";

export interface DexSearchBarProps {
  tab: DexTab;
  query: string;
  onQueryChange: (query: string) => void;
  selectedTypes: string[];
  onToggleType: (type: string) => void;
  onClearTypes: () => void;
  selectedCategory: string;
  onCategoryChange: (category: string) => void;
  totalCount: number;
  showingCount: number;
}

export default function DexSearchBar({
  tab,
  query,
  onQueryChange,
  selectedTypes,
  onToggleType,
  onClearTypes,
  selectedCategory,
  onCategoryChange,
  totalCount,
  showingCount,
}: DexSearchBarProps) {
  const placeholder = `Search ${tab}`;

  return (
    <div className={styles.searchBar}>
      <div className={styles.topBar}>
        <div className={styles.inputWrapper}>
          <input
            type="text"
            className={styles.searchInput}
            placeholder={placeholder}
            value={query}
            onChange={(e) => onQueryChange(e.target.value)}
            aria-label={placeholder}
          />
          {query && (
            <button
              type="button"
              className={styles.clearButton}
              onClick={() => onQueryChange("")}
              title="Clear search"
              aria-label="Clear search"
            >
              ✕
            </button>
          )}
        </div>

        <span className={styles.resultCount}>
          {showingCount} of {totalCount}
        </span>
      </div>

      {(tab === "species" || tab === "moves") && (
        <div className={styles.chipsSection}>
          <div className={styles.chipsHeader}>
            <span className={styles.chipsLabel}>
              {tab === "species"
                ? `Filter by type${selectedTypes.length > 0 ? ` (${selectedTypes.length}/2)` : ""}:`
                : "Filter by type:"}
            </span>
            {selectedTypes.length > 0 && (
              <button
                type="button"
                className={styles.clearChipsBtn}
                onClick={onClearTypes}
                aria-label="Clear type filter"
              >
                Clear types
              </button>
            )}
          </div>
          <div
            className={styles.chipsList}
            role="group"
            aria-label={tab === "species" ? "Filter by Pokémon type" : "Filter by move type"}
          >
            {ALL_POKEMON_TYPES.map((type) => {
              const isSelected = selectedTypes.includes(type.toLowerCase());
              const isDimmed = selectedTypes.length > 0 && !isSelected;
              return (
                <button
                  key={type}
                  type="button"
                  className={`${styles.typeChip} ${isSelected ? styles.typeChipSelected : ""} ${isDimmed ? styles.typeChipDimmed : ""}`}
                  onClick={() => onToggleType(type)}
                  aria-pressed={isSelected}
                  title={`${isSelected ? "Remove" : "Filter by"} ${type}`}
                >
                  <TypeBadge type={type} size="sm" fixedWidth={false} interactive={false} />
                </button>
              );
            })}
          </div>

          {tab === "moves" && (
            <div className={styles.categoryRow}>
              <div className={styles.chipsHeader}>
                <span className={styles.chipsLabel}>Category:</span>
                {selectedCategory !== "all" && (
                  <button
                    type="button"
                    className={styles.clearChipsBtn}
                    onClick={() => onCategoryChange("all")}
                    aria-label="Clear category filter"
                  >
                    Clear category
                  </button>
                )}
              </div>
              <div className={styles.chipsList} role="group" aria-label="Filter by move category">
                {([
                  { id: "physical", label: "Physical" },
                  { id: "special", label: "Special" },
                  { id: "status", label: "Status" },
                ] as const).map(({ id, label }) => {
                  const isSelected = selectedCategory === id;
                  const isDimmed = selectedCategory !== "all" && !isSelected;
                  return (
                    <button
                      key={id}
                      type="button"
                      className={`${styles.typeChip} ${isSelected ? styles.typeChipSelected : ""} ${isDimmed ? styles.typeChipDimmed : ""}`}
                      onClick={() => onCategoryChange(isSelected ? "all" : id)}
                      aria-pressed={isSelected}
                      title={`${isSelected ? "Remove" : "Filter by"} ${label}`}
                    >
                      <CategoryBadge category={label} size="sm" fixedWidth={false} />
                    </button>
                  );
                })}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
