import type { Battle, BattlePreview } from "battler-service-client";
import { type BattleState, stateSelectors } from "battler-state";
import type { PlayerBattleData } from "battler-types";

export function isMonFaintedInState(
  battleState: BattleState | null | undefined,
  sideIndex: number,
  activePosition: number,
  playerData?: PlayerBattleData | null,
): boolean {
  const playerSide = playerData?.side ?? 0;
  if (sideIndex === playerSide && playerData?.mons) {
    const mon = playerData.mons.find((m) => m.player_active_position === activePosition);
    if (!mon || !mon.active || (mon.hp ?? 0) <= 0) {
      return true;
    }
  }

  if (battleState) {
    try {
      const activeRef = stateSelectors.activeMonByPosition(battleState, sideIndex, activePosition);
      if (!activeRef) return true;
      return stateSelectors.monIsFainted(battleState, activeRef);
    } catch {
      const side = battleState.field?.sides?.[sideIndex];
      const activeRef = side?.active?.[activePosition];
      if (!activeRef) return true;
      const player = side?.players?.[activeRef.player];
      const mon = player?.mons?.[activeRef.mon_index];
      if (!mon || mon.fainted) {
        return true;
      }
    }
  }

  return false;
}

export function isMonDynamaxedInState(
  battleState: BattleState | null | undefined,
  sideIndex: number,
  activePosition: number,
): boolean {
  if (!battleState) return false;
  try {
    const activeRef = stateSelectors.activeMonByPosition(battleState, sideIndex, activePosition);
    if (!activeRef) return false;
    return stateSelectors.monIsDynamaxed(battleState, activeRef);
  } catch {
    return false;
  }
}

export function isMonActiveOnField(
  battleState: BattleState | null | undefined,
  sideIndex: number,
  playerId: string,
  monIndex: number,
  fallbackActive?: boolean,
): boolean {
  if (!battleState) return !!fallbackActive;
  const side = stateSelectors.side(battleState, sideIndex);
  const player = side?.players?.[playerId];
  if (player?.left_battle) return false;

  return (
    side?.active?.some(
      (active) =>
        active != null &&
        active.player === playerId &&
        active.mon_index === monIndex,
    ) ?? !!fallbackActive
  );
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

export interface BattleSessionLike {
  battleState?: BattleState | null;
  serviceBattle?: Battle | null;
  preview?: BattlePreview | null;
  isReplay?: boolean;
}

export function isBattleFinished(session?: BattleSessionLike | null): boolean {
  if (!session) return false;
  if (session.isReplay) {
    return session.battleState?.phase === "finished";
  }
  return (
    session.battleState?.phase === "finished" ||
    session.serviceBattle?.state === "finished" ||
    session.preview?.state === "finished"
  );
}

export function isBattlePreparing(session?: BattleSessionLike | null): boolean {
  if (!session || session.isReplay) return false;
  if (session.battleState) {
    return session.battleState.phase === "pre_battle";
  }
  return (
    session.serviceBattle?.state === "preparing" ||
    session.preview?.state === "preparing"
  );
}

export function getBattleTurnNumber(session?: BattleSessionLike | null): number {
  if (!session) return 0;
  const turn = session.battleState?.turn ?? session.preview?.turn;
  return turn != null ? Number(turn) : 0;
}

export function getBattleSessionStateLabel(session?: BattleSessionLike | null): string {
  if (!session) return "Preview";
  const isFinished = isBattleFinished(session);
  const isPreparing = isBattlePreparing(session);
  const turnNumber = getBattleTurnNumber(session);
  const state = isFinished
    ? "finished"
    : isPreparing
      ? "preparing"
      : session.serviceBattle?.state || session.preview?.state || "active";

  return getBattleStateLabel({
    state,
    phase: session.battleState?.phase,
    turn: turnNumber,
  });
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

export function getMonFromActiveRef(
  side: any,
  activeRef: { player: string; mon_index: number } | null | undefined,
) {
  if (!side || !activeRef) return null;
  return side.players?.[activeRef.player]?.mons?.[activeRef.mon_index] ?? null;
}

export function getMonNameFromState(
  state: BattleState | null | undefined,
  sideIdx: number,
  pos: number,
): string | null {
  const { side, activeRef } = getActiveRefFromState(state, sideIdx, pos);
  const mon = getMonFromActiveRef(side, activeRef);
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

export function getAllyPlayerIds(
  battleState: BattleState | null | undefined,
  playerId: string | null | undefined,
): string[] {
  if (!battleState?.field?.sides || !playerId) return [];
  for (const side of battleState.field.sides) {
    if (side.players && playerId in side.players) {
      return Object.keys(side.players).filter((id) => id !== playerId);
    }
  }
  return [];
}
