import { type BattleState, stateSelectors } from "battler-state";
import type { PlayerBattleData } from "battler-types";

export function isMonDynamaxedInState(
  battleState: BattleState | null | undefined,
  playerData: PlayerBattleData | { id?: string; name?: string } | null | undefined,
  teamPosition: number,
): boolean {
  if (!battleState || !playerData) return false;
  try {
    const playerKey = playerData.id || playerData.name || "";
    return stateSelectors.monIsDynamaxed(battleState, {
      player: playerKey,
      mon_index: teamPosition,
      battle_appearance_index: 0,
    });
  } catch {
    return false;
  }
}

export interface BattleStateInput {
  state?: string | null;
  phase?: unknown;
  turn?: number | bigint | null;
}

export function getBattleStateLabel(input: BattleStateInput): string {
  const { state, phase, turn } = input;

  if (state === "preparing") {
    return "Preparing";
  }

  const phaseStr =
    typeof phase === "string"
      ? phase
      : phase && typeof phase === "object"
        ? Object.keys(phase)[0]
        : null;

  if (state === "finished" || phaseStr === "finished") {
    return "Finished";
  }

  if (phaseStr === "pre_battle" || turn === 0 || turn === 0n) {
    return "Preview";
  }

  if (turn !== undefined && turn !== null && turn > 0) {
    return `Turn ${turn}`;
  }

  if (state === "active") {
    return "Preview";
  }

  return "Preview";
}

export function getActiveRefFromState(
  state: BattleState | null | undefined,
  sideIdx: number,
  pos: number,
) {
  const side = state?.field?.sides?.[sideIdx];
  const activeRef = side?.active?.[pos];
  return { side, activeRef };
}

export function getMonNameFromState(
  state: BattleState | null | undefined,
  sideIdx: number,
  pos: number,
): string | null {
  const { side, activeRef } = getActiveRefFromState(state, sideIdx, pos);
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
  const { side, activeRef } = getActiveRefFromState(state, sideIdx, pos);
  if (!side) return null;
  if (activeRef && activeRef.player !== undefined) {
    const playerName = side.players?.[activeRef.player]?.name;
    if (playerName) return playerName;
  }
  return side.name || null;
}
