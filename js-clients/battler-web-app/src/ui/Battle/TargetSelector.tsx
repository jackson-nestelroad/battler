import type { TargetOption } from "../../utils/targeting";
import styles from "./ActionPanel.module.scss";

interface TargetSelectorProps {
  selectedMoveTarget: string;
  dynamicTargets: TargetOption[];
  isLoading: boolean;
  onConfirmMove: (targetVal: number | null) => void;
}

const TARGET_REQUIRING_SELECT = [
  "Normal",
  "AdjacentFoe",
  "AdjacentAlly",
  "Any",
  "AdjacentAllyOrUser",
];

export default function TargetSelector({
  selectedMoveTarget,
  dynamicTargets,
  isLoading,
  onConfirmMove,
}: TargetSelectorProps) {
  const requiresSelect = TARGET_REQUIRING_SELECT.includes(selectedMoveTarget);

  return (
    <div className={styles.targetCard}>
      <h4>Select target</h4>

      <div className="flex-col gap-s">
        {!requiresSelect ? (
          <button
            onClick={() => onConfirmMove(null)}
            className="btn btn-primary w-full"
            disabled={isLoading}
          >
            Confirm
          </button>
        ) : dynamicTargets.length > 0 ? (
          <div className={styles.targetGrid}>
            {dynamicTargets.map((opt) => {
              const typeLabel =
                opt.type === "self" ? "Self" : opt.type === "foe" ? "Foe" : "Ally";
              const subText = opt.playerName ? `${typeLabel} • ${opt.playerName}` : typeLabel;

              return (
                <button
                  key={`${opt.type}-${opt.value}`}
                  onClick={() => onConfirmMove(opt.value)}
                  className={`${styles.targetBtn} btn ${
                    opt.type === "self" ? "btn-primary" : "btn-secondary"
                  }`}
                  disabled={isLoading}
                >
                  <span className={styles.targetMonName}>{opt.monName}</span>
                  <span className={`${styles.targetSubText} ${styles[opt.type]}`}>{subText}</span>
                </button>
              );
            })}
          </div>
        ) : (
          <div className={styles.targetGrid}>
            <button
              onClick={() => onConfirmMove(1)}
              className="btn btn-secondary"
              disabled={isLoading}
            >
              Opponent left
            </button>
            <button
              onClick={() => onConfirmMove(2)}
              className="btn btn-secondary"
              disabled={isLoading}
            >
              Opponent right
            </button>
            <button
              onClick={() => onConfirmMove(-1)}
              className="btn btn-secondary"
              disabled={isLoading}
            >
              Ally
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
