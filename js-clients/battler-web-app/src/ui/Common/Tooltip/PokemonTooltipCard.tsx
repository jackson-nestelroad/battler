import { type CSSProperties, useState } from "react";
import type { MonTooltipViewModel } from "../../../utils/monTooltipModel";
import { formatBallName, formatStatusBadge } from "../../../utils/monHelpers";
import ExpBar from "../ExpBar";
import HpBar from "../HpBar";
import styles from "./PokemonTooltipCard.module.scss";

interface PokemonTooltipCardProps {
  data: MonTooltipViewModel;
}

interface TooltipHeaderProps {
  species: string;
  name?: string;
  level?: number | null;
  gender?: string | null;
  shiny?: boolean;
  ownerBadge?: string | null;
  isTransformed?: boolean;
  isDynamaxed?: boolean;
  types?: string[];
  teraType?: string | null;
  isTerastallized?: boolean;
}

function TooltipHeader({
  species,
  name,
  level,
  gender,
  shiny,
  ownerBadge,
  isTransformed,
  isDynamaxed,
  types,
  teraType,
  isTerastallized,
}: TooltipHeaderProps) {
  const genderLower = gender?.toLowerCase();
  const isMale = genderLower === "m" || genderLower === "male";
  const isFemale = genderLower === "f" || genderLower === "female";
  const displayName = name || species;

  return (
    <header className={styles.header}>
      <div className={styles.headerTop}>
        <div className={styles.identity}>
          <span className={styles.monName}>{displayName}</span>
          {level !== null && level !== undefined && (
            <span className={styles.levelBadge}>L{level}</span>
          )}
          {isMale && <span className={styles.genderMale}>♂</span>}
          {isFemale && <span className={styles.genderFemale}>♀</span>}
          {shiny && (
            <span className={styles.shinyStar} title="Shiny">
              ✨
            </span>
          )}
        </div>
        {ownerBadge && <span className={styles.ownerBadge}>{ownerBadge}</span>}
      </div>

      <span className={styles.speciesSubtitle}>{species}</span>

      {/* Types (on their own line) */}
      {types && types.length > 0 && (
        <div className="flex-row align-center gap-xs flex-wrap">
          {types.map((type) => (
            <span
              key={type}
              className={styles.typeBadge}
              style={{
                backgroundColor: `var(--color-type-${type.toLowerCase()}, var(--border-color))`,
              }}
            >
              {type}
            </span>
          ))}
        </div>
      )}

      {/* Active Battle Modifiers (Tera, Dynamax, Transformed) */}
      {(teraType || isDynamaxed || isTransformed) && (
        <div className="flex-row align-center gap-xs flex-wrap">
          {teraType && (
            <span className={`${styles.specialBadge} ${styles.teraBadge}`}>
              {isTerastallized ? `Terastallized: ${teraType}` : `Tera Type: ${teraType}`}
            </span>
          )}
          {isDynamaxed && (
            <span className={`${styles.specialBadge} ${styles.dynamaxBadge}`}>
              Dynamax
            </span>
          )}
          {isTransformed && (
            <span className={`${styles.specialBadge} ${styles.transformedBadge}`}>
              Transformed
            </span>
          )}
        </div>
      )}
    </header>
  );
}

export default function PokemonTooltipCard({ data }: PokemonTooltipCardProps) {
  const [activeTab, setActiveTab] = useState<"battle" | "summary">("battle");

  // Switch between live battle view and base summary view without battler-state
  const current = activeTab === "summary" && data.baseSummary ? data.baseSummary : data;
  const hasSummaryTab = Boolean(data.baseSummary);

  const hp = current.hp ?? 0;
  const maxHp = current.maxHp ?? 100;
  const hpPct = current.hpPercentage ?? (maxHp > 0 ? Math.round((hp / maxHp) * 100) : 0);

  const statusBadge = formatStatusBadge(current.status);
  const isFainted = current.isFainted || hp === 0 || statusBadge?.code === "fnt";

  return (
    <div className={styles.card}>
      {/* Tab bar switcher for your Mons when base summary is available */}
      {hasSummaryTab && (
        <nav className={styles.tabBar} aria-label="Mon details views">
          <button
            type="button"
            className={`${styles.tabBtn} ${activeTab === "battle" ? styles.tabBtnActive : ""}`}
            onClick={() => setActiveTab("battle")}
          >
            Battle
          </button>
          <button
            type="button"
            className={`${styles.tabBtn} ${activeTab === "summary" ? styles.tabBtnActive : ""}`}
            onClick={() => setActiveTab("summary")}
          >
            Summary
          </button>
        </nav>
      )}

      {/* Header */}
      <TooltipHeader
        species={current.species}
        name={current.name}
        level={current.level}
        gender={current.gender}
        shiny={current.shiny}
        ownerBadge={current.ownerLabel}
        isTransformed={current.isTransformed}
        isDynamaxed={current.isDynamaxed}
        types={current.types}
        teraType={current.teraType}
        isTerastallized={current.isTerastallized}
      />

      {/* Health & Status bar */}
      <section className={styles.healthSection}>
        <div className={styles.healthMeta}>
          <div>
            {isFainted ? (
              <span className="status-badge fnt">FNT</span>
            ) : statusBadge ? (
              <span className={`status-badge ${statusBadge.code}`}>{statusBadge.label}</span>
            ) : (
              <span className="status-badge ok">OK</span>
            )}
          </div>
          <span className={styles.hpText}>
            {current.maxHp !== null && current.maxHp !== undefined
              ? `${Math.max(0, hp)}/${maxHp} (${hpPct}%)`
              : `${hpPct}%`}
          </span>
        </div>
        <HpBar hp={Math.max(0, hp)} maxHp={maxHp} />
        {current.experience !== undefined && current.experience !== null && (
          <div className={styles.expSection}>
            <div className={styles.expMeta}>
              <span className={styles.expLabel}>EXP</span>
              <span className={styles.expText}>
                {(current.level !== null && current.level !== undefined && current.level >= 100) ||
                current.nextLevelExperience === null ? (
                  "MAX"
                ) : (
                  <>
                    {current.experience.toLocaleString()}
                    {current.expToNextLevel !== undefined &&
                      current.expToNextLevel !== null && (
                        <span className={styles.expToNext}>
                          {" "}(Next: {current.expToNextLevel.toLocaleString()})
                        </span>
                      )}
                  </>
                )}
              </span>
            </div>
            <ExpBar progressPercent={current.expProgressPercent ?? 0} />
          </div>
        )}
      </section>

      {/* Modifiers (Stat stages and conditions) */}
      {(current.boosts.length > 0 || current.conditions.length > 0) && (
        <section className={styles.modifiersSection}>
          {current.boosts.map((boost) => (
            <span
              key={boost.stat}
              className={`${styles.modifierBadge} ${
                boost.stage > 0 ? styles.boostBadgePositive : styles.boostBadgeNegative
              }`}
            >
              {boost.label}
            </span>
          ))}
          {current.conditions.map((condition) => (
            <span key={condition} className={`${styles.modifierBadge} ${styles.conditionBadge}`}>
              {condition}
            </span>
          ))}
        </section>
      )}

      {/* Traits: Ability, Item, Weight, Nature, Hidden Power, Friendship */}
      <section className={styles.traitsGrid}>
        <div className={styles.traitRow}>
          <span className={styles.traitLabel}>Ability:</span>
          {current.ability && current.ability.trim() !== "" ? (
            <span className={styles.traitValue}>{current.ability}</span>
          ) : (
            <span className={styles.traitUnknown}>???</span>
          )}
        </div>

        <div className={styles.traitRow}>
          <span className={styles.traitLabel}>Item:</span>
          {current.item && current.item.trim() !== "" ? (
            <span className={styles.traitValue}>{current.item}</span>
          ) : (
            <span className={styles.traitUnknown}>???</span>
          )}
        </div>

        {current.ball && (
          <div className={styles.traitRow}>
            <span className={styles.traitLabel}>Ball:</span>
            <span className={styles.traitValue}>{formatBallName(current.ball)}</span>
          </div>
        )}

        {current.weightKg !== undefined && current.weightKg !== null && (
          <div className={styles.traitRow}>
            <span className={styles.traitLabel}>Weight:</span>
            <span className={styles.traitValue}>{current.weightKg} kg</span>
          </div>
        )}

        {current.nature && (
          <div className={styles.traitRow}>
            <span className={styles.traitLabel}>Nature:</span>
            <span className={styles.traitValue}>
              {current.nature}
              {current.natureModifiers?.plus && current.natureModifiers?.minus && (
                <>
                  <span className={styles.natureModifierPlus}>
                    +{current.natureModifiers.plus}
                  </span>
                  <span className={styles.natureModifierMinus}>
                    -{current.natureModifiers.minus}
                  </span>
                </>
              )}
            </span>
          </div>
        )}

        {current.hiddenPowerType && (
          <div className={styles.traitRow}>
            <span className={styles.traitLabel}>Hidden Power:</span>
            <span className={styles.traitValue}>{current.hiddenPowerType}</span>
          </div>
        )}

        {current.friendship !== undefined && current.friendship !== null && (
          <div className={styles.traitRow}>
            <span className={styles.traitLabel}>Friendship:</span>
            <span className={styles.traitValue}>{current.friendship}</span>
          </div>
        )}

        {current.moves.length === 0 && (
          <div className={styles.traitRow}>
            <span className={styles.traitLabel}>Moves:</span>
            <span className={styles.traitUnknown}>???</span>
          </div>
        )}
      </section>

      {/* Moveset Grid (only renders when moves are known) */}
      {current.moves.length > 0 && (
        <section className={styles.movesSection}>
          <span className={styles.sectionTitle}>
            Moves
          </span>
          <div className={styles.movesGrid}>
            {current.moves.map((move, idx) => {
              const typeColor = move.type
                ? `var(--color-type-${move.type.toLowerCase()})`
                : "var(--border-color)";

              const metaParts: string[] = [];
              if (move.type) {
                metaParts.push(move.type);
              }
              if (move.pp !== undefined) {
                metaParts.push(
                  move.maxPp !== undefined
                    ? `PP: ${move.pp}/${move.maxPp}`
                    : `PP: ${move.pp}`,
                );
              }
              const metaText = metaParts.join(" | ");

              return (
                <div
                  key={idx}
                  className={`${styles.moveSlot} type-border ${
                    move.disabled ? styles.moveDisabled : ""
                  }`}
                  style={{ "--type-color": typeColor } as CSSProperties}
                >
                  <div className={styles.moveTop}>
                    <span className={styles.moveName} title={move.name}>
                      {move.name}
                    </span>
                  </div>
                  {metaText && (
                    <span className={styles.moveMeta}>
                      {metaText}
                    </span>
                  )}
                </div>
              );
            })}
          </div>
        </section>
      )}

      {/* Stats Table */}
      {current.stats && current.stats.length > 0 && (
        <section className={styles.statsSection}>
          <span className={styles.sectionTitle}>
            Stats
          </span>
          <table className={styles.statsTable}>
            <thead>
              <tr>
                <th>Stat</th>
                <th>Value</th>
                {activeTab !== "summary" && <th>Boost</th>}
                <th>EV</th>
                <th>IV</th>
              </tr>
            </thead>
            <tbody>
              {current.stats.map((statRow) => {
                const statClass = statRow.isPlus
                  ? styles.statPlus
                  : statRow.isMinus
                    ? styles.statMinus
                    : "";

                let boostLabel = "-";
                let boostClass = styles.statBoostZero;
                if (statRow.boost && statRow.boost !== 0) {
                  boostLabel = statRow.boost > 0 ? `+${statRow.boost}` : `${statRow.boost}`;
                  boostClass = statRow.boost > 0 ? styles.statBoostPos : styles.statBoostNeg;
                }

                return (
                  <tr key={statRow.stat}>
                    <td className={statClass}>{statRow.label}</td>
                    <td className={styles.statVal}>{statRow.value ?? "-"}</td>
                    {activeTab !== "summary" && <td className={boostClass}>{boostLabel}</td>}
                    <td>{statRow.ev ?? 0}</td>
                    <td>{statRow.iv ?? 31}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </section>
      )}
    </div>
  );
}
