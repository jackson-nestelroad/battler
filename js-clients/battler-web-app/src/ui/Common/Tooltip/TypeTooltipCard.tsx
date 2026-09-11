import { toId } from "../../../hooks/useDataStore";
import {
  ALL_POKEMON_TYPES,
  getEffectiveness,
  useTypeChart,
} from "../../../hooks/useTypeChart";
import TypeBadge from "../TypeBadge";
import cardStyles from "./DataTooltipCard.module.scss";
import TooltipEffectButton from "./TooltipEffectButton";
import styles from "./TypeTooltipCard.module.scss";

export interface TypeTooltipCardProps {
  type: string;
}

function normalizeType(name: string): string {
  const trimmed = name.trim();
  return trimmed.charAt(0).toUpperCase() + trimmed.slice(1).toLowerCase();
}

function TypeListRow({
  label,
  multiplierText,
  multiplierClass,
  types,
}: {
  label: string;
  multiplierText: string;
  multiplierClass: string;
  types: string[];
}) {
  return (
    <div className={styles.typeRow}>
      <div className="flex-row align-center gap-xs">
        <span className={styles.typeRowLabel}>{label}</span>
        <span className={`${styles.multBadge} ${multiplierClass}`}>
          ({multiplierText})
        </span>
      </div>
      <div className={styles.typeList}>
        {types.length > 0 ? (
          types.map((t) => (
            <TypeBadge
              key={t}
              type={t}
              size="sm"
              fixedWidth={false}
              interactive={false}
            />
          ))
        ) : (
          <span className={styles.noneText}>None</span>
        )}
      </div>
    </div>
  );
}

export default function TypeTooltipCard({ type }: TypeTooltipCardProps) {
  const { typeChart, loading, error } = useTypeChart();
  const canonicalType = normalizeType(type);

  if (loading) {
    return (
      <div
        className={`${cardStyles.card} ${styles.typeTooltipCard} ${cardStyles.loadingCard}`}
        role="status"
        aria-live="polite"
      >
        <span className="spinner spinner-sm" />
        <span>Loading...</span>
      </div>
    );
  }

  if (error || !typeChart) {
    return (
      <div
        className={`${cardStyles.card} ${styles.typeTooltipCard} ${cardStyles.emptyCard}`}
        role="status"
        aria-live="polite"
      >
        <span className={cardStyles.emptyText}>
          {error || "Type chart unavailable"}
        </span>
      </div>
    );
  }

  const offensiveSuper: string[] = [];
  const offensiveResist: string[] = [];
  const offensiveImmune: string[] = [];

  const defensiveWeak: string[] = [];
  const defensiveResist: string[] = [];
  const defensiveImmune: string[] = [];

  for (const t of ALL_POKEMON_TYPES) {
    // Offense: canonicalType attacking t
    const off = getEffectiveness(typeChart, canonicalType, t);
    if (off === 2) offensiveSuper.push(t);
    else if (off === 0.5) offensiveResist.push(t);
    else if (off === 0) offensiveImmune.push(t);

    // Defense: t attacking canonicalType
    const def = getEffectiveness(typeChart, t, canonicalType);
    if (def === 2) defensiveWeak.push(t);
    else if (def === 0.5) defensiveResist.push(t);
    else if (def === 0) defensiveImmune.push(t);
  }

  return (
    <article className={`${cardStyles.card} ${styles.typeTooltipCard}`}>
      <header className={cardStyles.header}>
        <div className="flex-row justify-between align-center w-full">
          <span className={cardStyles.name}>{canonicalType}</span>
          <TooltipEffectButton
            type="condition"
            name={`${toId(canonicalType)}type`}
            displayName={`${canonicalType} Type`}
          />
        </div>
        <span className={cardStyles.subtitle}>Type</span>
        <div className="flex-row align-center gap-xs">
          <TypeBadge type={canonicalType} size="sm" interactive={false} />
        </div>
      </header>

      <section className="flex-col gap-xs">
        <h4 className={styles.sectionTitle}>Offense</h4>
        <TypeListRow
          label="Super effective"
          multiplierText="2×"
          multiplierClass={styles.multBadgeSuper}
          types={offensiveSuper}
        />
        <TypeListRow
          label="Not very effective"
          multiplierText="½×"
          multiplierClass={styles.multBadgeResist}
          types={offensiveResist}
        />
        {offensiveImmune.length > 0 && (
          <TypeListRow
            label="No effect"
            multiplierText="0×"
            multiplierClass={styles.multBadgeImmune}
            types={offensiveImmune}
          />
        )}
      </section>

      <section className="flex-col gap-xs">
        <h4 className={styles.sectionTitle}>Defense</h4>
        <TypeListRow
          label="Weaknesses"
          multiplierText="2×"
          multiplierClass={styles.multBadgeSuper}
          types={defensiveWeak}
        />
        <TypeListRow
          label="Resistances"
          multiplierText="½×"
          multiplierClass={styles.multBadgeResist}
          types={defensiveResist}
        />
        {defensiveImmune.length > 0 && (
          <TypeListRow
            label="Immunities"
            multiplierText="0×"
            multiplierClass={styles.multBadgeImmune}
            types={defensiveImmune}
          />
        )}
      </section>
    </article>
  );
}
