import styles from "./ResourcesHome.module.scss";

export interface ResourcesHomeProps {
  onSelectResource: (resource: "type-chart" | "dex") => void;
}

export default function ResourcesHome({ onSelectResource }: ResourcesHomeProps) {
  return (
    <div className="page-container">
      <header className="screen-header">
        <div className="screen-header-title flex-col gap-xs">
          <h2>Resources</h2>
        </div>
      </header>

      <div className={styles.resourcesGrid}>
        <button
          type="button"
          className={styles.resourceCard}
          onClick={() => onSelectResource("type-chart")}
        >
          <span className={styles.cardTitle}>Type Chart</span>
          <p className={styles.cardDescription}>
            Effectiveness matrix and matchup calculator.
          </p>
        </button>

        <button
          type="button"
          className={styles.resourceCard}
          onClick={() => onSelectResource("dex")}
        >
          <span className={styles.cardTitle}>Dex</span>
          <p className={styles.cardDescription}>
            Catalog of Pokémon species, moves, abilities, and items.
          </p>
        </button>
      </div>
    </div>
  );
}
