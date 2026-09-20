import type { BattleState, MonBattleAppearanceReference, UiMon } from "battler-state";
import type { MonBattleData } from "battler-types";
import {
  type FocusEvent,
  type KeyboardEvent as ReactKeyboardEvent,
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
import {
  TooltipParentContext,
  useTooltipChildTracker,
  hasActivePinnedTooltip,
  registerActivePinnedTooltip,
  MonTooltipContext,
  type MonTooltipContextValue,
} from "./TooltipContext";

function useInteractiveTooltip(
  openChildCount: number,
  closeChild?: () => void,
  isTargetInChild?: (target: Node) => boolean,
  triggerRef?: React.RefObject<HTMLElement | null>,
  contentRef?: React.RefObject<HTMLDivElement | null>,
  disableClick = false,
) {
  const [isOpen, setIsOpen] = useState(false);
  const [isPinned, setIsPinned] = useState(false);
  const [targetRect, setTargetRect] = useState<DOMRect | null>(null);
  const closeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isHoveringRef = useRef(false);
  const isPinnedRef = useRef(false);
  isPinnedRef.current = isPinned;
  const openChildCountRef = useRef(openChildCount);
  openChildCountRef.current = openChildCount;

  const clearCloseTimer = useCallback(() => {
    if (closeTimerRef.current) {
      clearTimeout(closeTimerRef.current);
      closeTimerRef.current = null;
    }
  }, []);

  const close = useCallback(() => {
    clearCloseTimer();
    isPinnedRef.current = false;
    isHoveringRef.current = false;
    setIsPinned(false);
    closeChild?.();
    setIsOpen(false);
  }, [clearCloseTimer, closeChild]);

  const scheduleClose = useCallback(() => {
    if (isPinnedRef.current) return;
    isHoveringRef.current = false;
    clearCloseTimer();
    closeTimerRef.current = setTimeout(() => {
      if (isPinnedRef.current) return;
      if (openChildCountRef.current > 0) return;
      close();
    }, 120);
  }, [clearCloseTimer, close]);

  const toggle = useCallback(
    (targetEl?: HTMLElement) => {
      clearCloseTimer();
      if (isOpen && isPinnedRef.current) {
        close();
        return;
      }
      if (targetEl) {
        setTargetRect(getElementRect(targetEl));
      } else if (triggerRef?.current) {
        setTargetRect(getElementRect(triggerRef.current));
      }
      isPinnedRef.current = true;
      isHoveringRef.current = true;
      setIsPinned(true);
      setIsOpen(true);
    },
    [clearCloseTimer, close, isOpen, triggerRef],
  );

  const open = useCallback(
    (targetEl?: HTMLElement) => {
      clearCloseTimer();
      isPinnedRef.current = true;
      isHoveringRef.current = true;
      if (targetEl) {
        setTargetRect(getElementRect(targetEl));
      } else if (triggerRef?.current) {
        setTargetRect(getElementRect(triggerRef.current));
      }
      setIsPinned(true);
      setIsOpen(true);
    },
    [clearCloseTimer, triggerRef],
  );

  const lastTouchTimeRef = useRef(0);

  const handlePointerDown = (e: React.PointerEvent<HTMLElement>) => {
    if (e.pointerType === "touch") {
      lastTouchTimeRef.current = Date.now();
    }
  };

  const handleTouchStart = () => {
    lastTouchTimeRef.current = Date.now();
  };

  const handleMouseEnter = (e: MouseEvent<HTMLElement>) => {
    if (Date.now() - lastTouchTimeRef.current < 600) return;
    if (isPinnedRef.current) return;
    if (hasActivePinnedTooltip()) return;
    if ((e.nativeEvent as PointerEvent).pointerType === "touch") return;
    isHoveringRef.current = true;
    clearCloseTimer();
    setTargetRect(getElementRect(e.currentTarget));
    setIsOpen(true);
  };

  const handleFocus = (e: FocusEvent<HTMLElement>) => {
    if (Date.now() - lastTouchTimeRef.current < 600) return;
    if (isPinnedRef.current) return;
    if (hasActivePinnedTooltip()) return;
    isHoveringRef.current = true;
    clearCloseTimer();
    setTargetRect(getElementRect(e.currentTarget));
    setIsOpen(true);
  };

  const handleBlur = () => {
    if (isPinnedRef.current) return;
    scheduleClose();
  };

  const handleTooltipMouseEnter = () => {
    isHoveringRef.current = true;
    clearCloseTimer();
  };

  const handleTooltipMouseLeave = () => {
    isHoveringRef.current = false;
    if (isPinnedRef.current) return;
    scheduleClose();
  };

  const handleClick = (e: MouseEvent<HTMLElement>) => {
    if (disableClick) return;
    e.stopPropagation();
    toggle(e.currentTarget);
  };

  const handleKeyDown = (e: ReactKeyboardEvent<HTMLElement>) => {
    if (disableClick) return;
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      e.stopPropagation();
      toggle(e.currentTarget);
    }
  };

  useEffect(() => {
    if (!isPinned && openChildCount === 0 && !isHoveringRef.current && isOpen) {
      scheduleClose();
    }
  }, [openChildCount, isOpen, isPinned, scheduleClose]);

  useEffect(() => {
    if (!isOpen) return;

    const handleWindowKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (openChildCountRef.current > 0) {
          closeChild?.();
          return;
        }
        close();
      }
    };

    const handleDocumentPointerDown = (e: PointerEvent) => {
      const target = e.target as Node | null;
      if (!target) return;
      if (triggerRef?.current?.contains(target)) {
        if (disableClick) {
          const isInfoBtn = Boolean((target as HTMLElement).closest?.(".info-btn"));
          if (!isInfoBtn) {
            // Clicked on the card trigger body (not the info button)
            close();
          }
        }
        return;
      }

      // Ignore clicks inside active dialog overlays / modals (e.g. FxLangModal)
      if (isTargetInsideModal(target)) {
        return;
      }

      if (contentRef?.current?.contains(target)) {
        clearCloseTimer();
        isHoveringRef.current = true;
        if (isTargetInChild && !isTargetInChild(target)) {
          closeChild?.();
        }
        return;
      }

      if (isTargetInChild?.(target)) return;

      close();
    };

    document.addEventListener("keydown", handleWindowKeyDown);
    document.addEventListener("pointerdown", handleDocumentPointerDown);
    return () => {
      document.removeEventListener("keydown", handleWindowKeyDown);
      document.removeEventListener("pointerdown", handleDocumentPointerDown);
    };
  }, [isOpen, clearCloseTimer, close, closeChild, isTargetInChild, contentRef, triggerRef, disableClick]);

  useEffect(() => {
    return () => {
      clearCloseTimer();
    };
  }, [clearCloseTimer]);

  return {
    isOpen,
    isPinned,
    targetRect,
    handlePointerDown,
    handleTouchStart,
    handleMouseEnter,
    handleMouseLeave: scheduleClose,
    handleFocus,
    handleBlur,
    handleTooltipMouseEnter,
    handleTooltipMouseLeave,
    handleClick,
    handleKeyDown,
    toggle,
    open,
    close,
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
  disableClick?: boolean;
  ariaLabel?: string;
  title?: string;
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
  disableClick = false,
  ariaLabel,
  title,
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
    isPinned,
    targetRect,
    handlePointerDown,
    handleTouchStart,
    handleMouseEnter,
    handleMouseLeave,
    handleFocus,
    handleBlur,
    handleTooltipMouseEnter,
    handleTooltipMouseLeave,
    handleClick,
    handleKeyDown,
    toggle,
    open,
    close,
  } = useInteractiveTooltip(
    openChildCount,
    closeChild,
    isTargetInChild,
    triggerRef,
    contentRef,
    disableClick,
  );

  useEffect(() => {
    if (!isOpen) return;
    const unregisterContent =
      contentRef.current && parentContext?.registerChildContent
        ? parentContext.registerChildContent(contentRef.current)
        : undefined;
    const unregisterOpen = parentContext?.registerChildOpen?.();
    const unregisterActiveChild = parentContext?.openChild?.(triggerId, () => {
      close();
    });
    return () => {
      unregisterContent?.();
      unregisterOpen?.();
      unregisterActiveChild?.();
    };
  }, [isOpen, parentContext, triggerId, close]);

  useEffect(() => {
    if (!isOpen || !isPinned || parentContext) return;
    return registerActivePinnedTooltip(triggerId, () => {
      close();
    });
  }, [isOpen, isPinned, parentContext, triggerId, close]);

  const monTooltipContextValue = useMemo<MonTooltipContextValue>(
    () => ({
      isOpen,
      isPinned,
      toggle,
      open,
      close,
    }),
    [isOpen, isPinned, toggle, open, close],
  );

  const Component = as;

  if (!viewModel) {
    return <Component className={className}>{children}</Component>;
  }

  const sharedAriaProps = {
    "aria-label": ariaLabel,
    title,
    "aria-haspopup": "dialog" as const,
    "aria-expanded": isOpen,
  };

  const interactiveProps = !disableClick
    ? {
        role: "button" as const,
        tabIndex: 0,
        onClick: handleClick,
        onKeyDown: handleKeyDown,
        onPointerDown: handlePointerDown,
        onTouchStart: handleTouchStart,
        ...sharedAriaProps,
      }
    : {
        onPointerDown: handlePointerDown,
        onTouchStart: handleTouchStart,
      };

  return (
    <MonTooltipContext.Provider value={monTooltipContextValue}>
      <Component
        ref={triggerRef as React.Ref<never>}
        className={className}
        onMouseEnter={handleMouseEnter}
        onMouseLeave={handleMouseLeave}
        onFocus={!disableClick ? handleFocus : undefined}
        onBlur={!disableClick ? handleBlur : undefined}
        {...interactiveProps}
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
    </MonTooltipContext.Provider>
  );
}
