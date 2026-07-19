import { useAppDispatch, useAppSelector } from "../../store/store";
import { selectBattle, removeBattle } from "../../store/battlesSlice";
import type { ActiveView } from "../../store/battlesSlice";
import { disconnectWamp } from "../../core/wamp";
import { BREAKPOINT_MOBILE_PX } from "../../utils/constants";
import { getBattleTitle } from "../../utils/battle";

import styles from "./Sidebar.module.scss";

interface SidebarProps {
  isCollapsed: boolean;
  setIsCollapsed: (collapsed: boolean) => void;
}

export default function Sidebar({ isCollapsed, setIsCollapsed }: SidebarProps) {
  const dispatch = useAppDispatch();
  const connection = useAppSelector((state) => state.connection);
  const { battles, activeBattleId, currentView } = useAppSelector((state) => state.battles);
  const proposalsMap = useAppSelector((state) => state.proposals.proposals);

  const activeBattlesList = Object.values(battles).filter((b) => !b.isReplay && !b.isProposal);
  const replayBattlesList = Object.values(battles).filter((b) => b.isReplay);

  const handleNav = (view: ActiveView, battleId: string | null = null) => {
    dispatch(selectBattle({ view, battleId }));

    // Automatically close sidebar drawer on navigation clicks on mobile
    if (typeof window !== "undefined" && window.innerWidth <= BREAKPOINT_MOBILE_PX) {
      setIsCollapsed(true);
    }
  };

  return (
    <aside className={`${styles.sidebar} ${isCollapsed ? styles.collapsed : ""}`}>
      <div className={styles.brand}>
        <h2>{isCollapsed ? "B" : "Battler"}</h2>
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
        {!isCollapsed &&
          (connection.status === "connected" ||
            (connection.status === "connecting" && !!connection.playerId)) && (
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
          title="Lobby"
        >
          {isCollapsed ? (
            <span className={styles.navIcon}>🌐</span>
          ) : (
            <span className={styles.navLabel}>Lobby</span>
          )}
        </button>
        <button
          className={`${styles.navItem} ${currentView === "teams" ? styles.active : ""}`}
          onClick={() => handleNav("teams")}
          title="Teams"
        >
          {isCollapsed ? (
            <span className={styles.navIcon}>📝</span>
          ) : (
            <span className={styles.navLabel}>Teams</span>
          )}
        </button>
        <button
          className={`${styles.navItem} ${currentView === "replays" ? styles.active : ""}`}
          onClick={() => handleNav("replays")}
          title="Replays"
        >
          {isCollapsed ? (
            <span className={styles.navIcon}>🎬</span>
          ) : (
            <span className={styles.navLabel}>Replays</span>
          )}
        </button>
      </nav>

      {(connection.status === "connected" ||
        (connection.status === "connecting" && !!connection.playerId)) && (
        <div className={styles.battlesSection}>
          {!isCollapsed && <h3>Battles</h3>}
          {activeBattlesList.length === 0 ? (
            !isCollapsed && <p className={styles.emptyBattles}>None</p>
          ) : (
            <div className={styles.battlesList}>
              {activeBattlesList.map((battle) => {
                const isSelected =
                  (currentView === "battle" || currentView === "proposal") &&
                  activeBattleId === battle.battleId;
                const hasPendingAction =
                  battle.activeRequest !== null && battle.battleState?.phase !== "finished";
                const title = getBattleTitle(
                  battle.battleState,
                  battle.serviceBattle,
                  proposalsMap[battle.battleId],
                );
                const turnNumber = battle.battleState?.turn || 0;
                const isFinished = battle.battleState?.phase === "finished";
                const isPreparing =
                  battle.serviceBattle?.state === "preparing" ||
                  battle.battleState?.phase === "pre_battle";
                const isDeleted = !battle.battleState && !!battle.error;
                const isCloseable = isFinished || isDeleted;

                return (
                  <div
                    key={battle.battleId}
                    className={`${styles.battleItemWrapper} flex-row align-center justify-between w-full`}
                  >
                    <button
                      className={`${styles.battleItem} ${isCloseable ? styles.closeableBattleItem : ""} ${isSelected ? styles.selected : ""}`}
                      onClick={() => handleNav("battle", battle.battleId)}
                      title={title}
                    >
                      <div className={styles.battleMeta}>
                        {isCollapsed ? (
                          <span className={styles.navIcon}>🎮</span>
                        ) : (
                          <>
                            <span className={styles.opponentName}>{title}</span>
                            <span
                              className={`${styles.turnLabel} ${isFinished ? styles.finishedLabel : isDeleted ? styles.errorLabel : ""}`}
                            >
                              {isFinished
                                ? "Finished"
                                : isDeleted
                                  ? "Deleted"
                                  : isPreparing
                                    ? "Preparing"
                                    : `Turn ${turnNumber}`}
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
                    {!isCollapsed && isCloseable && (
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
          {!isCollapsed && <h3>Replays</h3>}
          <div className={styles.battlesList}>
            {replayBattlesList.map((battle) => {
              const isSelected = currentView === "battle" && activeBattleId === battle.battleId;
              const title = getBattleTitle(battle.battleState);
              const turnNumber = battle.battleState?.turn || 0;

              return (
                <div
                  key={battle.battleId}
                  className={`${styles.battleItemWrapper} flex-row align-center justify-between w-full`}
                >
                  <button
                    className={`${styles.battleItem} ${styles.closeableBattleItem} ${isSelected ? styles.selected : ""}`}
                    onClick={() => handleNav("battle", battle.battleId)}
                    title={`Replay: ${title}`}
                  >
                    <div className={styles.battleMeta}>
                      {isCollapsed ? (
                        <span className={styles.navIcon} title="Replay">
                          🎬
                        </span>
                      ) : (
                        <>
                          <span className={styles.opponentName}>
                            {title}
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
