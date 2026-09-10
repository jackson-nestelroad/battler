import type { BattleState, MonBattleAppearanceReference, UiMon } from "battler-state";
import type { MonBattleData } from "battler-types";
import {
  type FocusEvent,
  type MouseEvent,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  appearanceRefToTooltip,
  monBattleDataToTooltip,
  publicMonStateToTooltip,
} from "../../../utils/monTooltipModel";
import { getElementRect } from "../../../utils/floatingCoords";
import { isTargetInsideModal } from "../../../utils/dom";
import FloatingTooltip from "./FloatingTooltip";
import MonTooltipCard from "./MonTooltipCard";
import { TooltipParentContext, useTooltipChildTracker } from "./TooltipContext";

function useInteractiveTooltip(
  openChildCount: number,
  closeChild?: () => void,
  isTargetInChild?: (target: Node) => boolean,
  triggerRef?: React.RefObject<HTMLElement | null>,
  contentRef?: React.RefObject<HTMLDivElement | null>,
) {
  const [isOpen, setIsOpen] = useState(false);
  const [targetRect, setTargetRect] = useState<DOMRect | null>(null);
  const closeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isHoveringRef = useRef(false);
  const openChildCountRef = useRef(openChildCount);
  openChildCountRef.current = openChildCount;

  const clearCloseTimer = useCallback(() => {
    if (closeTimerRef.current) {
      clearTimeout(closeTimerRef.current);
      closeTimerRef.current = null;
    }
  }, []);

  const handleMouseEnter = (e: MouseEvent<HTMLElement>) => {
    isHoveringRef.current = true;
    clearCloseTimer();
    setTargetRect(getElementRect(e.currentTarget));
    setIsOpen(true);
  };

  const handleFocus = (e: FocusEvent<HTMLElement>) => {
    isHoveringRef.current = true;
    clearCloseTimer();
    setTargetRect(getElementRect(e.currentTarget));
    setIsOpen(true);
  };

  const scheduleClose = useCallback(() => {
    isHoveringRef.current = false;
    clearCloseTimer();
    closeTimerRef.current = setTimeout(() => {
      if (openChildCountRef.current > 0) return;
      closeChild?.();
      setIsOpen(false);
    }, 40);
  }, [clearCloseTimer, closeChild]);

  const handleBlur = () => {
    scheduleClose();
  };

  const handleTooltipMouseEnter = () => {
    isHoveringRef.current = true;
    clearCloseTimer();
  };

  const handleTooltipMouseLeave = () => {
    isHoveringRef.current = false;
    scheduleClose();
  };

  useEffect(() => {
    if (openChildCount === 0 && !isHoveringRef.current && isOpen) {
      scheduleClose();
    }
  }, [openChildCount, isOpen, scheduleClose]);

  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (openChildCountRef.current > 0) {
          closeChild?.();
          return;
        }
        clearCloseTimer();
        setIsOpen(false);
      }
    };

    const handlePointerDown = (e: PointerEvent) => {
      const target = e.target as Node | null;
      if (!target) return;
      if (triggerRef?.current?.contains(target)) return;

      // Ignore clicks inside active dialog overlays / modals (e.g. FxLangModal)
      if (isTargetInsideModal(target)) {
        return;
      }

      if (contentRef?.current?.contains(target)) {
        if (isTargetInChild && !isTargetInChild(target)) {
          closeChild?.();
        }
        return;
      }

      if (isTargetInChild?.(target)) return;

      closeChild?.();
      clearCloseTimer();
      setIsOpen(false);
    };

    document.addEventListener("keydown", handleKeyDown);
    document.addEventListener("pointerdown", handlePointerDown);
    return () => {
      document.removeEventListener("keydown", handleKeyDown);
      document.removeEventListener("pointerdown", handlePointerDown);
    };
  }, [isOpen, clearCloseTimer, closeChild, isTargetInChild, contentRef, triggerRef]);

  useEffect(() => {
    return () => {
      clearCloseTimer();
    };
  }, [clearCloseTimer]);

  return {
    isOpen,
    targetRect,
    handleMouseEnter,
    handleMouseLeave: scheduleClose,
    handleFocus,
    handleBlur,
    handleTooltipMouseEnter,
    handleTooltipMouseLeave,
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
  const triggerId = useId();
  const parentContext = useContext(TooltipParentContext);
  const { openChildCount, isTargetInChild, closeChild, contextValue } =
    useTooltipChildTracker(parentContext);

  const triggerRef = useRef<HTMLElement | null>(null);
  const contentRef = useRef<HTMLDivElement | null>(null);

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
    handleFocus,
    handleBlur,
    handleTooltipMouseEnter,
    handleTooltipMouseLeave,
  } = useInteractiveTooltip(
    openChildCount,
    closeChild,
    isTargetInChild,
    triggerRef,
    contentRef,
  );

  useEffect(() => {
    if (!isOpen) return;
    const unregisterContent =
      contentRef.current && parentContext?.registerChildContent
        ? parentContext.registerChildContent(contentRef.current)
        : undefined;
    const unregisterOpen = parentContext?.registerChildOpen?.();
    const unregisterActiveChild = parentContext?.openChild?.(triggerId, () => {
      closeChild();
    });
    return () => {
      unregisterContent?.();
      unregisterOpen?.();
      unregisterActiveChild?.();
    };
  }, [isOpen, parentContext, triggerId, closeChild]);

  const Component = as;

  if (!viewModel) {
    return <Component className={className}>{children}</Component>;
  }

  return (
    <>
      <Component
        ref={triggerRef as React.Ref<never>}
        className={className}
        onMouseEnter={handleMouseEnter}
        onMouseLeave={handleMouseLeave}
        onFocus={handleFocus}
        onBlur={handleBlur}
      >
        {children}
      </Component>
      <FloatingTooltip
        isOpen={isOpen}
        targetRect={targetRect}
        targetRef={triggerRef}
        containerRef={contentRef}
        onMouseEnter={handleTooltipMouseEnter}
        onMouseLeave={handleTooltipMouseLeave}
        preferredPlacement={preferredPlacement}
      >
        <TooltipParentContext.Provider value={contextValue}>
          <MonTooltipCard data={viewModel} />
        </TooltipParentContext.Provider>
      </FloatingTooltip>
    </>
  );
}
