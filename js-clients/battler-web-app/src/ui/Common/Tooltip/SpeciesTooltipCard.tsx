import type { SpeciesData } from "battler-types";
import { Fragment } from "react";
import {
  formatDeciMetric,
  formatSpeciesClass,
  parseGenderRatio,
} from "../../../utils/dataTooltipFormatting";
import TypeBadge from "../TypeBadge";
import cardStyles from "./DataTooltipCard.module.scss";
import DataTooltipTrigger from "./DataTooltipTrigger";
import styles from "./SpeciesTooltipCard.module.scss";
import TooltipEffectButton from "./TooltipEffectButton";
import TooltipFlagsSection from "./TooltipFlagsSection";

export interface SpeciesTooltipCardProps {
  data: SpeciesData;
}

export default function SpeciesTooltipCard({ data }: SpeciesTooltipCardProps) {
  const monClass = formatSpeciesClass(data.class);

  const genderRatio = parseGenderRatio(data.gender_ratio ?? 255);
  const { malePercent = 0, femalePercent = 0 } = genderRatio;
  const eggGroups = (data.egg_groups || []).join(", ");

  const stats = data.base_stats ?? { hp: 0, atk: 0, def: 0, spa: 0, spd: 0, spe: 0 };
  const hpDisplay =
    data.max_hp != null ? `${stats.hp} (max ${data.max_hp})` : stats.hp;

  const bst = stats.hp + stats.atk + stats.def + stats.spa + stats.spd + stats.spe;

  const weightStr = formatDeciMetric(data.weight, "kg");
  const heightStr = formatDeciMetric(data.height, "m");

  const abilities = Array.from(new Set(data.abilities || [])).filter(
    (a) => a !== data.hidden_ability,
  );

  return (
    <article className={`${cardStyles.card} ${cardStyles.cardFixed}`}>
      <header className={cardStyles.header}>
        <div className="flex-row justify-between align-center">
          <span className={cardStyles.name}>{data.name}</span>
          <TooltipEffectButton type="species" name={data.name} />
        </div>
        <span className={cardStyles.subtitle}>{monClass}</span>
        <div className="flex-row align-center gap-xs">
          <TypeBadge type={data.primary_type} size="sm" />
          {data.secondary_type && <TypeBadge type={data.secondary_type} size="sm" />}
        </div>
      </header>

      <section className="flex-col gap-xxs">
        <span className={cardStyles.sectionTitle}>Base Stats</span>
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
              <td>{stats.atk}</td>
              <td>{stats.def}</td>
              <td>{stats.spa}</td>
              <td>{stats.spd}</td>
              <td>{stats.spe}</td>
              <td className={styles.bstVal}>{bst}</td>
            </tr>
          </tbody>
        </table>
      </section>

      <section className={cardStyles.traitsGrid}>
        {(abilities.length > 0 || data.hidden_ability) && (
          <div className={cardStyles.traitRow}>
            <span className={cardStyles.traitLabel}>Abilities:</span>
            <span className={cardStyles.traitValue}>
              {abilities.map((ability, idx) => (
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
                  {abilities.length > 0 && <span className={styles.separator}>/</span>}
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
        <div className={cardStyles.traitRow}>
          <span className={cardStyles.traitLabel}>Gender:</span>
          {genderRatio.type === "genderless" ? (
            <span className={cardStyles.traitValue}>Genderless</span>
          ) : (
            <div className={styles.genderWrapper}>
              <div
                className={styles.genderBar}
                role="meter"
                aria-label="Gender ratio"
                aria-valuenow={malePercent}
                aria-valuemin={0}
                aria-valuemax={100}
              >
                {malePercent > 0 && (
                  <div
                    className={styles.genderBarMale}
                    style={{ width: `${malePercent}%` }}
                  />
                )}
                {femalePercent > 0 && (
                  <div
                    className={styles.genderBarFemale}
                    style={{ width: `${femalePercent}%` }}
                  />
                )}
              </div>
              <div className={styles.genderRatioLabels}>
                {malePercent > 0 && (
                  <span className={styles.genderMaleLabel}>
                    {malePercent}% <span className={styles.genderSymbol}>♂</span>
                  </span>
                )}
                {femalePercent > 0 && (
                  <span className={styles.genderFemaleLabel}>
                    {femalePercent}% <span className={styles.genderSymbol}>♀</span>
                  </span>
                )}
              </div>
            </div>
          )}
        </div>
        {eggGroups && (
          <div className={cardStyles.traitRow}>
            <span className={cardStyles.traitLabel}>Egg Groups:</span>
            <span className={cardStyles.traitValue}>{eggGroups}</span>
          </div>
        )}
        {weightStr && (
          <div className={cardStyles.traitRow}>
            <span className={cardStyles.traitLabel}>Weight:</span>
            <span className={cardStyles.traitValue}>{weightStr}</span>
          </div>
        )}
        {heightStr && (
          <div className={cardStyles.traitRow}>
            <span className={cardStyles.traitLabel}>Height:</span>
            <span className={cardStyles.traitValue}>{heightStr}</span>
          </div>
        )}
      </section>

      <TooltipFlagsSection flags={data.flags} />
    </article>
  );
}
