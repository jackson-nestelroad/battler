import type { BattleState } from "battler-state";
import { useMemo } from "react";
import {
  extractFieldConditions,
  extractSideConditions,
  resolveSideLabels,
} from "../../utils/conditionData";
import styles from "./BattleConditionPopover.module.scss";

export type ConditionTab = "field" | "player" | "foe";

export interface BattleConditionPopoverProps {
  battleState: BattleState | null;
  playerId?: string | null;
  activeTab: ConditionTab;
  onTabChange: (tab: ConditionTab) => void;
}

export default function BattleConditionPopover({
  battleState,
  playerId,
  activeTab,
  onTabChange,
}: BattleConditionPopoverProps) {
  const {
    playerSideIndex,
    foeSideIndex,
    playerSideLabel,
    foeSideLabel,
  } = useMemo(
    () => resolveSideLabels(battleState, playerId),
    [battleState, playerId],
  );

  const fieldData = useMemo(
    () => extractFieldConditions(battleState),
    [battleState],
  );

  const playerData = useMemo(
    () => extractSideConditions(battleState, playerSideIndex, playerSideLabel),
    [battleState, playerSideIndex, playerSideLabel],
  );

  const foeData = useMemo(
    () => extractSideConditions(battleState, foeSideIndex, foeSideLabel),
    [battleState, foeSideIndex, foeSideLabel],
  );

  const currentSideData = activeTab === "player" ? playerData : foeData;
  const currentSideLabel = activeTab === "player" ? playerSideLabel : foeSideLabel;

  return (
    <div className={styles.card} role="dialog" aria-label="Battle Conditions">
      {/* Top Tab Bar */}
      <div className={styles.tabBar}>
        <button
          type="button"
          className={`${styles.tabBtn} ${activeTab === "field" ? styles.tabBtnActive : ""}`}
          onClick={() => onTabChange("field")}
        >
          <span>Field</span>
          <span
            className={`${styles.tabCountBadge} ${fieldData.allCount > 0 ? styles.hasItems : ""}`}
          >
            {fieldData.allCount}
          </span>
        </button>
        <button
          type="button"
          className={`${styles.tabBtn} ${activeTab === "player" ? styles.tabBtnActive : ""}`}
          onClick={() => onTabChange("player")}
        >
          <span>{playerSideLabel}</span>
          <span
            className={`${styles.tabCountBadge} ${playerData.allCount > 0 ? styles.hasItems : ""}`}
          >
            {playerData.allCount}
          </span>
        </button>
        <button
          type="button"
          className={`${styles.tabBtn} ${activeTab === "foe" ? styles.tabBtnActive : ""}`}
          onClick={() => onTabChange("foe")}
        >
          <span>{foeSideLabel}</span>
          <span
            className={`${styles.tabCountBadge} ${foeData.allCount > 0 ? styles.hasItems : ""}`}
          >
            {foeData.allCount}
          </span>
        </button>
      </div>

      {/* Header Bar */}
      <div className={styles.headerTitle}>
        {activeTab === "field" ? "Field Conditions" : `${currentSideLabel} Conditions`}
      </div>

      {/* Body Content */}
      <div className={styles.body}>
        {activeTab === "field" ? (
          <div className={styles.traitsGrid}>
            <div className={styles.traitRow}>
              <span className={styles.traitLabel}>Weather:</span>
              {fieldData.weather ? (
                <span className={styles.traitValue}>{fieldData.weather}</span>
              ) : (
                <span className={styles.traitNone}>Clear</span>
              )}
            </div>
            <div className={styles.traitRow}>
              <span className={styles.traitLabel}>Terrain:</span>
              {fieldData.terrain ? (
                <span className={styles.traitValue}>{fieldData.terrain}</span>
              ) : (
                <span className={styles.traitNone}>None</span>
              )}
            </div>
            {fieldData.otherConditions.length > 0 && (
              <div className={styles.traitRow}>
                <span className={styles.traitLabel}>Effects:</span>
                <div className={styles.badgeList}>
                  {fieldData.otherConditions.map((c) => (
                    <span key={c.id} className={styles.conditionBadge}>
                      {c.displayText}
                    </span>
                  ))}
                </div>
              </div>
            )}
          </div>
        ) : (
          <div className={styles.traitsGrid}>
            <div className={styles.traitRow}>
              <span className={styles.traitLabel}>Side:</span>
              {currentSideData.conditions.length > 0 ? (
                <div className={styles.badgeList}>
                  {currentSideData.conditions.map((c) => (
                    <span key={c.id} className={styles.conditionBadge}>
                      {c.displayText}
                    </span>
                  ))}
                </div>
              ) : (
                <span className={styles.traitNone}>None</span>
              )}
            </div>

            <div className={styles.traitRow}>
              <span className={styles.traitLabel}>Slots:</span>
              {currentSideData.slotConditions.length > 0 ? (
                <div className={styles.badgeList}>
                  {currentSideData.slotConditions.map((s) => (
                    <span key={`${s.slotIndex}-${s.id}`} className={styles.slotBadge}>
                      <span className={styles.slotTag}>{s.slotLabel}:</span>
                      <span className={styles.slotName}>{s.name}</span>
                    </span>
                  ))}
                </div>
              ) : (
                <span className={styles.traitNone}>None</span>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
