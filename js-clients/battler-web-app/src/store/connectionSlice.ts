import { createSlice } from "@reduxjs/toolkit";
import type { PayloadAction, Dispatch } from "@reduxjs/toolkit";
import { getCookie } from "../utils/cookie";

export interface ConnectionState {
  status: "disconnected" | "connecting" | "connected";
  playerId: string | null;
  serverUrl: string | null;
  isHydrated: boolean;
  error: string | null;
  savedPlayerId: string | null;
  savedServerUrl: string | null;
  autoconnect: boolean;
  retryDelay: number | null;
  retryCount: number | null;
}

const initialAutoconnect = getCookie("battler_autoconnect") === "true";
const initialSavedPlayerId = getCookie("battler_username");
const initialSavedServerUrl = getCookie("battler_server_url") || "ws://localhost:8080/ws";

const initialState: ConnectionState = {
  status: initialAutoconnect && initialSavedPlayerId ? "connecting" : "disconnected",
  playerId: null,
  serverUrl: null,
  isHydrated: false,
  error: null,
  savedPlayerId: initialSavedPlayerId,
  savedServerUrl: initialSavedServerUrl,
  autoconnect: initialAutoconnect,
  retryDelay: null,
  retryCount: null,
};

const connectionSlice = createSlice({
  name: "connection",
  initialState,
  reducers: {
    setConnectionStatus(state, action: PayloadAction<ConnectionState["status"]>) {
      state.status = action.payload;
      if (action.payload === "connected" || action.payload === "disconnected") {
        state.retryDelay = null;
        state.retryCount = null;
      }
    },
    setPlayerId(state, action: PayloadAction<string | null>) {
      state.playerId = action.payload;
    },
    setServerUrl(state, action: PayloadAction<string | null>) {
      state.serverUrl = action.payload;
    },
    setIsHydrated(state, action: PayloadAction<boolean>) {
      state.isHydrated = action.payload;
    },
    setError(state, action: PayloadAction<string | null>) {
      state.error = action.payload;
    },
    setSavedConnectionDetails(
      state,
      action: PayloadAction<{ playerId: string; serverUrl: string; autoconnect: boolean }>,
    ) {
      const { playerId, serverUrl, autoconnect } = action.payload;
      state.savedPlayerId = playerId;
      state.savedServerUrl = serverUrl;
      state.autoconnect = autoconnect;
    },
    setAutoconnect(state, action: PayloadAction<boolean>) {
      state.autoconnect = action.payload;
    },
    setRetryDetails(
      state,
      action: PayloadAction<{ retryDelay: number | null; retryCount: number | null } | null>,
    ) {
      if (action.payload) {
        state.retryDelay = action.payload.retryDelay;
        state.retryCount = action.payload.retryCount;
      } else {
        state.retryDelay = null;
        state.retryCount = null;
      }
    },
  },
});

export const {
  setConnectionStatus,
  setPlayerId,
  setServerUrl,
  setIsHydrated,
  setError,
  setSavedConnectionDetails,
  setAutoconnect,
  setRetryDetails,
} = connectionSlice.actions;

export const setConnectionError =
  (message: string | null, originalError?: unknown) => (dispatch: Dispatch) => {
    if (message) {
      if (originalError) {
        console.error(`[Connection Error] ${message}:`, originalError);
      } else {
        console.error(`[Connection Error] ${message}`);
      }
    }
    dispatch(setError(message));
  };

export default connectionSlice.reducer;
