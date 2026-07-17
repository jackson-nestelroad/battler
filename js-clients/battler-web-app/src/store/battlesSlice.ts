import { createSlice } from "@reduxjs/toolkit";
import type { PayloadAction } from "@reduxjs/toolkit";
import isEqual from "fast-deep-equal";
import type { BattleState } from "battler-state";
import type { Request, PlayerBattleData } from "battler-types";
import type { UiLogEntry } from "battler-state";
import type { Battle } from "battler-service-client";

export interface SerializedBattleSession {
  battleId: string;
  battleState: BattleState | null;
  activeRequest: Request | null;
  playerData: PlayerBattleData | null;
  uiLogs: UiLogEntry[];
  engineLogs: string[];
  choiceSubmitted?: boolean;
  error: string | null;
  isLoading: boolean;
  serviceBattle: Battle | null;
}

export interface BattlesState {
  battles: Record<string, SerializedBattleSession>;
  activeBattleId: string | null;
  currentView: "lobby" | "teams" | "battle";
}

const initialState: BattlesState = {
  battles: {},
  activeBattleId: null,
  currentView: "lobby",
};

const battlesSlice = createSlice({
  name: "battles",
  initialState,
  reducers: {
    battleSessionCreated(state, action: PayloadAction<string>) {
      if (!state.battles[action.payload]) {
        state.battles[action.payload] = {
          battleId: action.payload,
          battleState: null,
          activeRequest: null,
          playerData: null,
          uiLogs: [],
          engineLogs: [],
          choiceSubmitted: false,
          error: null,
          isLoading: false,
          serviceBattle: null,
        };
      }
      state.activeBattleId = action.payload;
      state.currentView = "battle";
    },
    battleStateUpdated(
      state,
      action: PayloadAction<{ battleId: string; state: BattleState; engineLogs?: string[] }>,
    ) {
      const { battleId, state: battleState, engineLogs } = action.payload;
      const battle = state.battles[battleId];
      if (battle) {
        const prevTurn = battle.battleState?.turn || 0;
        battle.battleState = battleState;
        if (engineLogs) {
          battle.engineLogs = engineLogs;
        }
        battle.uiLogs = battleState.ui_log.flat();

        if (battleState.turn > prevTurn) {
          battle.choiceSubmitted = false;
        }
      }
    },
    setBattleRequest(state, action: PayloadAction<{ battleId: string; request: Request | null }>) {
      const { battleId, request } = action.payload;
      const battle = state.battles[battleId];
      if (battle) {
        if (!isEqual(battle.activeRequest, request)) {
          battle.choiceSubmitted = false;
        }
        battle.activeRequest = request;
      }
    },
    setBattlePlayerData(
      state,
      action: PayloadAction<{ battleId: string; playerData: PlayerBattleData | null }>,
    ) {
      const { battleId, playerData } = action.payload;
      const battle = state.battles[battleId];
      if (battle) {
        battle.playerData = playerData;
      }
    },
    setChoiceSubmitted(state, action: PayloadAction<{ battleId: string; submitted: boolean }>) {
      const { battleId, submitted } = action.payload;
      const battle = state.battles[battleId];
      if (battle) {
        battle.choiceSubmitted = submitted;
      }
    },
    setBattleError(state, action: PayloadAction<{ battleId: string; error: string | null }>) {
      const { battleId, error } = action.payload;
      const battle = state.battles[battleId];
      if (battle) {
        battle.error = error;
      }
    },
    setBattleLoading(state, action: PayloadAction<{ battleId: string; isLoading: boolean }>) {
      const { battleId, isLoading } = action.payload;
      const battle = state.battles[battleId];
      if (battle) {
        battle.isLoading = isLoading;
      }
    },
    battleSessionEnded(state, action: PayloadAction<string>) {
      const battle = state.battles[action.payload];
      if (battle && battle.battleState) {
        battle.battleState.phase = "finished";
      }
    },

    switchActiveBattle(state, action: PayloadAction<string | null>) {
      state.activeBattleId = action.payload;
      if (action.payload) {
        state.currentView = "battle";
      }
    },
    setCurrentView(state, action: PayloadAction<"lobby" | "teams" | "battle">) {
      state.currentView = action.payload;
    },

    serviceBattleUpdated(
      state,
      action: PayloadAction<{ battleId: string; serviceBattle: Battle }>,
    ) {
      const { battleId, serviceBattle } = action.payload;
      const battle = state.battles[battleId];
      if (battle) {
        battle.serviceBattle = serviceBattle;
      }
    },
  },
});

export const {
  battleSessionCreated,
  battleStateUpdated,
  setBattleRequest,
  setBattleError,
  setBattleLoading,
  battleSessionEnded,
  switchActiveBattle,
  setCurrentView,
  serviceBattleUpdated,
  setChoiceSubmitted,
  setBattlePlayerData,
} = battlesSlice.actions;

export default battlesSlice.reducer;
