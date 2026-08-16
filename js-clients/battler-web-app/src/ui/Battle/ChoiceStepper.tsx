import type { BattleState } from "battler-state";
import type { PlayerBattleData, Request } from "battler-types";
import { useState } from "react";
import type { ParsedChoiceError } from "../../utils/choiceErrorParser";
import { formatTurnChoice } from "../../utils/choiceFormatter";
import {
  getMonDisplayName,
  getMonForSlot,
  getRequestSlotCount,
  getSlotLabel,
} from "../../utils/monHelpers";
import styles from "./ChoiceStepper.module.scss";

interface ChoiceStepperProps {
  request: Request | null;
  playerData: PlayerBattleData | null;
  battleState?: BattleState | null;
  choices: string[];
  currentSlotIndex: number;
  parsedChoiceError: ParsedChoiceError;
  isLoading: boolean;
  onJumpToSlot: (slotIndex: number) => void;
}

export default function ChoiceStepper({
  request,
  playerData,
  battleState,
  choices,
  currentSlotIndex,
  parsedChoiceError,
  isLoading,
  onJumpToSlot,
}: ChoiceStepperProps) {
  const [isCollapsed, setIsCollapsed] = useState(true);

  if (!request) return null;

  const slotCount = getRequestSlotCount(request);
  if (slotCount <= 1) return null;

  const slotItems = Array.from({ length: slotCount }).map((_, idx) => {
    const mon = getMonForSlot(playerData, request, idx);
    const activePos = request.type === "switch" ? request.needs_switch?.[idx] ?? idx : idx;
    const slotMonName = getSlotLabel(activePos + 1, getMonDisplayName(mon));
    return { slotMonName };
  });

  const titlePrefix = request.type === "turn" ? "Turn progress" : "Switch progress";
  const selectingText = request.type === "turn" ? "Selecting move..." : "Selecting switch...";

  return (
    <div className={styles.choiceStepper}>
      <div
        className={styles.stepperHeader}
        onClick={() => setIsCollapsed(!isCollapsed)}
        title={isCollapsed ? "Expand choices" : "Collapse choices"}
      >
        <span className={styles.stepperTitle}>
          {isCollapsed ? "▶" : "▼"} {titlePrefix} ({choices.length}/{slotItems.length} completed)
        </span>
        <span className={styles.stepperToggleText}>
          {isCollapsed ? "Show details" : "Hide"}
        </span>
      </div>

      {!isCollapsed && (
        <div className="flex-col gap-xs">
          {slotItems.map((item, idx) => {
            const isCompleted = idx < currentSlotIndex;
            const isActive = idx === currentSlotIndex;
            const isErrored = parsedChoiceError.failedSlotIndex === idx;
            const slotMonName = item.slotMonName;

            if (isCompleted && choices[idx] && !isErrored) {
              const formatted = formatTurnChoice(
                choices[idx],
                idx,
                request,
                playerData,
                battleState,
              );

              return (
                <button
                  key={idx}
                  type="button"
                  onClick={() => onJumpToSlot(idx)}
                  className={`${styles.choiceChip} ${styles.completed}`}
                  title="Click to edit choice for this slot"
                  disabled={isLoading}
                >
                  <span className={`${styles.chipStepBadge} ${styles.completed}`}>✓</span>
                  <div className={styles.chipContent}>
                    <span className={styles.chipMonName}>{slotMonName}</span>
                    <span className={styles.chipSummary}>
                      <span>{formatted.actionName}</span>
                      {formatted.modifiers.map((mod) => (
                        <span
                          key={mod}
                          className={`${styles.modifierBadge} ${
                            styles[`mod_${mod.toLowerCase().replace("-", "")}`] || ""
                          }`}
                        >
                          {mod}
                        </span>
                      ))}
                      {formatted.targetName && <span> → {formatted.targetName}</span>}
                    </span>
                  </div>
                </button>
              );
            }

            if (isActive || isErrored) {
              return (
                <div
                  key={idx}
                  className={`${styles.choiceChip} ${isActive ? styles.active : ""} ${
                    isErrored ? styles.errored : ""
                  }`}
                >
                  <span
                    className={`${styles.chipStepBadge} ${
                      isErrored ? styles.errored : styles.active
                    }`}
                  >
                    {isErrored ? "!" : idx + 1}
                  </span>
                  <div className={styles.chipContent}>
                    <span className={styles.chipMonName}>{slotMonName}</span>
                    <span className={styles.chipSummary}>
                      {isErrored ? parsedChoiceError.errorMessage : selectingText}
                    </span>
                  </div>
                </div>
              );
            }

            return (
              <div key={idx} className={`${styles.choiceChip} ${styles.pending}`}>
                <span className={`${styles.chipStepBadge} ${styles.pending}`}>{idx + 1}</span>
                <div className={styles.chipContent}>
                  <span className={styles.chipMonName}>{slotMonName}</span>
                  <span className={styles.chipSummary}>Waiting...</span>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
