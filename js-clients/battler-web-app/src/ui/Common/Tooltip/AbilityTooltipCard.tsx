import type { AbilityData } from "battler-types";
import styles from "./AbilityTooltipCard.module.scss";

interface AbilityTooltipCardProps {
  data: AbilityData;
}

export default function AbilityTooltipCard({ data }: AbilityTooltipCardProps) {
  const flags = (data.flags || []).slice().sort();

  return (
    <article className={styles.card}>
      <header className={styles.header}>
        <span className={styles.name}>{data.name}</span>
        <span className={styles.subtitle}>Ability</span>
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
