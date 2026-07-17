import { useAppDispatch, useAppSelector } from "../../store/store";
import { setCurrentView, switchActiveBattle } from "../../store/battlesSlice";
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

  const activeBattlesList = Object.values(battles);

  const handleNav = (view: "lobby" | "teams" | "battle", battleId: string | null = null) => {
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

                return (
                  <button
                    key={battle.battleId}
                    className={`${styles.battleItem} ${isSelected ? styles.selected : ""}`}
                    onClick={() => handleNav("battle", battle.battleId)}
                    title={`Battle vs ${opponentName}`}
                  >
                    <div className={styles.battleMeta}>
                      {isCollapsed ? (
                        <span className={styles.navIcon}>🎮</span>
                      ) : (
                        <>
                          <span className={styles.opponentName}>vs {opponentName}</span>
                          <span className={styles.turnLabel}>Turn {turnNumber}</span>
                        </>
                      )}
                    </div>
                    {hasPendingAction && (
                      <span className={styles.actionBadge} title="Your turn to act!">
                        !
                      </span>
                    )}
                  </button>
                );
              })}
            </div>
          )}
        </div>
      )}
    </aside>
  );
}
