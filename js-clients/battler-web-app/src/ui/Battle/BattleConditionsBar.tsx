import type { BattleState } from "battler-state";
import { type MouseEvent, useEffect, useMemo, useRef, useState } from "react";
import {
  extractFieldConditions,
  extractSideConditions,
  resolveSideLabels,
} from "../../utils/conditionData";
import FloatingTooltip from "../Common/Tooltip/FloatingTooltip";
import BattleConditionPopover, { type ConditionTab } from "./BattleConditionPopover";
import styles from "./BattleConditionsBar.module.scss";

export interface BattleConditionsBarProps {
  battleState: BattleState | null;
  playerId?: string | null;
}

export default function BattleConditionsBar({
  battleState,
  playerId,
}: BattleConditionsBarProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [isPinned, setIsPinned] = useState(false);
  const [activeTab, setActiveTab] = useState<ConditionTab>("field");
  const [targetRect, setTargetRect] = useState<DOMRect | null>(null);

  const containerRef = useRef<HTMLDivElement | null>(null);
  const closeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const {
    playerSideIndex,
    foeSideIndex,
    playerSideLabel,
    foeSideLabel,
  } = useMemo(
    () => resolveSideLabels(battleState, playerId),
    [battleState, playerId],
  );

  const fieldData = useMemo(
    () => extractFieldConditions(battleState),
    [battleState],
  );

  const playerData = useMemo(
    () => extractSideConditions(battleState, playerSideIndex, playerSideLabel),
    [battleState, playerSideIndex, playerSideLabel],
  );

  const foeData = useMemo(
    () => extractSideConditions(battleState, foeSideIndex, foeSideLabel),
    [battleState, foeSideIndex, foeSideLabel],
  );

  const clearCloseTimer = () => {
    if (closeTimerRef.current) {
      clearTimeout(closeTimerRef.current);
      closeTimerRef.current = null;
    }
  };

  const scheduleClose = () => {
    if (isPinned) return;
    clearCloseTimer();
    closeTimerRef.current = setTimeout(() => {
      setIsOpen(false);
    }, 120);
  };

  const handleChipMouseEnter = (
    e: MouseEvent<HTMLButtonElement>,
    tab: ConditionTab,
  ) => {
    if (isPinned) return;
    clearCloseTimer();
    const rect = e.currentTarget.getBoundingClientRect();
    setTargetRect(rect);
    setActiveTab(tab);
    setIsOpen(true);
  };

  const handleChipClick = (
    e: MouseEvent<HTMLButtonElement>,
    tab: ConditionTab,
  ) => {
    clearCloseTimer();
    const rect = e.currentTarget.getBoundingClientRect();
    setTargetRect(rect);

    if (isOpen && isPinned && activeTab === tab) {
      setIsPinned(false);
      setIsOpen(false);
    } else {
      setActiveTab(tab);
      setIsOpen(true);
      setIsPinned(true);
    }
  };

  // Close on outside click if pinned
  useEffect(() => {
    if (!isOpen) return;

    const handleDocumentClick = (e: globalThis.MouseEvent) => {
      const target = e.target as Node | null;
      if (!target) return;

      if (containerRef.current && containerRef.current.contains(target)) {
        return;
      }

      const dialog = document.querySelector('[role="dialog"][aria-label="Battle Conditions"]');
      if (dialog && dialog.contains(target)) {
        return;
      }

      setIsPinned(false);
      setIsOpen(false);
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setIsPinned(false);
        setIsOpen(false);
      }
    };

    document.addEventListener("mousedown", handleDocumentClick);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("mousedown", handleDocumentClick);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [isOpen]);

  useEffect(() => {
    return () => {
      clearCloseTimer();
    };
  }, []);

  return (
    <div className={styles.container} ref={containerRef}>
      {/* Field Trigger Chip */}
      <button
        type="button"
        className={`badge badge-secondary ${isOpen && activeTab === "field" ? "active" : ""}`}
        aria-expanded={isOpen && activeTab === "field"}
        onMouseEnter={(e) => handleChipMouseEnter(e, "field")}
        onMouseLeave={scheduleClose}
        onClick={(e) => handleChipClick(e, "field")}
        aria-label="View Field Conditions"
      >
        Field: {fieldData.summaryText}
      </button>

      {/* Your Side Trigger Chip */}
      <button
        type="button"
        className={`badge badge-secondary ${isOpen && activeTab === "player" ? "active" : ""}`}
        aria-expanded={isOpen && activeTab === "player"}
        onMouseEnter={(e) => handleChipMouseEnter(e, "player")}
        onMouseLeave={scheduleClose}
        onClick={(e) => handleChipClick(e, "player")}
        aria-label={`View ${playerSideLabel} Conditions`}
      >
        {playerSideLabel}: {playerData.allCount}
      </button>

      {/* Foe Side Trigger Chip */}
      <button
        type="button"
        className={`badge badge-secondary ${isOpen && activeTab === "foe" ? "active" : ""}`}
        aria-expanded={isOpen && activeTab === "foe"}
        onMouseEnter={(e) => handleChipMouseEnter(e, "foe")}
        onMouseLeave={scheduleClose}
        onClick={(e) => handleChipClick(e, "foe")}
        aria-label={`View ${foeSideLabel} Conditions`}
      >
        {foeSideLabel}: {foeData.allCount}
      </button>

      {/* Condition Popover inside FloatingTooltip */}
      <FloatingTooltip
        isOpen={isOpen}
        targetRect={targetRect}
        onMouseEnter={clearCloseTimer}
        onMouseLeave={scheduleClose}
      >
        <BattleConditionPopover
          battleState={battleState}
          playerId={playerId}
          activeTab={activeTab}
          onTabChange={(tab) => {
            setActiveTab(tab);
            setIsPinned(true);
          }}
        />
      </FloatingTooltip>
    </div>
  );
}
