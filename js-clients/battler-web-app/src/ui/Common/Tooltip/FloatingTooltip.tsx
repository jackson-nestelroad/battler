import {
  type ReactNode,
  type RefObject,
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
import {
  calculateFloatingCoords,
  getElementRect,
  type FloatingCoordsResult,
} from "../../../utils/floatingCoords";
import styles from "./FloatingTooltip.module.scss";

export interface FloatingTooltipProps {
  isOpen: boolean;
  targetRect?: DOMRect | null;
  targetRef?: RefObject<HTMLElement | null> | { current: HTMLElement | null };
  containerRef?:
    | RefObject<HTMLDivElement | null>
    | { current: HTMLDivElement | null }
    | ((node: HTMLDivElement | null) => void);
  children: ReactNode;
  onMouseEnter?: () => void;
  onMouseLeave?: () => void;
  preferredPlacement?: "top" | "bottom" | "left" | "right";
}

const BRIDGE_CLASSES: Record<FloatingCoordsResult["placement"], string> = {
  top: styles.bridgeTop,
  bottom: styles.bridgeBottom,
  left: styles.bridgeLeft,
  right: styles.bridgeRight,
};

export default function FloatingTooltip({
  isOpen,
  targetRect = null,
  targetRef,
  containerRef,
  children,
  onMouseEnter,
  onMouseLeave,
  preferredPlacement = "top",
}: FloatingTooltipProps) {
  const tooltipRef = useRef<HTMLDivElement | null>(null);
  const [mounted, setMounted] = useState(false);
  const [coords, setCoords] = useState<FloatingCoordsResult>({
    top: -9999,
    left: -9999,
    placement: preferredPlacement,
  });

  const anchoredTopRef = useRef<number | null>(null);
  const lastTargetTopRef = useRef<number | null>(null);
  const lastTargetLeftRef = useRef<number | null>(null);
  const lastPlacementRef = useRef<string | null>(null);

  useEffect(() => {
    setMounted(true);
  }, []);

  useEffect(() => {
    if (!isOpen) {
      anchoredTopRef.current = null;
      lastTargetTopRef.current = null;
      lastTargetLeftRef.current = null;
      lastPlacementRef.current = null;
      setCoords({ top: -9999, left: -9999, placement: preferredPlacement });
    }
  }, [isOpen, preferredPlacement]);

  const updatePosition = useCallback(() => {
    if (!isOpen || !tooltipRef.current) return;

    let rect = targetRect;
    if (targetRef?.current) {
      const liveRect = getElementRect(targetRef.current);
      if (liveRect.width !== 0 || liveRect.height !== 0) {
        rect = liveRect;
      }
    }
    if (!rect) return;

    const tooltipEl = tooltipRef.current;
    const tooltipWidth = tooltipEl.offsetWidth;
    const tooltipHeight = tooltipEl.offsetHeight;

    const newCoords = calculateFloatingCoords(
      rect,
      tooltipWidth,
      tooltipHeight,
      preferredPlacement,
    );

    // If target element moved (e.g. scroll), placement flipped, or initial position: establish fresh anchor
    if (
      lastTargetTopRef.current !== rect.top ||
      lastTargetLeftRef.current !== rect.left ||
      lastPlacementRef.current !== newCoords.placement ||
      anchoredTopRef.current === null
    ) {
      lastTargetTopRef.current = rect.top;
      lastTargetLeftRef.current = rect.left;
      lastPlacementRef.current = newCoords.placement;
      anchoredTopRef.current = newCoords.top;
      setCoords(newCoords);
      return;
    }

    // When tooltip content shrinks/expands while open without target moving (e.g. switching tabs):
    // Anchor top so it doesn't drop downwards away from the user's cursor
    const stableTop = Math.min(anchoredTopRef.current, newCoords.top);
    anchoredTopRef.current = stableTop;

    setCoords({
      ...newCoords,
      top: stableTop,
    });
  }, [isOpen, targetRect, targetRef, preferredPlacement]);

  // Recalculate immediately when props or children change
  useLayoutEffect(() => {
    updatePosition();
  }, [updatePosition, children]);

  // Observe element resizing (e.g. async card content loading) and window resize/scroll
  useEffect(() => {
    if (!isOpen || !tooltipRef.current) return;
    const el = tooltipRef.current;

    let resizeObserver: ResizeObserver | null = null;
    if (typeof ResizeObserver !== "undefined") {
      resizeObserver = new ResizeObserver(() => {
        updatePosition();
      });
      resizeObserver.observe(el);
    }

    const handleWindowChange = () => {
      updatePosition();
    };

    window.addEventListener("resize", handleWindowChange);
    window.addEventListener("scroll", handleWindowChange, true);

    return () => {
      resizeObserver?.disconnect();
      window.removeEventListener("resize", handleWindowChange);
      window.removeEventListener("scroll", handleWindowChange, true);
    };
  }, [isOpen, updatePosition]);

  if (!mounted || typeof document === "undefined") return null;

  const isPositioned = coords.top !== -9999;
  if (!isOpen && !isPositioned) return null;

  const placementClass = BRIDGE_CLASSES[coords.placement] ?? styles.bridgeTop;

  return createPortal(
    <div
      ref={(node) => {
        tooltipRef.current = node;
        if (typeof containerRef === "function") {
          containerRef(node);
        } else if (containerRef) {
          (containerRef as React.MutableRefObject<HTMLDivElement | null>).current = node;
        }
      }}
      className={`${styles.floatingPortal} ${placementClass} ${isOpen && isPositioned ? styles.visible : styles.hidden}`}
      style={{
        top: `${coords.top}px`,
        left: `${coords.left}px`,
      }}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
      onClick={(e) => e.stopPropagation()}
    >
      {children}
    </div>,
    document.body,
  );
}
