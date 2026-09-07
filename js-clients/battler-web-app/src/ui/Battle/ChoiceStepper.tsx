import type { BattleState } from "battler-state";
import type { PlayerBattleData, Request } from "battler-types";
import { useState } from "react";
import type { ParsedChoiceError } from "../../utils/choiceParser";
import { formatTurnChoice } from "../../utils/choiceFormatter";
import {
  getMonDisplayName,
  getMonForSlot,
  getRequestSlotCount,
  getSelectReason,
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

  const inner = (
    <>
      <span className={badgeClass}>{badgeContent}</span>
      <div className={styles.chipContent}>
        <span className={styles.chipMonName}>{monName}</span>
        <span className={styles.chipSummary}>{summaryContent}</span>
      </div>
    </>
  );

  if (onClick) {
    return (
      <button
        type="button"
        onClick={onClick}
        className={containerClass}
        title={title}
        disabled={disabled}
      >
        {inner}
      </button>
    );
  }

  return (
    <div className={containerClass} title={title}>
      {inner}
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

  const slotMonNames = Array.from({ length: slotCount }, (_, idx) => {
    const mon = getMonForSlot(playerData, request, idx);
    const activePos = getActiveSlotPosition(request, idx);
    return getSlotLabel(activePos + 1, getMonDisplayName(mon));
  });

  const titlePrefix =
    request.type === "turn"
      ? "Turn progress"
      : request.type === "select"
        ? "Select progress"
        : "Switch progress";

  return (
    <div className={styles.choiceStepper}>
      <div
        className={styles.stepperHeader}
        onClick={() => setIsCollapsed(!isCollapsed)}
        title={isCollapsed ? "Expand choices" : "Collapse choices"}
      >
        <span className={styles.stepperTitle}>
          {isCollapsed ? "▶" : "▼"} {titlePrefix} ({choices.length}/{slotMonNames.length} completed)
        </span>
        <span className={styles.stepperToggleText}>
          {isCollapsed ? "Show details" : "Hide"}
        </span>
      </div>

      {!isCollapsed && (
        <div className="flex-col gap-xs">
          {slotMonNames.map((slotMonName, idx) => {
            const isCompleted = idx < currentSlotIndex;
            const isActive = idx === currentSlotIndex;
            const isErrored = parsedChoiceError.failedSlotIndex === idx;

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
              const selectingText =
                request.type === "turn"
                  ? "Selecting move..."
                  : request.type === "select"
                    ? getSelectReason(request, idx) === "Revive"
                      ? "Reviving..."
                      : "Selecting..."
                    : "Selecting switch...";
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
