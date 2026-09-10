import type { Accuracy } from "battler-types";

/**
 * Normalizes an arbitrary string into a lowercased alphanumeric identifier.
 */
export const toId = (str: string): string => str.toLowerCase().replace(/[^a-z0-9]/g, "");

/**
 * Formats base power directly, showing an em dash for 0 or omitted power (status/variable moves).
 */
export function formatBasePower(basePower?: number | null): string {
  return basePower && basePower > 0 ? String(basePower) : "—";
}

/**
 * Formats base accuracy directly as reported by the datastore.
 */
export function formatAccuracy(accuracy?: Accuracy | null): string {
  if (typeof accuracy === "number") {
    return `${accuracy}%`;
  }
  return "—";
}

/**
 * Formats move PP including max PP calculation when boosts apply.
 */
export function formatPp(pp?: number | null, noPpBoosts?: boolean): string {
  if (!pp) return "—";
  if (noPpBoosts) return String(pp);
  const maxPp = Math.floor(pp * 1.6);
  return maxPp === pp ? String(pp) : `${pp} (max ${maxPp})`;
}

/**
 * Formats move priority as a signed string (e.g. "+1", "-1") or null if zero/omitted.
 */
export function formatPriority(priority?: number | null): string | null {
  if (!priority) return null;
  return priority > 0 ? `+${priority}` : `${priority}`;
}

/**
 * Formats a species classification string, replacing "Pokémon" with "Mon".
 */
export function formatSpeciesClass(rawClass?: string | null): string {
  if (!rawClass) return "Mon";
  const cleaned = rawClass.replace(/\s*Pok[eé]mon/gi, "").trim();
  if (!cleaned) return "Mon";
  if (cleaned.toLowerCase().endsWith("mon")) return cleaned;
  return `${cleaned} Mon`;
}

export interface GenderRatioDisplay {
  type: "genderless" | "male-only" | "female-only" | "split";
  malePercent?: number;
  femalePercent?: number;
}

/**
 * Parses gender ratio value into display type and percentage breakdown.
 */
export function parseGenderRatio(ratio?: number | null): GenderRatioDisplay {
  if (ratio == null || ratio === 255 || ratio < 0) {
    return { type: "genderless" };
  }
  if (ratio === 0) {
    return { type: "male-only", malePercent: 100, femalePercent: 0 };
  }
  if (ratio === 254) {
    return { type: "female-only", malePercent: 0, femalePercent: 100 };
  }
  let femalePercent: number;
  if (ratio === 31) femalePercent = 12.5;
  else if (ratio === 63) femalePercent = 25;
  else if (ratio === 127) femalePercent = 50;
  else if (ratio === 191) femalePercent = 75;
  else if (ratio === 223) femalePercent = 87.5;
  else {
    femalePercent = Math.round((ratio / 252) * 1000) / 10;
  }
  const malePercent = Math.round((100 - femalePercent) * 10) / 10;
  return { type: "split", malePercent, femalePercent };
}

/**
 * Formats a tenths-based metric integer (e.g. hectograms for weight or decimeters for height)
 * into a decimal string with unit, or null if omitted or non-positive.
 */
export function formatDeciMetric(value?: number | null, unit: string = ""): string | null {
  if (value == null || value <= 0) return null;
  const converted = (value / 10).toFixed(1);
  return unit ? `${converted} ${unit}` : converted;
}

/**
 * Safely extracts the canonical name from a raw resource data object.
 */
export function extractResourceName(data: unknown): string | undefined {
  if (data && typeof data === "object" && "name" in data && typeof data.name === "string" && data.name) {
    return data.name;
  }
  return undefined;
}
