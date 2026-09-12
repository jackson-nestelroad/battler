import type { Fraction } from "battler-types";

/**
 * Normalizes a Fraction value into a clean, rounded percentage string (e.g. "10%", "50%", "100%").
 * Returns null if the fraction is missing or unparseable.
 */
export function formatFractionPercent(fraction?: Fraction | null | unknown): string | null {
  if (fraction == null) {
    return null;
  }

  if (typeof fraction === "number") {
    if (isNaN(fraction)) return null;
    if (fraction <= 0) return "0%";
    if (fraction <= 1) {
      return `${Math.round(fraction * 100)}%`;
    }
    return `${Math.round(fraction)}%`;
  }

  if (typeof fraction === "string") {
    const s = fraction.trim();
    if (!s) return null;

    // Matches "10%" or "33.3%"
    const percentMatch = s.match(/^(\d+(?:\.\d+)?)\s*%$/);
    if (percentMatch) {
      const val = parseFloat(percentMatch[1]);
      return isNaN(val) ? null : `${Math.round(val)}%`;
    }

    // Matches "1/10" or "1 / 2"
    const ratioMatch = s.match(/^(\d+(?:\.\d+)?)\s*\/\s*(\d+(?:\.\d+)?)$/);
    if (ratioMatch) {
      const num = parseFloat(ratioMatch[1]);
      const den = parseFloat(ratioMatch[2]);
      if (isNaN(num) || isNaN(den) || den === 0) return null;
      return `${Math.round((num / den) * 100)}%`;
    }

    // Matches plain numeric string "0.1" or "10"
    const numMatch = s.match(/^(\d+(?:\.\d+)?)$/);
    if (numMatch) {
      const val = parseFloat(numMatch[1]);
      if (isNaN(val)) return null;
      if (val <= 0) return "0%";
      if (val <= 1) return `${Math.round(val * 100)}%`;
      return `${Math.round(val)}%`;
    }
  }

  if (Array.isArray(fraction) && fraction.length === 2) {
    const num = Number(fraction[0]);
    const den = Number(fraction[1]);
    if (!isNaN(num) && !isNaN(den) && den !== 0) {
      return `${Math.round((num / den) * 100)}%`;
    }
  }

  return null;
}
