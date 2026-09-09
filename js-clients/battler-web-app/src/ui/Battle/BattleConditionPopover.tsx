import type { BattleState } from "battler-state";
import { forwardRef, useMemo } from "react";
import {
  type BattleConditionsViewModel,
  type FormattedCondition,
  extractAllBattleConditions,
} from "../../utils/conditionData";
import DataTooltipTrigger from "../Common/Tooltip/DataTooltipTrigger";
import styles from "./BattleConditionPopover.module.scss";

export type ConditionTab = "field" | "player" | "foe";

export interface BattleConditionPopoverProps {
  battleState?: BattleState | null;
  playerId?: string | null;
  data?: BattleConditionsViewModel;
  activeTab: ConditionTab;
  onTabChange: (tab: ConditionTab) => void;
}

const BattleConditionPopover = forwardRef<HTMLDivElement, BattleConditionPopoverProps>(
  function BattleConditionPopover(
    {
      battleState,
      playerId,
      data,
      activeTab,
      onTabChange,
    },
    ref,
  ) {
    const conditions = useMemo(
      () => data ?? extractAllBattleConditions(battleState, playerId),
      [data, battleState, playerId],
    );

    const {
      fieldData,
      playerData,
      foeData,
      playerSideLabel,
      foeSideLabel,
    } = conditions;

    const currentSideData = activeTab === "foe" ? foeData : playerData;

    const tabs: { id: ConditionTab; label: string; title: string; count: number }[] = [
      { id: "field", label: "Field", title: "Field Conditions", count: fieldData.allCount },
      { id: "player", label: playerSideLabel, title: `${playerSideLabel} Conditions`, count: playerData.allCount },
      { id: "foe", label: foeSideLabel, title: `${foeSideLabel} Conditions`, count: foeData.allCount },
    ];

    const activeTabInfo = tabs.find((t) => t.id === activeTab) ?? tabs[0];

    const renderConditionBadges = (conditionList: FormattedCondition[]) => (
      <div className={styles.badgeList}>
        {conditionList.map((c) => (
          <DataTooltipTrigger
            key={c.id}
            resourceType="condition"
            name={c.displayText}
            showUnderline={false}
          >
            <span className={styles.conditionBadge}>
              {c.displayText}
            </span>
          </DataTooltipTrigger>
        ))}
      </div>
    );

    return (
      <div className={styles.card} role="dialog" aria-label="Battle Conditions" ref={ref}>
        {/* Top Tab Bar */}
        <div className={styles.tabBar} role="tablist" aria-label="Condition views">
          {tabs.map((tab) => {
            const isActive = activeTab === tab.id;
            return (
              <button
                key={tab.id}
                type="button"
                role="tab"
                aria-selected={isActive}
                className={styles.tabBtn}
                onClick={() => onTabChange(tab.id)}
              >
                <span>{tab.label}</span>
                <span
                  className={`${styles.tabCountBadge} ${tab.count > 0 ? styles.hasItems : ""}`}
                >
                  {tab.count}
                </span>
              </button>
            );
          })}
        </div>

        {/* Header Bar */}
        <div className={styles.headerTitle}>{activeTabInfo.title}</div>

        {/* Body Content */}
        <div className={styles.body}>
          {activeTab === "field" ? (
            <>
              <div className={styles.traitRow}>
                <span className={styles.traitLabel}>Weather:</span>
                {fieldData.weather ? (
                  <DataTooltipTrigger resourceType="condition" name={fieldData.weather}>
                    <span className={styles.traitValue}>{fieldData.weather}</span>
                  </DataTooltipTrigger>
                ) : (
                  <span className={styles.traitNone}>Clear</span>
                )}
              </div>
              <div className={styles.traitRow}>
                <span className={styles.traitLabel}>Terrain:</span>
                {fieldData.terrain ? (
                  <DataTooltipTrigger resourceType="condition" name={fieldData.terrain}>
                    <span className={styles.traitValue}>{fieldData.terrain}</span>
                  </DataTooltipTrigger>
                ) : (
                  <span className={styles.traitNone}>None</span>
                )}
              </div>
              {fieldData.otherConditions.length > 0 && (
                <div className={styles.traitRow}>
                  <span className={styles.traitLabel}>Effects:</span>
                  {renderConditionBadges(fieldData.otherConditions)}
                </div>
              )}
            </>
          ) : (
            <>
              <div className={styles.traitRow}>
                <span className={styles.traitLabel}>Side:</span>
                {currentSideData.conditions.length > 0 ? (
                  renderConditionBadges(currentSideData.conditions)
                ) : (
                  <span className={styles.traitNone}>None</span>
                )}
              </div>

              <div className={styles.traitRow}>
                <span className={styles.traitLabel}>Slots:</span>
                {currentSideData.slotConditions.length > 0 ? (
                  <div className={styles.badgeList}>
                    {currentSideData.slotConditions.map((s) => (
                      <DataTooltipTrigger
                        key={`${s.slotIndex}-${s.id}`}
                        resourceType="condition"
                        name={s.displayText}
                        showUnderline={false}
                      >
                        <span className={styles.conditionBadge}>
                          <span className={styles.slotTag}>{s.slotLabel}:</span>
                          <span>{s.displayText}</span>
                        </span>
                      </DataTooltipTrigger>
                    ))}
                  </div>
                ) : (
                  <span className={styles.traitNone}>None</span>
                )}
              </div>
            </>
          )}
        </div>
      </div>
    );
});

export default BattleConditionPopover;
