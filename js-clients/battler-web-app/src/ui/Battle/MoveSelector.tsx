import type { BattleState } from "battler-state";
import type { MonMoveSlotData, PlayerBattleData } from "battler-types";
import { useMemo } from "react";
import { useTypeChart } from "../../hooks/useTypeChart";
import { type ChoiceModifiers, UI_MODIFIER_KEYS, CHOICE_MODIFIER_CONFIGS, CHOICE_MODIFIER_KEYS } from "../../utils/choiceBuilder";
import { getAvailableMoves } from "../../utils/monHelpers";
import { getSingleOpposingTargetTypes } from "../../utils/typeEffectiveness";
import { MoveEffectivenessBadge } from "../Common/EffectivenessBadge";
import ActionButton from "./ActionButton";
import styles from "./ActionPanel.module.scss";

export interface MoveSelectorProps {
  activeReq: {
    moves: MonMoveSlotData[];
    z_moves?: (MonMoveSlotData | null)[];
    max_moves?: MonMoveSlotData[];
    can_mega_evolve?: boolean;
    can_terastallize?: boolean;
    can_z_move?: boolean;
    can_dynamax?: boolean;
    can_ultra_burst?: boolean;
  };
  isDynamaxed?: boolean;
  isLoading: boolean;
  modifiers: ChoiceModifiers;
  toggleModifier: (key: keyof ChoiceModifiers, value: boolean) => void;
  onSelectMove: (move: MonMoveSlotData, index: number) => void;
  onClearError: () => void;
  canShift?: boolean;
  onShift?: () => void;
  onBack?: () => void;
  battleState?: BattleState | null;
  playerData?: PlayerBattleData | null;
  battleType?: string | null;
  activeRequestsCount?: number;
}

export default function MoveSelector({
  activeReq,
  isDynamaxed = false,
  isLoading,
  modifiers,
  toggleModifier,
  onSelectMove,
  onClearError,
  canShift = false,
  onShift,
  onBack,
  battleState,
  playerData,
  battleType,
  activeRequestsCount = 1,
}: MoveSelectorProps) {
  const { typeChart } = useTypeChart();
  const isMaxMoveActive = modifiers.dyna || isDynamaxed;

  const singleOpponentTypes = useMemo(
    () =>
      getSingleOpposingTargetTypes(
        battleState,
        playerData,
        battleType,
        activeRequestsCount,
      ),
    [battleState, playerData, battleType, activeRequestsCount],
  );

  const hasModifiers = CHOICE_MODIFIER_KEYS.some(
    (key) => !!activeReq[CHOICE_MODIFIER_CONFIGS[key].requestFlag],
  );

  const availableMoves = getAvailableMoves(activeReq, {
    zmove: modifiers.zmove,
    dyna: isMaxMoveActive,
  });

  return (
    <div className="flex-col gap-s">
      <div className={styles.columnHeaderRow}>
        <h4 className={styles.summaryTitle}>Select move</h4>
        {onBack && (
          <button
            type="button"
            onClick={onBack}
            className="btn btn-sm btn-secondary"
            disabled={isLoading}
            title="Go back to previous choice"
          >
            ← Back
          </button>
        )}
      </div>

      {hasModifiers && (
        <div className="flex-row flex-wrap gap-s">
          {UI_MODIFIER_KEYS.map((key) => {
            const config = CHOICE_MODIFIER_CONFIGS[key];
            let flag = !!activeReq[config.requestFlag];
            if (key === "dyna" && isDynamaxed) flag = false;

            if (!flag) return null;

            return (
              <label
                key={key}
                className={`${styles.checkboxLabel} ${
                  styles[`modifierLabel_${key}`] || ""
                } ${modifiers[key] ? styles.checked : ""}`}
              >
                <input
                  type="checkbox"
                  checked={!!modifiers[key]}
                  onChange={(e) => {
                    onClearError();
                    toggleModifier(key, e.target.checked);
                  }}
                />
                {config.label}
              </label>
            );
          })}
        </div>
      )}

      <div className={styles.movesGrid}>
        {activeReq.moves.map((baseMove, index) => {
          const modifierMove = availableMoves[index];
          const moveToRender: MonMoveSlotData = modifierMove || baseMove;
          let badgeText: string | null = null;
          let isZMoveDisabled = false;

          if (modifiers.zmove) {
            if (modifierMove) {
              badgeText = "Z-Move";
            } else {
              isZMoveDisabled = true;
            }
          } else if (isMaxMoveActive && modifierMove) {
            badgeText = "Max Move";
          }

          const isMoveDisabled =
            isZMoveDisabled || baseMove.disabled || moveToRender.disabled;

          const subtitle =
            baseMove.max_pp > 0
              ? `${moveToRender.type} | PP: ${baseMove.pp}/${baseMove.max_pp}`
              : moveToRender.type;

          const effectivenessBadge = (
            <MoveEffectivenessBadge
              typeChart={typeChart}
              move={moveToRender}
              targetTypes={singleOpponentTypes}
            />
          );

          return (
            <ActionButton
              key={baseMove.id || index}
              title={moveToRender.name}
              subtitle={subtitle}
              onClick={() => onSelectMove(moveToRender, index)}
              disabled={isMoveDisabled || isLoading}
              typeColor={`var(--color-type-${moveToRender.type.toLowerCase()})`}
              badgeText={badgeText}
              effectivenessBadge={effectivenessBadge}
              infoResourceType="move"
              infoResourceName={moveToRender.name}
            />
          );
        })}
      </div>
      {canShift && onShift && (
        <ActionButton
          title="Shift"
          subtitle="Shift to center position"
          onClick={onShift}
          disabled={isLoading}
          htmlTitle="Shift position to the center slot"
        />
      )}
    </div>
  );
}

