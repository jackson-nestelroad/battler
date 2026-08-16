import type { BattleType, MoveTarget, PlayerBattleData } from "battler-types";
import type { BattleState } from "battler-state";

export interface TargetOption {
  value: number;
  monName: string;
  playerName?: string | null;
  label: string;
  type: "foe" | "ally" | "self";
  position: number;
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

export function getMonNameFromState(
  state: BattleState | null | undefined,
  sideIdx: number,
  pos: number,
): string | null {
  const side = state?.field?.sides?.[sideIdx];
  const activeRef = side?.active?.[pos];
  if (!activeRef) return null;
  const player = side?.players?.[activeRef.player];
  const mon = player?.mons?.[activeRef.mon_index];
  return mon?.physical_appearance?.name || null;
}

export function getPlayerNameFromState(
  state: BattleState | null | undefined,
  sideIdx: number,
  pos: number,
): string | null {
  const side = state?.field?.sides?.[sideIdx];
  if (!side) return null;
  const activeRef = side.active?.[pos];
  if (activeRef && activeRef.player !== undefined) {
    const playerName = side.players?.[activeRef.player]?.name;
    if (playerName) return playerName;
  }
  return side.name || null;
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

  let activePerPlayer = 2;
  if (battleType === "Triples") activePerPlayer = 3;
  else if (battleType === "Doubles") activePerPlayer = 2;
  else if (battleType === "Singles") activePerPlayer = 1;
  else if (activeRequestsCount > 1) activePerPlayer = activeRequestsCount;

  const targets: TargetOption[] = [];

  // 1. Foe targets (Side index 1 in battle state)
  if (info.canTargetFoe) {
    for (let pos = 0; pos < activePerPlayer; pos++) {
      if (info.isAdjacentOnly && !isAdjacent(currentSlotIndex, pos, true, activePerPlayer)) {
        continue;
      }
      const targetVal = pos + 1;
      const stateFoeName = getMonNameFromState(battleState, 1, pos);
      const foeMonName = stateFoeName || `Foe ${pos + 1}`;
      const foePlayerName = getPlayerNameFromState(battleState, 1, pos);
      const subText = foePlayerName ? `Foe • ${foePlayerName}` : "Foe";
      targets.push({
        value: targetVal,
        monName: foeMonName,
        playerName: foePlayerName,
        label: `${foeMonName} (${subText})`,
        type: "foe",
        position: pos,
      });
    }
  }

  // 2. Self target
  if (info.canTargetSelf) {
    const selfTargetVal = -1 * (currentSlotIndex + 1);
    const selfMon = playerData?.mons?.find((m) => m.player_active_position === currentSlotIndex);
    const stateSelfName = getMonNameFromState(battleState, 0, currentSlotIndex);
    const selfMonName = selfMon?.summary?.name || selfMon?.species || stateSelfName || "Self";
    const selfPlayerName = playerData?.name || getPlayerNameFromState(battleState, 0, currentSlotIndex);
    targets.push({
      value: selfTargetVal,
      monName: selfMonName,
      playerName: selfPlayerName,
      label: `Self (${selfMonName})`,
      type: "self",
      position: currentSlotIndex,
    });
  }

  // 3. Ally targets (excluding self, Side index 0 in battle state)
  if (info.canTargetAlly) {
    for (let pos = 0; pos < activePerPlayer; pos++) {
      if (pos === currentSlotIndex) continue;
      if (info.isAdjacentOnly && !isAdjacent(currentSlotIndex, pos, false, activePerPlayer)) {
        continue;
      }
      const allyTargetVal = -1 * (pos + 1);
      const allyMon = playerData?.mons?.find((m) => m.player_active_position === pos);
      const stateAllyName = getMonNameFromState(battleState, 0, pos);
      const allyMonName = allyMon?.summary?.name || allyMon?.species || stateAllyName || `Ally ${pos + 1}`;
      const allyPlayerName = getPlayerNameFromState(battleState, 0, pos);
      const subText = allyPlayerName ? `Ally • ${allyPlayerName}` : "Ally";
      targets.push({
        value: allyTargetVal,
        monName: allyMonName,
        playerName: allyPlayerName,
        label: `${allyMonName} (${subText})`,
        type: "ally",
        position: pos,
      });
    }
  }

  return targets;
}
