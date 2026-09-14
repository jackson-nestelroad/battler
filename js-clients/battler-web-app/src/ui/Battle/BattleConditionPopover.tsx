import type { BattleState } from "battler-state";
import { useMemo } from "react";
import {
  type BattleConditionsViewModel,
  type FormattedCondition,
  extractAllBattleConditions,
} from "../../utils/conditionData";
import DataTooltipTrigger from "../Common/Tooltip/DataTooltipTrigger";
import cardStyles from "../Common/Tooltip/DataTooltipCard.module.scss";

export type ConditionTab = "field" | "player" | "foe";

export interface BattleConditionPopoverProps {
  battleState?: BattleState | null;
  playerId?: string | null;
  data?: BattleConditionsViewModel;
  activeTab: ConditionTab;
  onTabChange: (tab: ConditionTab) => void;
}

export default function BattleConditionPopover({
  battleState,
  playerId,
  data,
  activeTab,
  onTabChange,
}: BattleConditionPopoverProps) {
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
    playerSideSubtitle,
    foeSideSubtitle,
  } = conditions;

  const currentSideData = activeTab === "foe" ? foeData : playerData;

  const tabs: {
    id: ConditionTab;
    label: string;
    title: string;
    subtitle?: string;
    count: number;
  }[] = [
    {
      id: "field",
      label: "Field",
      title: "Field Conditions",
      count: fieldData.allCount,
    },
    {
      id: "player",
      label: playerSideLabel,
      title: `${playerSideLabel} Conditions`,
      subtitle: playerSideSubtitle,
      count: playerData.allCount,
    },
    {
      id: "foe",
      label: foeSideLabel,
      title: `${foeSideLabel} Conditions`,
      subtitle: foeSideSubtitle,
      count: foeData.allCount,
    },
  ];

  const activeTabInfo = tabs.find((t) => t.id === activeTab) ?? tabs[0];

  const renderConditionBadges = (conditionList: FormattedCondition[]) => (
    <div className="flex-row flex-wrap align-center gap-xs flex-1 min-w-0">
      {conditionList.map((c) => (
        <DataTooltipTrigger
          key={c.id}
          resourceType="condition"
          name={c.name}
          showUnderline={false}
        >
          <span className={cardStyles.conditionBadge}>
            {c.name}
          </span>
        </DataTooltipTrigger>
      ))}
    </div>
  );

  return (
    <div
      className={`${cardStyles.card} ${cardStyles.cardFixed}`}
      role="dialog"
      aria-label="Battle Conditions"
    >
      {/* Top Tab Bar */}
      <div className={cardStyles.tabBar} role="tablist" aria-label="Condition views">
        {tabs.map((tab) => {
          const isActive = activeTab === tab.id;
          return (
            <button
              key={tab.id}
              id={`condition-tab-${tab.id}`}
              type="button"
              role="tab"
              aria-selected={isActive}
              aria-controls={`condition-panel-${tab.id}`}
              className={cardStyles.tabBtn}
              onClick={() => onTabChange(tab.id)}
              title={tab.subtitle ? `${tab.label} (${tab.subtitle})` : undefined}
            >
              <span>{tab.label}</span>
              <span
                className={`${cardStyles.tabCountBadge}${tab.count > 0 ? ` ${cardStyles.hasItems}` : ""}`}
              >
                {tab.count}
              </span>
            </button>
          );
        })}
      </div>

      {/* Header Bar */}
      <header className={cardStyles.headerSection}>
        <div className={cardStyles.headerTitle}>{activeTabInfo.title}</div>
        {activeTabInfo.subtitle && (
          <div className={cardStyles.subtitle}>{activeTabInfo.subtitle}</div>
        )}
      </header>

      {/* Body Content */}
      <div
        className={cardStyles.scrollBody}
        role="tabpanel"
        id={`condition-panel-${activeTab}`}
        aria-labelledby={`condition-tab-${activeTab}`}
      >
        {activeTab === "field" ? (
          <>
            <div className={cardStyles.traitRow}>
              <span className={cardStyles.traitLabel}>Weather:</span>
              {fieldData.weather ? (
                <DataTooltipTrigger resourceType="condition" name={fieldData.weather}>
                  <span className={cardStyles.traitValue}>{fieldData.weather}</span>
                </DataTooltipTrigger>
              ) : (
                <span className={cardStyles.traitEmpty}>Clear</span>
              )}
            </div>
            <div className={cardStyles.traitRow}>
              <span className={cardStyles.traitLabel}>Terrain:</span>
              {fieldData.terrain ? (
                <DataTooltipTrigger resourceType="condition" name={fieldData.terrain}>
                  <span className={cardStyles.traitValue}>{fieldData.terrain}</span>
                </DataTooltipTrigger>
              ) : (
                <span className={cardStyles.traitEmpty}>None</span>
              )}
            </div>
            {fieldData.otherConditions.length > 0 && (
              <div className={cardStyles.traitRow}>
                <span className={cardStyles.traitLabel}>Effects:</span>
                {renderConditionBadges(fieldData.otherConditions)}
              </div>
            )}
          </>
        ) : (
          <>
            <div className={cardStyles.traitRow}>
              <span className={cardStyles.traitLabel}>Side:</span>
              {currentSideData.conditions.length > 0 ? (
                renderConditionBadges(currentSideData.conditions)
              ) : (
                <span className={cardStyles.traitEmpty}>None</span>
              )}
            </div>

            <div className={cardStyles.traitRow}>
              <span className={cardStyles.traitLabel}>Slots:</span>
              {currentSideData.slotConditions.length > 0 ? (
                <div className="flex-row flex-wrap align-center gap-xs flex-1 min-w-0">
                  {currentSideData.slotConditions.map((s) => (
                    <DataTooltipTrigger
                      key={`${s.slotIndex}-${s.id}`}
                      resourceType="condition"
                      name={s.name}
                      showUnderline={false}
                    >
                      <span className={cardStyles.conditionBadge}>
                        <span className={cardStyles.slotTag}>{s.slotLabel}:</span>
                        <span>{s.name}</span>
                      </span>
                    </DataTooltipTrigger>
                  ))}
                </div>
              ) : (
                <span className={cardStyles.traitEmpty}>None</span>
              )}
            </div>
          </>
        )}
      </div>
    </div>
  );
}
