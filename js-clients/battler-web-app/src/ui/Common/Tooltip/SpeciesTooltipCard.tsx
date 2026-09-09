import type { SpeciesData } from "battler-types";
import { Fragment } from "react";
import TypeBadge from "../TypeBadge";
import DataTooltipTrigger from "./DataTooltipTrigger";
import styles from "./SpeciesTooltipCard.module.scss";

interface SpeciesTooltipCardProps {
  data: SpeciesData;
}

interface GenderRatioDisplay {
  type: "genderless" | "male-only" | "female-only" | "split";
  malePercent?: number;
  femalePercent?: number;
}

function parseGenderRatio(ratio: number): GenderRatioDisplay {
  if (ratio === 255) {
    return { type: "genderless" };
  }
  if (ratio === 0) {
    return { type: "male-only", malePercent: 100, femalePercent: 0 };
  }
  if (ratio === 254) {
    return { type: "female-only", malePercent: 0, femalePercent: 100 };
  }
  let femalePercent: number;
  if (ratio === 31) femalePercent = 12.5;
  else if (ratio === 63) femalePercent = 25;
  else if (ratio === 127) femalePercent = 50;
  else if (ratio === 191) femalePercent = 75;
  else if (ratio === 223) femalePercent = 87.5;
  else {
    femalePercent = Math.round((ratio / 252) * 1000) / 10;
  }
  const malePercent = Math.round((100 - femalePercent) * 10) / 10;
  return { type: "split", malePercent, femalePercent };
}

function formatSpeciesClass(rawClass?: string | null): string {
  if (!rawClass) return "Mon";
  const cleaned = rawClass.replace(/\s*Pok[eé]mon/gi, "").trim();
  if (!cleaned) return "Mon";
  if (cleaned.toLowerCase().endsWith("mon")) return cleaned;
  return `${cleaned} Mon`;
}

export default function SpeciesTooltipCard({ data }: SpeciesTooltipCardProps) {
  const flags = (data.flags || []).slice().sort();

  const monClass = formatSpeciesClass(data.class);

  const genderRatio = parseGenderRatio(data.gender_ratio);
  const eggGroups = (data.egg_groups || []).join(", ");

  const hpDisplay =
    data.max_hp != null
      ? `${data.base_stats.hp} (max ${data.max_hp})`
      : data.base_stats.hp;

  const bst =
    (data.base_stats.hp || 0) +
    (data.base_stats.atk || 0) +
    (data.base_stats.def || 0) +
    (data.base_stats.spa || 0) +
    (data.base_stats.spd || 0) +
    (data.base_stats.spe || 0);

  const weightKg = data.weight > 0 ? (data.weight / 10).toFixed(1) : null;
  const heightM = data.height > 0 ? (data.height / 10).toFixed(1) : null;

  return (
    <article className={styles.card}>
      <header className={styles.header}>
        <span className={styles.name}>{data.name}</span>
        <span className={styles.subtitle}>{monClass}</span>
        <div className="flex-row align-center gap-xs">
          <TypeBadge type={data.primary_type} size="sm" />
          {data.secondary_type && <TypeBadge type={data.secondary_type} size="sm" />}
        </div>
      </header>

      <section className="flex-col gap-xxs">
        <span className={styles.sectionTitle}>Base Stats</span>
        <table className={styles.statsTable}>
          <thead>
            <tr>
              <th>HP</th>
              <th>Atk</th>
              <th>Def</th>
              <th>SpA</th>
              <th>SpD</th>
              <th>Spe</th>
              <th>Total</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td>{hpDisplay}</td>
              <td>{data.base_stats.atk}</td>
              <td>{data.base_stats.def}</td>
              <td>{data.base_stats.spa}</td>
              <td>{data.base_stats.spd}</td>
              <td>{data.base_stats.spe}</td>
              <td className={styles.bstVal}>{bst}</td>
            </tr>
          </tbody>
        </table>
      </section>

      <section className={styles.traitsGrid}>
        {(data.abilities.length > 0 || data.hidden_ability) && (
          <div className={styles.traitRow}>
            <span className={styles.traitLabel}>Abilities:</span>
            <span className={styles.traitValue}>
              {data.abilities.map((ability, idx) => (
                <Fragment key={`${ability}-${idx}`}>
                  {idx > 0 && <span className={styles.separator}>/</span>}
                  <DataTooltipTrigger
                    resourceType="ability"
                    name={ability}
                    preferredPlacement="left"
                  >
                    <span>{ability}</span>
                  </DataTooltipTrigger>
                </Fragment>
              ))}
              {data.hidden_ability && (
                <Fragment key="ha">
                  {data.abilities.length > 0 && <span className={styles.separator}>/</span>}
                  <DataTooltipTrigger
                    resourceType="ability"
                    name={data.hidden_ability}
                    preferredPlacement="left"
                  >
                    <span>{data.hidden_ability}</span>
                  </DataTooltipTrigger>{" "}
                  <span className={styles.haTag} title="Hidden Ability">
                    (H)
                  </span>
                </Fragment>
              )}
            </span>
          </div>
        )}
        <div className={styles.traitRow}>
          <span className={styles.traitLabel}>Gender:</span>
          <div className={styles.genderContainer}>
            {genderRatio.type === "genderless" ? (
              <span className={styles.traitValue}>Genderless</span>
            ) : (
              <div className={styles.genderSplitWrapper}>
                <div
                  className={styles.genderBar}
                  role="meter"
                  aria-label="Gender ratio"
                  aria-valuenow={genderRatio.malePercent}
                >
                  {genderRatio.malePercent! > 0 && (
                    <div
                      className={styles.genderBarMale}
                      style={{ width: `${genderRatio.malePercent}%` }}
                    />
                  )}
                  {genderRatio.femalePercent! > 0 && (
                    <div
                      className={styles.genderBarFemale}
                      style={{ width: `${genderRatio.femalePercent}%` }}
                    />
                  )}
                </div>
                <div className={styles.genderRatioLabels}>
                  {genderRatio.malePercent! > 0 && (
                    <span className={styles.genderMaleLabel}>
                      {genderRatio.malePercent}% <span className={styles.genderSymbol}>♂</span>
                    </span>
                  )}
                  {genderRatio.femalePercent! > 0 && (
                    <span className={styles.genderFemaleLabel}>
                      {genderRatio.femalePercent}% <span className={styles.genderSymbol}>♀</span>
                    </span>
                  )}
                </div>
              </div>
            )}
          </div>
        </div>
        {eggGroups && (
          <div className={styles.traitRow}>
            <span className={styles.traitLabel}>Egg Groups:</span>
            <span className={styles.traitValue}>{eggGroups}</span>
          </div>
        )}
        {weightKg && (
          <div className={styles.traitRow}>
            <span className={styles.traitLabel}>Weight:</span>
            <span className={styles.traitValue}>{weightKg} kg</span>
          </div>
        )}
        {heightM && (
          <div className={styles.traitRow}>
            <span className={styles.traitLabel}>Height:</span>
            <span className={styles.traitValue}>{heightM} m</span>
          </div>
        )}
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
