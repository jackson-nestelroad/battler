import type { BattleState } from "battler-state";
import {
  type MouseEvent,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { extractAllBattleConditions } from "../../utils/conditionData";
import FloatingTooltip from "../Common/Tooltip/FloatingTooltip";
import { TooltipParentContext, useTooltipChildTracker } from "../Common/Tooltip/TooltipContext";
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

  const { openChildCount, isTargetInChild, closeChild, contextValue } =
    useTooltipChildTracker();

  const containerRef = useRef<HTMLDivElement | null>(null);
  const activeChipRef = useRef<HTMLButtonElement | null>(null);
  const popoverRef = useRef<HTMLDivElement | null>(null);
  const closeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isHoveringRef = useRef(false);
  const openChildCountRef = useRef(openChildCount);
  openChildCountRef.current = openChildCount;

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

  const clearCloseTimer = useCallback(() => {
    if (closeTimerRef.current) {
      clearTimeout(closeTimerRef.current);
      closeTimerRef.current = null;
    }
  }, []);

  const scheduleClose = useCallback(() => {
    isHoveringRef.current = false;
    if (isPinned) return;
    clearCloseTimer();
    closeTimerRef.current = setTimeout(() => {
      if (openChildCountRef.current > 0) return;
      setIsOpen(false);
    }, 120);
  }, [clearCloseTimer, isPinned]);

  const handleChipMouseEnter = (
    e: MouseEvent<HTMLButtonElement>,
    tab: ConditionTab,
  ) => {
    isHoveringRef.current = true;
    activeChipRef.current = e.currentTarget;
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
    activeChipRef.current = e.currentTarget;
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

  useEffect(() => {
    if (openChildCount === 0 && !isPinned && !isHoveringRef.current && isOpen) {
      scheduleClose();
    }
  }, [openChildCount, isPinned, isOpen, scheduleClose]);

  // Close on outside click if pinned
  useEffect(() => {
    if (!isOpen) return;

    const handlePointerDown = (e: PointerEvent) => {
      const target = e.target as Node | null;
      if (!target) return;

      // Ignore clicks inside active dialog overlays / modals (e.g. FxLangModal)
      if (target instanceof Element && target.closest?.('[role="dialog"], [aria-modal="true"]')) {
        return;
      }

      if (
        containerRef.current?.contains(target) ||
        popoverRef.current?.contains(target)
      ) {
        if (!isTargetInChild(target)) {
          closeChild();
        }
        return;
      }

      if (isTargetInChild(target)) return;

      closeChild();
      setIsPinned(false);
      setIsOpen(false);
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (openChildCountRef.current > 0) {
          closeChild();
          return;
        }
        setIsPinned(false);
        setIsOpen(false);
      }
    };

    document.addEventListener("pointerdown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [isOpen, isTargetInChild, closeChild]);

  useEffect(() => {
    return () => {
      clearCloseTimer();
    };
  }, [clearCloseTimer]);

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
          aria-haspopup="dialog"
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
        targetRef={activeChipRef}
        containerRef={popoverRef}
        onMouseEnter={() => {
          isHoveringRef.current = true;
          clearCloseTimer();
        }}
        onMouseLeave={scheduleClose}
      >
        <TooltipParentContext.Provider value={contextValue}>
          <BattleConditionPopover
            data={conditions}
            activeTab={activeTab}
            onTabChange={(tab) => {
              setActiveTab(tab);
              setIsPinned(true);
            }}
          />
        </TooltipParentContext.Provider>
      </FloatingTooltip>
    </div>
  );
}
