import { useEffect, useRef } from "react";
import { useAppDispatch, useAppSelector } from "../store/store";
import { setCurrentView, switchActiveBattle } from "../store/battlesSlice";
import type { ActiveView } from "../store/battlesSlice";

// Helper to get path relative to Vite's BASE_URL (e.g. /my-app/teams -> /teams)
const getCleanPathname = () => {
  const base = import.meta.env.BASE_URL || "/";
  const baseNoTrailing = base.endsWith("/") ? base.slice(0, -1) : base;
  return window.location.pathname.replace(baseNoTrailing, "") || "/";
};

const pushPath = (cleanPath: string) => {
  const base = import.meta.env.BASE_URL || "/";
  const baseNoTrailing = base.endsWith("/") ? base.slice(0, -1) : base;
  window.history.pushState(null, "", (baseNoTrailing + cleanPath).replace(/\/+/g, "/"));
};

export function useHistorySync() {
  const dispatch = useAppDispatch();
  const currentView = useAppSelector((state) => state.battles.currentView);
  const activeBattleId = useAppSelector((state) => state.battles.activeBattleId);
  const battles = useAppSelector((state) => state.battles.battles);

  const isHandlingPopState = useRef(false);
  const battlesRef = useRef(battles);

  // Keep battlesRef up-to-date
  useEffect(() => {
    battlesRef.current = battles;
  }, [battles]);

  // 1. Sync URL -> Redux (on load and back/forward navigation)
  useEffect(() => {
    const handlePopState = () => {
      isHandlingPopState.current = true;
      const path = getCleanPathname();

      let view: ActiveView = "lobby";
      let activeId: string | null = null;

      if (path.startsWith("/battle/")) {
        view = "battle";
        activeId = path.slice(8) || null;
      } else if (path.startsWith("/replay/")) {
        activeId = path.slice(8) || null;
        if (activeId && battlesRef.current[activeId]) {
          view = "battle";
        } else {
          view = "replays";
          activeId = null;
        }
      } else if (path === "/replays") {
        view = "replays";
      } else if (path.startsWith("/proposal/")) {
        view = "battle";
        activeId = path.slice(10) || null;
      } else if (path === "/teams") {
        view = "teams";
      }

      dispatch(setCurrentView(view));
      dispatch(switchActiveBattle(activeId));

      setTimeout(() => {
        isHandlingPopState.current = false;
      }, 0);
    };

    handlePopState();
    window.addEventListener("popstate", handlePopState);
    return () => window.removeEventListener("popstate", handlePopState);
  }, [dispatch]);

  // 2. Sync Redux -> URL (on state changes)
  useEffect(() => {
    if (isHandlingPopState.current) return;

    let targetPath = "/";
    if (currentView === "teams") {
      targetPath = "/teams";
    } else if (currentView === "replays") {
      targetPath = "/replays";
    } else if (currentView === "battle" && activeBattleId) {
      const battle = battles[activeBattleId];
      if (battle?.isReplay) {
        targetPath = `/replay/${activeBattleId}`;
      } else if (battle) {
        targetPath = `/battle/${activeBattleId}`;
      } else {
        targetPath = `/proposal/${activeBattleId}`;
      }
    }

    if (getCleanPathname() !== targetPath) {
      pushPath(targetPath);
    }
  }, [currentView, activeBattleId, battles]);
}
