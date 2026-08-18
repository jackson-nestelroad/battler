import type { BattleState, UiLogEntry } from "battler-state";
import type { FormattedUiLog } from "battler-log-formatter";
import { LogFormatter } from "battler-log-formatter";

function getFormatter(localPlayerId?: string): LogFormatter {
  // Try to use a singleton formatter, or re-instantiate if player ID changes
  // For now, just create one on the fly since we need it rarely
  return new LogFormatter({ localPlayerId });
}

export function formatUiLogEntry(
  entry: UiLogEntry,
  state?: BattleState,
  localPlayerId?: string,
): FormattedUiLog | null {
  const formatter = getFormatter(localPlayerId);
  return formatter.format(entry, state);
}
