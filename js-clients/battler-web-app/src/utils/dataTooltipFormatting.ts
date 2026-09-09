import type {
  Accuracy,
  Fraction,
  HitEffect,
  MoveFlag,
  MoveTarget,
  MultihitType,
  RecoilData,
  SecondaryEffectData,
} from "battler-types";

/**
 * Format base accuracy directly as reported by the datastore.
 */
export function formatAccuracy(accuracy?: Accuracy | null): string {
  if (typeof accuracy === "number") {
    return `${accuracy}%`;
  }
  return "—";
}

/**
 * Returns MoveTarget as-is directly from the datastore.
 */
export function formatMoveTarget(target: MoveTarget): string {
  return target;
}

/**
 * Formats a Fraction string/number to a readable percentage or fraction string.
 */
export function formatFraction(fraction?: Fraction | null): string {
  if (fraction == null) return "";
  if (typeof fraction === "number") {
    return `${Math.round(fraction * 100)}%`;
  }
  if (typeof fraction === "string") {
    if (fraction.endsWith("%")) return fraction;
    if (fraction.includes("/")) {
      const parts = fraction.split("/").map((p) => Number.parseFloat(p));
      if (parts.length === 2 && !Number.isNaN(parts[0]) && !Number.isNaN(parts[1]) && parts[1] !== 0) {
        return `${Math.round((parts[0] / parts[1]) * 100)}%`;
      }
    }
    const parsed = Number.parseFloat(fraction);
    if (!Number.isNaN(parsed)) {
      return `${Math.round(parsed * 100)}%`;
    }
    return fraction;
  }
  return String(fraction);
}

/**
 * Formats multihit count directly.
 */
export function formatMultihit(multihit?: MultihitType | null): string | null {
  if (multihit == null) return null;
  if (typeof multihit === "number") {
    return `Hits ${multihit}`;
  }
  if (Array.isArray(multihit) && multihit.length === 2) {
    return `Hits ${multihit[0]}–${multihit[1]}`;
  }
  return null;
}

/**
 * Formats recoil data directly.
 */
export function formatRecoil(recoil?: RecoilData | null): string | null {
  if (!recoil) return null;
  if (recoil.struggle) {
    return "25% struggle recoil";
  }
  const pct = formatFraction(recoil.percent);
  return pct ? `${pct} recoil` : "Recoil";
}

/**
 * Formats drain percentage directly.
 */
export function formatDrain(drain?: Fraction | null): string | null {
  if (!drain) return null;
  const pct = formatFraction(drain);
  return pct ? `Recovers ${pct}` : null;
}

/**
 * Returns move flags as-is directly from datastore, sorted for consistency.
 */
export function formatMoveFlags(flags?: Iterable<MoveFlag> | null): string[] {
  if (!flags) return [];
  return Array.from(flags).sort();
}

/**
 * Formats stat boost table into concise, direct strings grouped by stage.
 * E.g. "+1 Atk, Def, SpA, SpD, Spe (user)" or "-1 Spe (target)"
 */
export function formatBoosts(boosts?: HitEffect["boosts"] | null, targetLabel?: string): string[] {
  if (!boosts) return [];
  const entries: [string, number | undefined][] = [
    ["Atk", boosts.atk],
    ["Def", boosts.def],
    ["SpA", boosts.spa],
    ["SpD", boosts.spd],
    ["Spe", boosts.spe],
    ["Acc", boosts.acc],
    ["Eva", boosts.eva],
  ];

  const stageMap = new Map<number, string[]>();
  for (const [stat, val] of entries) {
    if (typeof val === "number" && val !== 0) {
      const list = stageMap.get(val) || [];
      list.push(stat);
      stageMap.set(val, list);
    }
  }

  const result: string[] = [];
  for (const [stage, stats] of stageMap.entries()) {
    const sign = stage > 0 ? `+${stage}` : `${stage}`;
    const targetSuffix = targetLabel ? ` (${targetLabel})` : "";
    result.push(`${sign} ${stats.join(", ")}${targetSuffix}`);
  }
  return result;
}

/**
 * Formats a HitEffect into direct, concise descriptions.
 */
export function formatHitEffect(effect: HitEffect | null, targetLabel?: string): string[] {
  if (!effect) return [];
  const lines: string[] = [];
  const targetSuffix = targetLabel ? ` (${targetLabel})` : "";

  if (effect.status) {
    lines.push(`${effect.status}${targetSuffix}`);
  }
  if (effect.volatile_status) {
    lines.push(`${effect.volatile_status}${targetSuffix}`);
  }
  if (effect.boosts) {
    lines.push(...formatBoosts(effect.boosts, targetLabel));
  }
  if (effect.force_switch) {
    lines.push(`Forces switch${targetSuffix}`);
  }
  if (effect.heal_percent) {
    lines.push(`Heals ${formatFraction(effect.heal_percent)}${targetSuffix}`);
  }
  if (effect.weather) {
    lines.push(`Weather: ${effect.weather}`);
  }
  if (effect.terrain) {
    lines.push(`Terrain: ${effect.terrain}`);
  }
  if (effect.side_condition) {
    lines.push(`Side condition: ${effect.side_condition}`);
  }
  if (effect.slot_condition) {
    lines.push(`Slot condition: ${effect.slot_condition}`);
  }

  return lines;
}

/**
 * Formats a secondary effect directly.
 */
export function formatSecondaryEffect(sec: SecondaryEffectData): string | null {
  const parts: string[] = [];
  if (sec.target) {
    parts.push(...formatHitEffect(sec.target, "target"));
  }
  if (sec.user) {
    parts.push(...formatHitEffect(sec.user, "user"));
  }
  if (parts.length === 0) return null;

  const effectDesc = parts.join(", ");
  if (sec.chance != null) {
    const chancePct = formatFraction(sec.chance);
    if (chancePct && chancePct !== "100%") {
      return `${chancePct}: ${effectDesc}`;
    }
  }
  return effectDesc;
}
