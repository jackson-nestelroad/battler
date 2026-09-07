import {
  type ReactNode,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
import {
  calculateFloatingCoords,
  type FloatingCoordsResult,
} from "../../../utils/floatingCoords";
import styles from "./FloatingTooltip.module.scss";

interface FloatingTooltipProps {
  isOpen: boolean;
  targetRect: DOMRect | null;
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

  const placementClass = BRIDGE_CLASSES[coords.placement];

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
