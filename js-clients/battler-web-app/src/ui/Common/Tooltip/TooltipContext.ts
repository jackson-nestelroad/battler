import { createContext, useCallback, useMemo, useRef, useState } from "react";

export interface TooltipParentContextValue {
  registerChildContent?: (el: HTMLElement) => () => void;
  registerChildOpen?: () => () => void;
  openChild?: (childId: string, closeChild: () => void) => () => void;
  closeChild?: () => void;
}

export const TooltipParentContext = createContext<TooltipParentContextValue | null>(null);

export interface UseTooltipChildTrackerResult {
  openChildCount: number;
  childContentEls: React.RefObject<Set<HTMLElement>>;
  isTargetInChild: (target: Node) => boolean;
  closeChild: () => void;
  contextValue: TooltipParentContextValue;
}

export function useTooltipChildTracker(
  parentContext?: TooltipParentContextValue | null,
): UseTooltipChildTrackerResult {
  const [openChildCount, setOpenChildCount] = useState(0);
  const childContentEls = useRef<Set<HTMLElement>>(new Set());
  const activeChildRef = useRef<{ id: string; close: () => void } | null>(null);

  const registerChildContent = useCallback(
    (el: HTMLElement) => {
      childContentEls.current.add(el);
      const unregisterFromParent = parentContext?.registerChildContent?.(el);
      return () => {
        childContentEls.current.delete(el);
        unregisterFromParent?.();
      };
    },
    [parentContext],
  );

  const registerChildOpen = useCallback(() => {
    setOpenChildCount((prev) => prev + 1);
    const unregisterFromParent = parentContext?.registerChildOpen?.();
    return () => {
      setOpenChildCount((prev) => Math.max(0, prev - 1));
      unregisterFromParent?.();
    };
  }, [parentContext]);

  const openChild = useCallback((childId: string, closeChildFn: () => void) => {
    if (activeChildRef.current && activeChildRef.current.id !== childId) {
      activeChildRef.current.close();
    }
    activeChildRef.current = { id: childId, close: closeChildFn };
    return () => {
      if (activeChildRef.current?.id === childId) {
        activeChildRef.current = null;
      }
    };
  }, []);

  const closeChild = useCallback(() => {
    if (activeChildRef.current) {
      const { close } = activeChildRef.current;
      activeChildRef.current = null;
      close();
    }
  }, []);

  const contextValue = useMemo(
    () => ({
      registerChildContent,
      registerChildOpen,
      openChild,
      closeChild,
    }),
    [registerChildContent, registerChildOpen, openChild, closeChild],
  );

  const isTargetInChild = useCallback((target: Node) => {
    for (const childEl of childContentEls.current) {
      if (childEl.contains(target)) return true;
    }
    return false;
  }, []);

  return {
    openChildCount,
    childContentEls,
    isTargetInChild,
    closeChild,
    contextValue,
  };
}
