export interface FloatingCoordsResult {
  top: number;
  left: number;
  placement: "top" | "bottom" | "left" | "right";
}

/**
 * Safely resolves the bounding client rect of an element, falling back to its
 * first child element if the container has 0 width and 0 height (e.g. inline wrappers).
 */
export function getElementRect(el: HTMLElement): DOMRect {
  let rect = el.getBoundingClientRect();
  if (rect.width === 0 && rect.height === 0 && el.firstElementChild) {
    rect = (el.firstElementChild as HTMLElement).getBoundingClientRect();
  }
  return rect;
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

    const spaceAbove = targetRect.top - gap - padding;
    const spaceBelow = viewportHeight - targetRect.bottom - gap - padding;

    if (placement === "top") {
      placement =
        spaceAbove >= tooltipHeight || spaceAbove >= spaceBelow ? "top" : "bottom";
    } else {
      placement =
        spaceBelow >= tooltipHeight || spaceBelow >= spaceAbove ? "bottom" : "top";
    }

    top =
      placement === "top"
        ? targetRect.top - tooltipHeight - gap
        : targetRect.bottom + gap;
  }

  // Strict boundary enforcement:
  // Tooltip must NEVER go below the bottom of the viewport or above the top
  if (top + tooltipHeight > viewportHeight - padding) {
    top = Math.max(padding, viewportHeight - tooltipHeight - padding);
  }
  if (top < padding) {
    top = padding;
  }

  // Tooltip must NEVER go off the right or left of the viewport
  if (left + tooltipWidth > viewportWidth - padding) {
    left = Math.max(padding, viewportWidth - tooltipWidth - padding);
  }
  if (left < padding) {
    left = padding;
  }

  return { top, left, placement };
}
