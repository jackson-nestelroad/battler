import type { Request, PlayerBattleData, MonMoveSlotData } from "battler-types";
import type { BattleState } from "battler-state";
import { getMonNameFromState } from "./battleState";

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
 * Finds a Pokémon in the player's party by its active field position (if any).
 */
export function getMonByActivePosition(
  playerData: { mons?: MonLike[] } | null | undefined,
  activePos: number,
): MonLike | null {
  if (!playerData?.mons) return null;
  return playerData.mons.find((m) => m.player_active_position === activePos) || null;
}

/**
 * Finds a Pokémon in the player's party by its team index.
 */
export function getMonByTeamPosition(
  playerData: { mons?: MonLike[] } | null | undefined,
  teamPos: number,
): MonLike | null {
  if (!playerData?.mons) return null;
  return playerData.mons.find((m) => getMonTeamPosition(m, -1) === teamPos) || null;
}


/**
 * Formats a clean slot label string e.g. "Slot 1: Pikachu" or "Slot 1".
 */
export function getSlotLabel(slotNumber: number, monName?: string | null, prefix: string = "Slot"): string {
  if (monName) {
    return `${prefix} ${slotNumber}: ${monName}`;
  }
  return `${prefix} ${slotNumber}`;
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
 * Resolves the actual active slot position for a given request and index.
 * Useful for resolving switch target slots which may differ from the index.
 */
export function getActiveSlotPosition(
  request: Request | null | undefined,
  slotIndex: number,
): number {
  if (request?.type === "switch" && request.needs_switch) {
    return request.needs_switch[slotIndex] ?? slotIndex;
  }
  return slotIndex;
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
    return getMonByTeamPosition(playerData, req.team_position);
  }

  if (request.type === "switch") {
    const activePos = getActiveSlotPosition(request, slotIndex);
    if (activePos === undefined) return null;

    // Returns the mon currently active in activePos (e.g. U-turn / Volt Switch),
    // or null if the slot is empty because a mon fainted.
    return getMonByActivePosition(playerData, activePos);
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

/**
 * Resolves the display name of an active Pokémon, falling back through the state and a default string.
 */
export function resolveActiveMonName(
  playerData: PlayerBattleData | null | undefined,
  battleState: BattleState | null | undefined,
  sideIdx: number,
  pos: number,
  fallbackName: string,
): string {
  const mon = sideIdx === 0 ? getMonByActivePosition(playerData, pos) : null;
  return (
    getMonDisplayName(mon) ||
    getMonNameFromState(battleState, sideIdx, pos) ||
    fallbackName
  );
}

/**
 * Calculates the number of healthy, non-active Pokémon in the player's party
 * that haven't already been selected to switch in.
 */
export function getAvailableBenchCount(
  playerData: PlayerBattleData | null | undefined,
  excludedPositions: number[],
): number {
  if (!playerData?.mons) return 0;
  return playerData.mons.filter((m, idx) => {
    const pos = getMonTeamPosition(m, idx);
    return !m.active && (m.hp ?? 0) > 0 && !excludedPositions.includes(pos);
  }).length;
}

/**
 * Determines the target team size for the Team Preview phase.
 */
export function getTeamPreviewTargetSize(
  request: Request | null,
  playerData: PlayerBattleData | null | undefined,
): number {
  if (request?.type !== "team" || !playerData?.mons) return 0;
  const maxTeamSize = request.max_team_size;
  return Math.min(playerData.mons.length, maxTeamSize ?? playerData.mons.length);
}

/**
 * Determines whether a switch action is allowed for the given request and slot.
 */
export function canSlotSwitch(
  request: Request | null,
  slotIndex: number,
  selectedMove: MonMoveSlotData | null,
): boolean {
  if (!request) return false;
  if (request.type === "switch") {
    return request.needs_switch?.[slotIndex] !== undefined;
  }
  if (request.type === "turn" && selectedMove === null) {
    const activeReq = request.active?.[slotIndex];
    return !!(activeReq && !activeReq.trapped);
  }
  return false;
}

/**
 * Resolves the available move list for an active slot, taking modifiers into account.
 */
export function getAvailableMoves(
  activeReq: any,
  modifiers: { zmove?: boolean; dyna?: boolean },
): MonMoveSlotData[] {
  if (modifiers.zmove && activeReq?.z_moves) return activeReq.z_moves;
  if (modifiers.dyna && activeReq?.max_moves) return activeReq.max_moves;
  return activeReq?.moves || [];
}

/**
 * Formats a fallback name for a Pokémon based on its slot index.
 */
export function getSlotMonName(mon: MonLike | null | undefined, slotIndex: number): string {
  return getMonDisplayName(mon) || `Mon #${slotIndex + 1}`;
}
