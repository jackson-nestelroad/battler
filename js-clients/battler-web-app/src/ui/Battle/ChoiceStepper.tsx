import type { BattleState } from "battler-state";
import type { PlayerBattleData, Request } from "battler-types";
import { useState } from "react";
import type { ParsedChoiceError } from "../../utils/choiceParser";
import { formatTurnChoice } from "../../utils/choiceFormatter";
import {
  getMonDisplayName,
  getMonForSlot,
  getRequestSlotCount,
  getSlotLabel,
  getActiveSlotPosition,
} from "../../utils/monHelpers";
import styles from "./ChoiceStepper.module.scss";

interface StepChipProps {
  status: "completed" | "active" | "errored" | "pending";
  badgeContent: React.ReactNode;
  monName: string;
  summaryContent: React.ReactNode;
  onClick?: () => void;
  disabled?: boolean;
  title?: string;
}

function StepChip({
  status,
  badgeContent,
  monName,
  summaryContent,
  onClick,
  disabled,
  title,
}: StepChipProps) {
  const containerClass = `${styles.choiceChip} ${styles[status]}`;
  const badgeClass = `${styles.chipStepBadge} ${styles[status]}`;

  if (onClick) {
    return (
      <button
        type="button"
        onClick={onClick}
        className={containerClass}
        title={title}
        disabled={disabled}
      >
        <span className={badgeClass}>{badgeContent}</span>
        <div className={styles.chipContent}>
          <span className={styles.chipMonName}>{monName}</span>
          <span className={styles.chipSummary}>{summaryContent}</span>
        </div>
      </button>
    );
  }

  return (
    <div className={containerClass} title={title}>
      <span className={badgeClass}>{badgeContent}</span>
      <div className={styles.chipContent}>
        <span className={styles.chipMonName}>{monName}</span>
        <span className={styles.chipSummary}>{summaryContent}</span>
      </div>
    </div>
  );
}

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
    const activePos = getActiveSlotPosition(request, idx);
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

              const summaryContent = (
                <>
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
                </>
              );

              return (
                <StepChip
                  key={idx}
                  status="completed"
                  badgeContent="✓"
                  monName={slotMonName}
                  summaryContent={summaryContent}
                  onClick={() => onJumpToSlot(idx)}
                  title="Click to edit choice for this slot"
                  disabled={isLoading}
                />
              );
            }

            if (isActive || isErrored) {
              const status = isErrored ? "errored" : "active";
              const badgeContent = isErrored ? "!" : idx + 1;
              const summaryContent = isErrored ? parsedChoiceError.errorMessage : selectingText;
              
              return (
                <StepChip
                  key={idx}
                  status={status}
                  badgeContent={badgeContent}
                  monName={slotMonName}
                  summaryContent={summaryContent}
                />
              );
            }

            return (
              <StepChip
                key={idx}
                status="pending"
                badgeContent={idx + 1}
                monName={slotMonName}
                summaryContent="Waiting..."
              />
            );
          })}
        </div>
      )}
    </div>
  );
}
