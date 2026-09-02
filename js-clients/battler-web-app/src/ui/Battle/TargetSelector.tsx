import { getMoveTargetInfo, type TargetOption } from "../../utils/targeting";
import styles from "./ActionPanel.module.scss";

interface TargetSelectorProps {
  selectedMoveTarget: string;
  dynamicTargets: TargetOption[];
  isLoading: boolean;
  onConfirmMove: (targetVal: number | null) => void;
  onBack?: () => void;
}

export default function TargetSelector({
  selectedMoveTarget,
  dynamicTargets,
  isLoading,
  onConfirmMove,
  onBack,
}: TargetSelectorProps) {
  const requiresSelect = getMoveTargetInfo(selectedMoveTarget).isChoosable;

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

              return (
                <button
                  type="button"
                  key={`${opt.type}-${opt.value}`}
                  onClick={() => onConfirmMove(opt.value)}
                  className={styles.targetBtn}
                  disabled={isLoading}
                >
                  <span className={styles.targetMonName}>{opt.monName}</span>
                  <span className={`${styles.targetSubText} ${styles[opt.type]}`}>{subText}</span>
                </button>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
