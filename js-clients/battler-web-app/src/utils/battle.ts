import type { BattleState } from "battler-state";
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
