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

  const spaceAbove = targetRect.top - gap - padding;
  const spaceBelow = viewportHeight - targetRect.bottom - gap - padding;
  const spaceLeft = targetRect.left - gap - padding;
  const spaceRight = viewportWidth - targetRect.right - gap - padding;

  const fitsTop = spaceAbove >= tooltipHeight;
  const fitsBottom = spaceBelow >= tooltipHeight;
  const fitsLeft = spaceLeft >= tooltipWidth;
  const fitsRight = spaceRight >= tooltipWidth;

  const clampVerticalCenter = () =>
    Math.max(
      padding,
      Math.min(
        targetRect.top + targetRect.height / 2 - tooltipHeight / 2,
        viewportHeight - tooltipHeight - padding,
      ),
    );

  const clampHorizontalCenter = () =>
    Math.max(
      padding,
      Math.min(
        targetRect.left + targetRect.width / 2 - tooltipWidth / 2,
        viewportWidth - tooltipWidth - padding,
      ),
    );

  // Candidate evaluation order based on preferredPlacement:
  // Try primary placement, then opposite, then orthogonal directions (ordered by available space).
  const getCandidateOrder = (): Array<"top" | "bottom" | "left" | "right"> => {
    switch (preferredPlacement) {
      case "top":
        return [
          "top",
          "bottom",
          spaceLeft >= spaceRight ? "left" : "right",
          spaceLeft >= spaceRight ? "right" : "left",
        ];
      case "bottom":
        return [
          "bottom",
          "top",
          spaceLeft >= spaceRight ? "left" : "right",
          spaceLeft >= spaceRight ? "right" : "left",
        ];
      case "left":
        return [
          "left",
          spaceAbove >= spaceBelow ? "top" : "bottom",
          spaceAbove >= spaceBelow ? "bottom" : "top",
          "right",
        ];
      case "right":
        return [
          "right",
          spaceAbove >= spaceBelow ? "top" : "bottom",
          spaceAbove >= spaceBelow ? "bottom" : "top",
          "left",
        ];
    }
  };

  const fitsMap: Record<"top" | "bottom" | "left" | "right", boolean> = {
    top: fitsTop,
    bottom: fitsBottom,
    left: fitsLeft,
    right: fitsRight,
  };

  const candidateOrder = getCandidateOrder();
  // Find the first placement candidate that fits along its primary axis without occluding the target
  let chosenPlacement = candidateOrder.find((p) => fitsMap[p]);

  // If none of the 4 placements fit completely, pick the best candidate
  if (!chosenPlacement) {
    const bestVertical = spaceAbove >= spaceBelow ? "top" : "bottom";
    const bestHorizontal = spaceLeft >= spaceRight ? "left" : "right";
    const verticalSpace = Math.max(spaceAbove, spaceBelow);
    const horizontalSpace = Math.max(spaceLeft, spaceRight);

    if (preferredPlacement === "top" || preferredPlacement === "bottom") {
      chosenPlacement =
        horizontalSpace > verticalSpace && horizontalSpace >= tooltipWidth * 0.75
          ? bestHorizontal
          : bestVertical;
    } else {
      chosenPlacement =
        verticalSpace > horizontalSpace && verticalSpace >= tooltipHeight * 0.75
          ? bestVertical
          : bestHorizontal;
    }
  }

  let top = 0;
  let left = 0;

  if (chosenPlacement === "top") {
    left = clampHorizontalCenter();
    top = targetRect.top - tooltipHeight - gap;
  } else if (chosenPlacement === "bottom") {
    left = clampHorizontalCenter();
    top = targetRect.bottom + gap;
  } else if (chosenPlacement === "left") {
    left = targetRect.left - tooltipWidth - gap;
    top = clampVerticalCenter();
  } else if (chosenPlacement === "right") {
    left = targetRect.right + gap;
    top = clampVerticalCenter();
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

  return { top, left, placement: chosenPlacement };
}
