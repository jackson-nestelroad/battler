import type { UiLogEntry, BattleState } from "battler-state";

type MonRef =
  | { Active: { side: number; position: number } }
  | { Inactive: { name?: string } };

function resolveMonName(monRef: MonRef | undefined, state?: BattleState): string {
  if (!monRef) return "Mon";
  if ("Active" in monRef && monRef.Active) {
    if (!state) return "Mon";
    const { side: sideIdx, position } = monRef.Active;
    const side = state.field?.sides?.[sideIdx];
    if (!side) return "Mon";
    const activeRef = side.active?.[position];
    if (!activeRef) return "Mon";
    const player = side.players?.[activeRef.player];
    if (!player) return "Mon";
    const mon = player.mons?.[activeRef.mon_index];
    return mon?.physical_appearance?.name || "Mon";
  } else if ("Inactive" in monRef && monRef.Inactive) {
    return monRef.Inactive.name || "Mon";
  }
  return "Mon";
}

interface LogEntryData {
  mon?: MonRef;
  name?: string;
  effect?: {
    target?: MonRef;
    source?: MonRef;
    source_effect?: {
      name?: string;
    } | null;
  } | null;
  stat?: string;
  by?: number | bigint;
  player?: string;
  item?: string;
  target?: MonRef;
  side?: number;
  title?: string;
}

export function formatUiLogEntry(
  entry: UiLogEntry,
  state?: BattleState,
  _lang = "en",
): string | null {
  if (typeof entry === "string") {
    if (entry === "TurnLimit") return "The turn limit has been reached.";
    if (entry === "Tie") return "The battle ended in a tie!";
    return entry;
  }

  const key = Object.keys(entry)[0];
  const data = (entry as Record<string, LogEntryData>)[key];

  // Simple translations mapping (English only to start)
  switch (key) {
    case "Move": {
      const monName = resolveMonName(data.mon, state);
      return `${monName} used ${data.name}!`;
    }
    case "Damage": {
      const targetName = resolveMonName(data.effect?.target, state);
      const sourceEffect = data.effect?.source_effect?.name;
      if (sourceEffect) {
        return `${targetName} took damage from ${sourceEffect}!`;
      }
      return `${targetName} took damage!`;
    }
    case "Heal": {
      const targetName = resolveMonName(data.effect?.target, state);
      return `${targetName} recovered HP!`;
    }
    case "Faint": {
      const targetName = resolveMonName(data.effect?.target, state);
      return `${targetName} fainted!`;
    }
    case "StatBoost": {
      const monName = resolveMonName(data.mon, state);
      const stat = data.stat;
      const by = Number(data.by || 0);
      if (by < 0) {
        return `${monName}'s ${stat} fell!`;
      }
      return `${monName}'s ${stat} rose!`;
    }
    case "Switch": {
      return `${data.player} switched in Mon index ${data.mon}!`;
    }
    case "SwitchOut": {
      const monName = resolveMonName(data.mon, state);
      return `${monName} switched out.`;
    }
    case "UseItem": {
      const targetName = data.target ? resolveMonName(data.target, state) : null;
      if (targetName) {
        return `${data.player} used ${data.item} on ${targetName}.`;
      }
      return `${data.player} used ${data.item}.`;
    }
    case "Win": {
      return `Player ${data.side} won the battle!`;
    }
    case "CannotEscape": {
      return `${data.player} cannot escape!`;
    }
    case "Caught": {
      return "The Mon was caught!";
    }
    case "Revive": {
      const targetName = resolveMonName(data.effect?.target, state);
      return `${targetName} was revived!`;
    }
    case "SetHealth": {
      const targetName = resolveMonName(data.effect?.target, state);
      return `${targetName}'s HP was set.`;
    }
    case "Transform": {
      const targetName = resolveMonName(data.target, state);
      const sourceName = resolveMonName(data.effect?.source, state);
      return `${sourceName} transformed into ${targetName}!`;
    }
    case "UpdateAppearance": {
      const targetName = resolveMonName(data.effect?.target, state);
      return `${targetName} changed form!`;
    }
    case "Waiting": {
      return `Waiting for turn resolution...`;
    }
    case "Debug": {
      return `[Debug] ${data.title}`;
    }
    default:
      return null;
  }
}
