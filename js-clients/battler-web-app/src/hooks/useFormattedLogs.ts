import { useMemo, useRef } from "react";
import type { BattleState, UiLogEntry } from "battler-state";
import type { FormattedLogDisplayItem } from "../utils/logFormatter";
import { formatUiLogEntry } from "../utils/logFormatter";

export interface UseFormattedLogsOptions {
  battleId?: string | null;
  uiLogs?: UiLogEntry[];
  battleState?: BattleState | null;
  localPlayerId?: string | null;
  isSpectator?: boolean;
}

export function useFormattedLogs({
  battleId,
  uiLogs = [],
  battleState,
  localPlayerId,
  isSpectator = false,
}: UseFormattedLogsOptions): FormattedLogDisplayItem[] {
  const logCacheRef = useRef<{
    battleId: string | null;
    localPlayerId: string | null | undefined;
    isSpectator: boolean;
    cachedCount: number;
    items: FormattedLogDisplayItem[];
  }>({
    battleId: null,
    localPlayerId: undefined,
    isSpectator: false,
    cachedCount: 0,
    items: [],
  });

  return useMemo(() => {
    if (!battleState || !uiLogs.length) {
      logCacheRef.current = {
        battleId: null,
        localPlayerId: undefined,
        isSpectator: false,
        cachedCount: 0,
        items: [],
      };
      return [];
    }

    const currentKeyMatches =
      logCacheRef.current.battleId === (battleId ?? null) &&
      logCacheRef.current.localPlayerId === localPlayerId &&
      logCacheRef.current.isSpectator === isSpectator;

    if (!currentKeyMatches || uiLogs.length < logCacheRef.current.cachedCount) {
      const items = uiLogs.flatMap((e) =>
        formatUiLogEntry(e, battleState, {
          localPlayerId: localPlayerId || undefined,
          isSpectator,
        }),
      );
      logCacheRef.current = {
        battleId: battleId ?? null,
        localPlayerId,
        isSpectator,
        cachedCount: uiLogs.length,
        items,
      };
      return items;
    }

    if (uiLogs.length > logCacheRef.current.cachedCount) {
      const newEntries = uiLogs.slice(logCacheRef.current.cachedCount);
      const newItems = newEntries.flatMap((e) =>
        formatUiLogEntry(e, battleState, {
          localPlayerId: localPlayerId || undefined,
          isSpectator,
        }),
      );
      const updatedItems = logCacheRef.current.items.concat(newItems);
      logCacheRef.current.cachedCount = uiLogs.length;
      logCacheRef.current.items = updatedItems;
      return updatedItems;
    }

    return logCacheRef.current.items;
  }, [battleState, uiLogs, battleId, localPlayerId, isSpectator]);
}
