import { useState } from "react";
import { useAppSelector } from "./store/store";
import Sidebar from "./ui/Sidebar/Sidebar";
import Lobby from "./ui/Lobby/Lobby";
import Teams from "./ui/Teams/Teams";
import BattleScreen from "./ui/Battle/BattleScreen";
import ReplaysHome from "./ui/Replays/ReplaysHome";
import { BREAKPOINT_TABLET_PX } from "./utils/constants";
import { useHistorySync } from "./hooks/useHistorySync";
import ConnectionRequired from "./ui/Common/ConnectionRequired";
import { useConnectionCountdown } from "./hooks/useConnectionCountdown";

import styles from "./App.module.scss";

export default function App() {
  useHistorySync();
  const connection = useAppSelector((state) => state.connection);
  const isHydrated = connection.isHydrated;
  const currentView = useAppSelector((state) => state.battles.currentView);
  const battleId = useAppSelector((state) => state.battles.activeBattleId);
  const isReplay = useAppSelector((state) =>
    battleId ? !!state.battles.battles[battleId]?.isReplay : false,
  );
  const { connectionMessage } = useConnectionCountdown();

  const [isCollapsed, setIsCollapsed] = useState(
    typeof window !== "undefined" ? window.innerWidth < BREAKPOINT_TABLET_PX : false,
  );

  const showAutoconnectLoader = connection.autoconnect && connection.status === "connecting";

  if (!isHydrated || showAutoconnectLoader) {
    return (
      <div className={styles.loadingScreen}>
        <div className="spinner"></div>
        <p>
          {showAutoconnectLoader ? connectionMessage : "Initializing..."}
        </p>
      </div>
    );
  }

  return (
    <div className={styles.appContainer}>
      <Sidebar isCollapsed={isCollapsed} setIsCollapsed={setIsCollapsed} />

      {!isCollapsed && <div className={styles.backdrop} onClick={() => setIsCollapsed(true)} />}

      <main className={styles.mainContent}>
        <header className={styles.mobileTopBar}>
          <button className={styles.menuTrigger} onClick={() => setIsCollapsed(false)}>
            ☰
          </button>
          <span className={styles.viewTitle}>
            {currentView === "lobby" && "Lobby"}
            {currentView === "teams" && "Teams"}
            {(currentView === "battle" || currentView === "proposal") && "Battles"}
            {currentView === "replays" && "Replays"}
          </span>
        </header>

        <div className={styles.viewWrapper}>
          {currentView === "lobby" && (
            <ConnectionRequired>
              <Lobby />
            </ConnectionRequired>
          )}
          {currentView === "teams" && <Teams />}
          {(currentView === "battle" || currentView === "proposal") && (
            <ConnectionRequired bypass={isReplay}>
              <BattleScreen />
            </ConnectionRequired>
          )}
          {currentView === "replays" && <ReplaysHome />}
        </div>
      </main>
    </div>
  );
}
