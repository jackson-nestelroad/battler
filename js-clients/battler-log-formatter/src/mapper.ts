import type { UiLogEntry, BattleState, UiMon } from "battler-state";
import { activeMonByPosition, monPhysicalAppearance } from "battler-state/state_selectors.js";
import i18next from "./i18n.js";
import { LogCategory } from "./types.js";
import type { MapperOptions, AnyMappedLog, ContextVar } from "./types.js";


function getPlayerName(state: BattleState | undefined, playerId: string | undefined): string {
  if (!playerId) return "";
  if (state?.field?.sides) {
    for (const side of state.field.sides) {
      if (side?.players?.[playerId]) {
        return side.players[playerId].name;
      }
    }
  }
  return playerId;
}

function getSideName(state: BattleState | undefined, sideIndex: number | undefined): string {
  if (sideIndex == null) return "";
  if (state?.field?.sides?.[sideIndex]) {
    return state.field.sides[sideIndex].name;
  }
  return `Player ${sideIndex}`;
}

function resolveMonContext(monRef: UiMon | undefined, state: BattleState | undefined, options: MapperOptions): ContextVar {
  if (!monRef) return { text: "Mon" };
  
  let name = "Mon";
  let playerId = "";
  let id = "";

  if ("Active" in monRef && monRef.Active) {
    if (monRef.Active.name) {
      name = monRef.Active.name;
    }
    if (monRef.Active.player) {
      playerId = monRef.Active.player;
    }
    const { position } = monRef.Active;
    if (playerId) {
      id = `${playerId}-active-${position}`;
    }

  } else if ("Inactive" in monRef && monRef.Inactive) {
    name = monRef.Inactive.name || "Mon";
    playerId = monRef.Inactive.player || "";
  }

  // Determine perspective string
  const isAlly = options.localPlayerId === playerId;
  const playerName = getPlayerName(state, playerId) || playerId;
  let text = name;
  let noAutoCapitalize = false;
  
  if (playerId && !isAlly) {
    if (options.foeFormat === "possessive") {
      text = i18next.t("mon.foe_possessive", { name, player: playerName });
      noAutoCapitalize = true; // Starts with player name
    } else if (options.foeFormat === "withPlayer") {
      text = i18next.t("mon.foe_with_player", { name, player: playerName });
    } else {
      text = i18next.t("mon.foe", { name });
    }
  } else {
    if (options.allyFormat === "possessive") {
      text = i18next.t("mon.ally_possessive", { name });
    } else {
      text = i18next.t("mon.ally", { name });
    }
  }
  
  return { text, id, noAutoCapitalize };
}

export function mapUiLogEntry(entry: UiLogEntry, state?: BattleState, options: MapperOptions = {}): AnyMappedLog | null {
  if (typeof entry === "string") {
    switch (entry) {
      case "TurnLimit": return { key: "TURN_LIMIT", category: LogCategory.Primary, context: {} };
      case "Tie": return { key: "TIE", category: LogCategory.Primary, context: {} };
      default: return null;
    }
  }

  const keyStr = Object.keys(entry)[0];
  const key = keyStr.toLowerCase();
  const data = (entry as Record<string, any>)[keyStr];

  const mapped = (() => {
  switch (key) {
    case "move": {
      return {
        key: "MOVE",
        category: LogCategory.Primary,
        context: {
          MON: resolveMonContext(data.mon, state, options),
          MOVE: data.name
        }
      };
    }
    case "damage": {
      const target = resolveMonContext(data.effect?.target, state, options);
      const effectName = data.effect?.source_effect?.name;
      if (effectName) {
        return {
          key: "DAMAGE_FROM_EFFECT",
          category: LogCategory.Secondary,
          context: { TARGET: target, EFFECT: effectName }
        };
      }
      return {
        key: "DAMAGE",
        key: "damage",
        category: LogCategory.Secondary,
        context: { TARGET: target }
      };
    }
    case "heal": {
      return {
        key: "heal",
        category: LogCategory.Secondary,
        context: { MON: resolveMonContext(data.mon, state, options) }
      };
    }
    case "sethp": {
      return {
        key: "sethp",
        category: LogCategory.Secondary,
        context: { MON: resolveMonContext(data.mon, state, options) }
      };
    }
    case "faint": {
      return {
        key: "faint",
        category: LogCategory.Primary,
        context: { TARGET: resolveMonContext(data.effect?.target, state, options) }
      };
    }
    case "statboost": {
      const isDrop = (data.by || 0) < 0;
      return {
        key: isDrop ? "stat_drop" : "stat_boost",
        category: LogCategory.Secondary,
        context: {
          MON: resolveMonContext(data.mon, state, options),
          STAT: data.stat
        }
      };
    }
    case "switchin":
    case "switch":
    case "replace": {
      return {
        key: "switch",
        category: LogCategory.Primary,
        context: { PLAYER: { text: getPlayerName(state, data.player), noAutoCapitalize: true }, MON: { text: data.name || "Mon" } }
      };
    }
    case "switchout": {
      return {
        key: "switchout",
        category: LogCategory.Primary,
        context: { MON: resolveMonContext(data.mon, state, options) }
      };
    }
    case "useitem": {
      if (data.target) {
        return {
          key: "useitem",
          category: LogCategory.Primary,
          context: { PLAYER: { text: getPlayerName(state, data.player), noAutoCapitalize: true }, ITEM: data.item, TARGET: resolveMonContext(data.target, state, options) }
        };
      }
      return {
        key: "useitem",
        category: LogCategory.Primary,
        context: { PLAYER: { text: getPlayerName(state, data.player), noAutoCapitalize: true }, ITEM: data.item }
      };
    }
    case "win": {
      return { key: "win", category: LogCategory.Primary, context: { SIDE: { text: getSideName(state, data.side), noAutoCapitalize: true } } };
    }
    case "debug": {
      return { key: "debug", category: LogCategory.Hint, context: { TITLE: data.title || "Unknown" } };
    }
    case "waiting": {
      return { key: "waiting", category: LogCategory.Hint, context: {} };
    }
    case "cannotescape":
      return { key: "cannot_escape", category: LogCategory.Primary, context: { PLAYER: { text: getPlayerName(state, data.player), noAutoCapitalize: true } } };
    case "leave": {
      if (data.title === "forfeited") return { key: "forfeited", category: LogCategory.Primary, context: { PLAYER: { text: getPlayerName(state, data.player), noAutoCapitalize: true } } };
      return null;
    }
    default: {
      const effectData = data.effect || {};
      const mon = resolveMonContext(effectData.target || data.mon, state, options);
      const source = resolveMonContext(effectData.source, state, options);
      const effectName = effectData.effect?.name || effectData.additional?.condition || effectData.source_effect?.name || data.item || data.ability;

      const title = data.title || key;
      const primaryEffects = ["learnedmove", "didnotlearnmove", "levelup", "mega", "primal", "ultra", "revertmega", "revertprimal", "revertultra", "revertgigantamax", "specieschange", "revive", "prepare", "status", "uncatchable", "tera", "cannot_escape", "forfeited"];
      const category = primaryEffects.includes(title) ? LogCategory.Primary : LogCategory.Secondary;
      
      const context: Record<string, any> = { ...data, MON: mon, EFFECT: effectName };
      if (data.move || effectData.additional?.move || effectData.effect?.name) context.MOVE = data.move || effectData.additional?.move || effectData.effect?.name;
      if (effectData.additional?.status || effectData.effect?.name) context.STATUS = effectData.additional?.status || effectData.effect?.name;
      if (effectData.additional?.types || effectData.effect?.name) context.TYPE = effectData.additional?.types || effectData.effect?.name;
      if (effectData.additional?.hits) context.COUNT = effectData.additional?.hits;
      if (data.ability) context.ABILITY = data.ability;
      if (data.item) context.ITEM = data.item;
      if (effectData.side !== undefined) context.SIDE = `Side ${effectData.side}`;
      if (data.weather) context.WEATHER = data.weather;
      if (data.level) context.LEVEL = data.level.toString();
      if (data.exp) context.EXP = data.exp.toString();

      return { key: title, category, context };
    }
    case "Transform":
      return { key: "TRANSFORM", category: LogCategory.Primary, context: { SOURCE: resolveMonContext(data.target, state, options), TARGET: data.effect?.additional?.species || data.effect?.effect?.name || "Unknown" } };
    case "UpdateAppearance":
      return { key: "SPECIES_CHANGE", category: LogCategory.Primary, context: { MON: resolveMonContext(data.effect?.target, state, options) } };
    case "Revive":
      return { key: "REVIVE", category: LogCategory.Primary, context: { MON: resolveMonContext(data.mon, state, options) } };
    case "SetHealth":
      return { key: "SET_HEALTH", category: LogCategory.Primary, context: { TARGET: resolveMonContext(data.effect?.target, state, options) } };
    case "Caught":
      return { key: "CAUGHT", category: LogCategory.Primary, context: { MON: resolveMonContext(data.effect?.target, state, options) } };
    case "Experience":
      return { key: "EXP_GAIN", category: LogCategory.Hint, context: { MON: resolveMonContext(data.mon, state, options) || "Mon", EXP: data.exp.toString() } };
    case "LevelUp":
      return { key: "LEVEL_UP", category: LogCategory.Primary, context: { MON: resolveMonContext(data.mon, state, options) || "Mon", LEVEL: data.level.toString() } };
    case "MoveUpdate":
      if (data.learned) {
        return { key: "LEARNED_MOVE", category: LogCategory.Primary, context: { MON: resolveMonContext(data.mon, state, options) || "Mon", MOVE: data.move_name } };
      }
      return { key: "DID_NOT_LEARN_MOVE", category: LogCategory.Hint, context: { MON: resolveMonContext(data.mon, state, options) || "Mon", MOVE: data.move_name } };
    case "Extension": {
      return null;
    }
  }
  })() as AnyMappedLog | null;

  if (mapped && data.effect) {
    mapped.effect = data.effect;
  }
  return mapped;
}
