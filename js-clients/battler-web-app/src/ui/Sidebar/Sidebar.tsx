import { useAppDispatch, useAppSelector } from "../../store/store";
import { setCurrentView, switchActiveBattle, removeBattle } from "../../store/battlesSlice";
import type { ActiveView } from "../../store/battlesSlice";
import { disconnectWamp } from "../../core/wamp";
import { BREAKPOINT_MOBILE_PX } from "../../utils/constants";

import styles from "./Sidebar.module.scss";

interface SidebarProps {
  isCollapsed: boolean;
  setIsCollapsed: (collapsed: boolean) => void;
}

export default function Sidebar({ isCollapsed, setIsCollapsed }: SidebarProps) {
  const dispatch = useAppDispatch();
  const connection = useAppSelector((state) => state.connection);
  const { battles, activeBattleId, currentView } = useAppSelector((state) => state.battles);

  const activeBattlesList = Object.values(battles).filter((b) => !b.isReplay);
  const replayBattlesList = Object.values(battles).filter((b) => b.isReplay);

  const handleNav = (view: ActiveView, battleId: string | null = null) => {
    dispatch(setCurrentView(view));
    dispatch(switchActiveBattle(battleId));

    // Automatically close sidebar drawer on navigation clicks on mobile
    if (typeof window !== "undefined" && window.innerWidth <= BREAKPOINT_MOBILE_PX) {
      setIsCollapsed(true);
    }
  };

  return (
    <aside className={`${styles.sidebar} ${isCollapsed ? styles.collapsed : ""}`}>
      <div className={styles.brand}>
        <h2>{isCollapsed ? "BC" : "Battler Console"}</h2>
        <button
          className={styles.toggleBtn}
          onClick={() => setIsCollapsed(!isCollapsed)}
          title={isCollapsed ? "Expand Sidebar" : "Collapse Sidebar"}
        >
          {isCollapsed ? "▶" : "◀"}
        </button>
      </div>

      <div className={styles.statusSection}>
        <div className={styles.statusIndicator}>
          <span className={`${styles.dot} ${styles[connection.status]}`} />
          {!isCollapsed && (
            <span className={styles.statusLabel}>
              {connection.status === "connected"
                ? "Connected"
                : connection.status === "connecting"
                  ? "Connecting..."
                  : "Offline"}
            </span>
          )}
        </div>
        {!isCollapsed && connection.status === "connected" && (
          <div className={styles.userInfo}>
            <div className={styles.playerMeta}>
              <span className={styles.username}>@{connection.playerId}</span>
              {connection.serverUrl && (
                <span className={styles.serverUrl} title={connection.serverUrl}>
                  {connection.serverUrl}
                </span>
              )}
            </div>
            <button className="btn btn-sm btn-danger" onClick={() => dispatch(disconnectWamp())}>
              Disconnect
            </button>
          </div>
        )}
      </div>

      <nav className={styles.nav}>
        <button
          className={`${styles.navItem} ${currentView === "lobby" ? styles.active : ""}`}
          onClick={() => handleNav("lobby")}
          title="Matchmaking Lobby"
        >
          {isCollapsed ? (
            <span className={styles.navIcon}>🌐</span>
          ) : (
            <span className={styles.navLabel}>Matchmaking Lobby</span>
          )}
        </button>
        <button
          className={`${styles.navItem} ${currentView === "teams" ? styles.active : ""}`}
          onClick={() => handleNav("teams")}
          title="Teams Editor"
        >
          {isCollapsed ? (
            <span className={styles.navIcon}>📝</span>
          ) : (
            <span className={styles.navLabel}>Teams Editor</span>
          )}
        </button>
        <button
          className={`${styles.navItem} ${currentView === "replays" ? styles.active : ""}`}
          onClick={() => handleNav("replays")}
          title="Battle Replays"
        >
          {isCollapsed ? (
            <span className={styles.navIcon}>🎬</span>
          ) : (
            <span className={styles.navLabel}>Battle Replays</span>
          )}
        </button>
      </nav>

      {connection.status === "connected" && (
        <div className={styles.battlesSection}>
          {!isCollapsed && <h3>Active Battles</h3>}
          {activeBattlesList.length === 0 ? (
            !isCollapsed && <p className={styles.emptyBattles}>No active battles</p>
          ) : (
            <div className={styles.battlesList}>
              {activeBattlesList.map((battle) => {
                const isSelected = currentView === "battle" && activeBattleId === battle.battleId;
                const hasPendingAction =
                  battle.activeRequest !== null && battle.battleState?.phase !== "finished";
                const opposingSide = battle.battleState?.field?.sides?.find(
                  (side) => side.name !== connection.playerId,
                );
                const opponentName = opposingSide?.name || "Opponent";
                const turnNumber = battle.battleState?.turn || 0;
                const isFinished = battle.battleState?.phase === "finished";

                return (
                  <div
                    key={battle.battleId}
                    className={`${styles.battleItemWrapper} flex-row align-center justify-between w-full`}
                  >
                    <button
                      className={`${styles.battleItem} ${isFinished ? styles.closeableBattleItem : ""} ${isSelected ? styles.selected : ""}`}
                      onClick={() => handleNav("battle", battle.battleId)}
                      title={`Battle vs ${opponentName}`}
                    >
                      <div className={styles.battleMeta}>
                        {isCollapsed ? (
                          <span className={styles.navIcon}>🎮</span>
                        ) : (
                          <>
                            <span className={styles.opponentName}>vs {opponentName}</span>
                            <span className={`${styles.turnLabel} ${isFinished ? styles.finishedLabel : ""}`}>
                              {isFinished ? "Finished" : `Turn ${turnNumber}`}
                            </span>
                          </>
                        )}
                      </div>
                      {hasPendingAction && (
                        <span className={styles.actionBadge} title="Your turn to act!">
                          !
                        </span>
                      )}
                    </button>
                    {!isCollapsed && isFinished && (
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          dispatch(removeBattle(battle.battleId));
                        }}
                        className={styles.closeBtn}
                        title="Close Battle"
                      >
                        ✕
                      </button>
                    )}
                  </div>
                );
              })}
            </div>
          )}
        </div>
      )}

      {/* Replays section */}
      {replayBattlesList.length > 0 && (
        <div className={`${styles.battlesSection} ${styles.replaysSection}`}>
          {!isCollapsed && <h3>Replays (Session)</h3>}
          <div className={styles.battlesList}>
            {replayBattlesList.map((battle) => {
              const isSelected = currentView === "battle" && activeBattleId === battle.battleId;
              const side0 = battle.battleState?.field?.sides?.[0];
              const side1 = battle.battleState?.field?.sides?.[1];
              const p0 = side0?.name || "Player 1";
              const p1 = side1?.name || "Player 2";
              const turnNumber = battle.battleState?.turn || 0;

              return (
                <div
                  key={battle.battleId}
                  className={`${styles.battleItemWrapper} flex-row align-center justify-between w-full`}
                >
                  <button
                    className={`${styles.battleItem} ${styles.closeableBattleItem} ${isSelected ? styles.selected : ""}`}
                    onClick={() => handleNav("battle", battle.battleId)}
                    title={`Replay: ${p0} vs ${p1}`}
                  >
                    <div className={styles.battleMeta}>
                      {isCollapsed ? (
                        <span className={styles.navIcon} title="Replay">
                          🎬
                        </span>
                      ) : (
                        <>
                          <span className={styles.opponentName}>
                            {p0} vs {p1}
                          </span>
                          <span className={styles.turnLabel}>Turn {turnNumber}</span>
                        </>
                      )}
                    </div>
                  </button>
                  {!isCollapsed && (
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        dispatch(removeBattle(battle.battleId));
                      }}
                      className={styles.closeBtn}
                      title="Close Replay"
                    >
                      ✕
                    </button>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      )}
    </aside>
  );
}
