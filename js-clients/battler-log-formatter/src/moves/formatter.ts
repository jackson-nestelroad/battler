import type { BoostTable, HitEffect, MoveData } from "battler-types";
import i18next from "../i18n.js";
import { formatFractionPercent } from "./fraction.js";
import type { FormattedMoveEffect, MoveEffectSubject } from "./types.js";

const STAT_ORDER: Record<string, number> = {
  hp: 0,
  atk: 1,
  def: 2,
  spa: 3,
  spd: 4,
  spe: 5,
  acc: 6,
  eva: 7,
};

const CORE_STATS = ["atk", "def", "spa", "spd", "spe"];

function capitalize(s: string): string {
  if (!s) return s;
  return s.charAt(0).toUpperCase() + s.slice(1);
}

function joinConjunction(items: string[]): string {
  if (items.length === 0) return "";
  if (items.length === 1) return items[0];
  if (items.length === 2) return `${items[0]} and ${items[1]}`;
  return `${items.slice(0, -1).join(", ")}, and ${items[items.length - 1]}`;
}

function resolveHitEffectSubject(target?: string | null): MoveEffectSubject {
  if (target === "User") {
    return "user";
  }
  if (
    target === "Allies" ||
    target === "AdjacentAlly" ||
    target === "AllyTeam" ||
    target === "AdjacentAllyOrUser"
  ) {
    return "allies";
  }
  return "target";
}

function formatStatusEffect(
  status: string,
  subject: MoveEffectSubject,
  chance?: string,
): FormattedMoveEffect | null {
  const normStatus = status.toLowerCase();
  const subKey = subject === "user" ? "user" : "target";

  const actionKey = chance
    ? `move_effects.status.infinitive.${subKey}.${normStatus}`
    : `move_effects.status.declarative.${subKey}.${normStatus}`;

  const exists = i18next.exists(actionKey);
  const action = exists
    ? i18next.t(actionKey)
    : chance
      ? `inflict ${normStatus} on the ${subject}`
      : `Inflicts ${normStatus} on the ${subject}`;

  const text = chance
    ? i18next.t("move_effects.chance_clause", { chance, action })
    : capitalize(i18next.t("move_effects.declarative_clause", { action }));

  return {
    text,
    chance,
    type: "status",
    subject,
  };
}

function formatHealEffect(
  healPercent: unknown,
  subject: MoveEffectSubject,
  chance?: string,
): FormattedMoveEffect | null {
  const percentStr = formatFractionPercent(healPercent);
  if (!percentStr) return null;

  const actionKey = chance
    ? `move_effects.heal.infinitive.${subject}`
    : `move_effects.heal.declarative.${subject}`;

  const action = i18next.t(actionKey, { percent: percentStr });

  const text = chance
    ? i18next.t("move_effects.chance_clause", { chance, action })
    : capitalize(i18next.t("move_effects.declarative_clause", { action }));

  return {
    text,
    chance,
    type: "heal",
    subject,
  };
}

function formatBoostEffects(
  boosts: BoostTable,
  subject: MoveEffectSubject,
  chance?: string,
): FormattedMoveEffect[] {
  // Extract non-zero entries
  const entries = Object.entries(boosts)
    .filter(([_, val]) => typeof val === "number" && val !== 0)
    .sort(([a], [b]) => (STAT_ORDER[a] ?? 99) - (STAT_ORDER[b] ?? 99));

  if (entries.length === 0) return [];

  // Group by stage delta
  const deltaGroups = new Map<number, string[]>();
  for (const [stat, delta] of entries) {
    const list = deltaGroups.get(delta) || [];
    list.push(stat);
    deltaGroups.set(delta, list);
  }

  // Sort groups: positive deltas descending, then negative deltas ascending
  const sortedDeltas = Array.from(deltaGroups.keys()).sort((a, b) => {
    if (a > 0 && b > 0) return b - a;
    if (a < 0 && b < 0) return a - b;
    return b - a;
  });

  const results: FormattedMoveEffect[] = [];

  for (const delta of sortedDeltas) {
    const statKeys = deltaGroups.get(delta)!;
    const direction = delta > 0 ? "raise" : "lower";
    const absCount = Math.abs(delta);
    const stages = i18next.t("move_effects.stage", { count: absCount });

    // Check for "all stats" condition (atk, def, spa, spd, spe all present)
    const isAllCoreStats =
      delta > 0 &&
      CORE_STATS.every((s) => statKeys.includes(s)) &&
      statKeys.length >= CORE_STATS.length;

    const subjectStr = i18next.t(`move_effects.subjects.${subject}`, {
      defaultValue: subject,
    });

    let action: string;
    if (isAllCoreStats) {
      const templateKey = chance
        ? `move_effects.boost.infinitive_${direction}_all`
        : `move_effects.boost.declarative_${direction}_all`;
      action = i18next.t(templateKey, {
        subject: subjectStr,
        stages,
      });
    } else {
      const statNames = statKeys.map((k) =>
        i18next.t(`move_effects.stats.${k}`, { defaultValue: k.toUpperCase() }),
      );
      const statsStr = joinConjunction(statNames);
      const templateKey = chance
        ? `move_effects.boost.infinitive_${direction}`
        : `move_effects.boost.declarative_${direction}`;
      action = i18next.t(templateKey, {
        subject: subjectStr,
        stats: statsStr,
        stages,
      });
    }

    const text = chance
      ? i18next.t("move_effects.chance_clause", { chance, action })
      : capitalize(i18next.t("move_effects.declarative_clause", { action }));

    results.push({
      text,
      chance,
      type: "boost",
      subject,
    });
  }

  return results;
}

function formatVolatileStatusEffect(
  volatileStatus: string,
  subject: MoveEffectSubject,
  chance?: string,
): FormattedMoveEffect | null {
  const norm = volatileStatus.toLowerCase();
  // Specifically support confusion and flinch
  if (norm !== "confusion" && norm !== "flinch") {
    return null;
  }

  const subKey = subject === "user" ? "user" : "target";
  const actionKey = chance
    ? `move_effects.volatile_status.infinitive.${subKey}.${norm}`
    : `move_effects.volatile_status.declarative.${subKey}.${norm}`;

  const exists = i18next.exists(actionKey);
  if (!exists) return null;

  const action = i18next.t(actionKey);
  const text = chance
    ? i18next.t("move_effects.chance_clause", { chance, action })
    : capitalize(i18next.t("move_effects.declarative_clause", { action }));

  return {
    text,
    chance,
    type: "volatile_status",
    subject,
  };
}

function processHitEffect(
  effect: HitEffect | null | undefined,
  subject: MoveEffectSubject,
  chance?: string,
): FormattedMoveEffect[] {
  if (!effect) return [];
  const results: FormattedMoveEffect[] = [];

  // 1. Status effects
  if (effect.status) {
    const res = formatStatusEffect(effect.status, subject, chance);
    if (res) results.push(res);
  }

  // 2. Specific volatile statuses (confusion, flinch)
  if (effect.volatile_status) {
    const res = formatVolatileStatusEffect(effect.volatile_status, subject, chance);
    if (res) results.push(res);
  }

  // 3. Stat boosts / drops
  if (effect.boosts) {
    results.push(...formatBoostEffects(effect.boosts, subject, chance));
  }

  // 4. HP healing
  if (effect.heal_percent) {
    const res = formatHealEffect(effect.heal_percent, subject, chance);
    if (res) results.push(res);
  }

  return results;
}

/**
 * Formats all primary and secondary effects for a MoveData into human-readable,
 * localized sentences.
 *
 * Rules:
 * 1. hit_effect: Never shows chance (guaranteed on hit).
 * 2. user_effect: Only shows chance if user_effect_chance is defined.
 * 3. secondary_effects: ALWAYS shows chance, defaulting to 100% if omitted.
 */
export function formatMoveEffects(move: MoveData): FormattedMoveEffect[] {
  const effects: FormattedMoveEffect[] = [];

  // 1. Primary hit_effect on target/allies (NO chance)
  if (move.hit_effect) {
    const subject = resolveHitEffectSubject(move.target);
    effects.push(...processHitEffect(move.hit_effect, subject, undefined));
  }

  // 2. Primary user_effect on user (chance ONLY if user_effect_chance is defined)
  if (move.user_effect) {
    const userChance = move.user_effect_chance
      ? formatFractionPercent(move.user_effect_chance) ?? undefined
      : undefined;
    effects.push(...processHitEffect(move.user_effect, "user", userChance));
  }

  // 3. Secondary effects (ALWAYS show chance %, defaults to 100% if omitted)
  if (move.secondary_effects && move.secondary_effects.length > 0) {
    for (const sec of move.secondary_effects) {
      const chance = sec.chance ? formatFractionPercent(sec.chance) ?? "100%" : "100%";

      if (sec.target) {
        effects.push(...processHitEffect(sec.target, "target", chance));
      }

      if (sec.user) {
        effects.push(...processHitEffect(sec.user, "user", chance));
      }
    }
  }

  return effects;
}
