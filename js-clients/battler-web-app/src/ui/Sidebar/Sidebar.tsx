import { useState } from "react";
import { closeBattleSession, disconnectWamp } from "../../core/wamp";
import type { ActiveView, SerializedBattleSession } from "../../store/battlesSlice";
import { isSpectatorSession, selectBattle } from "../../store/battlesSlice";
import { useAppDispatch, useAppSelector } from "../../store/store";
import AboutModal from "../Common/AboutModal/AboutModal";
import BugReportModal from "../Common/BugReportModal/BugReportModal";
import InfoIcon from "../Common/InfoIcon";
import { getBattleSessionTitle } from "../../utils/battle";
import { getBattleSessionStateLabel, isBattleFinished } from "../../utils/battleState";
import { BREAKPOINT_MOBILE_PX } from "../../utils/constants";

import styles from "./Sidebar.module.scss";

interface SidebarProps {
  isCollapsed: boolean;
  setIsCollapsed: (collapsed: boolean) => void;
}

export default function Sidebar({ isCollapsed, setIsCollapsed }: SidebarProps) {
  const dispatch = useAppDispatch();
  const [showBugReportModal, setShowBugReportModal] = useState(false);
  const [showAboutModal, setShowAboutModal] = useState(false);
  const connection = useAppSelector((state) => state.connection);
  const { battles, activeBattleId, currentView } = useAppSelector((state) => state.battles);
  const proposalsMap = useAppSelector((state) => state.proposals.proposals);

  const activeBattlesList = Object.values(battles).filter((b) => !b.isReplay && !b.isProposal);
  const playingBattlesList = activeBattlesList.filter(
    (b) => !isSpectatorSession(b, connection.playerId),
  );
  const spectatingBattlesList = activeBattlesList.filter((b) =>
    isSpectatorSession(b, connection.playerId),
  );
  const replayBattlesList = Object.values(battles).filter((b) => b.isReplay);

  const renderBattleItem = (battle: SerializedBattleSession) => {
    const isReplay = !!battle.isReplay;
    const isSpectator = !isReplay && isSpectatorSession(battle, connection.playerId);
    const isSelected =
      (currentView === "battle" || currentView === "proposal") &&
      activeBattleId === battle.battleId;
    const isFinished = isBattleFinished(battle);
    const hasPendingAction =
      !isReplay &&
      !isSpectator &&
      battle.activeRequest !== null &&
      !isFinished;
    const isDeleted = !isReplay && (battle.isDeleted || (!battle.battleState && !battle.preview && !!battle.error));
    const title = getBattleSessionTitle(battle, proposalsMap[battle.battleId], isDeleted);
    const isCloseable = isReplay || isFinished || isDeleted || isSpectator;

    return (
      <div
        key={battle.battleId}
        className={`${styles.battleItemWrapper} flex-row align-center justify-between w-full`}
      >
        <button
          className={`${styles.battleItem} ${isCloseable ? styles.closeableBattleItem : ""} ${isSelected ? styles.selected : ""}`}
          onClick={() => handleNav("battle", battle.battleId)}
          title={isReplay ? `Replay: ${title}` : isSpectator ? `Spectating: ${title}` : title}
        >
          <div className={styles.battleMeta}>
            {isCollapsed ? (
              <span
                className={styles.navIcon}
                title={isReplay ? "Replay" : isSpectator ? "Spectating" : "Playing"}
              >
                {isReplay ? "🎬" : "🎮"}
              </span>
            ) : (
              <>
                <span className={styles.opponentName}>{title}</span>
                <span
                  className={`${styles.turnLabel} ${isDeleted ? styles.errorLabel : isFinished ? styles.finishedLabel : ""}`}
                >
                  {isDeleted ? "Deleted" : getBattleSessionStateLabel(battle)}
                  {isSpectator && <span className={styles.spectatorBadge}> • Spectating</span>}
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
              dispatch(closeBattleSession(battle.battleId));
            }}
            className={styles.closeBtn}
            title={isReplay ? "Close Replay" : isSpectator ? "Close Spectating Battle" : "Close Battle"}
          >
            ✕
          </button>
        )}
      </div>
    );
  };

  const incomingProposals = Object.values(proposalsMap).filter((p) => {
    if (!connection.playerId) return false;
    const player = p.sides.flatMap((s) => s.players).find((pl) => pl.id === connection.playerId);
    const isResolved = !!p.battle;
    const isDeclined = !!p.rejection || !!p.deletionReason;
    const hasAccepted = player?.status === "accepted";
    return !!player && !isResolved && !isDeclined && !hasAccepted;
  });
  const hasIncomingProposals = incomingProposals.length > 0;

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
        {isCollapsed ? (
          <img
            src="/logo.svg"
            alt="Battler"
            className={styles.brandLogo}
            onClick={() => setIsCollapsed(false)}
            title="Expand Sidebar"
          />
        ) : (
          <div className="flex-row align-center gap-xs">
            <img src="/logo.svg" alt="" className={styles.brandLogo} />
            <h2>Battler</h2>
          </div>
        )}
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
            (connection.status === "connecting" && connection.hasConnected)) && (
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
          {hasIncomingProposals && (
            <span className={styles.actionBadge} title="New proposal received!">
              !
            </span>
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
        <button
          className={`${styles.navItem} ${currentView === "resources" ? styles.active : ""}`}
          onClick={() => handleNav("resources")}
          title="Resources"
        >
          {isCollapsed ? (
            <span className={styles.navIcon}>📖</span>
          ) : (
            <span className={styles.navLabel}>Resources</span>
          )}
        </button>
      </nav>

      {(connection.status === "connected" ||
        (connection.status === "connecting" && connection.hasConnected)) && (
        <div className={styles.battlesSection}>
          {!isCollapsed && <h3>Battles</h3>}
          {activeBattlesList.length === 0 ? (
            !isCollapsed && <p className={styles.emptyBattles}>None</p>
          ) : (
            <div className={styles.battlesList}>
              {playingBattlesList.length > 0 && spectatingBattlesList.length > 0 ? (
                <>
                  {!isCollapsed && <div className={styles.subHeader}>Playing</div>}
                  {playingBattlesList.map(renderBattleItem)}
                  {!isCollapsed && <div className={styles.subHeader}>Spectating</div>}
                  {spectatingBattlesList.map(renderBattleItem)}
                </>
              ) : playingBattlesList.length > 0 ? (
                playingBattlesList.map(renderBattleItem)
              ) : (
                <>
                  {!isCollapsed && <div className={styles.subHeader}>Spectating</div>}
                  {spectatingBattlesList.map(renderBattleItem)}
                </>
              )}
            </div>
          )}
        </div>
      )}

      {/* Replays section */}
      {replayBattlesList.length > 0 && (
        <div className={`${styles.battlesSection} ${styles.replaysSection}`}>
          {!isCollapsed && <h3>Replays</h3>}
          <div className={styles.battlesList}>{replayBattlesList.map(renderBattleItem)}</div>
        </div>
      )}

      <div className={styles.sidebarFooter}>
        {isCollapsed ? (
          <div className="flex-col align-center gap-xs">
            <button
              type="button"
              className={styles.footerIconBtn}
              onClick={() => setShowAboutModal(true)}
              title="About"
              aria-label="About"
            >
              <InfoIcon size={14} />
            </button>
            <button
              type="button"
              className={styles.footerIconBtn}
              onClick={() => setShowBugReportModal(true)}
              title="Report bug"
              aria-label="Report bug"
            >
              🐞
            </button>
          </div>
        ) : (
          <div className="flex-row align-center justify-between gap-xs">
            <button
              type="button"
              className={styles.reportBugBtn}
              onClick={() => setShowBugReportModal(true)}
              aria-label="Report bug"
            >
              🐞 Report bug
            </button>
            <button
              type="button"
              className={styles.aboutBtn}
              onClick={() => setShowAboutModal(true)}
              title="About"
              aria-label="About"
            >
              <InfoIcon size={13} />
              <span>About</span>
            </button>
          </div>
        )}
      </div>

      {showBugReportModal && (
        <BugReportModal
          isOpen={showBugReportModal}
          onClose={() => setShowBugReportModal(false)}
        />
      )}

      {showAboutModal && (
        <AboutModal
          isOpen={showAboutModal}
          onClose={() => setShowAboutModal(false)}
        />
      )}
    </aside>
  );
}
