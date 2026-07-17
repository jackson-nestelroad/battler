import { configureStore, createAsyncThunk } from "@reduxjs/toolkit";
import type { Middleware, Dispatch, UnknownAction } from "@reduxjs/toolkit";
import { useDispatch, useSelector } from "react-redux";
import type { TypedUseSelectorHook } from "react-redux";
import connectionReducer, { setIsHydrated, setConnectionError } from "./connectionSlice";
import proposalsReducer from "./proposalsSlice";
import battlesReducer from "./battlesSlice";
import teamsReducer, { teamsLoaded } from "./teamsSlice";
import { LocalStoragePersistentStorage } from "../core/storage";
import type { MonData } from "battler-types";

const storage = new LocalStoragePersistentStorage();

const teamsPersistenceMiddleware: Middleware = (storeApi) => {
  let lastSavedTeams: ReturnType<typeof teamsReducer> | null = null;
  return (next) => (action: any) => {
    const result = next(action);
    const state = storeApi.getState() as { teams: ReturnType<typeof teamsReducer> };
    if (action.type !== teamsLoaded.type && state.teams !== lastSavedTeams) {
      lastSavedTeams = state.teams;
      Promise.all([
        storage.setItem("battler_teams", state.teams.teams),
        storage.setItem("battler_default_team", state.teams.defaultTeam),
        storage.setItem("battler_team_order", state.teams.teamOrder),
      ]).catch((e) => {
        (storeApi.dispatch as (action: UnknownAction | ((dispatch: Dispatch) => void)) => void)(
          setConnectionError(
            "Failed to persist teams to storage. Please check disk space or browser settings.",
            e,
          ),
        );
      });
    }
    return result;
  };
};

export const store = configureStore({
  reducer: {
    connection: connectionReducer,
    proposals: proposalsReducer,
    battles: battlesReducer,
    teams: teamsReducer,
  },
  middleware: (getDefaultMiddleware) => getDefaultMiddleware().concat(teamsPersistenceMiddleware),
});

// Async hydration thunk triggered on mount
export const hydrateStore = createAsyncThunk<void, void, { dispatch: AppDispatch }>(
  "store/hydrate",
  async (_, { dispatch }) => {
    try {
      const teams = (await storage.getItem<Record<string, MonData[]>>("battler_teams")) || {};
      const defaultTeam = (await storage.getItem<string | null>("battler_default_team")) || null;
      const teamOrder =
        (await storage.getItem<string[]>("battler_team_order")) || Object.keys(teams);
      dispatch(teamsLoaded({ teams, defaultTeam, teamOrder }));
    } catch (e) {
      dispatch(setConnectionError("Failed to load saved teams from browser storage.", e));
    } finally {
      dispatch(setIsHydrated(true));
    }
  },
);

export type RootState = ReturnType<typeof store.getState>;
export type AppDispatch = typeof store.dispatch;

// Use throughout your app instead of plain `useDispatch` and `useSelector`
export const useAppDispatch = () => useDispatch<AppDispatch>();
export const useAppSelector: TypedUseSelectorHook<RootState> = useSelector;
