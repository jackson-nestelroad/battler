import { createAsyncThunk } from "@reduxjs/toolkit";
import type { Dispatch } from "@reduxjs/toolkit";
import { WampSessionProvider } from "battler-wamp-client";
import { BattlerMultiplayerClient } from "battler-multiplayer-client";
import { BattlerClient } from "battler-client";
import { BattlerServiceClient } from "battler-service-client";
import { BattlerMultiplayerServiceClient } from "battler-multiplayer-service-client";
import type { ProposedBattleOptions } from "battler-multiplayer-service-client";
import type { Subscription, IConnectionOptions } from "autobahn";
import type { MonData } from "battler-types";
import {
  setConnectionStatus,
  setPlayerId,
  setServerUrl,
  setConnectionError,
  setSavedConnectionDetails,
  setAutoconnect,
} from "../store/connectionSlice";
import { setProposals, updateProposal, clearProposals } from "../store/proposalsSlice";
import type { ProposedBattleWithDetails } from "../store/proposalsSlice";
import {
  battleSessionCreated,
  battleStateUpdated,
  setBattleRequest,
  setBattleError,
  setBattleLoading,
  battleSessionEnded,
  serviceBattleUpdated,
  setChoiceSubmitted,
  setBattlePlayerData,
  clearBattles,
} from "../store/battlesSlice";
import { setCookie } from "../utils/cookie";

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

function handleBattleError(
  dispatch: Dispatch,
  battleId: string,
  message: string,
  error: unknown,
  level: "warn" | "error" = "warn",
  prefixMessageOnUi: boolean = true,
): string {
  const formatted = formatWampError(error);
  const uiErrorMsg = prefixMessageOnUi ? `${message}: ${formatted}` : formatted;

  if (level === "error") {
    console.error(`[WAMP] [Battle: ${battleId}] ${message}:`, error);
  } else {
    console.warn(`[WAMP] [Battle: ${battleId}] ${message}:`, error);
  }

  dispatch(setBattleError({ battleId, error: uiErrorMsg }));
  return formatted;
}

class WampConnectionManager {
  public sessionProvider: WampSessionProvider | null = null;
  public serviceClient: BattlerServiceClient | null = null;
  public mpServiceClient: BattlerMultiplayerServiceClient | null = null;
  public multiplayerClient: BattlerMultiplayerClient | null = null;
  public proposalSubscription: Subscription | null = null;
  public readonly clientsRegistry = new Map<string, BattlerClient>();

  public clear() {
    this.sessionProvider = null;
    this.serviceClient = null;
    this.mpServiceClient = null;
    this.multiplayerClient = null;
    this.proposalSubscription = null;
    this.clientsRegistry.clear();
  }
}

export const connectionManager = new WampConnectionManager();

// Helper to initialize active battle client
export async function initializeBattleClient(
  battleId: string,
  playerId: string,
  dispatch: Dispatch,
) {
  if (connectionManager.clientsRegistry.has(battleId)) {
    return connectionManager.clientsRegistry.get(battleId)!;
  }
  if (!connectionManager.serviceClient) return;

  try {
    dispatch(setBattleLoading({ battleId, isLoading: true }));
    dispatch(setBattleError({ battleId, error: null }));

    const client = await BattlerClient.create(battleId, playerId, connectionManager.serviceClient);
    connectionManager.clientsRegistry.set(battleId, client);

    // Initial setup dispatch
    dispatch(battleStateUpdated({ battleId, state: client.state(), engineLogs: client.getLogs() }));

    // Fetch initial service battle state
    if (connectionManager.serviceClient) {
      try {
        const serviceBattle = await connectionManager.serviceClient.battle(battleId);
        dispatch(serviceBattleUpdated({ battleId, serviceBattle }));
      } catch (e) {
        handleBattleError(dispatch, battleId, "Failed to fetch initial service battle state", e);
      }
      if (client.getRole().type === "player") {
        try {
          const playerData = await connectionManager.serviceClient.playerData(battleId, playerId);
          dispatch(setBattlePlayerData({ battleId, playerData }));
        } catch (e) {
          handleBattleError(dispatch, battleId, "Failed to fetch initial player data", e);
        }
      }
    }

    client.on("update", async () => {
      const state = client.state();
      dispatch(battleStateUpdated({ battleId, state, engineLogs: client.getLogs() }));

      // Fetch the service battle state to update player Ready/Waiting status!
      if (connectionManager.serviceClient) {
        try {
          const serviceBattle = await connectionManager.serviceClient.battle(battleId);
          dispatch(serviceBattleUpdated({ battleId, serviceBattle }));
        } catch (e) {
          handleBattleError(dispatch, battleId, "Failed to fetch service battle update", e);
        }
        if (client.getRole().type === "player") {
          try {
            const playerData = await connectionManager.serviceClient.playerData(battleId, playerId);
            dispatch(setBattlePlayerData({ battleId, playerData }));
          } catch (e) {
            handleBattleError(dispatch, battleId, "Failed to fetch player data on update", e);
          }
        }
      }
    });

    client.on("request", async (req) => {
      dispatch(setBattleRequest({ battleId, request: req }));
      if (connectionManager.serviceClient && client.getRole().type === "player") {
        try {
          const playerData = await connectionManager.serviceClient.playerData(battleId, playerId);
          dispatch(setBattlePlayerData({ battleId, playerData }));
        } catch (e) {
          handleBattleError(dispatch, battleId, "Failed to fetch player data on request", e);
        }
      }
    });

    client.on("error", (err) => {
      dispatch(setBattleError({ battleId, error: formatWampError(err) }));
    });

    client.on("end", () => {
      dispatch(battleSessionEnded(battleId));
    });

    // Sync the client now that listeners are registered to fetch initial request/state
    await client.sync();

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
  } finally {
    dispatch(setBattleLoading({ battleId, isLoading: false }));
  }
}

// Connect thunk
export const connectWamp = createAsyncThunk(
  "wamp/connect",
  async (
    {
      url,
      playerId,
      autoconnect = false,
    }: { url: string; playerId: string; autoconnect?: boolean },
    { dispatch },
  ) => {
    dispatch(setConnectionStatus("connecting"));
    dispatch(setConnectionError(null));

    try {
      // Disconnect existing session if any
      if (connectionManager.sessionProvider) {
        await connectionManager.sessionProvider.disconnect();
      }

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

      // Connect
      await connectionManager.sessionProvider.connect();

      // Instantiate services
      connectionManager.serviceClient = new BattlerServiceClient(connectionManager.sessionProvider);
      connectionManager.mpServiceClient = new BattlerMultiplayerServiceClient(
        connectionManager.sessionProvider,
      );
      connectionManager.multiplayerClient = new BattlerMultiplayerClient(
        playerId,
        connectionManager.mpServiceClient,
        connectionManager.serviceClient,
      );

      dispatch(setPlayerId(playerId));
      dispatch(setServerUrl(url));
      dispatch(setConnectionStatus("connected"));
      dispatch(setSavedConnectionDetails({ playerId, serverUrl: url, autoconnect }));

      // Save settings to cookies
      setCookie("battler_username", playerId);
      setCookie("battler_server_url", url);
      setCookie("battler_autoconnect", autoconnect ? "true" : "false");

      // Sync active proposals
      const activeProposals = await connectionManager.multiplayerClient.proposedBattles(20, 0);
      dispatch(setProposals(activeProposals));

      // Subscribe to proposal updates
      connectionManager.proposalSubscription =
        await connectionManager.multiplayerClient.proposedBattleUpdates(async (update) => {
          const proposalWithDetails: ProposedBattleWithDetails = {
            ...update.proposed_battle,
            rejection: update.rejection || null,
            deletionReason: update.deletion_reason || null,
          };
          dispatch(updateProposal(proposalWithDetails));
          if (update.proposed_battle.battle) {
            const battleId = update.proposed_battle.battle;
            dispatch(battleSessionCreated(battleId));
            const client = await initializeBattleClient(battleId, playerId, dispatch);
            if (client && update.deletion_reason === "fulfilled") {
              client.sync().catch((err: unknown) => {
                handleBattleError(
                  dispatch,
                  battleId,
                  "Failed to sync battle client on fulfillment",
                  err,
                );
              });
            }
          }
        });

      interface EventEmitterLike {
        on(event: string, listener: (...args: unknown[]) => void): void;
      }
      const provider = connectionManager.sessionProvider as unknown as EventEmitterLike;

      // Register reconnection handlers
      provider.on("disconnect", () => {
        dispatch(setConnectionStatus("connecting"));
        dispatch(setConnectionError("Connection lost. Reconnecting..."));
      });

      provider.on("connect", async () => {
        dispatch(setConnectionStatus("connected"));
        dispatch(setConnectionError(null));
        // Catch up on reconnection
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
  dispatch(clearBattles());

  // Disable autoconnect on next visit since user manually disconnected
  setCookie("battler_autoconnect", "false");
});

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
export const respondToProposal = createAsyncThunk(
  "wamp/respondToProposal",
  async (
    { proposedBattleId, accept }: { proposedBattleId: string; accept: boolean },
    { rejectWithValue },
  ) => {
    if (!connectionManager.multiplayerClient) return rejectWithValue("Not connected");
    try {
      const updated = await connectionManager.multiplayerClient.respondToProposal(
        proposedBattleId,
        accept,
      );
      return updated;
    } catch (err: unknown) {
      console.error("[WAMP] Respond to proposal failed:", err);
      return rejectWithValue(formatWampError(err));
    }
  },
);

// Submit Choice thunk
export const submitChoice = createAsyncThunk(
  "wamp/submitChoice",
  async (
    { battleId, choice }: { battleId: string; choice: string },
    { dispatch, rejectWithValue },
  ) => {
    const client = connectionManager.clientsRegistry.get(battleId);
    if (!client) return rejectWithValue(`No client found for battle ${battleId}`);

    try {
      dispatch(setBattleLoading({ battleId, isLoading: true }));
      dispatch(setBattleError({ battleId, error: null }));
      dispatch(setChoiceSubmitted({ battleId, submitted: true }));
      await client.makeChoice(choice);
    } catch (err: unknown) {
      const formatted = handleBattleError(
        dispatch,
        battleId,
        "Submit choice failed",
        err,
        "error",
        false,
      );
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
    { battleId, team }: { battleId: string; team: MonData[] },
    { dispatch, rejectWithValue },
  ) => {
    const client = connectionManager.clientsRegistry.get(battleId);
    if (!client) return rejectWithValue(`No client found for battle ${battleId}`);
    try {
      dispatch(setBattleLoading({ battleId, isLoading: true }));
      dispatch(setBattleError({ battleId, error: null }));
      await client.updateTeam({ members: team, bag: { items: {} } });

      // Fetch latest service battle state to refresh ready status!
      if (connectionManager.serviceClient) {
        const serviceBattle = await connectionManager.serviceClient.battle(battleId);
        dispatch(serviceBattleUpdated({ battleId, serviceBattle }));
        if (serviceBattle.state === "active") {
          client.sync().catch((err: unknown) => {
            handleBattleError(
              dispatch,
              battleId,
              "Failed to sync battle client on submit team",
              err,
            );
          });
        }
      }
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
