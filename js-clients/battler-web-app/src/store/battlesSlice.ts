import { createSlice } from "@reduxjs/toolkit";
import type { PayloadAction } from "@reduxjs/toolkit";
import isEqual from "fast-deep-equal";
import type { BattleState } from "battler-state";
import type { Request, PlayerBattleData } from "battler-types";
import type { UiLogEntry } from "battler-state";
import { formatUuid } from "../utils/uuid";
import type { Battle, BattleMetadata } from "battler-service-client";
import { resolveReplayTurnState, getReplayStepBoundary } from "../utils/replay";
import type { ReplayKeyframe } from "../utils/replay";

export interface BaseBattleSession {
  battleId: string;
  battleState: BattleState | null;
  activeRequest: Request | null;
  playerData: PlayerBattleData | null;
  uiLogs: UiLogEntry[];
  engineLogs: string[];
  choiceSubmitted?: boolean;
  error: string | null;
  choiceError: string | null;
  isLoading: boolean;
  serviceBattle: Battle | null;
  isDeleted?: boolean;
  isProposal?: boolean;
  metadata?: BattleMetadata;
}

export interface LiveBattleSession extends BaseBattleSession {
  isReplay?: false;
}

export interface ReplayBattleSession extends BaseBattleSession {
  isReplay: true;
  replayCurrentTurn: number;
  replayStates: (BattleState | undefined)[];
  replayEngineLogs: string[];
  replayKeyframes: ReplayKeyframe[];
}

export type SerializedBattleSession = LiveBattleSession | ReplayBattleSession;

export function isReplaySession(
  session: SerializedBattleSession | undefined,
): session is ReplayBattleSession {
  return !!session?.isReplay;
}

export type ActiveView = "lobby" | "teams" | "battle" | "replays" | "proposal";

export interface BattlesState {
  battles: Record<string, SerializedBattleSession>;
  activeBattleId: string | null;
  currentView: ActiveView;
}

const initialState: BattlesState = {
  battles: {},
  activeBattleId: null,
  currentView: "lobby",
};

const normalizeId = (id: string): string => formatUuid(id);

const battlesSlice = createSlice({
  name: "battles",
  initialState,
  reducers: {
    battleSessionCreated(state, action: PayloadAction<string>) {
      const battleId = normalizeId(action.payload);
      if (!state.battles[battleId]) {
        state.battles[battleId] = {
          battleId,
          battleState: null,
          activeRequest: null,
          playerData: null,
          uiLogs: [],
          engineLogs: [],
          choiceSubmitted: false,
          error: null,
          choiceError: null,
          isLoading: false,
          serviceBattle: null,
        };
      }
      state.activeBattleId = battleId;
      state.currentView = "battle";
    },
    battleStateUpdated(
      state,
      action: PayloadAction<{ battleId: string; state: BattleState; engineLogs?: string[] }>,
    ) {
      const { battleId: rawId, state: battleState, engineLogs } = action.payload;
      const battleId = normalizeId(rawId);
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
          battle.choiceError = null;
        }
      }
    },
    setBattleRequest(state, action: PayloadAction<{ battleId: string; request: Request | null }>) {
      const { battleId: rawId, request } = action.payload;
      const battleId = normalizeId(rawId);
      const battle = state.battles[battleId];
      if (battle) {
        if (!isEqual(battle.activeRequest, request)) {
          battle.choiceSubmitted = false;
          battle.choiceError = null;
        }
        battle.activeRequest = request;
      }
    },
    setBattlePlayerData(
      state,
      action: PayloadAction<{ battleId: string; playerData: PlayerBattleData | null }>,
    ) {
      const { battleId: rawId, playerData } = action.payload;
      const battleId = normalizeId(rawId);
      const battle = state.battles[battleId];
      if (battle) {
        battle.playerData = playerData;
      }
    },
    setChoiceSubmitted(state, action: PayloadAction<{ battleId: string; submitted: boolean }>) {
      const { battleId: rawId, submitted } = action.payload;
      const battleId = normalizeId(rawId);
      const battle = state.battles[battleId];
      if (battle) {
        battle.choiceSubmitted = submitted;
      }
    },
    setBattleError(state, action: PayloadAction<{ battleId: string; error: string | null }>) {
      const { battleId: rawId, error } = action.payload;
      const battleId = normalizeId(rawId);
      const battle = state.battles[battleId];
      if (battle) {
        battle.error = error;
      }
    },
    setChoiceError(state, action: PayloadAction<{ battleId: string; error: string | null }>) {
      const { battleId: rawId, error } = action.payload;
      const battleId = normalizeId(rawId);
      const battle = state.battles[battleId];
      if (battle) {
        battle.choiceError = error;
      }
    },
    setBattleLoading(state, action: PayloadAction<{ battleId: string; isLoading: boolean }>) {
      const { battleId: rawId, isLoading } = action.payload;
      const battleId = normalizeId(rawId);
      const battle = state.battles[battleId];
      if (battle) {
        battle.isLoading = isLoading;
      }
    },
    battleSessionEnded(state, action: PayloadAction<string>) {
      const battleId = normalizeId(action.payload);
      const battle = state.battles[battleId];
      if (battle && battle.battleState) {
        battle.battleState.phase = "finished";
      }
    },

    battleSessionRestored(
      state,
      action: PayloadAction<string | { battleId: string; isProposal?: boolean }>,
    ) {
      const payload = action.payload;
      const battleId =
        typeof payload === "string" ? normalizeId(payload) : normalizeId(payload.battleId);
      const isProposal = typeof payload === "string" ? false : !!payload.isProposal;

      if (!state.battles[battleId]) {
        state.battles[battleId] = {
          battleId,
          battleState: null,
          activeRequest: null,
          playerData: null,
          uiLogs: [],
          engineLogs: [],
          choiceSubmitted: false,
          error: null,
          choiceError: null,
          isLoading: false,
          serviceBattle: null,
          isProposal,
        };
      }
    },

    clearBattleState(state, action: PayloadAction<string>) {
      const battleId = normalizeId(action.payload);
      const battle = state.battles[battleId];
      if (battle) {
        battle.isDeleted = true;
      }
    },

    switchActiveBattle(state, action: PayloadAction<string | null>) {
      const battleId = action.payload ? normalizeId(action.payload) : null;
      state.activeBattleId = battleId;
      if (battleId) {
        if (state.currentView !== "proposal" && state.currentView !== "battle") {
          state.currentView = "battle";
        }
      } else if (state.currentView === "battle" || state.currentView === "proposal") {
        state.currentView = "lobby";
      }
    },
    setCurrentView(state, action: PayloadAction<ActiveView>) {
      state.currentView = action.payload;
    },
    selectBattle(state, action: PayloadAction<{ view: ActiveView; battleId: string | null }>) {
      const { view, battleId } = action.payload;
      state.currentView = view;
      state.activeBattleId = battleId ? normalizeId(battleId) : null;
    },

    serviceBattleUpdated(
      state,
      action: PayloadAction<{ battleId: string; serviceBattle: Battle }>,
    ) {
      const { battleId: rawId, serviceBattle } = action.payload;
      const battleId = normalizeId(rawId);
      const battle = state.battles[battleId];
      if (battle) {
        battle.serviceBattle = serviceBattle;
      }
    },
    clearBattles(state) {
      state.battles = {};
    },
    resetBattlesState(state) {
      state.battles = {};
      state.activeBattleId = null;
      state.currentView = "lobby";
    },
    battleReplayLoaded(
      state,
      action: PayloadAction<{
        battleId: string;
        engineLogs: string[];
        keyframes: ReplayKeyframe[];
        maxTurn: number;
        metadata?: BattleMetadata;
      }>,
    ) {
      const { battleId, engineLogs, keyframes, maxTurn, metadata } = action.payload;
      const firstState = keyframes.find((k) => k.turn === 0)?.state || null;

      // Initialize sparse array for replayStates matching size of maxTurn + 2
      const replayStates = new Array(maxTurn + 2);
      // Pre-fill keyframes into the cache
      for (const kf of keyframes) {
        replayStates[kf.turn] = kf.state;
      }

      const initialBoundaryIdx = getReplayStepBoundary(engineLogs, 0, maxTurn);

      state.battles[battleId] = {
        battleId,
        battleState: firstState,
        activeRequest: null,
        playerData: null,
        uiLogs: firstState ? firstState.ui_log.flat() : [],
        engineLogs: engineLogs.slice(0, initialBoundaryIdx),
        choiceSubmitted: false,
        error: null,
        choiceError: null,
        isLoading: false,
        serviceBattle: null,
        isReplay: true,
        replayCurrentTurn: 0,
        replayStates,
        replayEngineLogs: engineLogs,
        replayKeyframes: keyframes,
        metadata,
      };
      state.activeBattleId = battleId;
      state.currentView = "battle";
    },
    setReplayTurn(state, action: PayloadAction<{ battleId: string; turn: number }>) {
      const { battleId: rawId, turn } = action.payload;
      const battleId = normalizeId(rawId);
      const battle = state.battles[battleId];
      if (isReplaySession(battle)) {
        const maxTurn = battle.replayStates.length - 2;
        const maxStep = maxTurn + 1;
        const turnIndex = Math.max(0, Math.min(turn, maxStep));
        battle.replayCurrentTurn = turnIndex;

        // Resolve target state using keyframe hybrid lookup (processes max 9 turns incrementally)
        const targetState = resolveReplayTurnState(battle, turnIndex);
        battle.battleState = targetState;
        battle.uiLogs = targetState ? targetState.ui_log.flat() : [];

        // Set engineLogs up to this turn
        const boundaryIdx = getReplayStepBoundary(battle.replayEngineLogs, turnIndex, maxTurn);
        battle.engineLogs = battle.replayEngineLogs.slice(0, boundaryIdx);
      }
    },
    removeBattle(state, action: PayloadAction<string>) {
      const battleId = normalizeId(action.payload);
      const isReplay = state.battles[battleId]?.isReplay;
      delete state.battles[battleId];
      if (state.activeBattleId === battleId) {
        state.activeBattleId = null;
        state.currentView = isReplay ? "replays" : "lobby";
      }
    },
  },
});

export const {
  battleSessionCreated,
  battleStateUpdated,
  setBattleRequest,
  setBattleError,
  setChoiceError,
  setBattleLoading,
  battleSessionEnded,
  battleSessionRestored,
  clearBattleState,
  switchActiveBattle,
  setCurrentView,
  selectBattle,
  serviceBattleUpdated,
  setChoiceSubmitted,
  setBattlePlayerData,
  clearBattles,
  resetBattlesState,
  battleReplayLoaded,
  setReplayTurn,
  removeBattle,
} = battlesSlice.actions;

export default battlesSlice.reducer;
