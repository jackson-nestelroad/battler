import { itemIconUrl, monRenderUrl } from "../../../utils/assets";
import { type CSSProperties, useState } from "react";
import type { MonTooltipViewModel } from "../../../utils/monTooltipModel";
import { computeHpPercentage, formatBallName } from "../../../utils/monHelpers";
import ExpBar from "../ExpBar";
import HpBar from "../HpBar";
import StatusBadge from "../StatusBadge";
import TypeBadge from "../TypeBadge";
import cardStyles from "./DataTooltipCard.module.scss";
import DataTooltipTrigger from "./DataTooltipTrigger";
import styles from "./MonTooltipCard.module.scss";

export interface MonTooltipCardProps {
  data: MonTooltipViewModel;
}

function TooltipHeader({ data }: { data: MonTooltipViewModel }) {
  const {
    species,
    name,
    level,
    gender,
    shiny,
    ball,
    ownerLabel,
    types,
    teraType,
    isTerastallized,
    ability,
    item,
    nature,
    natureModifiers,
  } = data;
  const genderLower = gender?.toLowerCase();
  const isMale = genderLower === "m" || genderLower === "male";
  const isFemale = genderLower === "f" || genderLower === "female";
  const displayName = name || species;

  const activeTeraType = isTerastallized && teraType ? teraType : null;
  const ballName = ball ? formatBallName(ball) : "";
  const ballIconSrc = ball ? itemIconUrl(ball) : "";

  return (
    <header className={cardStyles.header}>
      <div className="flex-row justify-between align-start gap-s">
        <div className="flex-col gap-xxs flex-1 min-w-0">
          <div className={styles.identity}>
            <span className={styles.monName}>{displayName}</span>
            {level != null && (
              <span className={styles.levelBadge}>L{level}</span>
            )}
            {isMale && <span className={styles.genderMale}>♂</span>}
            {isFemale && <span className={styles.genderFemale}>♀</span>}
            {shiny && (
              <span className={styles.shinyStar} title="Shiny">
                ✨
              </span>
            )}
            {ball && ballIconSrc && (
              <DataTooltipTrigger
                resourceType="item"
                name={ballName}
                title={ballName}
                ariaLabel={`Ball: ${ballName}`}
                showUnderline={false}
                className={styles.ballTrigger}
              >
                <img
                  src={ballIconSrc}
                  alt=""
                  aria-hidden="true"
                  className={styles.ballIcon}
                  draggable={false}
                />
              </DataTooltipTrigger>
            )}
          </div>

          {species && (
            <DataTooltipTrigger resourceType="species" name={species}>
              <span className={styles.speciesSubtitle}>{species}</span>
            </DataTooltipTrigger>
          )}

          {/* Types and Tera state */}
          <div className="flex-col gap-xxs">
            {activeTeraType ? (
              <>
                {/* Active Tera Type */}
                <div className="flex-row align-center gap-xs flex-wrap">
                  <TypeBadge type={activeTeraType} size="md" variant="tera" interactive />
                  <span className={`${styles.specialBadge} ${styles.teraBadge}`}>
                    Terastallized
                  </span>
                </div>

                {/* Base Types composite pill matching Tera pill */}
                {types && types.length > 0 && (
                  <div className={styles.baseTypesPill}>
                    <span className={styles.baseTypesLabel}>Base:</span>
                    <div className="flex-row align-center gap-xxs flex-wrap">
                      {types.map((type) => (
                        <TypeBadge key={type} type={type} size="sm" interactive />
                      ))}
                    </div>
                  </div>
                )}
              </>
            ) : (
              <>
                {/* Real types on their own line */}
                {types && types.length > 0 && (
                  <div className="flex-row align-center gap-xs flex-wrap">
                    {types.map((type) => (
                      <TypeBadge key={type} type={type} size="md" interactive />
                    ))}
                  </div>
                )}

                {/* Tera Type on its own line below real types, with tera purple pill + type badge */}
                {teraType && (
                  <div className={styles.teraPill}>
                    <span className={styles.teraPillLabel}>Tera Type:</span>
                    <TypeBadge type={teraType} size="sm" variant="tera" interactive />
                  </div>
                )}
              </>
            )}

            {ownerLabel && (
              <span className={styles.ownerText}>{ownerLabel}</span>
            )}

            <div className={`${cardStyles.traitsGrid} ${styles.headerTraits}`}>
              <div className={cardStyles.traitRow}>
                <span className={cardStyles.traitLabel}>Ability:</span>
                {renderAbilityContent(ability)}
              </div>

              <div className={cardStyles.traitRow}>
                <span className={cardStyles.traitLabel}>Item:</span>
                {renderItemContent(item)}
              </div>

              {nature && (
                <div className={cardStyles.traitRow}>
                  <span className={cardStyles.traitLabel}>Nature:</span>
                  <span className={cardStyles.traitValue}>
                    {nature}
                    {natureModifiers?.plus && natureModifiers?.minus && (
                      <>
                        <span className={styles.natureModifierPlus}>
                          +{natureModifiers.plus}
                        </span>
                        <span className={styles.natureModifierMinus}>
                          -{natureModifiers.minus}
                        </span>
                      </>
                    )}
                  </span>
                </div>
              )}
            </div>
          </div>
        </div>

        {species && (
          <img
            src={monRenderUrl(species)}
            alt=""
            aria-hidden="true"
            className={cardStyles.monRender}
            draggable={false}
          />
        )}
      </div>
    </header>
  );
}

function renderAbilityContent(abilityStr?: string | null) {
  if (!abilityStr || !abilityStr.trim() || abilityStr === "???") {
    return <span className={cardStyles.traitEmpty}>???</span>;
  }
  const trimmed = abilityStr.trim();
  if (trimmed === "None") {
    return <span className={cardStyles.traitValue}>None</span>;
  }
  return (
    <DataTooltipTrigger resourceType="ability" name={trimmed}>
      <span className={cardStyles.traitValue}>{trimmed}</span>
    </DataTooltipTrigger>
  );
}

function renderItemContent(itemStr?: string | null) {
  if (!itemStr || !itemStr.trim() || itemStr === "???") {
    return <span className={cardStyles.traitEmpty}>???</span>;
  }
  const trimmed = itemStr.trim();
  if (trimmed === "None") {
    return <span className={cardStyles.traitValue}>None</span>;
  }
  const wasMatch = trimmed.match(/^None \(was (.+)\)$/);
  if (wasMatch) {
    const previousItemName = wasMatch[1];
    return (
      <span className={cardStyles.traitValue}>
        None (was{" "}
        <DataTooltipTrigger resourceType="item" name={previousItemName}>
          {previousItemName}
        </DataTooltipTrigger>
        )
      </span>
    );
  }
  return (
    <DataTooltipTrigger resourceType="item" name={trimmed}>
      <span className={cardStyles.traitValue}>{trimmed}</span>
    </DataTooltipTrigger>
  );
}

export default function MonTooltipCard({ data }: MonTooltipCardProps) {
  const [activeTab, setActiveTab] = useState<"battle" | "summary">("battle");

  // Switch between live battle view and base summary view without battler-state
  const current = activeTab === "summary" && data.baseSummary ? data.baseSummary : data;
  const hasSummaryTab = Boolean(data.baseSummary);

  const isFainted = Boolean(
    current.isFainted ||
    current.status === "fnt" ||
    (current.hp != null && current.hp <= 0),
  );
  const maxHp = current.maxHp ?? 100;
  const hp = isFainted ? (current.hp ?? 0) : (current.hp ?? maxHp);
  const hpPct = current.hpPercentage ?? computeHpPercentage(hp, maxHp);

  const conditions = (current.conditions || []).filter(
    (c) => c.toLowerCase() !== "transformed",
  );
  if (
    current.isDynamaxed &&
    !conditions.some((c) => c.toLowerCase() === "dynamax")
  ) {
    conditions.push("Dynamax");
  }

  const hasModifiers =
    current.boosts.length > 0 ||
    conditions.length > 0 ||
    Boolean(current.isTransformed);

  return (
    <div className={`${cardStyles.card} ${cardStyles.cardFixed}`}>
      {/* Tab bar switcher for your Mons when base summary is available */}
      {hasSummaryTab && (
        <div className={cardStyles.tabBar} role="tablist" aria-label="Mon details views">
          <button
            key="battle"
            id="mon-tab-battle"
            type="button"
            role="tab"
            aria-selected={activeTab === "battle"}
            aria-controls="mon-panel-battle"
            className={cardStyles.tabBtn}
            onClick={() => setActiveTab("battle")}
          >
            Battle
          </button>
          <button
            key="summary"
            id="mon-tab-summary"
            type="button"
            role="tab"
            aria-selected={activeTab === "summary"}
            aria-controls="mon-panel-summary"
            className={cardStyles.tabBtn}
            onClick={() => setActiveTab("summary")}
          >
            Team
          </button>
        </div>
      )}

      {/* Tab Content Panel */}
      <div
        className="flex-col gap-s"
        {...(hasSummaryTab
          ? {
              role: "tabpanel",
              id: `mon-panel-${activeTab}`,
              "aria-labelledby": `mon-tab-${activeTab}`,
            }
          : {})}
      >
        {/* Header */}
        <TooltipHeader data={current} />

      {/* Health & Status bar */}
      <section className={styles.healthSection}>
        <div className={styles.healthMeta}>
          <StatusBadge status={current.status} isFainted={isFainted} interactive={true} />
          <span className={styles.hpText}>
            {current.maxHp != null
              ? `${Math.max(0, hp)}/${maxHp} (${hpPct}%)`
              : `${hpPct}%`}
          </span>
        </div>
        <HpBar hp={Math.max(0, hp)} maxHp={maxHp} />
        {current.experience != null && (
          <div className={styles.expSection}>
            <div className={styles.expMeta}>
              <span className={styles.expLabel}>EXP</span>
              <span className={styles.expText}>
                {(current.level ?? 0) >= 100 || current.nextLevelExperience === null ? (
                  "MAX"
                ) : (
                  <>
                    {current.experience.toLocaleString()}
                    {current.expToNextLevel != null && (
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

      {/* Modifiers (Stat stages, transformed, and conditions) */}
      {hasModifiers && (
        <section className="flex-row align-center gap-xs flex-wrap">
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
          {current.isTransformed && (
            <span
              className={`${styles.modifierBadge} ${styles.transformedBadge}`}
            >
              {current.originalSpecies
                ? `Transformed (${current.originalSpecies})`
                : "Transformed"}
            </span>
          )}
          {conditions.map((condition) => {
            const isDynamax = condition.toLowerCase() === "dynamax";
            return (
              <DataTooltipTrigger
                key={condition}
                resourceType="condition"
                name={condition}
                showUnderline={false}
              >
                <span
                  className={`${styles.modifierBadge} ${
                    isDynamax ? styles.dynamaxBadge : styles.conditionBadge
                  }`}
                >
                  {condition}
                </span>
              </DataTooltipTrigger>
            );
          })}
        </section>
      )}

      {/* Secondary traits: Weight, Friendship */}
      {(current.weightKg != null ||
        current.friendship != null) && (
        <section className={cardStyles.traitsGrid}>
          {current.weightKg != null && (
            <div className={cardStyles.traitRow}>
              <span className={cardStyles.traitLabel}>Weight:</span>
              <span className={cardStyles.traitValue}>{current.weightKg} kg</span>
            </div>
          )}

          {current.friendship != null && (
            <div className={cardStyles.traitRow}>
              <span className={cardStyles.traitLabel}>Friendship:</span>
              <span className={cardStyles.traitValue}>{current.friendship}</span>
            </div>
          )}
        </section>
      )}

      {current.moves.length === 0 && (
        <section className={cardStyles.traitsGrid}>
          <div className={cardStyles.traitRow}>
            <span className={cardStyles.traitLabel}>Moves:</span>
            <span className={cardStyles.traitEmpty}>???</span>
          </div>
        </section>
      )}

      {/* Moveset Grid (only renders when moves are known) */}
      {current.moves.length > 0 && (
        <section className={styles.movesSection}>
          <span className={cardStyles.sectionTitle}>
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
              if (move.pp != null) {
                metaParts.push(
                  move.maxPp != null
                    ? `PP: ${move.pp}/${move.maxPp}`
                    : `PP: ${move.pp}`,
                );
              }
              const metaText = metaParts.join(" | ");

              return (
                <DataTooltipTrigger
                  key={move.name ? `${move.name}-${idx}` : idx}
                  resourceType="move"
                  name={move.name}
                  as="div"
                  className={styles.moveTrigger}
                  showUnderline={false}
                >
                  <div
                    className={`${styles.moveSlot} type-border ${
                      move.disabled ? styles.moveDisabled : ""
                    }`}
                    style={{ "--type-color": typeColor } as CSSProperties}
                  >
                    <span className={styles.moveName} title={move.name}>
                      {move.name}
                    </span>
                    {metaText && (
                      <span className={styles.moveMeta}>
                        {metaText}
                      </span>
                    )}
                  </div>
                </DataTooltipTrigger>
              );
            })}
          </div>
        </section>
      )}

      {/* Stats Table */}
      {current.stats && current.stats.length > 0 && (
        <section className={styles.statsSection}>
          <span className={cardStyles.sectionTitle}>
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
                let boostClass = "";
                if (statRow.boost) {
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
    </div>
  );
}
