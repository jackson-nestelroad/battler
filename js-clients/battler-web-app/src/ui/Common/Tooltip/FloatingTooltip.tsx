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
}

export default function FloatingTooltip({
  isOpen,
  targetRect,
  children,
  onMouseEnter,
  onMouseLeave,
}: FloatingTooltipProps) {
  const tooltipRef = useRef<HTMLDivElement | null>(null);
  const [coords, setCoords] = useState<{
    top: number;
    left: number;
    placement: "top" | "bottom";
  }>({ top: -9999, left: -9999, placement: "top" });

  useEffect(() => {
    if (!isOpen) {
      setCoords({ top: -9999, left: -9999, placement: "top" });
    }
  }, [isOpen]);

  useLayoutEffect(() => {
    if (!isOpen || !targetRect || !tooltipRef.current) return;

    const tooltipEl = tooltipRef.current;
    const tooltipWidth = tooltipEl.offsetWidth;
    const tooltipHeight = tooltipEl.offsetHeight;

    const padding = 12;
    const gap = 8;

    // Center horizontally on target
    let left = targetRect.left + targetRect.width / 2 - tooltipWidth / 2;
    // Clamp to viewport
    left = Math.max(padding, Math.min(left, window.innerWidth - tooltipWidth - padding));

    // Prefer placing above target
    let top = targetRect.top - tooltipHeight - gap;
    let placement: "top" | "bottom" = "top";

    // If clipping off top edge, place below target
    if (top < padding) {
      top = targetRect.bottom + gap;
      placement = "bottom";
    }

    // If also clipping bottom, clamp within screen
    if (top + tooltipHeight > window.innerHeight - padding) {
      top = Math.max(padding, window.innerHeight - tooltipHeight - padding);
    }

    setCoords({ top, left, placement });
  }, [isOpen, targetRect]);

  const isPositioned = coords.top !== -9999;
  if (!isOpen && !isPositioned) return null;

  const placementClass = coords.placement === "top" ? styles.bridgeTop : styles.bridgeBottom;

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
