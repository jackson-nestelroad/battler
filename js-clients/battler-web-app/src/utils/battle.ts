import type { BattleState } from "battler-state";
import type { Battle } from "battler-service-client";
import type { ProposedBattleWithDetails } from "../store/proposalsSlice";

export function getOpponentName(
  playerId: string | null,
  battleState?: BattleState | null,
  serviceBattle?: Battle | null,
  proposal?: ProposedBattleWithDetails | null,
): string {
  if (!playerId) return "Opponent";
  const playerLower = playerId.toLowerCase();

  const opposingSide =
    battleState?.field?.sides?.find((side) => side.name?.toLowerCase() !== playerLower) ||
    serviceBattle?.sides?.find((side) => side.name?.toLowerCase() !== playerLower) ||
    proposal?.sides?.find((side) => side.name?.toLowerCase() !== playerLower);

  return opposingSide?.name || "Opponent";
}
