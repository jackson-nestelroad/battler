import type { BattleState, EffectData, UiLogEntry, UiMon } from "battler-state";
import i18next from "i18next";
import rules from "./config/mapper-rules.json" with { type: "json" };
import categoryRules from "./config/category-rules.json" with { type: "json" };
import type {
  AnyMappedLog,
  ContextValue,
  ContextVar,
  LogContext,
  MappedLogMetadata,
  MapperOptions,
} from "./types.js";
import { LogCategory } from "./types.js";

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

export function isWildPlayer(state: BattleState | undefined, playerId: string | undefined): boolean {
  if (!playerId) return false;
  if (playerId.startsWith("wild")) return true;
  if (state?.field?.sides) {
    for (const side of state.field.sides) {
      const player = side?.players?.[playerId];
      if (player) {
        const pType = (player as unknown as { player_type?: unknown; type?: unknown }).player_type ??
                      (player as unknown as { player_type?: unknown; type?: unknown }).type;
        if (typeof pType === "string" && pType.toLowerCase() === "wild") return true;
        if (typeof pType === "object" && pType !== null && (("type" in pType && (pType as { type: string }).type === "wild") || "Wild" in pType)) return true;
      }
    }
  }
  return false;
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
  player: ContextVar;
  player_possessive: ContextVar;
  playerId?: string;
  raw: string;
  raw_possessive: string;
  ref?: UiMon;
  rel: Relationship;
} {
  if (!monRef || typeof monRef !== "object") {
    return {
      standard: { text: "Mon" },
      possessive: { text: "Mon's" },
      player: { text: "Player" },
      player_possessive: { text: "Player's" },
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
  } else if ("name" in monRef && (monRef as unknown as { name?: string }).name) {
    name = (monRef as unknown as { name: string }).name;
    if ((monRef as unknown as { player?: string }).player) {
      playerId = (monRef as unknown as { player: string }).player;
    }
  }

  const rel = playerId ? getRelationship(state, options.localPlayerId, playerId) : "foe";
  const playerResolved = resolvePlayerContext(playerId, state, options);
  const playerName = playerResolved.standard.text;

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
    const isWild = isWildPlayer(state, playerId);
    if (isWild) {
      text = i18next.t("mon.foe_wild", { name });
      possessiveText = i18next.t("mon.foe_possessive_wild", { name });
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
  }

  return {
    standard: { text, monRef, noAutoCapitalize },
    possessive: { text: possessiveText, monRef, noAutoCapitalize: possessiveNoAutoCapitalize },
    player: playerResolved.standard,
    player_possessive: playerResolved.possessive,
    playerId,
    raw: name,
    raw_possessive: `${name}'s`,
    ref: monRef,
    rel,
  };
}

export function resolveCategory(title: string, tags: string[], entryTitle: string): LogCategory {
  for (const rule of categoryRules) {
    const condition = rule.condition as {
      default?: boolean;
      title?: string;
      titleIn?: string[];
      withoutTags?: string[];
      withTags?: string[];
    };
    if (condition.default) {
      return rule.category as LogCategory;
    }
    if (condition.title && condition.title !== title && condition.title !== entryTitle.toLowerCase()) {
      continue;
    }
    if (
      condition.titleIn &&
      !condition.titleIn.includes(title) &&
      !condition.titleIn.includes(entryTitle.toLowerCase())
    ) {
      continue;
    }
    if (condition.withoutTags) {
      const hasAnyExcludedTag = condition.withoutTags.some((prefix: string) =>
        tags.some((t) => t.startsWith(prefix + ":") || t === prefix)
      );
      if (hasAnyExcludedTag) continue;
    }
    if (condition.withTags) {
      const hasAllRequiredTags = condition.withTags.every((prefix: string) =>
        tags.some((t) => t.startsWith(prefix + ":") || t === prefix)
      );
      if (!hasAllRequiredTags) continue;
    }
    return rule.category as LogCategory;
  }
  return LogCategory.Secondary;
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
    const lower = (entry as string).toLowerCase();
    const context = {};
    return { patterns: [lower], category: LogCategory.Primary, context };
  }

  let title = entry.title.toLowerCase();
  const metadata: MappedLogMetadata = {};

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
    context[`${prefix}_PLAYER`] = resolved.player;
    context[`${prefix}_PLAYER_POSSESSIVE`] = resolved.player_possessive;
    if (prefix === "MON") {
      if (!context.PLAYER) context.PLAYER = resolved.player;
      if (!context.PLAYER_POSSESSIVE) context.PLAYER_POSSESSIVE = resolved.player_possessive;
    }
    context.FOE_SIDE = (resolved.rel === "self" || resolved.rel === "ally") ? i18next.t("side.foe") : i18next.t("side.self");
    metadata.target = { raw: resolved.raw, raw_possessive: resolved.raw_possessive, ref: resolved.ref };
  }

  if (entry.source) {
    tags.push("of:*");
    const resolved = resolveMonContext(entry.source, state, options);
    context.SOURCE = resolved.standard;
    context.SOURCE_POSSESSIVE = resolved.possessive;
    context.SOURCE_PLAYER = resolved.player;
    context.SOURCE_PLAYER_POSSESSIVE = resolved.player_possessive;
    context.FOE_SIDE = (resolved.rel === "self" || resolved.rel === "ally") ? i18next.t("side.foe") : i18next.t("side.self");
    metadata.source = { raw: resolved.raw, raw_possessive: resolved.raw_possessive, ref: resolved.ref };
  }

  if (entry.effect) {
    if (entry.effect.effect_type) tags.push(`${entry.effect.effect_type.toLowerCase()}:${entry.effect.name}`);
    else tags.push(`effect:${entry.effect.name}`);
    
    if (entry.effect.name) {
      context.EFFECT = entry.effect.name;
      if (entry.effect.effect_type) {
        context[entry.effect.effect_type.toUpperCase()] = entry.effect.name;
      }
    }
  }

  if (entry.source_effect) {
    if (entry.source_effect.effect_type) tags.push(`from:${entry.source_effect.effect_type.toLowerCase()}:${entry.source_effect.name}`);
    else tags.push(`from:${entry.source_effect.name}`);

    if (entry.source_effect.name) {
      context.FROM = entry.source_effect.name;
      if (entry.source_effect.effect_type) {
        context[entry.source_effect.effect_type.toUpperCase()] = entry.source_effect.name;
        context[`FROM_${entry.source_effect.effect_type.toUpperCase()}`] = entry.source_effect.name;
      }
    }
  }

  // --- Process generic dynamic values ---
  if (entry.values) {
    const processValue = (k: string, v: unknown) => {
      if (v === undefined || v === null) return;
      if (k === "title" || k === "player" || k === "side") return;

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
            context[`${prefix}_PLAYER`] = resolved.player;
            context[`${prefix}_PLAYER_POSSESSIVE`] = resolved.player_possessive;
            if (prefix === "MON") {
              if (!context.PLAYER) context.PLAYER = resolved.player;
              if (!context.PLAYER_POSSESSIVE) context.PLAYER_POSSESSIVE = resolved.player_possessive;
            }
            context.FOE_SIDE = (resolved.rel === "self" || resolved.rel === "ally") ? i18next.t("side.foe") : i18next.t("side.self");
            
            if (mappedK === "source") metadata.source = { raw: resolved.raw, raw_possessive: resolved.raw_possessive, ref: resolved.ref };
            else if (mappedK === "target") metadata.target = { raw: resolved.raw, raw_possessive: resolved.raw_possessive, ref: resolved.ref };
            else metadata.mon = { raw: resolved.raw, raw_possessive: resolved.raw_possessive, ref: resolved.ref };
            return;
        }
      }

      if (k === "from" && typeof v === "string") {
        if (v.includes(":")) {
          const parts = v.split(":");
          const fromType = parts[0];
          const fromName = parts.slice(1).join(":");
          context.FROM = fromName;
          context[fromType.toUpperCase()] = fromName;
          context[`FROM_${fromType.toUpperCase()}`] = fromName;
        } else {
          context.FROM = v;
        }
      }

      if (k === "move_name" && (title === "learnedmove" || title === "didnotlearnmove" || entry.title.toLowerCase() === "moveupdate")) {
        tags.push(`move:${v}`);
        context.MOVE = v as ContextValue;
      } else if (k === "name" && title === "move") {
        tags.push(`name:*`);
        context.MOVE = v as ContextValue;
      } else if (k === "magnitude") {
        context.MAGNITUDE = String(v);
        tags.push(`magnitude:${v}`);
      } else if (k === "boosts") {
        const statsList = String(v).split(",").map((s) => s.trim().toLowerCase()).filter(Boolean);
        if (statsList.length === 1) {
          const statKey = statsList[0];
          const statName = i18next.exists(`stats.${statKey}`) ? i18next.t(`stats.${statKey}`) : statKey;
          context.STAT = statName;
          context.count = 1;
          tags.push(`boosts:${statKey}`);
        } else if (statsList.length > 1) {
          const statsWord = i18next.exists("vocabulary.stats") ? i18next.t("vocabulary.stats") : "stats";
          context.STAT = statsWord;
          context.count = statsList.length;
          tags.push(`boosts:stats`);
        } else {
          tags.push(`boosts:${v}`);
        }
      } else if (k === "species") {
        const speciesStr = String(v);
        context.SPECIES = speciesStr;
        context.FORME = speciesStr;
        tags.push(`species:${speciesStr.toLowerCase().replace(/[^a-z0-9]/g, "")}`);
      } else if (typeof v === "string" || typeof v === "number" || typeof v === "boolean") {
        const upperK = k.toUpperCase();
        context[upperK] = String(v);

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
    
    const monName = (entry.values?.name as string) || "Mon";
    const playerId = entry.player || (entry.values?.player as string);
    const position = entry.values?.position as number | undefined;

    // Check if there was an active mon previously on this slot
    if (state?.field?.sides && playerId !== undefined && position !== undefined) {
      let sideIndex = entry.side;
      if (sideIndex === undefined || sideIndex === null) {
        for (let i = 0; i < state.field.sides.length; i++) {
          if (state.field.sides[i]?.players?.[playerId]) {
            sideIndex = i;
            break;
          }
        }
      }
      if (sideIndex !== undefined && sideIndex !== null) {
        const side = state.field.sides[sideIndex];
        const activeRef = side?.active?.[position];
        if (activeRef) {
          let prevName: string | undefined = undefined;
          let prevPlayer: string | undefined = undefined;
          if (typeof activeRef === "object") {
            if ("name" in activeRef && typeof (activeRef as { name?: string }).name === "string") {
              prevName = (activeRef as { name: string }).name;
            }
            if ("player" in activeRef && typeof (activeRef as { player?: string }).player === "string") {
              prevPlayer = (activeRef as { player: string }).player;
            }
            if (!prevName && prevPlayer && "mon_index" in activeRef) {
              const monIdx = (activeRef as { mon_index: number }).mon_index;
              const mon = side?.players?.[prevPlayer]?.mons?.[monIdx];
              if (mon?.physical_appearance?.name) {
                prevName = mon.physical_appearance.name;
              }
            }
          }
          if (prevName && prevName !== monName) {
            const prevUiMon: UiMon = {
              Active: {
                name: prevName,
                player: prevPlayer || playerId,
                position: position,
                side: sideIndex,
              },
            };
            const prevResolved = resolveMonContext(prevUiMon, state, options);
            context.PREV_MON = prevResolved.standard;
            context.PREV_MON_POSSESSIVE = prevResolved.possessive;
            context.PREV_MON_PLAYER = prevResolved.player;
            context.PREV_MON_PLAYER_POSSESSIVE = prevResolved.player_possessive;
            metadata.prev_mon = { raw: prevResolved.raw, raw_possessive: prevResolved.raw_possessive, ref: prevResolved.ref };
          }
        }
      }
    }

    const monRefObj = (entry.values?.mon as UiMon) || {
      Active: {
        name: monName,
        player: playerId || "",
        position: position || 0,
        side: entry.side ?? 0
      }
    };
    const resolved = resolveMonContext(monRefObj, state, options);
    context.MON = resolved.standard;
    context.MON_POSSESSIVE = resolved.possessive;
    context.MON_PLAYER = resolved.player;
    context.MON_PLAYER_POSSESSIVE = resolved.player_possessive;
    if (!context.PLAYER) context.PLAYER = resolved.player;
    if (!context.PLAYER_POSSESSIVE) context.PLAYER_POSSESSIVE = resolved.player_possessive;
    if (!metadata.mon) metadata.mon = { raw: resolved.raw, raw_possessive: resolved.raw_possessive, ref: resolved.ref };
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
    finalFlags = flags.filter((f) => !["notarget"].includes(f));
    finalTags = combinatoricTags.filter(
      (t) => !t.startsWith("animate:") && !t.startsWith("animate_only:"),
    );
    if (flags.includes("zpower") || entry.values?.zpower === true) {
      if (typeof context.MOVE === "string" && !context.MOVE.startsWith("Z-")) {
        context.MOVE = `Z-${context.MOVE}`;
      }
    }
  }

  const category = resolveCategory(title, tags, entry.title);

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
