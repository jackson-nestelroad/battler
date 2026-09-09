import type { BattleState, MonBattleAppearanceReference, UiMon } from "battler-state";
import type { MonBattleData } from "battler-types";
import { type MouseEvent, type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import {
  appearanceRefToTooltip,
  monBattleDataToTooltip,
  publicMonStateToTooltip,
} from "../../../utils/monTooltipModel";
import FloatingTooltip from "./FloatingTooltip";
import MonTooltipCard from "./MonTooltipCard";

function useInteractiveTooltip(viewModel: unknown) {
  const [isOpen, setIsOpen] = useState(false);
  const [targetRect, setTargetRect] = useState<DOMRect | null>(null);
  const closeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const clearCloseTimer = () => {
    if (closeTimerRef.current) {
      clearTimeout(closeTimerRef.current);
      closeTimerRef.current = null;
    }
  };

  const handleMouseEnter = (e: MouseEvent<HTMLElement>) => {
    if (!viewModel) return;
    clearCloseTimer();
    const el = e.currentTarget;
    let rect = el.getBoundingClientRect();
    if (rect.width === 0 && rect.height === 0 && el.firstElementChild) {
      rect = (el.firstElementChild as HTMLElement).getBoundingClientRect();
    }
    setTargetRect(rect);
    setIsOpen(true);
  };

  const scheduleClose = () => {
    clearCloseTimer();
    closeTimerRef.current = setTimeout(() => {
      setIsOpen(false);
    }, 40);
  };

  useEffect(() => {
    return () => {
      clearCloseTimer();
    };
  }, []);

  return {
    isOpen,
    targetRect,
    handleMouseEnter,
    handleMouseLeave: scheduleClose,
    handleTooltipMouseEnter: clearCloseTimer,
    handleTooltipMouseLeave: scheduleClose,
  };
}

export interface MonTooltipTriggerProps {
  mon?: MonBattleData | null;
  monRef?: UiMon;
  appearanceRef?: MonBattleAppearanceReference;
  battleState?: BattleState | null;
  rules?: string[] | null;
  children: ReactNode;
  className?: string;
  as?: "span" | "div";
  preferredPlacement?: "top" | "bottom" | "left" | "right";
}

export default function MonTooltipTrigger({
  mon,
  monRef,
  appearanceRef,
  battleState,
  rules,
  children,
  className,
  as = "span",
  preferredPlacement = "top",
}: MonTooltipTriggerProps) {
  const viewModel = useMemo(() => {
    if (mon) {
      return monBattleDataToTooltip(mon, battleState, rules);
    }
    if (battleState && appearanceRef) {
      return appearanceRefToTooltip(battleState, appearanceRef, rules);
    }
    if (battleState && monRef) {
      return publicMonStateToTooltip(battleState, monRef, rules);
    }
    return null;
  }, [mon, appearanceRef, battleState, monRef, rules]);

  const {
    isOpen,
    targetRect,
    handleMouseEnter,
    handleMouseLeave,
    handleTooltipMouseEnter,
    handleTooltipMouseLeave,
  } = useInteractiveTooltip(viewModel);

  const Component = as;

  if (!viewModel) {
    return <Component className={className}>{children}</Component>;
  }

  return (
    <>
      <Component
        className={className}
        onMouseEnter={handleMouseEnter}
        onMouseLeave={handleMouseLeave}
      >
        {children}
      </Component>
      <FloatingTooltip
        isOpen={isOpen}
        targetRect={targetRect}
        onMouseEnter={handleTooltipMouseEnter}
        onMouseLeave={handleTooltipMouseLeave}
        preferredPlacement={preferredPlacement}
      >
        <MonTooltipCard data={viewModel} />
      </FloatingTooltip>
    </>
  );
}
