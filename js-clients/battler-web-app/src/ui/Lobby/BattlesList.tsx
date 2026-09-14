import { useCallback, useEffect, useState } from "react";
import type { BattlePreview } from "battler-service-client";
import { fetchBattles, restoreBattleSession } from "../../core/wamp";
import { selectBattle } from "../../store/battlesSlice";
import { useAppDispatch, useAppSelector } from "../../store/store";
import { getBattleStateLabel } from "../../utils/battleState";
import { formatUuid } from "../../utils/uuid";
import CopyableId from "../Common/CopyableId";
import styles from "./BattlesList.module.scss";

const PAGE_SIZE = 10;

function formatSides(sides: BattlePreview["sides"]): string {
  if (!sides || sides.length === 0) return "Unknown Battle";
  return sides
    .map((s, idx) => {
      const names = s.players?.map((p) => p.name || p.id).join(", ") || `Side ${idx + 1}`;
      return names;
    })
    .join(" vs ");
}

interface BattlesListProps {
  refreshTrigger?: number;
}

export default function BattlesList({ refreshTrigger = 0 }: BattlesListProps) {
  const dispatch = useAppDispatch();
  const playerId = useAppSelector((state) => state.connection.playerId || "");
  const connectionStatus = useAppSelector((state) => state.connection.status);
  const isConnected = connectionStatus === "connected";

  const [page, setPage] = useState(1);
  const [battles, setBattles] = useState<BattlePreview[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadPage = useCallback(
    async (pageIndex: number) => {
      if (!isConnected) return;
      setIsLoading(true);
      setError(null);
      try {
        const offset = (pageIndex - 1) * PAGE_SIZE;
        const result = await dispatch(fetchBattles({ count: PAGE_SIZE, offset })).unwrap();
        setBattles(result);
      } catch (err: unknown) {
        console.error("[BattlesList] Failed to fetch battles:", err);
        setError(typeof err === "string" ? err : String(err));
      } finally {
        setIsLoading(false);
      }
    },
    [dispatch, isConnected],
  );

  useEffect(() => {
    if (isConnected) {
      loadPage(page);
    } else {
      setBattles([]);
      setError(null);
    }
  }, [loadPage, isConnected, page, refreshTrigger]);

  const handleWatch = (battleId: string) => {
    restoreBattleSession(battleId, playerId, dispatch);
    dispatch(selectBattle({ view: "battle", battleId }));
  };

  const handlePrevPage = () => {
    if (page > 1) {
      setPage((p) => p - 1);
    }
  };

  const handleNextPage = () => {
    if (battles.length === PAGE_SIZE) {
      setPage((p) => p + 1);
    }
  };

  return (
    <section className="card">
      <div className="card-header">
        <h3>Battles</h3>
      </div>

      {!isConnected || isLoading ? (
        <p className={styles.emptyText}>Loading...</p>
      ) : error ? (
        <p className={styles.emptyText}>{error}</p>
      ) : battles.length === 0 ? (
        <p className={styles.emptyText}>None</p>
      ) : (
        <div className="flex-col gap-xs">
          {battles.map((b) => {
            const battleId = formatUuid(b.uuid);
            const stateStr = getBattleStateLabel({ state: b.state, turn: b.turn });
            return (
              <div
                key={b.uuid}
                className={styles.battleItem}
                onClick={() => handleWatch(battleId)}
                role="button"
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    handleWatch(battleId);
                  }
                }}
              >
                <div className={styles.battleInfo}>
                  <span className={styles.playersText}>{formatSides(b.sides)}</span>
                  <span className={styles.dotSeparator}>•</span>
                  <div className="flex-row align-center gap-xs flex-wrap">
                    <div className={styles.battleSubtitle} onClick={(e) => e.stopPropagation()}>
                      <CopyableId id={battleId} type="battle" />
                    </div>
                    {(b.special || b.battle_type) && (
                      <>
                        <span className={styles.dotSeparator}>•</span>
                        <span className={styles.metaText}>{b.special || b.battle_type}</span>
                      </>
                    )}
                    {stateStr && (
                      <>
                        <span className={styles.dotSeparator}>•</span>
                        <span className={styles.metaText}>{stateStr}</span>
                      </>
                    )}
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {isConnected && (
        <div className={`${styles.paginationFooter} flex-row align-center justify-center gap-m`}>
          <button
            type="button"
            className="btn btn-secondary btn-sm"
            disabled={page <= 1 || isLoading}
            onClick={handlePrevPage}
          >
            Previous
          </button>
          <span className={styles.pageIndicator}>Page {page}</span>
          <button
            type="button"
            className="btn btn-secondary btn-sm"
            disabled={battles.length < PAGE_SIZE || isLoading}
            onClick={handleNextPage}
          >
            Next
          </button>
        </div>
      )}
    </section>
  );
}
