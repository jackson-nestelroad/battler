import type { BattleState } from "battler-state";
import { type MouseEvent, useEffect, useMemo, useRef, useState } from "react";
import { extractAllBattleConditions } from "../../utils/conditionData";
import FloatingTooltip from "../Common/Tooltip/FloatingTooltip";
import BattleConditionPopover, { type ConditionTab } from "./BattleConditionPopover";

export interface BattleConditionsBarProps {
  battleState?: BattleState | null;
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
  const popoverRef = useRef<HTMLDivElement | null>(null);
  const closeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const conditions = useMemo(
    () => extractAllBattleConditions(battleState, playerId),
    [battleState, playerId],
  );

  const {
    fieldData,
    playerData,
    foeData,
    playerSideLabel,
    foeSideLabel,
  } = conditions;

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

      if (
        containerRef.current?.contains(target) ||
        popoverRef.current?.contains(target)
      ) {
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

  const chips: { tab: ConditionTab; label: string; text: string }[] = [
    {
      tab: "field",
      label: "View field conditions",
      text: `Field: ${fieldData.summaryText}`,
    },
    {
      tab: "player",
      label: `View ${playerSideLabel} conditions`,
      text: `${playerSideLabel}: ${playerData.allCount}`,
    },
    {
      tab: "foe",
      label: `View ${foeSideLabel} conditions`,
      text: `${foeSideLabel}: ${foeData.allCount}`,
    },
  ];

  return (
    <div className="flex-row align-center gap-s flex-wrap" ref={containerRef}>
      {chips.map(({ tab, label, text }) => (
        <button
          key={tab}
          type="button"
          className="badge badge-secondary"
          aria-expanded={isOpen && activeTab === tab}
          onMouseEnter={(e) => handleChipMouseEnter(e, tab)}
          onMouseLeave={scheduleClose}
          onClick={(e) => handleChipClick(e, tab)}
          aria-label={label}
        >
          {text}
        </button>
      ))}

      {/* Condition Popover inside FloatingTooltip */}
      <FloatingTooltip
        isOpen={isOpen}
        targetRect={targetRect}
        onMouseEnter={clearCloseTimer}
        onMouseLeave={scheduleClose}
      >
        <BattleConditionPopover
          ref={popoverRef}
          data={conditions}
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
