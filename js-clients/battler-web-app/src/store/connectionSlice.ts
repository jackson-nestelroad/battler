import { createSlice } from "@reduxjs/toolkit";
import type { PayloadAction, Dispatch } from "@reduxjs/toolkit";

export interface ConnectionState {
  status: "disconnected" | "connecting" | "connected";
  playerId: string | null;
  serverUrl: string | null;
  isHydrated: boolean;
  error: string | null;
}

const initialState: ConnectionState = {
  status: "disconnected",
  playerId: null,
  serverUrl: null,
  isHydrated: false,
  error: null,
};

const connectionSlice = createSlice({
  name: "connection",
  initialState,
  reducers: {
    setConnectionStatus(state, action: PayloadAction<ConnectionState["status"]>) {
      state.status = action.payload;
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
  },
});

export const { setConnectionStatus, setPlayerId, setServerUrl, setIsHydrated, setError } =
  connectionSlice.actions;

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
