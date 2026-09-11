import type { Request, PlayerBattleData, MonMoveSlotData, MonMoveRequest, SelectReason } from "battler-types";
import type { BattleState } from "battler-state";
import { getMonNameFromState } from "./battleState";

export interface MonLike {
  name?: string | null;
  species?: string | null;
  summary?: { name?: string; level?: number } | null;
  player_team_position?: number | null;
  team_position?: number | null;
  player_active_position?: number | null;
  hp?: number | null;
}

/**
 * Returns the display name for a Mon (custom nickname or species fallback).
 */
export function getMonDisplayName(mon?: MonLike | null): string {
  if (!mon) return "";
  return mon.summary?.name || mon.name || mon.species || "";
}

/**
 * Returns the 0-indexed team position for a Mon with safe fallbacks.
 */
export function getMonTeamPosition(mon?: MonLike | null, fallbackIndex: number = 0): number {
  if (!mon) return fallbackIndex;
  if (typeof mon.player_team_position === "number") return mon.player_team_position;
  if (typeof mon.team_position === "number") return mon.team_position;
  return fallbackIndex;
}

/**
 * Finds a Mon in the player's party by its active field position (if any).
 */
export function getMonByActivePosition(
  playerData: { mons?: MonLike[] } | null | undefined,
  activePos: number,
): MonLike | null {
  if (!playerData?.mons) return null;
  return playerData.mons.find((m) => m.player_active_position === activePos) || null;
}

/**
 * Finds a Mon in the player's party by its team index.
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
export function getRequestSlotCount(request: Request | null | undefined): number {
  if (!request) return 0;
  if (request.type === "turn") {
    return request.active?.length || 0;
  }
  if (request.type === "switch") {
    return request.needs_switch?.length || 0;
  }
  if (request.type === "select") {
    return request.positions?.length || 0;
  }
  return 0;
}

/**
 * Resolves the actual active slot position for a given request and index.
 * Useful for resolving switch or select target slots which may differ from the index.
 */
export function getActiveSlotPosition(
  request: Request | null | undefined,
  slotIndex: number,
): number {
  if (request?.type === "switch" && request.needs_switch) {
    return request.needs_switch[slotIndex] ?? slotIndex;
  }
  if (request?.type === "select" && request.positions) {
    return request.positions[slotIndex]?.position ?? slotIndex;
  }
  return slotIndex;
}

/**
 * Resolves the target Mon for a specific slot index in a turn, switch, or select request.
 * Returns the active Mon for turn requests, mid-turn switches (e.g. U-turn), and select requests (e.g. Revival Blessing),
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

  if (request.type === "switch" || request.type === "select") {
    const activePos = getActiveSlotPosition(request, slotIndex);
    // Returns the mon currently active in activePos (e.g. U-turn / Volt Switch / Revival Blessing),
    // or null if the slot is empty because a mon fainted.
    return getMonByActivePosition(playerData, activePos);
  }

  return null;
}

/**
 * Determines whether a select action is allowed for the given request and slot.
 */
export function canSlotSelect(
  request: Request | null | undefined,
  slotIndex: number,
): boolean {
  return request?.type === "select" && request.positions?.[slotIndex] !== undefined;
}

/**
 * Returns the SelectReason for a given request and slot, if applicable.
 */
export function getSelectReason(
  request: Request | null | undefined,
  slotIndex: number,
): SelectReason | null {
  return request?.type === "select" ? (request.positions?.[slotIndex]?.reason ?? null) : null;
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
 * Resolves the display name of an active Mon, falling back through the state and a default string.
 */
export function resolveActiveMonName(
  playerData: PlayerBattleData | null | undefined,
  battleState: BattleState | null | undefined,
  sideIdx: number,
  pos: number,
  fallbackName: string,
): string {
  const playerSide = playerData?.side ?? 0;
  const mon = sideIdx === playerSide ? getMonByActivePosition(playerData, pos) : null;
  return (
    getMonDisplayName(mon) ||
    getMonNameFromState(battleState, sideIdx, pos) ||
    fallbackName
  );
}

/**
 * Calculates the number of healthy, non-active Mons in the player's party
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
  request: Request | null | undefined,
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
  request: Request | null | undefined,
  slotIndex: number,
  selectedMove: MonMoveSlotData | null = null,
): boolean {
  if (request?.type === "switch") {
    return request.needs_switch?.[slotIndex] !== undefined;
  }
  if (request?.type === "turn" && selectedMove === null) {
    return !!request.active?.[slotIndex];
  }
  return false;
}

/**
 * Resolves the available move list for an active slot, taking modifiers into account.
 */
export function getAvailableMoves(
  activeReq:
    | MonMoveRequest
    | {
        moves?: MonMoveSlotData[];
        z_moves?: (MonMoveSlotData | null)[];
        max_moves?: MonMoveSlotData[];
      }
    | null
    | undefined,
  modifiers: { zmove?: boolean; dyna?: boolean },
): (MonMoveSlotData | null)[] {
  if (modifiers.zmove && activeReq?.z_moves) return activeReq.z_moves;
  if (modifiers.dyna && activeReq?.max_moves) return activeReq.max_moves;
  return activeReq?.moves || [];
}

/**
 * Formats a fallback name for a Mon based on its slot index.
 */
export function getSlotMonName(mon: MonLike | null | undefined, slotIndex: number): string {
  return getMonDisplayName(mon) || `Mon #${slotIndex + 1}`;
}

export interface StatusDisplayInfo {
  code: string;
  label: string;
}

/**
 * Normalizes Mon status strings (whether from engine request IDs like "psn"
 * or battle log condition names like "Poison") into standard 3-letter badge codes and labels.
 */
export function formatStatusBadge(status?: string | null): StatusDisplayInfo | null {
  if (!status) return null;
  const s = status.trim().toLowerCase();
  switch (s) {
    case "psn":
    case "poison":
      return { code: "psn", label: "PSN" };
    case "tox":
    case "bad poison":
    case "toxic":
      return { code: "tox", label: "TOX" };
    case "brn":
    case "burn":
      return { code: "brn", label: "BRN" };
    case "par":
    case "paralysis":
    case "paralyze":
      return { code: "par", label: "PAR" };
    case "slp":
    case "sleep":
    case "asleep":
      return { code: "slp", label: "SLP" };
    case "frz":
    case "freeze":
    case "frozen":
      return { code: "frz", label: "FRZ" };
    case "fnt":
    case "faint":
    case "fainted":
      return { code: "fnt", label: "FNT" };
    default:
      return { code: s, label: status.toUpperCase() };
  }
}

/**
 * Formats a Poké Ball identifier (e.g. "pokeball", "ultra_ball", "masterball") into human-readable display text.
 */
export function formatBallName(ball?: string | null): string {
  if (!ball) return "";
  const lower = ball.toLowerCase().replace(/[-_ ]/g, "");
  if (lower === "pokeball") return "Poké Ball";
  if (lower.endsWith("ball") && lower !== "ball") {
    const prefix = lower.slice(0, -4);
    return prefix.charAt(0).toUpperCase() + prefix.slice(1) + " Ball";
  }
  return ball.charAt(0).toUpperCase() + ball.slice(1);
}

/**
 * Computes a clamped integer percentage [0, 100] of HP remaining.
 */
export function computeHpPercentage(hp: number, maxHp: number): number {
  if (maxHp <= 0) return 0;
  return Math.min(100, Math.max(0, Math.round((hp / maxHp) * 100)));
}

/**
 * Normalizes a status condition string (e.g. "Burn", "brn", "Fainted", "fnt") to its canonical lowercase code (e.g. "brn", "fnt").
 */
export function normalizeStatusCode(status?: string | null): string | null {
  if (!status) return null;
  return formatStatusBadge(status)?.code ?? status.toLowerCase();
}
