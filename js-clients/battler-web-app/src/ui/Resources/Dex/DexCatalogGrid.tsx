import { useMemo } from "react";
import type {
  AbilitySummary,
  ItemSummary,
  MoveSummary,
  SpeciesSummary,
} from "../../../hooks/useDexCatalog";
import { paginateList } from "../../../utils/dexFilter";
import DexCatalogItem, { type DexTab } from "./DexCatalogItem";
import styles from "./DexCatalogGrid.module.scss";

export interface DexCatalogGridProps {
  tab: DexTab;
  items: (SpeciesSummary | MoveSummary | AbilitySummary | ItemSummary)[];
  selectedId: string | null;
  onSelectItem: (id: string, name: string) => void;
  page: number;
  onPageChange: (newPage: number) => void;
  pageSize?: number;
}

export default function DexCatalogGrid({
  tab,
  items,
  selectedId,
  onSelectItem,
  page,
  onPageChange,
  pageSize = 50,
}: DexCatalogGridProps) {
  const { pageItems, totalPages } = useMemo(() => {
    return paginateList(items, page, pageSize);
  }, [items, page, pageSize]);

  if (items.length === 0) {
    return (
      <div className={styles.gridContainer}>
        <div className={styles.emptyState} role="status">
          None
        </div>
      </div>
    );
  }

  const renderPagination = (isHeader: boolean) => (
    <div
      className={`${isHeader ? styles.paginationHeader : styles.paginationFooter} flex-row align-center justify-center gap-m`}
    >
      <button
        type="button"
        className="btn btn-secondary btn-sm"
        disabled={page <= 1}
        onClick={() => onPageChange(page - 1)}
        aria-label="Previous page"
      >
        Previous
      </button>
      <span className={styles.pageIndicator}>
        Page {page} of {totalPages}
      </span>
      <button
        type="button"
        className="btn btn-secondary btn-sm"
        disabled={page >= totalPages}
        onClick={() => onPageChange(page + 1)}
        aria-label="Next page"
      >
        Next
      </button>
    </div>
  );

  return (
    <div className={styles.gridContainer}>
      {totalPages > 1 && renderPagination(true)}

      <div className={styles.itemsGrid} role="list">
        {pageItems.map((item) => (
          <DexCatalogItem
            key={item.id}
            tab={tab}
            item={item}
            isSelected={selectedId === item.id}
            onClick={() => onSelectItem(item.id, item.name)}
          />
        ))}
      </div>

      {totalPages > 1 && renderPagination(false)}
    </div>
  );
}
