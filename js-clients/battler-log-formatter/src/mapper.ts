import type { BattleState, UiLogEntry, UiMon } from "battler-state";
import i18next from "i18next";
import categoryRules from "./config/category-rules.json" with { type: "json" };
import rules from "./config/mapper-rules.json" with { type: "json" };
import { serializePattern } from "./pattern.js";
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

export function isWildPlayer(
  state: BattleState | undefined,
  playerId: string | undefined,
): boolean {
  if (!playerId) return false;
  if (state?.field?.sides) {
    for (const side of state.field.sides) {
      const player = side?.players?.[playerId];
      if (player) {
        if ((player as unknown as { wild?: boolean }).wild !== undefined) {
          return Boolean((player as unknown as { wild?: boolean }).wild);
        }
        const pType =
          (player as unknown as { player_type?: unknown; type?: unknown }).player_type ??
          (player as unknown as { player_type?: unknown; type?: unknown }).type;
        if (typeof pType === "string") {
          return pType.toLowerCase() === "wild";
        }
        if (typeof pType === "object" && pType !== null) {
          return (
            ("type" in pType && (pType as { type: string }).type === "wild") || "Wild" in pType
          );
        }
      }
    }
  }
  return playerId.toLowerCase().startsWith("wild");
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
      standard: { text: name, noAutoCapitalize: true },
      possessive: { text: `${name}'s`, noAutoCapitalize: true },
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
    standard: { text, noAutoCapitalize: rel !== "self" },
    possessive: { text: possessiveText, noAutoCapitalize: rel !== "self" },
  };
}

export function getSideName(sideIndex: number | undefined, state: BattleState | undefined): string {
  if (sideIndex != null && state?.field?.sides?.[sideIndex]?.name) {
    return state.field.sides[sideIndex].name;
  }
  if (sideIndex != null) {
    return `Side ${sideIndex + 1}`;
  }
  return "Side";
}

export function resolveSideNameContext(
  sideIndex: number | undefined,
  state: BattleState | undefined,
): { standard: ContextVar; possessive: ContextVar } {
  const name = getSideName(sideIndex, state);
  return {
    standard: { text: name, noAutoCapitalize: true },
    possessive: { text: `${name}'s`, noAutoCapitalize: true },
  };
}

export function getLocalSideIndex(
  state: BattleState | undefined,
  localPlayerId: string | undefined,
): number | undefined {
  if (!state?.field?.sides || !localPlayerId) return undefined;
  for (let i = 0; i < state.field.sides.length; i++) {
    if (state.field.sides[i]?.players?.[localPlayerId]) {
      return i;
    }
  }
  return undefined;
}

export function getFoeSideIndex(
  state: BattleState | undefined,
  localPlayerId: string | undefined,
  currentSideIndex?: number,
): number | undefined {
  if (!state?.field?.sides || state.field.sides.length === 0) return undefined;

  const localIdx = getLocalSideIndex(state, localPlayerId);
  if (localIdx !== undefined) {
    for (let i = 0; i < state.field.sides.length; i++) {
      if (i !== localIdx) return i;
    }
  }

  if (currentSideIndex !== undefined) {
    for (let i = 0; i < state.field.sides.length; i++) {
      if (i !== currentSideIndex) return i;
    }
  }

  if (state.field.sides.length > 1) {
    return 1;
  }

  return undefined;
}

export function resolveSideContext(
  sideIndex: number | undefined,
  state: BattleState | undefined,
  options: MapperOptions,
): { standard: ContextVar; possessive: ContextVar } {
  if (sideIndex == null) return { standard: { text: "Side" }, possessive: { text: "Side's" } };

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
    standard: { text },
    possessive: { text: possessiveText },
  };
}

export function resolveMonContext(
  monRef: UiMon | undefined,
  state: BattleState | undefined,
  options: MapperOptions,
): {
  standard: ContextVar;
  possessive: ContextVar;
  its: ContextVar;
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
      its: { text: i18next.t("mon.its") },
      player: { text: "Player", noAutoCapitalize: true },
      player_possessive: { text: "Player's", noAutoCapitalize: true },
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
    text = i18next.t("mon.ally", { name, player: playerName });
    possessiveText = i18next.t("mon.ally_possessive", { name, player: playerName });
    noAutoCapitalize = true;
    possessiveNoAutoCapitalize = true;
  } else {
    const isWild = isWildPlayer(state, playerId);
    if (isWild) {
      text = i18next.t("mon.foe_wild", { name });
      possessiveText = i18next.t("mon.foe_possessive_wild", { name });
    } else {
      const isMulti = state?.battle_type?.toLowerCase() === "multi";
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

  const itsText = i18next.t("mon.its");

  return {
    standard: { text, monRef, noAutoCapitalize },
    possessive: { text: possessiveText, monRef, noAutoCapitalize: possessiveNoAutoCapitalize },
    its: { text: itsText, monRef, noAutoCapitalize: false },
    player: playerResolved.standard,
    player_possessive: playerResolved.possessive,
    playerId,
    raw: name,
    raw_possessive: `${name}'s`,
    ref: monRef,
    rel,
  };
}

export function formatFraction(
  fraction: unknown[],
  format: "fraction" | "percentage" | undefined,
): string {
  if (fraction.length !== 2) return "";
  if (format === "percentage") {
    return `${Math.ceil((Number(fraction[0]) / Number(fraction[1])) * 100)}%`;
  }
  return `${fraction[0]}/${fraction[1]}`;
}

export function bindMonParticipant(
  role: "mon" | "target" | "source" | "prev_mon",
  uiMon: UiMon,
  state: BattleState | undefined,
  options: MapperOptions,
  context: LogContext,
  metadata: MappedLogMetadata,
  tags: string[],
): void {
  if (role !== "prev_mon") {
    const tagK = role === "source" ? "of" : role;
    tags.push(`${tagK}:*`);
  }

  const resolved = resolveMonContext(uiMon, state, options);
  const prefix = role.toUpperCase();

  const nameVar: ContextVar = {
    text: resolved.raw,
    monRef: resolved.ref,
    noAutoCapitalize: false,
  };
  const namePossessiveVar: ContextVar = {
    text: resolved.raw_possessive,
    monRef: resolved.ref,
    noAutoCapitalize: false,
  };

  context[prefix] = resolved.standard;
  context[`${prefix}_POSSESSIVE`] = resolved.possessive;
  context[`${prefix}_NAME`] = nameVar;
  context[`${prefix}_NAME_POSSESSIVE`] = namePossessiveVar;
  context[`${prefix}_PLAYER`] = resolved.player;
  context[`${prefix}_PLAYER_POSSESSIVE`] = resolved.player_possessive;

  if (role === "mon") {
    if (!context.PLAYER) context.PLAYER = resolved.player;
    if (!context.PLAYER_POSSESSIVE) context.PLAYER_POSSESSIVE = resolved.player_possessive;
    if (!context.TARGET) {
      context.TARGET = resolved.standard;
      context.TARGET_POSSESSIVE = resolved.possessive;
      context.TARGET_NAME = nameVar;
      context.TARGET_NAME_POSSESSIVE = namePossessiveVar;
      context.TARGET_PLAYER = resolved.player;
      context.TARGET_PLAYER_POSSESSIVE = resolved.player_possessive;
    }
    if (!context.OF_OR_MON_POSSESSIVE) {
      context.OF_OR_MON_POSSESSIVE = resolved.its;
    }
    metadata.mon = {
      raw: resolved.raw,
      raw_possessive: resolved.raw_possessive,
      possessive: resolved.possessive,
      ref: resolved.ref,
    };
    if (!metadata.target) metadata.target = metadata.mon;
  } else if (role === "target") {
    if (!context.MON) {
      context.MON = resolved.standard;
      context.MON_POSSESSIVE = resolved.possessive;
      context.MON_NAME = nameVar;
      context.MON_NAME_POSSESSIVE = namePossessiveVar;
      context.MON_PLAYER = resolved.player;
      context.MON_PLAYER_POSSESSIVE = resolved.player_possessive;
    }
    if (!context.PLAYER) context.PLAYER = resolved.player;
    if (!context.PLAYER_POSSESSIVE) context.PLAYER_POSSESSIVE = resolved.player_possessive;
    if (!context.OF_OR_MON_POSSESSIVE) {
      context.OF_OR_MON_POSSESSIVE = resolved.its;
    }
    metadata.target = {
      raw: resolved.raw,
      raw_possessive: resolved.raw_possessive,
      possessive: resolved.possessive,
      ref: resolved.ref,
    };
    if (!metadata.mon) metadata.mon = metadata.target;
  } else if (role === "source") {
    context.OF = resolved.standard;
    context.OF_POSSESSIVE = resolved.possessive;
    context.OF_NAME = nameVar;
    context.OF_NAME_POSSESSIVE = namePossessiveVar;
    context.OF_OR_MON_POSSESSIVE = resolved.possessive;
    metadata.source = {
      raw: resolved.raw,
      raw_possessive: resolved.raw_possessive,
      possessive: resolved.possessive,
      ref: resolved.ref,
    };
  } else if (role === "prev_mon") {
    context.PREV_MON = resolved.standard;
    context.PREV_MON_POSSESSIVE = resolved.possessive;
    context.PREV_MON_NAME = nameVar;
    context.PREV_MON_NAME_POSSESSIVE = namePossessiveVar;
    context.PREV_MON_PLAYER = resolved.player;
    context.PREV_MON_PLAYER_POSSESSIVE = resolved.player_possessive;
    metadata.prev_mon = {
      raw: resolved.raw,
      raw_possessive: resolved.raw_possessive,
      possessive: resolved.possessive,
      ref: resolved.ref,
    };
  }

  if (role === "mon" || (role === "target" && !context.FOE_SIDE)) {
    context.FOE_SIDE =
      resolved.rel === "self" || resolved.rel === "ally"
        ? i18next.t("side.foe")
        : i18next.t("side.self");
  }
}

export function extractExtensionSource(entry: UiLogEntry): string | null {
  if (entry.values?.source && typeof entry.values.source === "string") {
    return entry.values.source.replace(/^[-:]+|[:]+$/g, "");
  }
  if (entry.title.startsWith("-") && entry.title.includes(":")) {
    return entry.title.split(":")[0].replace(/^-+/, "");
  }
  return null;
}

export function normalizeTitle(entry: UiLogEntry): string {
  let title = entry.title.toLowerCase();
  if (title.startsWith("-") && title.includes(":")) {
    title = title.split(":")[1].toLowerCase();
  }
  return title;
}

export function resolveCategory(
  title: string,
  tags: string[],
  entryTitle: string,
  extensionSource?: string | null,
): LogCategory {
  for (const rule of categoryRules) {
    const condition = rule.condition as {
      default?: boolean;
      extension?: string;
      extensionIn?: string[];
      title?: string;
      titleIn?: string[];
      withoutTags?: string[];
      withTags?: string[];
    };
    if (condition.default) {
      return rule.category as LogCategory;
    }
    if (condition.extension && (!extensionSource || extensionSource !== condition.extension)) {
      continue;
    }
    if (
      condition.extensionIn &&
      (!extensionSource || !condition.extensionIn.includes(extensionSource))
    ) {
      continue;
    }
    if (
      condition.title &&
      condition.title !== title &&
      condition.title !== entryTitle.toLowerCase()
    ) {
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
        tags.some((t) => t.startsWith(prefix + ":") || t === prefix),
      );
      if (hasAnyExcludedTag) continue;
    }
    if (condition.withTags) {
      const hasAllRequiredTags = condition.withTags.every((prefix: string) =>
        tags.some((t) => t.startsWith(prefix + ":") || t === prefix),
      );
      if (!hasAllRequiredTags) continue;
    }
    return rule.category as LogCategory;
  }
  return LogCategory.Secondary;
}

function buildPattern(title: string, tags: string[], flags: string[]): string {
  return serializePattern({ title, tags, flags });
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

      // 2. Include generic wildcard tag (if applicable)
      if (rules.wildcardableTags.includes(k) && parts[1] !== "*") {
        recurse(index + 1, [...currentDimensions, `${k}:*`]);
      }
      if (k === "from" && parts.length === 3 && parts[2] !== "*") {
        recurse(index + 1, [...currentDimensions, `${parts[0]}:${parts[1]}:*`]);
      }
    }

    // 3. Omit dimension entirely (if omittable)
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

function handleBoostsTag(v: unknown, context: LogContext, tags: string[]): void {
  const statsList = String(v)
    .split(",")
    .map((s) => s.trim().toLowerCase())
    .filter(Boolean);
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
}

function handleNumericBucketing(k: string, v: unknown, title: string, tags: string[]): boolean {
  const num = Number(v);
  if (isNaN(num)) return false;
  for (const bucket of rules.numericBuckets) {
    if (bucket.tag === k && bucket.titles.includes(title) && num >= bucket.min) {
      tags.push(`${k}:${bucket.min}${bucket.suffix}`);
      return true;
    }
  }
  return false;
}

export function mapUiLogEntry(
  entry: UiLogEntry,
  state?: BattleState,
  options: MapperOptions = {},
): AnyMappedLog | null {
  if (typeof entry === "string") {
    const lower = (entry as string).toLowerCase();
    return { patterns: [lower], category: LogCategory.Primary, context: {} };
  }

  const title = normalizeTitle(entry);
  if (!title) return null;

  const extensionSource = extractExtensionSource(entry);
  const metadata: MappedLogMetadata = {};
  const tags: string[] = [];
  const flags: string[] = [];
  const context: LogContext = {};

  // --- Process explicit struct fields ---
  const playerId =
    entry.player ||
    (typeof entry.values?.player === "string" ? entry.values.player : undefined) ||
    (typeof entry.values?.action === "string" ? entry.values.action : undefined);
  if (playerId) {
    tags.push("player:*");
    const resolved = resolvePlayerContext(playerId, state, options);
    context.PLAYER = resolved.standard;
    context.PLAYER_POSSESSIVE = resolved.possessive;
  }

  if (title === "timer") {
    if (entry.values?.action !== undefined || entry.values?.turn !== undefined) {
      flags.push("turn");
    }

    const rawSecs =
      entry.values?.remainingsecs ?? entry.values?.remaining_secs ?? entry.values?.remaining;
    if (rawSecs !== undefined && rawSecs !== null) {
      const numSecs = Number(rawSecs);
      if (!isNaN(numSecs)) {
        context.REMAININGSECS = numSecs;
        if (numSecs < 60) {
          context.TIME = `${numSecs} second${numSecs === 1 ? "" : "s"}`;
        } else {
          const minutes = Math.floor(numSecs / 60);
          const seconds = numSecs % 60;
          if (seconds === 0) {
            context.TIME = `${minutes} minute${minutes === 1 ? "" : "s"}`;
          } else {
            const padded = seconds.toString().padStart(2, "0");
            context.TIME = `${minutes}:${padded}`;
          }
        }
        if (numSecs === 0) {
          flags.push("done");
        }
      }
    }
  }

  const side =
    entry.side !== undefined && entry.side !== null
      ? entry.side
      : typeof entry.values?.side === "number"
        ? entry.values.side
        : undefined;

  if (side !== undefined && side !== null) {
    tags.push("side:*");
    const resolved = resolveSideContext(side, state, options);
    context.SIDE = resolved.standard;
    context.SIDE_POSSESSIVE = resolved.possessive;

    const sideNameResolved = resolveSideNameContext(side, state);
    context.SIDE_NAME = sideNameResolved.standard;
    context.SIDE_NAME_POSSESSIVE = sideNameResolved.possessive;
  }

  const foeSideIdx = getFoeSideIndex(state, options.localPlayerId, side);
  const foeSideNameResolved = resolveSideNameContext(
    foeSideIdx !== undefined ? foeSideIdx : 1,
    state,
  );
  context.FOE_SIDE_NAME = foeSideNameResolved.standard;
  context.FOE_SIDE_NAME_POSSESSIVE = foeSideNameResolved.possessive;

  const localSideIdx = getLocalSideIndex(state, options.localPlayerId);
  if (localSideIdx !== undefined) {
    const localSideNameResolved = resolveSideNameContext(localSideIdx, state);
    context.SELF_SIDE_NAME = localSideNameResolved.standard;
    context.SELF_SIDE_NAME_POSSESSIVE = localSideNameResolved.possessive;
    context.PLAYER_SIDE_NAME = localSideNameResolved.standard;
    context.PLAYER_SIDE_NAME_POSSESSIVE = localSideNameResolved.possessive;
  }

  if (entry.target) {
    bindMonParticipant("target", entry.target, state, options, context, metadata, tags);
  }

  if (entry.source) {
    bindMonParticipant("source", entry.source, state, options, context, metadata, tags);
  }

  let primaryEffect = entry.effect;
  let sourceEffect = entry.source_effect;

  if (entry.effect) {
    if (entry.effect.effect_type)
      tags.push(`${entry.effect.effect_type.toLowerCase()}:${entry.effect.name}`);
    else tags.push(`effect:${entry.effect.name}`);

    if (entry.effect.name) {
      context.EFFECT = entry.effect.name;
      if (entry.effect.effect_type) {
        context[entry.effect.effect_type.toUpperCase()] = entry.effect.name;
      }
    }
  }

  if (entry.source_effect) {
    if (entry.source_effect.effect_type) {
      tags.push(
        `from:${entry.source_effect.effect_type.toLowerCase()}:${entry.source_effect.name}`,
      );
    } else {
      tags.push(`from:${entry.source_effect.name}`);
    }

    if (entry.source_effect.name) {
      context.FROM = entry.source_effect.name;
      if (entry.source_effect.effect_type) {
        context[`FROM_${entry.source_effect.effect_type.toUpperCase()}`] = entry.source_effect.name;
      }
    }
  }

  // --- Process dynamic values map ---
  if (entry.values) {
    // If values specifies mon/name/position/player for switch/appearance, bind mon
    if (!context.MON && (entry.values.mon || entry.values.name)) {
      const monName = typeof entry.values.name === "string" ? entry.values.name : "Mon";
      const playerId =
        entry.player || (typeof entry.values.player === "string" ? entry.values.player : "");
      const position = typeof entry.values.position === "number" ? entry.values.position : 0;
      const monObj = (entry.values.mon as UiMon) || {
        Active: {
          name: monName,
          player: playerId,
          position: position,
          side: entry.side ?? 0,
        },
      };
      bindMonParticipant("mon", monObj, state, options, context, metadata, tags);
    }

    for (const [k, v] of Object.entries(entry.values)) {
      if (v === undefined || v === null) continue;
      if (k === "title" || k === "player" || k === "side") continue;

      // Participant references in values map
      if (k === "mon" || k === "target" || k === "source" || k === "of" || k === "prev_mon") {
        if (typeof v === "object" && v !== null && ("Active" in v || "Inactive" in v)) {
          const role =
            k === "of" || k === "source"
              ? "source"
              : k === "target"
                ? "target"
                : k === "prev_mon"
                  ? "prev_mon"
                  : "mon";
          bindMonParticipant(role, v as UiMon, state, options, context, metadata, tags);
          continue;
        }
      }

      // From / Effect references in values map
      if (k === "from" && typeof v === "string") {
        if (v.includes(":")) {
          const parts = v.split(":");
          const fromType = parts[0];
          const fromName = parts.slice(1).join(":");
          context.FROM = fromName;
          context[`FROM_${fromType.toUpperCase()}`] = fromName;
          if (!sourceEffect) {
            sourceEffect = {
              effect_type: fromType.charAt(0).toUpperCase() + fromType.slice(1).toLowerCase(),
              name: fromName,
            };
          }
        } else {
          context.FROM = v;
          if (!sourceEffect) {
            sourceEffect = {
              effect_type: null,
              name: v,
            };
          }
        }
      }

      if (k === "ability" && typeof v === "string" && !primaryEffect) {
        primaryEffect = { effect_type: "Ability", name: v };
      } else if (k === "item" && typeof v === "string" && !primaryEffect) {
        primaryEffect = { effect_type: "Item", name: v };
      }

      // Move name aliases
      if (k === "name" && title === "move") {
        context.MOVE = v as ContextValue;
      } else if (k === "boosts") {
        handleBoostsTag(v, context, tags);
      } else if (k === "species") {
        const speciesStr = String(v);
        context.SPECIES = speciesStr;
        context.FORME = speciesStr;
        tags.push(`species:${speciesStr.toLowerCase().replace(/[^a-z0-9]/g, "")}`);
      } else if (
        typeof v === "string" ||
        typeof v === "number" ||
        typeof v === "boolean" ||
        typeof v === "bigint"
      ) {
        const upperK = k.toUpperCase();
        context[upperK] = String(v);

        if (v === true || v === "") {
          flags.push(k.replace(/_/g, ""));
        } else if (v === false) {
          // Do not push false booleans to tags
        } else if (handleNumericBucketing(k, v, title, tags)) {
          // Applied bucket
        } else if (k === "stat") {
          context.STAT = i18next.t(`stats.${v}`);
          context.count = 1;
          tags.push(`stat:*`);
        } else {
          tags.push(`${k}:${v}`);
        }
      } else if (Array.isArray(v)) {
        tags.push(`${k}:*`);
        if ((k === "health" || k === "damage" || k === "heal") && v.length === 2) {
          context[k.toUpperCase()] = formatFraction(v, options.healthFormat);
        }
      } else {
        tags.push(`${k}:*`);
      }
    }
  }

  const primaryPlayerId =
    (metadata.mon?.ref &&
      ("Active" in metadata.mon.ref
        ? metadata.mon.ref.Active?.player
        : "Inactive" in metadata.mon.ref
          ? metadata.mon.ref.Inactive?.player
          : (metadata.mon.ref as unknown as { player?: string }).player)) ||
    entry.player ||
    (entry.values?.player as string) ||
    (metadata.target?.ref &&
      ("Active" in metadata.target.ref
        ? metadata.target.ref.Active?.player
        : "Inactive" in metadata.target.ref
          ? metadata.target.ref.Inactive?.player
          : (metadata.target.ref as unknown as { player?: string }).player));

  if (primaryPlayerId && isWildPlayer(state, primaryPlayerId)) {
    flags.push("wild");
  }

  const combinatoricTags = tags.filter((t) => {
    const rawTag = t.split(":")[0];
    if (title === "timer" && rawTag === "player") {
      return true;
    }
    return (
      !rules.excludeTags.includes(rawTag) && !rules.excludeTags.includes(rawTag.replace(/_/g, ""))
    );
  });
  const combinatoricFlags = flags.filter((f) => {
    if (title === "timer" && f === "turn") {
      return true;
    }
    return !rules.excludeTags.includes(f) && !rules.excludeTags.includes(f.replace(/_/g, ""));
  });

  if (state?.battle_type) {
    combinatoricTags.push(`battletype:${state.battle_type.toLowerCase()}`);
  }

  const category = resolveCategory(title, tags, entry.title, extensionSource);

  const patterns = generateCombinatorics(
    title,
    Array.from(new Set(combinatoricTags)),
    Array.from(new Set(combinatoricFlags)),
  );

  return {
    patterns,
    category,
    context,
    effect: primaryEffect ?? undefined,
    source_effect: sourceEffect ?? undefined,
    metadata: Object.keys(metadata).length > 0 ? metadata : undefined,
    extension: extensionSource ?? undefined,
  };
}

export function getLogPatterns(entry: UiLogEntry): string[] {
  return mapUiLogEntry(entry)?.patterns || [];
}
