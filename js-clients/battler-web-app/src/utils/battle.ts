import type { BattleState, UiLogEntry } from "battler-state";
import type { Battle } from "battler-service-client";
import type { ProposedBattleWithDetails } from "../store/proposalsSlice";

export function getBattleTitle(
  battleState?: BattleState | null,
  serviceBattle?: Battle | null,
  proposal?: ProposedBattleWithDetails | null,
): string {
  const side0Name =
    battleState?.field?.sides?.[0]?.name ||
    serviceBattle?.sides?.[0]?.name ||
    proposal?.sides?.[0]?.name ||
    "Side 1";

  const side1Name =
    battleState?.field?.sides?.[1]?.name ||
    serviceBattle?.sides?.[1]?.name ||
    proposal?.sides?.[1]?.name ||
    "Side 2";

  return `${side0Name} vs ${side1Name}`;
}

export function formatDeletionReason(reason: string | null | undefined): string {
  if (!reason) return "Declined";
  if (reason === "deleted") return "Deleted";
  return reason.charAt(0).toUpperCase() + reason.slice(1);
}

export function getRuleBadgeClass(rule: string): string {
  if (rule.startsWith("-")) return "badge-danger";
  if (rule.startsWith("+")) return "badge-success";
  if (rule.startsWith("!")) return "badge-warning";
  if (rule.includes("=")) return "badge-secondary";
  return "badge-primary";
}

export interface ParsedTimerLog {
  type: "battle" | "player" | "action" | "teampreview";
  playerId?: string;
  remainingSecs: number;
  deadlineSecs: number;
  isWarning: boolean;
  isDone: boolean;
  isInactive?: boolean;
}

export function parseTimerLog(entry: UiLogEntry): ParsedTimerLog | null {
  if (typeof entry !== "object" || entry === null || !("Extension" in entry)) return null;
  const ext = entry.Extension;
  if (ext.source !== "-battlerservice" || ext.title !== "timer") return null;

  const values = ext.values;
  const remainingsecsStr = values["remainingsecs"];
  if (remainingsecsStr === undefined) return null;
  const remainingSecs = parseInt(remainingsecsStr, 10);

  let type: "battle" | "player" | "action" | "teampreview" = "battle";
  let playerId: string | undefined = undefined;

  if ("battle" in values) {
    type = "battle";
  } else if ("player" in values) {
    type = "player";
    playerId = values["player"];
  } else if ("action" in values) {
    type = "action";
    playerId = values["action"];
  } else if ("teampreview" in values) {
    type = "teampreview";
    playerId = values["teampreview"];
  } else {
    return null;
  }

  const isWarning = "warning" in values;
  const isDone = "done" in values || remainingSecs === 0;
  const isInactive = "inactive" in values;

  // Parse absolute deadline timestamp (in seconds)
  const deadlineSecs = values["deadline"] ? parseInt(values["deadline"], 10) : 0;

  return {
    type,
    playerId,
    remainingSecs,
    deadlineSecs,
    isWarning,
    isDone,
    isInactive,
  };
}

export function getPlayerName(
  playerId: string,
  battleState?: BattleState | null,
  serviceBattle?: Battle | null,
): string {
  if (battleState?.field?.sides) {
    for (const side of battleState.field.sides) {
      if (side.players) {
        const p = side.players[playerId];
        if (p) return p.name;
      }
    }
  }
  if (serviceBattle?.sides) {
    for (const side of serviceBattle.sides) {
      if (side.players) {
        const p = side.players.find((player) => player.id === playerId);
        if (p) return p.name;
      }
    }
  }
  return playerId;
}

export function formatSeconds(secs: number): string {
  if (secs < 60) {
    return `${secs}s`;
  }
  const minutes = Math.floor(secs / 60);
  const seconds = secs % 60;
  const paddedSeconds = seconds.toString().padStart(2, "0");
  return `${minutes}:${paddedSeconds}`;
}
