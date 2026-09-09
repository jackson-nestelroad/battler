import type { ItemData } from "battler-types";
import styles from "./ItemTooltipCard.module.scss";

interface ItemTooltipCardProps {
  data: ItemData;
}

export default function ItemTooltipCard({ data }: ItemTooltipCardProps) {
  const flags = (data.flags || []).slice().sort();

  return (
    <article className={styles.card}>
      <header className={styles.header}>
        <span className={styles.name}>{data.name}</span>
        <span className={styles.subtitle}>Item</span>
      </header>

      {flags.length > 0 && (
        <section className="flex-col gap-xxs">
          <span className={styles.sectionTitle}>Flags</span>
          <div className={styles.flagsList}>
            {flags.map((flag) => (
              <span key={flag} className={styles.flagBadge}>
                {flag}
              </span>
            ))}
          </div>
        </section>
      )}
    </article>
  );
}
