import type { Dispatch } from "@reduxjs/toolkit";
import { createAsyncThunk } from "@reduxjs/toolkit";
import type { RootState } from "../store/store";

import type { IConnectionOptions, Subscription } from "autobahn";
import { BattlerClient } from "battler-client";
import { BattlerMultiplayerClient } from "battler-multiplayer-client";
import type {
  ProposedBattle,
  ProposedBattleOptions,
  ProposedBattleRejection,
} from "battler-multiplayer-service-client";
import { BattlerMultiplayerServiceClient } from "battler-multiplayer-service-client";
import { BattlerServiceClient, ValidationError, type BattlePreview } from "battler-service-client";
import type { MonData } from "battler-types";
import { WampSessionProvider } from "battler-wamp-client";
import {
  addSpectatingBattle,
  battleSessionCreated,
  battleSessionEnded,
  battleSessionRestored,
  battleStateUpdated,
  clearBattles,
  clearBattleState,
  isSpectatorSession,
  removeBattle,
  removeSpectatingBattle,
  resetBattlesState,
  selectBattle,
  serviceBattleUpdated,
  setBattleError,
  setBattleLoading,
  setBattlePlayerData,
  setBattleRequest,
  setChoiceError,
  setChoiceSubmitted,
  setIsSpectator,
  setSpectatingBattles,
} from "../store/battlesSlice";
import {
  setAutoconnect,
  setConnectionError,
  setConnectionStatus,
  setPlayerId,
  setRetryDetails,
  setSavedConnectionDetails,
  setServerUrl,
} from "../store/connectionSlice";
import type { ProposedBattleWithDetails } from "../store/proposalsSlice";
import { addProposals, clearProposals, updateProposal } from "../store/proposalsSlice";
import { formatUuid } from "../utils/uuid";
import { LocalStoragePersistentStorage } from "./storage";

function saveItem(key: string, value: string) {
  if (typeof window !== "undefined" && window.localStorage) {
    window.localStorage.setItem(key, value);
  }
}

function formatWampError(err: unknown): string {
  if (!err) return "Unknown error";
  if (typeof err === "object") {
    const e = err as Record<string, unknown>;
    if (Array.isArray(e.args) && e.args.length > 0) {
      return String(e.args[0]);
    }
    if (e.error) {
      return String(e.error);
    }
    if (e.message) {
      return String(e.message);
    }
    try {
      return JSON.stringify(e);
    } catch {
      return String(e);
    }
  }
  return String(err);
}

interface WampErrorObject {
  error: string;
}

function getWampErrorUri(error: unknown): string {
  if (error && typeof error === "object" && "error" in error) {
    return String((error as WampErrorObject).error);
  }
  return "";
}

function handleProposalNotFound(dispatch: Dispatch, proposedBattleId: string, state?: RootState) {
  dispatch(clearBattleState(proposedBattleId));
  if (state) {
    const existing = state.proposals.proposals[proposedBattleId];
    if (existing) {
      dispatch(
        updateProposal({
          ...existing,
          deletionReason: existing.deletionReason || "deleted",
        }),
      );
      if (existing.battle) {
        dispatch(selectBattle({ view: "battle", battleId: existing.battle }));
        return;
      }
    }
  }
}

function handleBattleError(
  dispatch: Dispatch,
  battleId: string,
  message: string,
  error: unknown,
  level: "warn" | "error" = "warn",
  prefixMessageOnUi: boolean = true,
): string {
  const formatted = formatWampError(error);
  const errorUri = getWampErrorUri(error);

  const uiErrorMsg = prefixMessageOnUi ? `${message}: ${formatted}` : formatted;

  if (level === "error") {
    console.error(`[WAMP] [Battle: ${battleId}] ${message}:`, error);
  } else {
    console.warn(`[WAMP] [Battle: ${battleId}] ${message}:`, error);
  }

  let validationProblems = null;
  let finalUiErrorMsg = uiErrorMsg;
  if (error instanceof ValidationError) {
    validationProblems = error.problems;
    finalUiErrorMsg = prefixMessageOnUi ? `${message}: Validation failed` : "Validation failed";
  }

  dispatch(setBattleError({ battleId, error: finalUiErrorMsg, validationProblems }));

  if (errorUri === "com.battler.battler_service.error.battle_not_found") {
    dispatch(removeSpectatingBattle(battleId));
    dispatch(clearBattleState(battleId));
  }

  return formatted;
}

class WampConnectionManager {
  public sessionProvider: WampSessionProvider | null = null;
  public serviceClient: BattlerServiceClient | null = null;
  public mpServiceClient: BattlerMultiplayerServiceClient | null = null;
  public multiplayerClient: BattlerMultiplayerClient | null = null;
  public proposalSubscription: Subscription | null = null;
  public readonly clientsRegistry = new Map<string, BattlerClient>();
  public readonly pendingInitializations = new Map<string, Promise<BattlerClient | undefined>>();

  public clear() {
    this.sessionProvider = null;
    this.serviceClient = null;
    this.mpServiceClient = null;
    this.multiplayerClient = null;
    this.proposalSubscription = null;
    this.clientsRegistry.clear();
    this.pendingInitializations.clear();
  }
}

export const connectionManager = new WampConnectionManager();

function bindClientEvents(
  client: BattlerClient,
  battleId: string,
  playerId: string,
  dispatch: Dispatch,
  getState?: () => RootState,
) {
  client.on("update", async () => {
    const state = client.state();
    dispatch(battleStateUpdated({ battleId, state, engineLogs: client.getLogs() }));

    const session = getState ? getState().battles.battles[battleId] : undefined;
    if (session?.isDeleted) return;

    // Only update high-level serviceBattle status while the battle is in preparation mode
    if (connectionManager.serviceClient && state.phase === "pre_battle") {
      try {
        const serviceBattle = await connectionManager.serviceClient.battle(battleId);
        dispatch(serviceBattleUpdated({ battleId, serviceBattle }));
      } catch {
        // Ignored
      }
    }
  });

  client.on("request", async (req) => {
    const session = getState ? getState().battles.battles[battleId] : undefined;
    if (session?.isDeleted) return;
    dispatch(setBattleRequest({ battleId, request: req }));
    if (connectionManager.serviceClient && client.role().type === "player") {
      try {
        const playerData = await connectionManager.serviceClient.playerData(battleId, playerId);
        dispatch(setBattlePlayerData({ battleId, playerData }));
      } catch {
        // Ignored
      }
    }
  });

  client.on("error", (err) => {
    handleBattleError(dispatch, battleId, "Battle error", err);
  });

  client.on("end", () => {
    dispatch(battleSessionEnded(battleId));
  });

  client.on("deleted", () => {
    dispatch(clearBattleState(battleId));
  });
}

// Helper to initialize active battle client
export async function initializeBattleClient(
  rawBattleId: string,
  playerId: string,
  dispatch: Dispatch,
  getState?: () => RootState,
): Promise<BattlerClient | undefined> {
  const battleId = formatUuid(rawBattleId);
  if (connectionManager.clientsRegistry.has(battleId)) {
    return connectionManager.clientsRegistry.get(battleId)!;
  }
  if (connectionManager.pendingInitializations.has(battleId)) {
    return connectionManager.pendingInitializations.get(battleId)!;
  }
  if (!connectionManager.serviceClient) {
    dispatch(setBattleLoading({ battleId, isLoading: false }));
    dispatch(setBattleError({ battleId, error: "Not connected to battle server" }));
    return;
  }

  const initPromise = (async () => {
    try {
      dispatch(setBattleLoading({ battleId, isLoading: true }));
      const existingSession = getState ? getState().battles.battles[battleId] : undefined;
      if (!existingSession?.isDeleted) {
        dispatch(setBattleError({ battleId, error: null }));
      }

      const client = await BattlerClient.create(
        battleId,
        playerId,
        connectionManager.serviceClient!,
      );
      connectionManager.clientsRegistry.set(battleId, client);
      const isSpectator = client.role().type === "spectator";
      dispatch(setIsSpectator({ battleId, isSpectator }));
      if (isSpectator) {
        dispatch(addSpectatingBattle(battleId));
      } else {
        dispatch(removeSpectatingBattle(battleId));
      }

      // Initial setup dispatch
      dispatch(
        battleStateUpdated({ battleId, state: client.state(), engineLogs: client.getLogs() }),
      );
      dispatch(setBattleRequest({ battleId, request: client.request() }));

      // Fetch initial service battle state
      if (connectionManager.serviceClient) {
        try {
          const serviceBattle = await connectionManager.serviceClient.battle(battleId);
          dispatch(serviceBattleUpdated({ battleId, serviceBattle }));
        } catch (e) {
          handleBattleError(dispatch, battleId, "Failed to fetch initial service battle state", e);
        }
        if (client.role().type === "player") {
          try {
            const playerData = await connectionManager.serviceClient.playerData(battleId, playerId);
            dispatch(setBattlePlayerData({ battleId, playerData }));
          } catch (e) {
            handleBattleError(dispatch, battleId, "Failed to fetch initial player data", e);
          }
        }
      }

      bindClientEvents(client, battleId, playerId, dispatch, getState);

      return client;
    } catch (err: unknown) {
      handleBattleError(
        dispatch,
        battleId,
        "Failed to initialize battle client",
        err,
        "error",
        false,
      );
      return undefined;
    } finally {
      dispatch(setBattleLoading({ battleId, isLoading: false }));
      connectionManager.pendingInitializations.delete(battleId);
    }
  })();

  connectionManager.pendingInitializations.set(battleId, initPromise);
  return initPromise;
}

// Helper to restore an active/historical battle session in the background
export function restoreBattleSession(
  rawBattleId: string,
  playerId: string,
  dispatch: Dispatch,
  isSpectator?: boolean,
) {
  const battleId = formatUuid(rawBattleId);
  dispatch(battleSessionRestored(battleId));
  if (isSpectator !== undefined) {
    dispatch(setIsSpectator({ battleId, isSpectator }));
    if (isSpectator) {
      dispatch(addSpectatingBattle(battleId));
    } else {
      dispatch(removeSpectatingBattle(battleId));
    }
  }
  initializeBattleClient(battleId, playerId, dispatch).catch((err) => {
    console.error(`[WAMP] Failed to restore battle client for ${battleId}:`, err);
  });
}

// Helper to restore/fetch a proposed battle session in the background
export function restoreProposalSession(
  rawBattleId: string,
  playerId: string,
  dispatch: Dispatch,
  _getState?: () => RootState,
) {
  const battleId = formatUuid(rawBattleId);
  dispatch(battleSessionRestored({ battleId, isProposal: true }));
  dispatch(setBattleLoading({ battleId, isLoading: true }));

  if (!connectionManager.multiplayerClient) {
    dispatch(setBattleLoading({ battleId, isLoading: false }));
    dispatch(setBattleError({ battleId, error: "Not connected to battle server" }));
    return;
  }
  connectionManager.multiplayerClient
    .proposedBattle(battleId)
    .then(async (proposal) => {
      dispatch(updateProposal(proposal));
      dispatch(setBattleLoading({ battleId, isLoading: false }));
      if (proposal.battle) {
        const actualBattleId = proposal.battle;
        dispatch(battleSessionCreated(actualBattleId));
        await initializeBattleClient(actualBattleId, playerId, dispatch);
      }
    })
    .catch((err) => {
      dispatch(setBattleLoading({ battleId, isLoading: false }));
      handleBattleError(
        dispatch,
        battleId,
        "Failed to fetch proposal details for active battle",
        err,
      );
      if (
        getWampErrorUri(err) ===
        "com.battler.battler_multiplayer_service.error.proposed_battle_not_found"
      ) {
        handleProposalNotFound(dispatch, battleId);
      }
    });
}

interface ProposedBattleUpdate {
  proposed_battle: ProposedBattle;
  rejection?: ProposedBattleRejection | null;
  deletion_reason?: string | null;
}

function getProposalUpdateHandler(
  playerId: string,
  dispatch: Dispatch,
  getState?: () => RootState,
) {
  return async (update: ProposedBattleUpdate) => {
    const proposalWithDetails: ProposedBattleWithDetails = {
      ...update.proposed_battle,
      rejection: update.rejection || null,
      deletionReason: update.deletion_reason || null,
    };
    dispatch(updateProposal(proposalWithDetails));
    if (update.proposed_battle.battle) {
      const battleId = update.proposed_battle.battle;
      dispatch(battleSessionCreated(battleId));
      if (update.deletion_reason && update.deletion_reason !== "fulfilled") {
        dispatch(clearBattleState(battleId));
        dispatch(
          setBattleError({
            battleId,
            error: `Battle proposal failed: ${update.deletion_reason}`,
          }),
        );
      } else {
        const client = await initializeBattleClient(battleId, playerId, dispatch, getState);
        if (client && update.deletion_reason === "fulfilled") {
          if (getState) {
            const currentBattlesState = getState().battles;
            const currentView = currentBattlesState.currentView;
            const currentBattleId = currentBattlesState.activeBattleId;
            const proposalId = update.proposed_battle.uuid;
            if (currentView === "proposal" && currentBattleId === proposalId) {
              dispatch(selectBattle({ view: "battle", battleId }));
            }
          } else {
            dispatch(selectBattle({ view: "battle", battleId }));
          }
        }
      }
    }
  };
}

// Connect thunk
export const connectWamp = createAsyncThunk<
  void,
  { url: string; playerId: string; autoconnect?: boolean },
  { state: RootState }
>(
  "wamp/connect",
  async (
    {
      url,
      playerId,
      autoconnect = false,
    }: { url: string; playerId: string; autoconnect?: boolean },
    { dispatch, getState },
  ) => {
    dispatch(setConnectionStatus("connecting"));
    dispatch(setConnectionError(null));
    dispatch(setRetryDetails(null));
    dispatch(clearBattles());

    try {
      // Cancel active battle clients and clear registry
      for (const client of connectionManager.clientsRegistry.values()) {
        try {
          await client.cancel();
        } catch (err: unknown) {
          console.warn(`[WAMP] Failed to cancel battle client during reconnect:`, err);
        }
      }
      if (connectionManager.sessionProvider) {
        try {
          connectionManager.sessionProvider.removeAllListeners();
          await connectionManager.sessionProvider.disconnect();
        } catch (e) {
          console.warn("[WAMP] error during disconnect in connectWamp:", e);
        }
      }
      connectionManager.clear();

      connectionManager.sessionProvider = new WampSessionProvider({
        url,
        realm: "com.battler",
        use_es6_promises: true,
        authmethods: ["wamp-battler-undisputed"],
        authid: playerId,
        authextra: {
          role: "user",
        },
        onchallenge: () => "role:user",
      } as IConnectionOptions);

      interface EventEmitterLike {
        on(
          event: "disconnect",
          listener: (
            reason: string | null,
            details:
              | { retry_delay?: number; retry_count?: number; will_retry?: boolean }
              | null
              | undefined,
          ) => void,
        ): void;
        on(event: "connect", listener: () => void): void;
      }
      const provider = connectionManager.sessionProvider as unknown as EventEmitterLike;

      // Register reconnection handlers
      provider.on("disconnect", (reason, details) => {
        if (details?.will_retry === false) {
          dispatch(setConnectionStatus("disconnected"));
          dispatch(setRetryDetails(null));
          if (reason) {
            dispatch(setConnectionError(`Disconnected: ${reason}`));
          }
        } else {
          dispatch(setConnectionStatus("connecting"));
          const retryDelay = details?.retry_delay ?? null;
          const retryCount = details?.retry_count ?? null;
          dispatch(setRetryDetails({ retryDelay, retryCount }));
        }
      });

      provider.on("connect", async () => {
        dispatch(setConnectionStatus("connected"));
        dispatch(setConnectionError(null));

        // Re-subscribe to proposal updates on connection restoration
        if (connectionManager.multiplayerClient) {
          if (connectionManager.proposalSubscription) {
            try {
              await connectionManager.mpServiceClient?.unsubscribe(
                connectionManager.proposalSubscription,
              );
            } catch {
              // Ignore error on dead session
            }
            connectionManager.proposalSubscription = null;
          }
          try {
            connectionManager.proposalSubscription =
              await connectionManager.multiplayerClient.proposedBattleUpdates(
                getProposalUpdateHandler(playerId, dispatch, getState),
              );
          } catch {
            // Ignored
          }
        }

        // Catch up and re-subscribe on reconnection
        for (const [battleId, client] of connectionManager.clientsRegistry.entries()) {
          try {
            await client.sync();
          } catch (e: unknown) {
            handleBattleError(
              dispatch,
              battleId,
              `Failed to sync battle ${battleId} on reconnect`,
              e,
              "error",
              false,
            );
          }
        }
      });

      connectionManager.serviceClient = new BattlerServiceClient(connectionManager.sessionProvider);
      connectionManager.mpServiceClient = new BattlerMultiplayerServiceClient(
        connectionManager.sessionProvider,
      );
      connectionManager.multiplayerClient = new BattlerMultiplayerClient(
        playerId,
        connectionManager.mpServiceClient,
        connectionManager.serviceClient,
      );

      // Connect
      await connectionManager.sessionProvider.connect();

      dispatch(setPlayerId(playerId));
      dispatch(setServerUrl(url));
      dispatch(setSavedConnectionDetails({ playerId, serverUrl: url, autoconnect }));

      // Save settings to local storage
      saveItem("battler_username", playerId);
      saveItem("battler_server_url", url);
      saveItem("battler_autoconnect", autoconnect ? "true" : "false");

      // Sync active proposals
      dispatch(clearProposals());
      let proposalsOffset = 0;
      const proposalsLimit = 50;
      while (true) {
        const page = await connectionManager.multiplayerClient.proposedBattles(
          proposalsLimit,
          proposalsOffset,
        );
        if (page.length > 0) {
          dispatch(addProposals(page));
        }
        if (page.length < proposalsLimit) {
          break;
        }
        proposalsOffset += proposalsLimit;
      }

      // Restore active battles for the player
      const restoredIds = new Set<string>();
      try {
        let battlesOffset = 0;
        const battlesLimit = 50;
        while (true) {
          const page = await connectionManager.serviceClient.battlesForPlayer(
            playerId,
            battlesLimit,
            battlesOffset,
          );
          for (const b of page) {
            restoredIds.add(formatUuid(b.uuid));
            restoreBattleSession(b.uuid, playerId, dispatch, false);
          }
          if (page.length < battlesLimit) {
            break;
          }
          battlesOffset += battlesLimit;
        }
      } catch (err: unknown) {
        console.error("[WAMP] Failed to fetch active battles for player:", err);
        dispatch(setConnectionError(`Failed to fetch active battles: ${formatWampError(err)}`));
      }

      // Restore saved spectating battles
      try {
        const storage = new LocalStoragePersistentStorage();
        const savedSpectating =
          (await storage.getItem<string[]>("battler_spectating_battles")) || [];
        dispatch(setSpectatingBattles(savedSpectating));
        for (const spectatedId of savedSpectating) {
          const normId = formatUuid(spectatedId);
          if (!restoredIds.has(normId)) {
            restoreBattleSession(spectatedId, playerId, dispatch, true);
          }
        }
      } catch (err: unknown) {
        console.error("[WAMP] Failed to restore spectating battles:", err);
      }

      // Subscribe to proposal updates
      connectionManager.proposalSubscription =
        await connectionManager.multiplayerClient.proposedBattleUpdates(
          getProposalUpdateHandler(playerId, dispatch, getState),
        );

      dispatch(setConnectionStatus("connected"));
    } catch (err: unknown) {
      dispatch(setConnectionStatus("disconnected"));
      const errorMsg = formatWampError(err);
      dispatch(setConnectionError(errorMsg, err));
      throw err;
    }
  },
);

// Disconnect thunk
export const disconnectWamp = createAsyncThunk("wamp/disconnect", async (_, { dispatch }) => {
  // Cancel active battle clients
  for (const client of connectionManager.clientsRegistry.values()) {
    try {
      await client.cancel();
    } catch (err: unknown) {
      console.warn(`[WAMP] Failed to cancel battle client during disconnect:`, err);
    }
  }

  if (connectionManager.proposalSubscription && connectionManager.multiplayerClient) {
    try {
      await connectionManager.mpServiceClient?.unsubscribe(connectionManager.proposalSubscription);
    } catch (err: unknown) {
      console.error("[WAMP] Failed to unsubscribe during disconnect:", err);
    }
  }
  if (connectionManager.sessionProvider) {
    try {
      connectionManager.sessionProvider.removeAllListeners();
      await connectionManager.sessionProvider.disconnect();
    } catch (err: unknown) {
      console.error("[WAMP] Failed to disconnect session:", err);
    }
  }
  connectionManager.clear();

  dispatch(setConnectionStatus("disconnected"));
  dispatch(setPlayerId(null));
  dispatch(setConnectionError(null));
  dispatch(setAutoconnect(false));
  dispatch(clearProposals());
  dispatch(resetBattlesState());

  // Disable autoconnect on next visit since user manually disconnected
  saveItem("battler_autoconnect", "false");
});

// Close Battle Session thunk
export const closeBattleSession = createAsyncThunk(
  "wamp/closeBattleSession",
  async (rawBattleId: string, { dispatch }) => {
    const battleId = formatUuid(rawBattleId);
    const client = connectionManager.clientsRegistry.get(battleId);
    if (client) {
      try {
        await client.cancel();
      } catch (err) {
        console.warn(`[WAMP] Failed to cancel battle client for ${battleId}:`, err);
      }
      connectionManager.clientsRegistry.delete(battleId);
    }
    dispatch(removeSpectatingBattle(battleId));
    dispatch(removeBattle(battleId));
  },
);

// Propose Battle thunk
export const proposeBattle = createAsyncThunk(
  "wamp/proposeBattle",
  async (options: ProposedBattleOptions, { rejectWithValue }) => {
    if (!connectionManager.multiplayerClient) return rejectWithValue("Not connected");
    try {
      const proposal = await connectionManager.multiplayerClient.proposeBattle(options);
      return proposal;
    } catch (err: unknown) {
      console.error("[WAMP] Propose battle failed:", err);
      return rejectWithValue(formatWampError(err));
    }
  },
);

// Respond to Proposal thunk
export const respondToProposal = createAsyncThunk<
  unknown,
  { proposedBattleId: string; accept: boolean },
  { state: RootState }
>(
  "wamp/respondToProposal",
  async ({ proposedBattleId, accept }: { proposedBattleId: string; accept: boolean }, thunkAPI) => {
    const { dispatch, getState, rejectWithValue } = thunkAPI;
    if (!connectionManager.multiplayerClient) return rejectWithValue("Not connected");
    try {
      const updated = await connectionManager.multiplayerClient.respondToProposal(
        proposedBattleId,
        accept,
      );
      return updated;
    } catch (err: unknown) {
      console.error("[WAMP] Respond to proposal failed:", err);
      const errorMsg = formatWampError(err);
      const errorUri = getWampErrorUri(err);
      if (errorUri === "com.battler.battler_multiplayer_service.error.proposed_battle_not_found") {
        handleProposalNotFound(dispatch, proposedBattleId, getState());
      }
      return rejectWithValue(errorMsg);
    }
  },
);

// Submit Choice thunk
export const submitChoice = createAsyncThunk(
  "wamp/submitChoice",
  async (
    { battleId: rawId, choice }: { battleId: string; choice: string },
    { dispatch, rejectWithValue },
  ) => {
    const battleId = formatUuid(rawId);
    const client = connectionManager.clientsRegistry.get(battleId);
    if (!client) return rejectWithValue(`No client found for battle ${battleId}`);

    try {
      dispatch(setBattleLoading({ battleId, isLoading: true }));
      await client.makeChoice(choice);
      dispatch(setChoiceError({ battleId, error: null }));
      dispatch(setChoiceSubmitted({ battleId, submitted: true }));
    } catch (err: unknown) {
      const formatted = formatWampError(err);
      const errorUri = getWampErrorUri(err);
      if (errorUri === "com.battler.battler_service.error.battle_not_found") {
        dispatch(clearBattleState(battleId));
      }
      dispatch(setChoiceError({ battleId, error: formatted }));
      dispatch(setChoiceSubmitted({ battleId, submitted: false }));
      return rejectWithValue(formatted);
    } finally {
      dispatch(setBattleLoading({ battleId, isLoading: false }));
    }
  },
);

// Submit Battle Team thunk
export const submitBattleTeam = createAsyncThunk(
  "wamp/submitBattleTeam",
  async (
    { battleId: rawId, team }: { battleId: string; team: MonData[] },
    { dispatch, rejectWithValue },
  ) => {
    const battleId = formatUuid(rawId);
    const client = connectionManager.clientsRegistry.get(battleId);
    if (!client) return rejectWithValue(`No client found for battle ${battleId}`);
    try {
      dispatch(setBattleLoading({ battleId, isLoading: true }));
      dispatch(setBattleError({ battleId, error: null }));
      await client.updateTeam({ members: team, bag: { items: {} } });

      dispatch(setChoiceSubmitted({ battleId, submitted: true }));
    } catch (err: unknown) {
      const formatted = handleBattleError(
        dispatch,
        battleId,
        "Submit battle team failed",
        err,
        "error",
        false,
      );
      return rejectWithValue(formatted);
    } finally {
      dispatch(setBattleLoading({ battleId, isLoading: false }));
    }
  },
);

// Refresh Lobby thunk
export const refreshLobby = createAsyncThunk<void, string, { state: RootState }>(
  "wamp/refreshLobby",
  async (playerId: string, { dispatch, getState }) => {
    if (!connectionManager.multiplayerClient) return;

    // Restart subscription if active
    if (connectionManager.proposalSubscription) {
      try {
        await connectionManager.mpServiceClient?.unsubscribe(
          connectionManager.proposalSubscription,
        );
      } catch (err) {
        console.warn("[WAMP] Failed to unsubscribe from proposals during refresh:", err);
      }
      connectionManager.proposalSubscription = null;
    }

    dispatch(clearProposals());

    // Re-subscribe
    try {
      connectionManager.proposalSubscription =
        await connectionManager.multiplayerClient.proposedBattleUpdates(
          getProposalUpdateHandler(playerId, dispatch, getState),
        );
    } catch (err) {
      console.error("[WAMP] Failed to re-subscribe to proposal updates during refresh:", err);
    }

    // Fetch proposed battles
    try {
      let proposalsOffset = 0;
      const proposalsLimit = 50;
      while (true) {
        const page = await connectionManager.multiplayerClient.proposedBattles(
          proposalsLimit,
          proposalsOffset,
        );
        if (page.length > 0) {
          dispatch(addProposals(page));
        }
        if (page.length < proposalsLimit) {
          break;
        }
        proposalsOffset += proposalsLimit;
      }
    } catch (err) {
      console.error("[WAMP] Failed to fetch proposed battles during refresh:", err);
      dispatch(setConnectionError(`Failed to refresh proposed battles: ${formatWampError(err)}`));
    }

    // Fetch active battles for the player to refresh sidebar/battles list
    try {
      let battlesOffset = 0;
      const battlesLimit = 50;
      const fetchedBattleIds = new Set<string>();
      while (true) {
        if (!connectionManager.serviceClient) break;
        const page = await connectionManager.serviceClient.battlesForPlayer(
          playerId,
          battlesLimit,
          battlesOffset,
        );
        for (const b of page) {
          const battleId = formatUuid(b.uuid);
          fetchedBattleIds.add(battleId);
          const existingClient = connectionManager.clientsRegistry.get(battleId);
          if (existingClient) {
            existingClient.sync().catch((err: unknown) => {
              console.warn(
                `[WAMP] Failed to sync existing battle client for ${battleId} during lobby refresh:`,
                err,
              );
              handleBattleError(dispatch, battleId, "Failed to sync battle client", err);
            });
          } else {
            restoreBattleSession(battleId, playerId, dispatch);
          }
        }
        if (page.length < battlesLimit) {
          break;
        }
        battlesOffset += battlesLimit;
      }

      if (connectionManager.serviceClient) {
        const currentBattles = getState().battles.battles;
        for (const [id, battle] of Object.entries(currentBattles)) {
          if (!battle.isReplay && !battle.isProposal && !battle.isDeleted) {
            const isSpectator = isSpectatorSession(battle, playerId);
            if (!isSpectator && !fetchedBattleIds.has(id)) {
              dispatch(clearBattleState(id));
            }
          }
        }
      }
    } catch (err) {
      console.error("[WAMP] Failed to fetch active battles during lobby refresh:", err);
      dispatch(setConnectionError(`Failed to refresh active battles: ${formatWampError(err)}`));
    }
  },
);

// Refresh Proposal Session thunk
export const refreshProposalSession = createAsyncThunk<
  unknown,
  { battleId: string; playerId: string },
  { state: RootState }
>(
  "wamp/refreshProposalSession",
  async ({ battleId: rawId, playerId }: { battleId: string; playerId: string }, thunkAPI) => {
    const { dispatch, getState } = thunkAPI;
    const battleId = formatUuid(rawId);
    if (!connectionManager.multiplayerClient) return;

    dispatch(setBattleLoading({ battleId, isLoading: true }));
    try {
      const proposal = await connectionManager.multiplayerClient.proposedBattle(battleId);
      dispatch(updateProposal(proposal));
      if (proposal.battle) {
        const actualBattleId = proposal.battle;
        dispatch(battleSessionCreated(actualBattleId));
        const existingClient = connectionManager.clientsRegistry.get(actualBattleId);
        if (existingClient) {
          existingClient.sync().catch((err: unknown) => {
            console.warn(
              `[WAMP] Failed to sync existing battle client for ${actualBattleId}:`,
              err,
            );
          });
        } else {
          await initializeBattleClient(actualBattleId, playerId, dispatch);
        }
      }
    } catch (err: unknown) {
      console.error(`[WAMP] Failed to refresh proposal for ${battleId}:`, err);
      const errorMsg = formatWampError(err);
      const errorUri = getWampErrorUri(err);
      dispatch(setBattleError({ battleId, error: errorMsg }));
      if (errorUri === "com.battler.battler_multiplayer_service.error.proposed_battle_not_found") {
        handleProposalNotFound(dispatch, battleId, getState());
      }
    } finally {
      dispatch(setBattleLoading({ battleId, isLoading: false }));
    }
  },
);

// Refresh Battle Session thunk
export const refreshBattleSession = createAsyncThunk<
  void,
  { battleId: string; playerId: string },
  { state: RootState }
>(
  "wamp/refreshBattleSession",
  async (
    { battleId: rawId, playerId }: { battleId: string; playerId: string },
    { dispatch, getState },
  ) => {
    const battleId = formatUuid(rawId);
    const client = connectionManager.clientsRegistry.get(battleId);
    if (client) {
      try {
        await client.cancel();
      } catch (err) {
        console.warn(`[WAMP] Failed to cancel existing battle client for ${battleId}:`, err);
      }
      connectionManager.clientsRegistry.delete(battleId);
    }

    dispatch(setBattleLoading({ battleId, isLoading: true }));
    try {
      await initializeBattleClient(battleId, playerId, dispatch, getState);
    } catch (err) {
      console.error(`[WAMP] Failed to refresh battle client for ${battleId}:`, err);
      handleBattleError(dispatch, battleId, "Failed to refresh battle client", err);
    } finally {
      dispatch(setBattleLoading({ battleId, isLoading: false }));
    }
  },
);

// Check Battle Status thunk
export const checkBattleStatus = createAsyncThunk(
  "wamp/checkBattleStatus",
  async (battleId: string, { dispatch }) => {
    const formattedId = formatUuid(battleId);
    if (!connectionManager.serviceClient) return;
    try {
      await connectionManager.serviceClient.battle(formattedId);
    } catch (err) {
      handleBattleError(dispatch, formattedId, "Battle status check failed", err);
    }
  },
);

// Fetch Paginated Battles thunk
export const fetchBattles = createAsyncThunk<
  BattlePreview[],
  { count: number; offset: number },
  { rejectValue: string }
>("wamp/fetchBattles", async ({ count, offset }, { rejectWithValue }) => {
  if (!connectionManager.serviceClient) {
    return rejectWithValue("Not connected to battle server");
  }
  try {
    return await connectionManager.serviceClient.battles(count, offset);
  } catch (err) {
    return rejectWithValue(formatWampError(err));
  }
});
