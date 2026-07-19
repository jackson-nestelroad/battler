import styles from "./ActionPanel.module.scss";

interface TargetSelectorProps {
  selectedMoveTarget: string;
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
