import type { MoveData } from "battler-types";
import { formatAccuracy, formatMoveTarget } from "../../../utils/dataTooltipFormatting";
import TypeBadge from "../TypeBadge";
import styles from "./MoveTooltipCard.module.scss";

interface MoveTooltipCardProps {
  data: MoveData;
}

export default function MoveTooltipCard({ data }: MoveTooltipCardProps) {
  const flags = (data.flags || []).slice().sort();

  const basePowerStr = data.base_power && data.base_power > 0 ? String(data.base_power) : "—";
  const accuracyStr = formatAccuracy(data.accuracy);

  const maxPp = data.no_pp_boosts ? data.pp : Math.floor(data.pp * 1.6);
  const ppStr = data.pp ? `${data.pp} (max ${maxPp})` : "—";

  const priorityStr =
    data.priority > 0
      ? `+${data.priority}`
      : data.priority < 0
        ? `${data.priority}`
        : null;

  const categoryClass =
    data.category === "Physical"
      ? styles.categoryPhysical
      : data.category === "Special"
        ? styles.categorySpecial
        : styles.categoryStatus;

  return (
    <article className={styles.card}>
      <header className={styles.header}>
        <span className={styles.name}>{data.name}</span>
        <span className={styles.subtitle}>Move</span>
        <div className="flex-row align-center gap-xs">
          <span className={categoryClass}>{data.category}</span>
          <TypeBadge type={data.primary_type} size="sm" />
        </div>
      </header>

      <section className={styles.traitsGrid}>
        <div className={styles.traitRow}>
          <span className={styles.traitLabel}>Base Power:</span>
          <span className={styles.traitValue}>{basePowerStr}</span>
        </div>
        <div className={styles.traitRow}>
          <span className={styles.traitLabel}>Accuracy:</span>
          <span className={styles.traitValue}>{accuracyStr}</span>
        </div>
        <div className={styles.traitRow}>
          <span className={styles.traitLabel}>PP:</span>
          <span className={styles.traitValue}>{ppStr}</span>
        </div>
        {priorityStr && (
          <div className={styles.traitRow}>
            <span className={styles.traitLabel}>Priority:</span>
            <span className={styles.traitValue}>{priorityStr}</span>
          </div>
        )}
        <div className={styles.traitRow}>
          <span className={styles.traitLabel}>Target:</span>
          <span className={styles.traitValue}>{formatMoveTarget(data.target)}</span>
        </div>
      </section>

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
