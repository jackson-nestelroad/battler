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

  const key = Object.keys(entry)[0];
  const data = (entry as Record<string, any>)[key];

  const mapped = (() => {
  switch (key) {
    case "Move": {
      return {
        key: "MOVE",
        category: LogCategory.Primary,
        context: {
          MON: resolveMonContext(data.mon, state, options),
          MOVE: data.name
        }
      };
    }
    case "Damage": {
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
        category: LogCategory.Secondary,
        context: { TARGET: target }
      };
    }
    case "Heal": {
      return {
        key: "HEAL",
        category: LogCategory.Secondary,
        context: { TARGET: resolveMonContext(data.effect?.target, state, options) }
      };
    }
    case "Faint": {
      return {
        key: "FAINT",
        category: LogCategory.Primary,
        context: { TARGET: resolveMonContext(data.effect?.target, state, options) }
      };
    }
    case "StatBoost": {
      const isDrop = (data.by || 0) < 0;
      return {
        key: isDrop ? "STAT_DROP" : "STAT_BOOST",
        category: LogCategory.Secondary,
        context: {
          MON: resolveMonContext(data.mon, state, options),
          STAT: data.stat
        }
      };
    }
    case "SwitchIn":
    case "Switch": {
      let switchName = "Mon";
      if (state && data.player && state.field?.sides) {
        let foundPlayer = null;
        for (const side of state.field.sides) {
          if (side?.players?.[data.player]) {
            foundPlayer = side.players[data.player];
            break;
          }
        }
        if (foundPlayer) {
          const monData = foundPlayer.mons?.[data.mon];
          if (monData?.physical_appearance?.name) {
            switchName = monData.physical_appearance.name;
          }
        }
      }
      return {
        key: "SWITCH_IN",
        category: LogCategory.Primary,
        context: { PLAYER: { text: getPlayerName(state, data.player), noAutoCapitalize: true }, MON: { text: switchName } }
      };
    }
    case "SwitchOut": {
      return {
        key: "SWITCH_OUT",
        category: LogCategory.Primary,
        context: { MON: resolveMonContext(data.mon, state, options) }
      };
    }
    case "UseItem": {
      if (data.target) {
        return {
          key: "USE_ITEM_ON_TARGET",
          category: LogCategory.Primary,
          context: { PLAYER: { text: getPlayerName(state, data.player), noAutoCapitalize: true }, ITEM: data.item, TARGET: resolveMonContext(data.target, state, options) }
        };
      }
      return {
        key: "USE_ITEM",
        category: LogCategory.Primary,
        context: { PLAYER: { text: getPlayerName(state, data.player), noAutoCapitalize: true }, ITEM: data.item }
      };
    }
    case "Win": {
      return { key: "WIN", category: LogCategory.Primary, context: { SIDE: { text: getSideName(state, data.side), noAutoCapitalize: true } } };
    }
    case "Debug": {
      return { key: "DEBUG", category: LogCategory.Hint, context: { TITLE: data.title || "Unknown" } };
    }
    case "Waiting": {
      return { key: "WAITING", category: LogCategory.Hint, context: {} };
    }
    case "Effect": {
      const effectData = data.effect || {};
      const mon = resolveMonContext(effectData.target, state, options);
      const source = resolveMonContext(effectData.source, state, options);
      const effectName = effectData.effect?.name || effectData.additional?.condition || effectData.source_effect?.name;

      switch (data.title) {
        case "activate":
          if (!effectData.target) {
            return { key: "ACTIVATE_FIELD", category: LogCategory.Secondary, context: { EFFECT: effectName } };
          }
          return { key: "ACTIVATE", category: LogCategory.Secondary, context: { MON: mon, EFFECT: effectName } };
        case "fieldactivate":
          return { key: "ACTIVATE_FIELD", category: LogCategory.Secondary, context: { EFFECT: effectName } };
        case "addedtype":
          return { key: "ADDED_TYPE", category: LogCategory.Secondary, context: { MON: mon, TYPE: effectName } };
        case "block":
          return { key: "BLOCK", category: LogCategory.Secondary, context: { MON: mon, EFFECT: effectName } };
        case "end":
          return { key: "END", category: LogCategory.Secondary, context: { MON: mon, EFFECT: effectName } };
        case "fail":
          return { key: "FAIL", category: LogCategory.Secondary, context: {} };
        case "miss":
          return { key: "MISS", category: LogCategory.Secondary, context: { MON: mon } };
        case "supereffective":
          return { key: "SUPEREFFECTIVE", category: LogCategory.Secondary, context: { MON: mon } };
        case "resisted":
          return { key: "RESISTED", category: LogCategory.Secondary, context: { MON: mon } };
        case "crit":
          return { key: "CRIT", category: LogCategory.Secondary, context: { MON: mon } };
        case "ohko":
          return { key: "OHKO", category: LogCategory.Secondary, context: { MON: mon } };
        case "fieldstart":
          return { key: "FIELD_START", category: LogCategory.Secondary, context: { EFFECT: effectName } };
        case "fieldend":
          return { key: "FIELD_END", category: LogCategory.Secondary, context: { EFFECT: effectName } };
        case "item":
          return { key: "ITEM", category: LogCategory.Secondary, context: { MON: mon, ITEM: effectName } };
        case "itemend":
          return { key: "ITEM_END", category: LogCategory.Secondary, context: { MON: mon, ITEM: effectName } };
        case "sidestart":
          return { key: "SIDE_START", category: LogCategory.Secondary, context: { SIDE: effectData.side !== undefined ? `Side ${effectData.side}` : "the side", EFFECT: effectName } };
        case "sideend":
          return { key: "SIDE_END", category: LogCategory.Secondary, context: { SIDE: effectData.side !== undefined ? `Side ${effectData.side}` : "the side", EFFECT: effectName } };
        case "start":
          return { key: "START", category: LogCategory.Secondary, context: { MON: mon, EFFECT: effectName } };
        case "weather":
          return { key: "WEATHER", category: LogCategory.Secondary, context: { WEATHER: effectName } };
        case "clearweather":
          return { key: "CLEAR_WEATHER", category: LogCategory.Secondary, context: {} };
        case "ability":
          return { key: "ABILITY", category: LogCategory.Secondary, context: { MON: mon, ABILITY: effectName } };
        case "abilityend":
          return { key: "ABILITY_END", category: LogCategory.Secondary, context: { MON: mon, ABILITY: effectName } };
        case "immune":
          return { key: "IMMUNE", category: LogCategory.Secondary, context: { MON: mon } };
        case "invertboosts":
          return { key: "INVERT_BOOSTS", category: LogCategory.Secondary, context: { MON: mon } };
        case "learnedmove":
          return { key: "LEARNED_MOVE", category: LogCategory.Primary, context: { MON: mon, MOVE: effectData.additional?.move || effectData.effect?.name || "" } };
        case "prepare":
          return { key: "PREPARE", category: LogCategory.Primary, context: { MON: mon, MOVE: effectData.additional?.move || effectData.effect?.name || "" } };
        case "protectweaken":
          return { key: "PROTECT_WEAKEN", category: LogCategory.Secondary, context: { MON: mon } };
        case "restorepp":
          return { key: "RESTORE_PP", category: LogCategory.Secondary, context: { MON: mon, MOVE: effectData.move || effectData.additional?.move || effectData.effect?.name || "" } };
        case "setpp":
          return { key: "SET_PP", category: LogCategory.Secondary, context: { MON: mon } };
        case "singlemove":
          return { key: "SINGLE_MOVE", category: LogCategory.Secondary, context: { MON: mon, MOVE: effectData.additional?.move || effectData.effect?.name || "" } };
        case "singleturn":
          return { key: "SINGLE_TURN", category: LogCategory.Secondary, context: { MON: mon } };
        case "status":
          return { key: "STATUS", category: LogCategory.Primary, context: { MON: mon, STATUS: effectData.additional?.status || effectData.effect?.name || "" } };
        case "swapboosts":
          return { key: "SWAP_BOOSTS", category: LogCategory.Secondary, context: { MON: mon } };
        case "typechange":
          return { key: "TYPE_CHANGE", category: LogCategory.Secondary, context: { MON: mon, TYPE: effectData.additional?.types || effectData.effect?.name || "" } };
        case "resettypechange":
          return { key: "RESET_TYPE_CHANGE", category: LogCategory.Secondary, context: { MON: mon } };
        case "hitcount":
          return { key: "HIT_COUNT", category: LogCategory.Secondary, context: { COUNT: effectData.additional?.hits || "1" } };
        case "uncatchable":
          return { key: "UNCATCHABLE", category: LogCategory.Primary, context: {} };
        case "cant":
          return { key: "FAIL", category: LogCategory.Secondary, context: {} };
        case "catchfailed":
          return { key: "FAIL", category: LogCategory.Secondary, context: {} };
        case "clearboosts":
        case "clearnegativeboosts":
        case "clearpositiveboosts":
          return { key: "END", category: LogCategory.Secondary, context: { MON: mon, EFFECT: "stat changes" } };
        case "copyboosts":
          return { key: "SWAP_BOOSTS", category: LogCategory.Secondary, context: { MON: mon } };
        case "curestatus":
          return { key: "END", category: LogCategory.Secondary, context: { MON: mon, EFFECT: effectData.effect?.name || "status" } };
        case "deductpp":
          return { key: "SET_PP", category: LogCategory.Secondary, context: { MON: mon } };
        case "dynamax":
          return { key: "START", category: LogCategory.Secondary, context: { MON: mon, EFFECT: "Dynamax" } };
        case "revertdynamax":
          return { key: "END", category: LogCategory.Secondary, context: { MON: mon, EFFECT: "Dynamax" } };
        case "tera":
          return { key: "TERA", category: LogCategory.Primary, context: { MON: mon, TYPE: effectData.effect?.name || "" } };
        case "reverttera":
          return { key: "REVERT_TERA", category: LogCategory.Secondary, context: { MON: mon } };
        case "clearallboosts":
          return { key: "CLEAR_ALL_BOOSTS", category: LogCategory.Secondary, context: {} };

        default:
          return null; // Some effects are purely state-sync and don't render strings
      }
    }
    case "CannotEscape":
      return { key: "CANNOT_ESCAPE", category: LogCategory.Primary, context: { PLAYER: { text: getPlayerName(state, data.player), noAutoCapitalize: true } } };
    case "Leave": {
      if (data.title === "escaped") return { key: "ESCAPED", category: LogCategory.Primary, context: { PLAYER: { text: getPlayerName(state, data.player), noAutoCapitalize: true } } };
      if (data.title === "forfeited") return { key: "FORFEITED", category: LogCategory.Primary, context: { PLAYER: { text: getPlayerName(state, data.player), noAutoCapitalize: true } } };
      return null;
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
    default:
      return null;
  }
  })() as AnyMappedLog | null;

  if (mapped && data.effect) {
    mapped.effect = data.effect;
  }
  return mapped;
}
