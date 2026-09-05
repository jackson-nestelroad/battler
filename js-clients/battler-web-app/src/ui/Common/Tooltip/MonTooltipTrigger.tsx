import type { BattleState, UiMon } from "battler-state";
import type { MonBattleData } from "battler-types";
import React, { useEffect, useMemo, useRef, useState } from "react";
import {
  monBattleDataToTooltip,
  publicMonStateToTooltip,
} from "../../../utils/monTooltipModel";
import FloatingTooltip from "./FloatingTooltip";
import PokemonTooltipCard from "./PokemonTooltipCard";

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

  const handleMouseEnter = (e: React.MouseEvent<HTMLElement>) => {
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

  const handleMouseLeave = () => {
    clearCloseTimer();
    closeTimerRef.current = setTimeout(() => {
      setIsOpen(false);
    }, 40);
  };

  const handleTooltipMouseEnter = () => {
    clearCloseTimer();
  };

  const handleTooltipMouseLeave = () => {
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
    handleMouseLeave,
    handleTooltipMouseEnter,
    handleTooltipMouseLeave,
  };
}

export interface MonTooltipTriggerProps {
  mon?: MonBattleData | null;
  monRef?: UiMon;
  battleState?: BattleState | null;
  children: React.ReactNode;
  className?: string;
}

export default function MonTooltipTrigger({
  mon,
  monRef,
  battleState,
  children,
  className,
}: MonTooltipTriggerProps) {
  const triggerRef = useRef<HTMLSpanElement | null>(null);

  const viewModel = useMemo(() => {
    if (mon) {
      return monBattleDataToTooltip(mon, battleState);
    }
    if (battleState && monRef) {
      return publicMonStateToTooltip(battleState, monRef);
    }
    return null;
  }, [mon, battleState, monRef]);

  const {
    isOpen,
    targetRect,
    handleMouseEnter,
    handleMouseLeave,
    handleTooltipMouseEnter,
    handleTooltipMouseLeave,
  } = useInteractiveTooltip(viewModel);

  if (!viewModel) {
    return <span className={className}>{children}</span>;
  }

  return (
    <>
      <span
        ref={triggerRef}
        className={className}
        onMouseEnter={handleMouseEnter}
        onMouseLeave={handleMouseLeave}
      >
        {children}
      </span>
      <FloatingTooltip
        isOpen={isOpen}
        targetRect={targetRect}
        onMouseEnter={handleTooltipMouseEnter}
        onMouseLeave={handleTooltipMouseLeave}
      >
        <PokemonTooltipCard data={viewModel} />
      </FloatingTooltip>
    </>
  );
}
