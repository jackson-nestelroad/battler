import type { ItemData, MoveData } from "battler-types";
import type { ResourceType } from "../../../hooks/useDataStore";

function isPrimitive(v: unknown): boolean {
  return v === null || typeof v === "boolean" || typeof v === "number" || typeof v === "string";
}

/**
 * Strips null, undefined, false, and 0-value boosts from structured data before rendering.
 */
export function cleanJsonData(val: unknown): unknown {
  if (val === null || val === undefined || val === false) return undefined;
  if (Array.isArray(val)) {
    const cleanedArr = val.map(cleanJsonData).filter((item) => item !== undefined);
    return cleanedArr.length > 0 ? cleanedArr : undefined;
  }
  if (typeof val === "object") {
    const res: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(val as Record<string, unknown>)) {
      if ((k === "boosts" || k === "boost") && typeof v === "object" && v !== null) {
        const activeBoosts: Record<string, number> = {};
        for (const [stat, b] of Object.entries(v as Record<string, number>)) {
          if (b !== 0) activeBoosts[stat] = b;
        }
        if (Object.keys(activeBoosts).length > 0) {
          res[k] = activeBoosts;
        }
        continue;
      }
      const cleaned = cleanJsonData(v);
      if (cleaned !== undefined) {
        if (typeof cleaned === "object" && cleaned !== null && Object.keys(cleaned).length === 0) {
          continue;
        }
        res[k] = cleaned;
      }
    }
    return Object.keys(res).length > 0 ? res : undefined;
  }
  return val;
}

/**
 * Formats JSON AST and effects data matching battle-data conventions:
 * - Single-element arrays (e.g. ["cure_status: $mon"]) stay on one line with their brackets.
 * - Multi-statement arrays/callbacks remain multiline (one line per statement).
 * - Short primitive single-property objects collapse onto a single line.
 */
export function formatCompactJson(value: unknown, indent = 2, currentIndent = 0): string {
  const spaces = " ".repeat(currentIndent);
  const nextSpaces = " ".repeat(currentIndent + indent);

  if (value === null) return "null";
  if (typeof value === "boolean" || typeof value === "number") return String(value);
  if (typeof value === "string") return JSON.stringify(value);

  if (Array.isArray(value)) {
    if (value.length === 0) return "[]";

    // Single-item array: collapse onto single line with its brackets if it fits (e.g. ["cure_status: $mon"])
    if (value.length === 1) {
      const inner = formatCompactJson(value[0], indent, 0);
      if (!inner.includes("\n") && inner.length <= 80) {
        return `[${inner}]`;
      }
    } else {
      // Multiple items: only collapse if ALL items are short primitive identifiers without spaces (e.g. short flags)
      const allShortPrimitivesNoSpace = value.every(
        (v) =>
          isPrimitive(v) &&
          typeof v !== "object" &&
          !String(v).includes(" ") &&
          String(v).length <= 20,
      );
      if (allShortPrimitivesNoSpace) {
        const singleLine = `[${value.map((v) => formatCompactJson(v, indent, 0)).join(", ")}]`;
        if (singleLine.length <= 50 && !singleLine.includes("\n")) {
          return singleLine;
        }
      }
    }

    const items = value.map(
      (v) => nextSpaces + formatCompactJson(v, indent, currentIndent + indent),
    );
    return `[\n${items.join(",\n")}\n${spaces}]`;
  }

  if (typeof value === "object") {
    const entries = Object.entries(value as Record<string, unknown>);
    if (entries.length === 0) return "{}";

    // Single-property object: collapse onto single line if short primitive (e.g. { "status": "par" })
    if (entries.length === 1) {
      const [k, v] = entries[0];
      if (isPrimitive(v)) {
        const singleLine = `{ ${JSON.stringify(k)}: ${formatCompactJson(v, indent, 0)} }`;
        if (singleLine.length <= 50) {
          return singleLine;
        }
      }
    }

    const lines = entries.map(([k, v]) => {
      const formattedVal = formatCompactJson(v, indent, currentIndent + indent);
      return `${nextSpaces}${JSON.stringify(k)}: ${formattedVal}`;
    });
    return `{\n${lines.join(",\n")}\n${spaces}}`;
  }

  return JSON.stringify(value);
}

/**
 * Wraps any delegate or definition reference of the generic form "prefix:id",
 * or any HitEffect condition field referencing a condition resource, in
 * interactive span elements that can be clicked to navigate definitions.
 */
export function linkifyDelegates(html: string): string {
  // 1. Standard delegate format "prefix:id"
  let result = html.replace(/"([a-zA-Z0-9_-]+):([a-zA-Z0-9_-]+)"/g, (_, prefix, name) => {
    const fullDelegate = `${prefix}:${name}`;
    return `"<span role="button" tabindex="0" class="fxlang-delegate-link" data-delegate-prefix="${prefix}" data-delegate-name="${name}" title="View definition of ${fullDelegate}">${fullDelegate}</span>"`;
  });

  // 2. HitEffect condition fields pointing to condition resources:
  // status, volatile_status, side_condition, slot_condition, weather, pseudo_weather, terrain
  const HIT_EFFECT_FIELD_REGEX =
    /"(status|volatile_status|side_condition|slot_condition|weather|pseudo_weather|terrain)"(\s*(?:<[^>]+>\s*)*:\s*(?:<[^>]+>\s*)*)"([a-zA-Z0-9_-]+)"/g;

  result = result.replace(HIT_EFFECT_FIELD_REGEX, (_, field, middle, name) => {
    return `"${field}"${middle}"<span role="button" tabindex="0" class="fxlang-delegate-link" data-delegate-prefix="condition" data-delegate-name="${name}" title="View definition of condition:${name}">${name}</span>"`;
  });

  return result;
}

/**
 * Maps a delegate prefix (e.g. "condition", "ability", "abilitycondition", "item", "move", "species")
 * to the underlying ecosystem ResourceType to query from the data service.
 */
export function resolveDelegateTarget(
  prefix: string,
  name: string,
): { type: ResourceType; name: string } {
  const lower = prefix.toLowerCase();
  if (lower.startsWith("ability")) return { type: "ability", name };
  if (lower.startsWith("move")) return { type: "move", name };
  if (lower.startsWith("item")) return { type: "item", name };
  if (lower.startsWith("species")) return { type: "species", name };
  return { type: "condition", name };
}

/**
 * Extracts and cleans fxlang AST data (`effect` and `condition`) from any resource payload.
 */
export function extractFxlangData(data: unknown): Record<string, unknown> | undefined {
  if (!data || typeof data !== "object") return undefined;
  const res = data as Record<string, unknown>;
  const fx: Record<string, unknown> = {};
  if (res.effect && typeof res.effect === "object" && Object.keys(res.effect).length > 0) {
    fx.effect = res.effect;
  }
  if (res.condition && typeof res.condition === "object" && Object.keys(res.condition).length > 0) {
    fx.condition = res.condition;
  }
  return Object.keys(fx).length > 0 ? (cleanJsonData(fx) as Record<string, unknown>) : undefined;
}

/**
 * Extracts and cleans structured move effects and mechanics data.
 * followed by the core hit/user/secondary effect pipeline.
 */
export function extractMoveEffects(move: MoveData): Record<string, unknown> | undefined {
  const effectsObj: Record<string, unknown> = {};

  // 1. General mechanics (alphabetical)
  if (move.advanced_targeting != null) effectsObj.advanced_targeting = move.advanced_targeting;
  if (move.crit_ratio != null && move.crit_ratio !== 1) effectsObj.crit_ratio = move.crit_ratio;
  if (move.damage != null) effectsObj.damage = move.damage;
  if (move.drain_percent != null) effectsObj.drain_percent = move.drain_percent;
  if (move.force_stab) effectsObj.force_stab = move.force_stab;
  if (move.ignore_accuracy) effectsObj.ignore_accuracy = move.ignore_accuracy;
  if (move.ignore_defensive) effectsObj.ignore_defensive = move.ignore_defensive;
  if (move.ignore_evasion) effectsObj.ignore_evasion = move.ignore_evasion;
  if (move.ignore_offensive) effectsObj.ignore_offensive = move.ignore_offensive;
  if (move.multiaccuracy) effectsObj.multiaccuracy = move.multiaccuracy;
  if (move.multihit != null) effectsObj.multihit = move.multihit;
  if (move.ohko_type != null) effectsObj.ohko_type = move.ohko_type;
  if (move.override_defensive_mon != null)
    effectsObj.override_defensive_mon = move.override_defensive_mon;
  if (move.override_defensive_stat != null)
    effectsObj.override_defensive_stat = move.override_defensive_stat;
  if (move.override_offensive_mon != null)
    effectsObj.override_offensive_mon = move.override_offensive_mon;
  if (move.override_offensive_stat != null)
    effectsObj.override_offensive_stat = move.override_offensive_stat;
  if (move.recoil != null) effectsObj.recoil = move.recoil;
  if (move.self_destruct != null) effectsObj.self_destruct = move.self_destruct;
  if (move.user_switch != null) effectsObj.user_switch = move.user_switch;
  if (move.will_crit) effectsObj.will_crit = move.will_crit;

  // 2. Effect pipeline (hit => user => user chance => secondaries)
  if (move.hit_effect != null) effectsObj.hit_effect = move.hit_effect;
  if (move.user_effect != null) effectsObj.user_effect = move.user_effect;
  if (move.user_effect_chance != null) effectsObj.user_effect_chance = move.user_effect_chance;
  if (move.secondary_effects && move.secondary_effects.length > 0) {
    effectsObj.secondary_effects = move.secondary_effects;
  }

  return cleanJsonData(effectsObj) as Record<string, unknown> | undefined;
}

/**
 * Extracts and cleans special item data, including special effects, forme changes, and player usage mechanics.
 */
export function extractItemSpecial(item: ItemData): Record<string, unknown> | undefined {
  const result: Record<string, unknown> = {};

  if (item.force_forme) result.force_forme = item.force_forme;
  if (item.target) result.target = item.target;
  if (item.input) result.input = item.input;

  if (item.special_data && typeof item.special_data === "object") {
    const cleaned = cleanJsonData(item.special_data) as Record<string, unknown> | undefined;
    if (cleaned && Object.keys(cleaned).length > 0) {
      result.special_data = cleaned;
    }
  }

  return Object.keys(result).length > 0 ? result : undefined;
}
