import {
  type ReactNode,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
import styles from "./FloatingTooltip.module.scss";

interface FloatingTooltipProps {
  isOpen: boolean;
  targetRect: DOMRect | null;
  children: ReactNode;
  onMouseEnter?: () => void;
  onMouseLeave?: () => void;
  preferredPlacement?: "top" | "bottom" | "left" | "right";
}

export interface FloatingCoordsResult {
  top: number;
  left: number;
  placement: "top" | "bottom" | "left" | "right";
}

export function calculateFloatingCoords(
  targetRect: DOMRect,
  tooltipWidth: number,
  tooltipHeight: number,
  preferredPlacement: "top" | "bottom" | "left" | "right" = "top",
  viewportWidth = typeof window !== "undefined" ? window.innerWidth : 1024,
  viewportHeight = typeof window !== "undefined" ? window.innerHeight : 768,
): FloatingCoordsResult {
  const padding = 12;
  const gap = 8;

  let placement: "top" | "bottom" | "left" | "right" = preferredPlacement;
  let left = 0;
  let top = 0;

  if (placement === "left") {
    const roomOnLeft = targetRect.left - tooltipWidth - gap >= padding;
    if (roomOnLeft) {
      left = targetRect.left - tooltipWidth - gap;
      top = Math.max(
        padding,
        Math.min(
          targetRect.top + targetRect.height / 2 - tooltipHeight / 2,
          viewportHeight - tooltipHeight - padding,
        ),
      );
    } else {
      placement = "top";
    }
  } else if (placement === "right") {
    const roomOnRight =
      viewportWidth - targetRect.right - tooltipWidth - gap >= padding;
    if (roomOnRight) {
      left = targetRect.right + gap;
      top = Math.max(
        padding,
        Math.min(
          targetRect.top + targetRect.height / 2 - tooltipHeight / 2,
          viewportHeight - tooltipHeight - padding,
        ),
      );
    } else {
      placement = "top";
    }
  }

  if (placement === "top" || placement === "bottom") {
    // Center horizontally on target
    left = targetRect.left + targetRect.width / 2 - tooltipWidth / 2;
    // Clamp to viewport
    left = Math.max(padding, Math.min(left, viewportWidth - tooltipWidth - padding));

    // Prefer placing above target
    top = targetRect.top - tooltipHeight - gap;
    placement = "top";

    // If clipping off top edge, place below target
    if (top < padding) {
      top = targetRect.bottom + gap;
      placement = "bottom";
    }

    // If also clipping bottom, clamp within screen
    if (top + tooltipHeight > viewportHeight - padding) {
      top = Math.max(padding, viewportHeight - tooltipHeight - padding);
    }
  }

  return { top, left, placement };
}

export default function FloatingTooltip({
  isOpen,
  targetRect,
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

  useEffect(() => {
    setMounted(true);
  }, []);

  useEffect(() => {
    if (!isOpen) {
      setCoords({ top: -9999, left: -9999, placement: preferredPlacement });
    }
  }, [isOpen, preferredPlacement]);

  useLayoutEffect(() => {
    if (!isOpen || !targetRect || !tooltipRef.current) return;

    const tooltipEl = tooltipRef.current;
    const tooltipWidth = tooltipEl.offsetWidth;
    const tooltipHeight = tooltipEl.offsetHeight;

    setCoords(
      calculateFloatingCoords(
        targetRect,
        tooltipWidth,
        tooltipHeight,
        preferredPlacement,
      ),
    );
  }, [isOpen, targetRect, preferredPlacement]);

  const isPositioned = coords.top !== -9999;
  if (!isOpen && !isPositioned) return null;

  const placementClass =
    coords.placement === "top"
      ? styles.bridgeTop
      : coords.placement === "bottom"
        ? styles.bridgeBottom
        : coords.placement === "left"
          ? styles.bridgeLeft
          : styles.bridgeRight;

  if (!mounted || typeof document === "undefined") return null;

  return createPortal(
    <div
      ref={tooltipRef}
      className={`${styles.floatingPortal} ${placementClass} ${isOpen && isPositioned ? styles.visible : styles.hidden}`}
      style={{
        top: `${coords.top}px`,
        left: `${coords.left}px`,
      }}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
    >
      {children}
    </div>,
    document.body,
  );
}
