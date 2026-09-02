import type { Battle } from "battler-service-client";
import type { BattleState, UiLogEntry } from "battler-state";
import type { ProposedBattleWithDetails } from "../store/proposalsSlice";

export function getBattleTitle(
  battleState?: BattleState | null,
  serviceBattle?: Battle | null,
  proposal?: ProposedBattleWithDetails | null,
  isDeleted?: boolean,
): string {
  const side0Name =
    battleState?.field?.sides?.[0]?.name ||
    serviceBattle?.sides?.[0]?.name ||
    proposal?.sides?.[0]?.name;

  const side1Name =
    battleState?.field?.sides?.[1]?.name ||
    serviceBattle?.sides?.[1]?.name ||
    proposal?.sides?.[1]?.name;

  if (side0Name && side1Name) {
    return `${side0Name} vs ${side1Name}`;
  }

  if (isDeleted) {
    return "Deleted Battle";
  }

  return `${side0Name || "Side 1"} vs ${side1Name || "Side 2"}`;
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
  isClear?: boolean;
}

function parseNumericSafe(val: unknown, fallback: number = NaN): number {
  if (typeof val === "number") return isNaN(val) ? fallback : val;
  if (typeof val === "bigint") return Number(val);
  if (val !== undefined && val !== null) {
    const parsed = parseInt(String(val), 10);
    return isNaN(parsed) ? fallback : parsed;
  }
  return fallback;
}

export function parseTimerLog(entry: UiLogEntry): ParsedTimerLog | null {
  if (typeof entry !== "object" || entry === null) return null;
  if (entry.title !== "timer" || !entry.values) return null;
  if (entry.values.source && entry.values.source !== "-battlerservice") return null;

  const values = entry.values as Record<string, unknown>;

  const remainingSecs = parseNumericSafe(values["remainingsecs"]);
  if (isNaN(remainingSecs)) return null;

  let type: "battle" | "player" | "action" | "teampreview" = "battle";
  let playerId: string | undefined = undefined;

  const extractPlayerId = (val: unknown) =>
    (typeof val === "string" && val ? val : entry.player) || undefined;

  if ("battle" in values) {
    type = "battle";
  } else if ("player" in values) {
    type = "player";
    playerId = extractPlayerId(values.player);
  } else if ("action" in values) {
    type = "action";
    playerId = extractPlayerId(values.action);
  } else if ("teampreview" in values) {
    type = "teampreview";
    playerId = extractPlayerId(values.teampreview);
  } else {
    return null;
  }

  const isFlagTruthy = (val: unknown) => val !== undefined && val !== null && val !== false;

  const isWarning = isFlagTruthy(values.warning);
  const isDone = isFlagTruthy(values.done) || remainingSecs === 0;
  const isInactive = isFlagTruthy(values.inactive);
  const isClear = isFlagTruthy(values.clear);

  // Parse absolute deadline timestamp (in seconds)
  const deadlineSecs = parseNumericSafe(values["deadline"], 0);

  return {
    type,
    playerId,
    remainingSecs,
    deadlineSecs,
    isWarning,
    isDone,
    isInactive,
    isClear,
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
