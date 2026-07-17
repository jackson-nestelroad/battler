import { useState } from "react";
import { useAppSelector } from "./store/store";
import Sidebar from "./ui/Sidebar/Sidebar";
import Lobby from "./ui/Lobby/Lobby";
import Teams from "./ui/Teams/Teams";
import BattleScreen from "./ui/Battle/BattleScreen";
import { BREAKPOINT_TABLET_PX } from "./utils/constants";

import styles from "./App.module.scss";

export default function App() {
  const isHydrated = useAppSelector((state) => state.connection.isHydrated);
  const currentView = useAppSelector((state) => state.battles.currentView);

  const [isCollapsed, setIsCollapsed] = useState(
    typeof window !== "undefined" ? window.innerWidth < BREAKPOINT_TABLET_PX : false,
  );

  if (!isHydrated) {
    return (
      <div className={styles.loadingScreen}>
        <div className="spinner"></div>
        <p>Initializing Battler App...</p>
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
            {currentView === "lobby" && "Matchmaking Lobby"}
            {currentView === "teams" && "Teams Editor"}
            {currentView === "battle" && "Battle Workspace"}
          </span>
        </header>

        <div className={styles.viewWrapper}>
          {currentView === "lobby" && <Lobby />}
          {currentView === "teams" && <Teams />}
          {currentView === "battle" && <BattleScreen />}
        </div>
      </main>
    </div>
  );
}
