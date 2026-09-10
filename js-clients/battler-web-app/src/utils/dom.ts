/**
 * Resolves an event target to an Element, resolving text/comment nodes to their parent element.
 */
export function toElement(target: EventTarget | null): Element | null {
  if (!target) return null;
  if ("nodeType" in target && typeof (target as Node).nodeType === "number") {
    const node = target as Node;
    return node.nodeType === 1 ? (node as Element) : node.parentElement;
  }
  if (typeof Element !== "undefined" && target instanceof Element) {
    return target;
  }
  if (typeof Node !== "undefined" && target instanceof Node) {
    return target.parentElement;
  }
  if ("closest" in target && typeof (target as { closest?: unknown }).closest === "function") {
    return target as unknown as Element;
  }
  return null;
}

/**
 * Checks whether an event target is contained within an active dialog overlay or modal.
 */
export function isTargetInsideModal(target: EventTarget | null): boolean {
  const element = toElement(target);
  return Boolean(element?.closest?.('dialog, [role="dialog"], [aria-modal="true"]'));
}

