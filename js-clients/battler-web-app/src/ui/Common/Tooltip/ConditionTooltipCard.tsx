import type { ConditionData } from "battler-types";
import styles from "./ConditionTooltipCard.module.scss";

interface ConditionTooltipCardProps {
  data: ConditionData;
}

export default function ConditionTooltipCard({ data }: ConditionTooltipCardProps) {
  return (
    <article className={styles.card}>
      <header className={styles.header}>
        <span className={styles.name}>{data.name}</span>
        <span className={styles.subtitle}>{data.condition_type}</span>
      </header>

      {data.no_copy && (
        <section className={styles.traitsGrid}>
          <div className={styles.traitRow}>
            <span className={styles.traitLabel}>Baton Pass:</span>
            <span className={styles.traitValue}>No copy</span>
          </div>
        </section>
      )}
    </article>
  );
}
