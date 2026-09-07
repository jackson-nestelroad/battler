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

  const clampVerticalCenter = () =>
    Math.max(
      padding,
      Math.min(
        targetRect.top + targetRect.height / 2 - tooltipHeight / 2,
        viewportHeight - tooltipHeight - padding,
      ),
    );

  if (placement === "left") {
    const roomOnLeft = targetRect.left - tooltipWidth - gap >= padding;
    if (roomOnLeft) {
      left = targetRect.left - tooltipWidth - gap;
      top = clampVerticalCenter();
    } else {
      placement = "top";
    }
  } else if (placement === "right") {
    const roomOnRight =
      viewportWidth - targetRect.right - tooltipWidth - gap >= padding;
    if (roomOnRight) {
      left = targetRect.right + gap;
      top = clampVerticalCenter();
    } else {
      placement = "top";
    }
  }

  if (placement === "top" || placement === "bottom") {
    // Center horizontally on target
    left = targetRect.left + targetRect.width / 2 - tooltipWidth / 2;
    // Clamp to viewport
    left = Math.max(padding, Math.min(left, viewportWidth - tooltipWidth - padding));

    if (placement === "bottom") {
      top = targetRect.bottom + gap;
      if (top + tooltipHeight > viewportHeight - padding) {
        const topCandidate = targetRect.top - tooltipHeight - gap;
        if (topCandidate >= padding) {
          top = topCandidate;
          placement = "top";
        } else {
          top = Math.max(padding, viewportHeight - tooltipHeight - padding);
        }
      }
    } else {
      top = targetRect.top - tooltipHeight - gap;
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
  }

  return { top, left, placement };
}
