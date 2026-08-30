import type { BattleState, EffectData, UiLogEntry, UiMon } from "battler-state";
import i18next from "i18next";
import rules from "./config/mapper-rules.json" with { type: "json" };
import type {
  AnyMappedLog,
  ContextValue,
  ContextVar,
  LogContext,
  MappedLogMetadata,
  MapperOptions,
} from "./types.js";
import { LogCategory } from "./types.js";

export const ALLOWED_CONTEXT_VARS = rules.allowedContextVars;

export type Relationship = "self" | "ally" | "foe";

export function getRelationship(
  state: BattleState | undefined,
  localPlayerId: string | undefined,
  targetPlayerId: string,
): Relationship {
  if (!localPlayerId) return "foe";
  if (localPlayerId === targetPlayerId) return "self";

  if (state?.field?.sides) {
    let localSideIndex = -1;
    let targetSideIndex = -1;

    for (let i = 0; i < state.field.sides.length; i++) {
      const side = state.field.sides[i];
      if (side?.players?.[localPlayerId]) localSideIndex = i;
      if (side?.players?.[targetPlayerId]) targetSideIndex = i;
    }

    if (localSideIndex !== -1 && localSideIndex === targetSideIndex) {
      return "ally";
    }
  }

  return "foe";
}

export function getSideRelationship(
  state: BattleState | undefined,
  localPlayerId: string | undefined,
  sideIndex: number,
): Relationship {
  if (!localPlayerId) return "foe";
  if (state?.field?.sides?.[sideIndex]?.players?.[localPlayerId]) {
    return "self";
  }
  return "foe";
}

export function resolvePlayerContext(
  playerId: string | undefined,
  state: BattleState | undefined,
  options: MapperOptions,
): { standard: ContextVar; possessive: ContextVar } {
  let name = playerId || "Player";
  if (playerId && state?.field?.sides) {
    for (const side of state.field.sides) {
      if (side?.players?.[playerId]) {
        name = side.players[playerId].name;
        break;
      }
    }
  }

  if (!playerId) {
    return {
      standard: { text: name },
      possessive: { text: `${name}'s` },
    };
  }

  const rel = getRelationship(state, options.localPlayerId, playerId);

  let text = "";
  let possessiveText = "";

  if (rel === "self") {
    text = i18next.t("player.self");
    possessiveText = i18next.t("player.self_possessive");
  } else if (rel === "ally") {
    text = i18next.t("player.ally", { player: name });
    possessiveText = i18next.t("player.ally_possessive", { player: name });
  } else {
    text = i18next.t("player.foe", { player: name });
    possessiveText = i18next.t("player.foe_possessive", { player: name });
  }

  return {
    standard: { text, noAutoCapitalize: rel === "self" },
    possessive: { text: possessiveText, noAutoCapitalize: rel === "self" },
  };
}

export function resolveSideContext(
  sideIndex: number | undefined,
  state: BattleState | undefined,
  options: MapperOptions,
): { standard: ContextVar; possessive: ContextVar } {
  if (sideIndex == null) return { standard: { text: "Side" }, possessive: { text: "Side's" } };

  let name = `Player ${sideIndex}`;
  if (state?.field?.sides?.[sideIndex]) {
    name = state.field.sides[sideIndex].name;
  }

  const rel = getSideRelationship(state, options.localPlayerId, sideIndex);

  let text = "";
  let possessiveText = "";

  if (rel === "self") {
    text = i18next.t("side.self");
    possessiveText = i18next.t("side.self_possessive");
  } else {
    text = i18next.t("side.foe");
    possessiveText = i18next.t("side.foe_possessive");
  }

  return {
    standard: { text, noAutoCapitalize: true },
    possessive: { text: possessiveText, noAutoCapitalize: true },
  };
}

export function resolveMonContext(
  monRef: UiMon | undefined,
  state: BattleState | undefined,
  options: MapperOptions,
): {
  standard: ContextVar;
  possessive: ContextVar;
  raw: string;
  raw_possessive: string;
  ref?: UiMon;
  rel: Relationship;
} {
  if (!monRef || typeof monRef !== "object") {
    return {
      standard: { text: "Mon" },
      possessive: { text: "Mon's" },
      raw: "Mon",
      raw_possessive: "Mon's",
      rel: "foe",
    };
  }

  let name = "Mon";
  let playerId = "";

  if ("Active" in monRef && monRef.Active) {
    if (monRef.Active.name) name = monRef.Active.name;
    if (monRef.Active.player) playerId = monRef.Active.player;
  } else if ("Inactive" in monRef && monRef.Inactive) {
    if (monRef.Inactive.name) name = monRef.Inactive.name;
    if (monRef.Inactive.player) playerId = monRef.Inactive.player;
  }

  const rel = playerId ? getRelationship(state, options.localPlayerId, playerId) : "foe";
  const playerName = resolvePlayerContext(playerId, state, options).standard.text;

  let text = name;
  let possessiveText = `${name}'s`;
  let noAutoCapitalize = false;
  let possessiveNoAutoCapitalize = false;

  if (rel === "self") {
    text = i18next.t("mon.self", { name });
    possessiveText = i18next.t("mon.self_possessive", { name });
  } else if (rel === "ally") {
    text = i18next.t("mon.ally", { name });
    possessiveText = i18next.t("mon.ally_possessive", { name });
  } else {
    const isMulti = state?.battle_type === "Multi";
    if (isMulti) {
      text = i18next.t("mon.foe_multi", { name, player: playerName });
      possessiveText = i18next.t("mon.foe_possessive_multi", { name, player: playerName });
      noAutoCapitalize = true;
      possessiveNoAutoCapitalize = true;
    } else {
      text = i18next.t("mon.foe_single", { name });
      possessiveText = i18next.t("mon.foe_possessive_single", { name });
    }
  }

  return {
    standard: { text, monRef, noAutoCapitalize },
    possessive: { text: possessiveText, monRef, noAutoCapitalize: possessiveNoAutoCapitalize },
    raw: name,
    raw_possessive: `${name}'s`,
    ref: monRef,
    rel,
  };
}
function buildPattern(title: string, tags: string[], flags: string[]): string {
  const sortedTags = [...tags].sort();
  const sortedFlags = [...flags].sort();
  return [title, ...sortedTags, ...sortedFlags].join("|");
}

export function generateCombinatorics(
  title: string,
  baseTags: string[],
  baseFlags: string[],
): string[] {
  const results: Set<string> = new Set();
  const allDimensions = [...baseTags, ...baseFlags];

  function recurse(index: number, currentDimensions: string[]) {
    if (index === allDimensions.length) {
      const tags = currentDimensions.filter((d) => d.includes(":"));
      const flags = currentDimensions.filter((d) => !d.includes(":"));
      results.add(buildPattern(title, tags, flags));
      return;
    }

    const dim = allDimensions[index];
    
    // 1. Include specific dimension
    recurse(index + 1, [...currentDimensions, dim]);

    let k = dim;
    if (dim.includes(":")) {
      const parts = dim.split(":");
      k = parts[0];

      // 2. Include generic tag (if applicable)
      if (rules.wildcardableTags.includes(k) && parts[1] !== "*") {
        recurse(index + 1, [...currentDimensions, `${k}:*`]);
      }
      if (k === "from" && parts.length === 3 && parts[2] !== "*") {
        recurse(index + 1, [...currentDimensions, `${parts[0]}:${parts[1]}:*`]);
      }
    }

    // 3. Omit dimension entirely
    let canOmit = rules.omittableTags.always.includes(k);

    if (dim.includes(":") && !rules.wildcardableTags.includes(k)) {
      canOmit = true;
    }

    const condRule = (
      rules.omittableTags.conditional as Record<string, { excludeTitles: string[] }>
    )[k];
    if (condRule && !condRule.excludeTitles.includes(title)) {
      canOmit = true;
    }

    if (canOmit) {
      recurse(index + 1, currentDimensions);
    }
  }

  recurse(0, []);

  return Array.from(results).sort((a, b) => b.length - a.length);
}

export function mapUiLogEntry(
  entry: UiLogEntry,
  state?: BattleState,
  options: MapperOptions = {},
): AnyMappedLog | null {
  if (typeof entry === "string") {
    const lower = entry.toLowerCase();
    let context = {};
    return { patterns: [lower], category: LogCategory.Primary, context };
  }

  let title = entry.title.toLowerCase();
  let metadata: MappedLogMetadata = {};

  if (rules.aliases[title as keyof typeof rules.aliases]) {
    title = rules.aliases[title as keyof typeof rules.aliases];
  } else if (title === "statboost") title = (entry.values?.by as number) < 0 ? "unboost" : "boost";
  else if (title === "itemend") title = "itemend";
  else if (title === "clearweather") title = "clearweather";
  else if (title === "fieldstart") title = "fieldstart";
  else if (title === "fieldend") title = "fieldend";
  else if (title === "sidestart") title = "sidestart";
  else if (title === "sideend") title = "sideend";
  else if (title === "curestatus") title = "curestatus";
  else if (title === "updateappearance") {
    if (entry.values?.title === "specieschange") title = "specieschange";
  } else if (title === "effect") {
    if (entry.effect?.effect_type === "Ability") {
      title = "ability";
    }
  } else if (title === "extension") {
    if (entry.values?.title === "Affection") title = "affection";
    if (entry.values?.title === "TierChange") title = "tierchange";
  }

  if (entry.values?.title && typeof entry.values.title === "string") {
    title = entry.values.title.toLowerCase();
  }

  if (title === "leave" && entry.values?.title !== "forfeited" && entry.values?.title !== "escaped") return null;
  if (title === "extension") return null;

  const tags: string[] = [];
  const flags: string[] = [];
  const context: LogContext = {};

  let category = LogCategory.Secondary;
  if (rules.primaryKeys.includes(entry.title.toLowerCase()) || rules.primaryKeys.includes(title)) {
    category = LogCategory.Primary;
  } else if (rules.hintKeys.includes(entry.title.toLowerCase()) || rules.hintKeys.includes(title)) {
    category = LogCategory.Hint;
  }

  // --- Process explicit struct fields ---
  if (entry.player) {
    tags.push("player:*");
    const resolved = resolvePlayerContext(entry.player, state, options);
    context.PLAYER = resolved.standard;
    context.PLAYER_POSSESSIVE = resolved.possessive;
  }

  if (entry.side !== undefined && entry.side !== null) {
    tags.push("side:*");
    const resolved = resolveSideContext(entry.side, state, options);
    context.SIDE = resolved.standard;
    context.SIDE_POSSESSIVE = resolved.possessive;
  }

  if (entry.target) {
    tags.push(entry.title.toLowerCase() === "effect" ? "mon:*" : "target:*");
    const resolved = resolveMonContext(entry.target, state, options);
    const prefix = entry.title.toLowerCase() === "effect" ? "MON" : "TARGET";
    context[prefix] = resolved.standard;
    context[`${prefix}_POSSESSIVE`] = resolved.possessive;
    context.FOE_SIDE = (resolved.rel === "self" || resolved.rel === "ally") ? i18next.t("side.foe") : i18next.t("side.self");
    metadata.target = { raw: resolved.raw, raw_possessive: resolved.raw_possessive, ref: resolved.ref };
  }

  if (entry.source) {
    tags.push("of:*");
    const resolved = resolveMonContext(entry.source, state, options);
    context.SOURCE = resolved.standard;
    context.SOURCE_POSSESSIVE = resolved.possessive;
    context.FOE_SIDE = (resolved.rel === "self" || resolved.rel === "ally") ? i18next.t("side.foe") : i18next.t("side.self");
    metadata.source = { raw: resolved.raw, raw_possessive: resolved.raw_possessive, ref: resolved.ref };
  }

  if (entry.effect) {
    if (entry.effect.effect_type) tags.push(`${entry.effect.effect_type.toLowerCase()}:${entry.effect.name}`);
    else tags.push(`effect:${entry.effect.name}`);
  }

  if (entry.source_effect) {
    if (entry.source_effect.effect_type) tags.push(`from:${entry.source_effect.effect_type.toLowerCase()}:${entry.source_effect.name}`);
    else tags.push(`from:${entry.source_effect.name}`);
  }

  // --- Process generic dynamic values ---
  if (entry.values) {
    const processValue = (k: string, v: unknown) => {
      if (v === undefined || v === null) return;
      if (k === "title") return;

      if (k === "animate") {
        if (v === false) flags.push("noanim");
        return;
      }
      if (k === "animate_only") {
        if (v === true && title === "move") title = "animatemove";
        return;
      }

      if (k === "learned" && entry.title.toLowerCase() === "moveupdate") {
        title = v ? "learnedmove" : "didnotlearnmove";
        return;
      }
      
      // Opportunistic UI context binding: mon context
      if (k === "mon" || k === "target" || k === "source" || k === "of") {
        if (typeof v === "object" && v !== null && ("Active" in v || "Inactive" in v)) {
            const mappedK = (k === "of" || k === "source") ? "source" : (k === "target" ? "target" : "mon");
            const tagK = mappedK === "source" ? "of" : mappedK;
            tags.push(`${tagK}:*`);
            const resolved = resolveMonContext(v as UiMon, state, options);
            const prefix = mappedK.toUpperCase();
            context[prefix] = resolved.standard;
            context[`${prefix}_POSSESSIVE`] = resolved.possessive;
            context.FOE_SIDE = (resolved.rel === "self" || resolved.rel === "ally") ? i18next.t("side.foe") : i18next.t("side.self");
            
            if (mappedK === "source") metadata.source = { raw: resolved.raw, raw_possessive: resolved.raw_possessive, ref: resolved.ref };
            else if (mappedK === "target") metadata.target = { raw: resolved.raw, raw_possessive: resolved.raw_possessive, ref: resolved.ref };
            else metadata.mon = { raw: resolved.raw, raw_possessive: resolved.raw_possessive, ref: resolved.ref };
            return;
        }
      }

      if (k === "move_name" && (title === "learnedmove" || title === "didnotlearnmove" || entry.title.toLowerCase() === "moveupdate")) {
        tags.push(`move:${v}`);
        context.MOVE = v as ContextValue;
      } else if (k === "name" && title === "move") {
        tags.push(`name:*`);
        context.MOVE = v as ContextValue;
      } else if (typeof v === "string" || typeof v === "number" || typeof v === "boolean") {
        if (v === true || v === "") {
          flags.push(k.replace(/_/g, ""));
        } else if (v === false) {
          // Do not push false booleans to tags
        } else {
          if (["ability", "item", "move", "effect", "condition", "weather", "status", "volatile", "from"].includes(k)) {
            tags.push(`${k}:${v}`);
          } else if (k === "by") {
            const num = Number(v);
            let appliedBucket = false;
            if (!isNaN(num)) {
              for (const bucket of rules.numericBuckets) {
                if (bucket.tag === k && bucket.titles.includes(title) && num >= bucket.min) {
                  tags.push(`${k}:${bucket.min}${bucket.suffix}`);
                  appliedBucket = true;
                  break;
                }
              }
            }
            if (!appliedBucket) tags.push(`by:${v}`);
          } else if (k === "stat") {
            context.STAT = i18next.t(`stats.${v}`);
            tags.push(`${k}:*`);
          } else {
            tags.push(`${k}:${v}`);
          }
        }
      } else if (Array.isArray(v)) {
         tags.push(`${k}:*`);
         if (k === "health" && v.length === 2) {
             // For Health Fraction
             if (options.healthFormat === "percentage") {
                 context.HEALTH = `${Math.ceil((Number(v[0]) / Number(v[1])) * 100)}%`;
             } else {
                 context.HEALTH = `${v[0]}/${v[1]}`;
             }
         }
      } else {
        tags.push(`${k}:*`);
      }
    };

    for (const [k, v] of Object.entries(entry.values)) {
      processValue(k, v);
    }
  }

  // --- Special hardcoded parsing logic ---
  if (["switch", "drag", "replace", "appear"].includes(title)) {
    tags.length = 0;
    ["player", "position", "name", "health", "species", "level", "gender"].forEach((t) => tags.push(`${t}:*`));
    // The player context should have already been handled by entry.player, or entry.values
    
    // We expect values to contain 'name', 'mon_index', etc. for string mapping, but not 'mon' UiMon.
    const monName = (entry.values?.name as string) || "Mon";
    context.MON = { text: monName };
    context.MON_POSSESSIVE = { text: `${monName}'s` };
    if (!metadata.mon) metadata.mon = { raw: monName, raw_possessive: `${monName}'s` };
  } else if (title === "switchout") {
    tags.length = 0;
    tags.push("mon:*");
  }

  const combinatoricTags = tags.filter((t) => !rules.excludeTags.includes(t.split(":")[0]));

  if (state?.battle_type) {
    combinatoricTags.push(`battletype:${state.battle_type.toLowerCase()}`);
  }

  let finalFlags = flags;
  let finalTags = combinatoricTags;

  if (title === "move") {
    finalFlags = flags.filter((f) => !["zpower", "notarget"].includes(f));
    finalTags = combinatoricTags.filter(
      (t) => !t.startsWith("from:") && !t.startsWith("animate:") && !t.startsWith("animate_only:"),
    );
  }

  const patterns = generateCombinatorics(
    title,
    Array.from(new Set(finalTags)),
    Array.from(new Set(finalFlags)),
  );

  // We need to synthesize an EffectData object for the consumer if there isn't one already available in effect
  const effectData: EffectData | undefined = entry.effect ? {
      effect: entry.effect,
      side: entry.side ?? undefined,
      slot: entry.slot ?? undefined,
      player: entry.player ?? undefined,
      target: entry.target ?? undefined,
      source: entry.source ?? undefined,
      source_effect: entry.source_effect ?? undefined,
      additional: {}
  } as unknown as EffectData : undefined;

  const mapped: AnyMappedLog = {
    patterns,
    category,
    context,
    effect: effectData,
    metadata: Object.keys(metadata).length > 0 ? metadata : undefined,
  };
  return mapped;
}

export function getLogPatterns(entry: UiLogEntry): string[] {
  return mapUiLogEntry(entry)?.patterns || [];
}
