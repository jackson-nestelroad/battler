import type { Request } from "battler-types";

export interface MonLike {
  species?: string | null;
  summary?: { name?: string; level?: number } | null;
  player_team_position?: number | null;
  team_position?: number | null;
  player_active_position?: number | null;
  hp?: number | null;
}

/**
 * Returns the display name for a Pokémon (custom nickname or species fallback).
 */
export function getMonDisplayName(mon?: MonLike | null): string {
  if (!mon) return "";
  return mon.summary?.name || mon.species || "";
}

/**
 * Returns the 0-indexed team position for a Pokémon with safe fallbacks.
 */
export function getMonTeamPosition(mon?: MonLike | null, fallbackIndex: number = 0): number {
  if (!mon) return fallbackIndex;
  if (typeof mon.player_team_position === "number") return mon.player_team_position;
  if (typeof mon.team_position === "number") return mon.team_position;
  return fallbackIndex;
}

/**
 * Formats a clean slot label string e.g. "Slot 1: Pikachu" or "Slot 1".
 */
export function getSlotLabel(slotNumber: number, monName?: string | null): string {
  if (monName) {
    return `Slot ${slotNumber}: ${monName}`;
  }
  return `Slot ${slotNumber}`;
}

/**
 * Gets the total number of slots required for a battle request.
 */
export function getRequestSlotCount(request: Request | null): number {
  if (!request) return 0;
  if (request.type === "turn") {
    return request.active?.length || 0;
  }
  if (request.type === "switch") {
    return request.needs_switch?.length || 0;
  }
  return 0;
}

/**
 * Resolves the target Pokémon for a specific slot index in a turn or switch request.
 * Returns the active Mon for turn requests and mid-turn switches (e.g. U-turn),
 * or null for faint switches where the field slot is empty.
 */
export function getMonForSlot(
  playerData: { mons?: MonLike[] } | null | undefined,
  request: Request | null | undefined,
  slotIndex: number,
): MonLike | null {
  if (!playerData?.mons || !request) return null;

  if (request.type === "turn") {
    const req = request.active?.[slotIndex];
    if (!req) return null;
    return playerData.mons.find((m) => getMonTeamPosition(m, -1) === req.team_position) || null;
  }

  if (request.type === "switch") {
    const activePos = request.needs_switch?.[slotIndex];
    if (activePos === undefined) return null;

    // Returns the mon currently active in activePos (e.g. U-turn / Volt Switch),
    // or null if the slot is empty because a mon fainted.
    return playerData.mons.find((m) => m.player_active_position === activePos) || null;
  }

  return null;
}

/**
 * Determines whether a slot can perform a Shift action.
 */
export function canSlotShift(
  currentSlotIndex: number,
  activeRequestsCount: number,
  isTrapped: boolean = false,
): boolean {
  if (isTrapped || activeRequestsCount <= 2) return false;
  const centerSlotIndex = Math.floor((activeRequestsCount - 1) / 2);
  return currentSlotIndex !== centerSlotIndex;
}
