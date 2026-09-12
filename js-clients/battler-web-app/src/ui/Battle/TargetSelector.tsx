import type { BattleState } from "battler-state";
import type { MonMoveSlotData, PlayerBattleData } from "battler-types";
import { useTypeChart } from "../../hooks/useTypeChart";
import { getMoveTargetInfo, parseTargetValue, type TargetOption } from "../../utils/targeting";
import { resolveTargetTypes } from "../../utils/typeEffectiveness";
import { MoveEffectivenessBadge } from "../Common/EffectivenessBadge";
import styles from "./ActionPanel.module.scss";

interface TargetSelectorProps {
  selectedMoveTarget: string;
  selectedMove?: MonMoveSlotData | null;
  dynamicTargets: TargetOption[];
  isLoading: boolean;
  battleState?: BattleState | null;
  playerData?: PlayerBattleData | null;
  currentSlotIndex?: number;
  onConfirmMove: (targetVal: number | null) => void;
  onBack?: () => void;
}

export default function TargetSelector({
  selectedMoveTarget,
  selectedMove,
  dynamicTargets,
  isLoading,
  battleState,
  playerData,
  currentSlotIndex,
  onConfirmMove,
  onBack,
}: TargetSelectorProps) {
  const requiresSelect = getMoveTargetInfo(selectedMoveTarget).isChoosable;
  const { typeChart } = useTypeChart();

  return (
    <div className="flex-col gap-s">
      <div className={styles.columnHeaderRow}>
        <h4 className={styles.summaryTitle}>Select target</h4>
        {onBack && (
          <button
            type="button"
            onClick={onBack}
            className="btn btn-sm btn-secondary"
            disabled={isLoading}
            title="Go back to move selection"
          >
            ← Back
          </button>
        )}
      </div>

      <div className="flex-col gap-s">
        {!requiresSelect ? (
          <button
            type="button"
            onClick={() => onConfirmMove(null)}
            className="btn btn-primary w-full"
            disabled={isLoading}
          >
            Confirm
          </button>
        ) : (
          <div className={styles.targetGrid}>
            {dynamicTargets.map((opt) => {
              const subText = opt.subText;
              let multBadge = null;

              if (selectedMove) {
                const playerSide = playerData?.side ?? 0;
                const { sideIdx, pos } = parseTargetValue(
                  opt.value,
                  currentSlotIndex ?? 0,
                  playerSide,
                );
                const targetTypes = resolveTargetTypes(battleState, playerData, sideIdx, pos);
                multBadge = (
                  <MoveEffectivenessBadge
                    typeChart={typeChart}
                    move={selectedMove}
                    targetTypes={targetTypes}
                  />
                );
              }

              return (
                <button
                  type="button"
                  key={`${opt.type}-${opt.value}`}
                  onClick={() => onConfirmMove(opt.value)}
                  className={styles.targetBtn}
                  disabled={isLoading}
                >
                  <div className={styles.targetHeaderRow}>
                    <span className={styles.targetMonName}>{opt.monName}</span>
                    {multBadge}
                  </div>
                  <span className={`${styles.targetSubText} ${styles[opt.type]}`}>
                    {subText}
                  </span>
                </button>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
