import type { BattleType, MoveTarget, PlayerBattleData } from "battler-types";
import type { BattleState } from "battler-state";
import { resolveActiveMonName } from "./monHelpers";
import { getPlayerNameFromState } from "./battleState";

export function parseTargetValue(targetVal: number, currentSlotIndex: number) {
  const isFoe = targetVal > 0;
  const sideIdx = isFoe ? 1 : 0;
  const pos = isFoe ? targetVal - 1 : Math.abs(targetVal) - 1;
  const isSelf = sideIdx === 0 && pos === currentSlotIndex;
  const type: "foe" | "ally" | "self" = isFoe ? "foe" : isSelf ? "self" : "ally";
  return { sideIdx, pos, isSelf, type };
}

export function buildTargetValue(type: "foe" | "ally" | "self", pos: number): number {
  return type === "foe" ? pos + 1 : -1 * (pos + 1);
}

export function getTargetDisplayInfo(
  playerData: PlayerBattleData | null | undefined,
  battleState: BattleState | null | undefined,
  type: "foe" | "ally" | "self",
  pos: number,
) {
  const sideIdx = type === "foe" ? 1 : 0;
  const fallback = type === "self" ? "Self" : `${getTargetTypeLabel(type)} ${pos + 1}`;
  const monName = resolveActiveMonName(playerData, battleState, sideIdx, pos, fallback);
  const playerName =
    type === "self"
      ? playerData?.name || getPlayerNameFromState(battleState, sideIdx, pos)
      : getPlayerNameFromState(battleState, sideIdx, pos);
  const subText = formatTargetSubText(type, playerName);
  const label = type === "self" ? `Self (${monName})` : `${monName} (${subText})`;

  return { monName, playerName, subText, label };
}

export function resolveTargetLabel(
  targetVal: number | null | undefined,
  currentSlotIndex: number,
  battleState?: BattleState | null,
  playerData?: PlayerBattleData | null,
): string | null {
  if (!targetVal) return null;

  const { pos, type } = parseTargetValue(targetVal, currentSlotIndex);
  const { label } = getTargetDisplayInfo(playerData, battleState, type, pos);
  return label;
}

export interface TargetOption {
  value: number;
  monName: string;
  playerName?: string | null;
  label: string;
  subText?: string;
  type: "foe" | "ally" | "self";
  position: number;
}

export function getTargetTypeLabel(type: "foe" | "ally" | "self"): string {
  return type === "self" ? "Self" : type === "foe" ? "Foe" : "Ally";
}

export function formatTargetSubText(
  type: "foe" | "ally" | "self",
  playerName?: string | null,
): string {
  const typeLabel = getTargetTypeLabel(type);
  return playerName ? `${typeLabel} • ${playerName}` : typeLabel;
}

export interface MoveTargetInfo {
  isChoosable: boolean;
  canTargetFoe: boolean;
  canTargetAlly: boolean;
  canTargetSelf: boolean;
  isAdjacentOnly: boolean;
}

export const CHOOSABLE_MOVE_TARGETS: ReadonlySet<string> = new Set([
  "Normal",
  "AdjacentFoe",
  "AdjacentAlly",
  "AdjacentAllyOrUser",
  "Any",
]);

export function getMoveTargetInfo(target: MoveTarget | string): MoveTargetInfo {
  const isChoosable = CHOOSABLE_MOVE_TARGETS.has(target);
  const canTargetFoe = target === "AdjacentFoe" || target === "Normal" || target === "Any";
  const canTargetAlly =
    target === "AdjacentAlly" ||
    target === "AdjacentAllyOrUser" ||
    target === "Normal" ||
    target === "Any";
  const canTargetSelf = target === "AdjacentAllyOrUser";
  const isAdjacentOnly = target !== "Any";

  return {
    isChoosable,
    canTargetFoe,
    canTargetAlly,
    canTargetSelf,
    isAdjacentOnly,
  };
}

/**
 * Determines whether targetPosition is adjacent to userPosition.
 *
 * Adjacency reach rule: distance <= 1.
 * For allies (same side): distance = Math.abs(userPosition - targetPosition) === 1.
 * For foes (opposite side):
 * Foe positions are flipped spatially relative to player side:
 * Foe 0 (Left from foe perspective) is opposite Player 2 (Right from player perspective).
 * effectiveFoePosition = activePerPlayer - targetPosition - 1.
 *
 * For Triples (activePerPlayer === 3):
 * - userPosition 0 (Left): Foe 1 (Center) and Foe 2 (Right) are adjacent. Foe 0 (Left) is NOT adjacent.
 * - userPosition 1 (Center): Foe 0, Foe 1, and Foe 2 are all adjacent.
 * - userPosition 2 (Right): Foe 0 (Left) and Foe 1 (Center) are adjacent. Foe 2 (Right) is NOT adjacent.
 */
export function isAdjacent(
  userPosition: number,
  targetPosition: number,
  isFoe: boolean,
  activePerPlayer: number,
): boolean {
  if (!isFoe) {
    return Math.abs(userPosition - targetPosition) === 1;
  }

  if (activePerPlayer <= 2) {
    return true;
  }

  const effectiveFoePosition = activePerPlayer - targetPosition - 1;
  return Math.abs(userPosition - effectiveFoePosition) <= 1;
}

export function getActivePerPlayer(
  battleType?: BattleType | string | null,
  activeRequestsCount: number = 1,
): number {
  if (battleType === "Triples") return 3;
  if (battleType === "Doubles") return 2;
  if (battleType === "Singles") return 1;
  if (activeRequestsCount > 1) return activeRequestsCount;
  return 2;
}

export interface GetValidTargetsParams {
  moveTarget: string;
  currentSlotIndex: number;
  battleType?: BattleType | string | null;
  activeRequestsCount?: number;
  playerData?: PlayerBattleData | null;
  battleState?: BattleState | null;
}

export function getValidTargets({
  moveTarget,
  currentSlotIndex,
  battleType,
  activeRequestsCount = 1,
  playerData,
  battleState,
}: GetValidTargetsParams): TargetOption[] {
  const info = getMoveTargetInfo(moveTarget);
  if (!info.isChoosable) {
    return [];
  }

  const activePerPlayer = getActivePerPlayer(battleType, activeRequestsCount);

  const targets: TargetOption[] = [];

  const addTarget = (type: "foe" | "ally" | "self", pos: number, targetVal: number) => {
    const { monName, playerName, subText, label } = getTargetDisplayInfo(playerData, battleState, type, pos);
    targets.push({
      value: targetVal,
      monName,
      playerName,
      label,
      subText,
      type,
      position: pos,
    });
  };

  const processSide = (type: "foe" | "ally") => {
    if (type === "foe" && !info.canTargetFoe) return;
    if (type === "ally" && !info.canTargetAlly) return;
    
    for (let pos = 0; pos < activePerPlayer; pos++) {
      if (type === "ally" && pos === currentSlotIndex) continue;
      if (info.isAdjacentOnly && !isAdjacent(currentSlotIndex, pos, type === "foe", activePerPlayer)) {
        continue;
      }
      const targetVal = buildTargetValue(type, pos);
      addTarget(type, pos, targetVal);
    }
  };

  processSide("foe");

  if (info.canTargetSelf) {
    addTarget("self", currentSlotIndex, buildTargetValue("self", currentSlotIndex));
  }

  processSide("ally");

  return targets;
}
