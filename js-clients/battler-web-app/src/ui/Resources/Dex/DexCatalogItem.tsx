import type {
  AbilitySummary,
  ItemSummary,
  MoveSummary,
  SpeciesSummary,
} from "../../../hooks/useDexCatalog";
import { itemIconUrl, monIconUrl } from "../../../utils/assets";
import CategoryBadge from "../../Common/CategoryBadge";
import TypeBadge from "../../Common/TypeBadge";
import styles from "./DexCatalogItem.module.scss";

export type DexTab = "species" | "moves" | "abilities" | "items";

export interface DexCatalogItemProps {
  tab: DexTab;
  item: SpeciesSummary | MoveSummary | AbilitySummary | ItemSummary;
  isSelected: boolean;
  onClick: () => void;
}

export default function DexCatalogItem({
  tab,
  item,
  isSelected,
  onClick,
}: DexCatalogItemProps) {
  if (tab === "species") {
    const s = item as SpeciesSummary;
    return (
      <button
        type="button"
        className={`${styles.catalogItem} ${isSelected ? styles.selected : ""}`}
        onClick={onClick}
        aria-selected={isSelected}
      >
        <img
          src={monIconUrl(s.id)}
          alt=""
          className={styles.itemIcon}
          loading="lazy"
          onError={(e) => {
            (e.currentTarget as HTMLElement).style.visibility = "hidden";
          }}
        />
        <div className={styles.itemContent}>
          <span className={styles.itemName}>{s.name}</span>
          <div className={styles.itemMeta}>
            <TypeBadge type={s.primary_type} size="sm" interactive={false} fixedWidth={false} />
            {s.secondary_type && (
              <TypeBadge type={s.secondary_type} size="sm" interactive={false} fixedWidth={false} />
            )}
          </div>
        </div>
      </button>
    );
  }

  if (tab === "moves") {
    const m = item as MoveSummary;
    return (
      <button
        type="button"
        className={`${styles.catalogItem} ${isSelected ? styles.selected : ""}`}
        onClick={onClick}
        aria-selected={isSelected}
      >
        <div className={styles.itemContent}>
          <span className={styles.itemName}>{m.name}</span>
          <div className={styles.itemMeta}>
            <CategoryBadge category={m.category} size="sm" fixedWidth={false} />
            <TypeBadge type={m.primary_type} size="sm" interactive={false} fixedWidth={false} />
          </div>
        </div>
      </button>
    );
  }

  if (tab === "abilities") {
    const a = item as AbilitySummary;
    return (
      <button
        type="button"
        className={`${styles.catalogItem} ${isSelected ? styles.selected : ""}`}
        onClick={onClick}
        aria-selected={isSelected}
      >
        <div className={styles.itemContent}>
          <span className={styles.itemName}>{a.name}</span>
          {a.description && (
            <p className={styles.itemDescription} title={a.description}>
              {a.description}
            </p>
          )}
        </div>
      </button>
    );
  }

  // Items
  const i = item as ItemSummary;
  return (
    <button
      type="button"
      className={`${styles.catalogItem} ${isSelected ? styles.selected : ""}`}
      onClick={onClick}
      aria-selected={isSelected}
    >
      <img
        src={itemIconUrl(i.id)}
        alt=""
        className={styles.itemIcon}
        loading="lazy"
        onError={(e) => {
          (e.currentTarget as HTMLElement).style.visibility = "hidden";
        }}
      />
      <div className={styles.itemContent}>
        <span className={styles.itemName}>{i.name}</span>
        {i.description && (
          <p className={styles.itemDescription} title={i.description}>
            {i.description}
          </p>
        )}
      </div>
    </button>
  );
}
