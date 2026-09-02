import { configureStore } from "@reduxjs/toolkit";
import type { BattleState, UiLogEntry } from "battler-state";
import { describe, expect, it } from "vitest";
import battlesReducer, {
  battleSessionCreated,
  battleStateUpdated,
  setBattleRequest,
  setChoiceSubmitted,
} from "./battlesSlice";

describe("battlesSlice active timers", () => {
  it("should retain active timers with isDone status instead of deleting them when they expire", () => {
    const store = configureStore({
      reducer: {
        battles: battlesReducer,
      },
    });

    const battleId = "15cf2863-792b-4afc-8852-3aa6481m268e";
    store.dispatch(battleSessionCreated(battleId));

    // Construct a mock ui_log containing a timer done event
    const doneTimerEntry: UiLogEntry = {
      title: "timer",
      side: null,
      slot: null,
      player: null,
      target: null,
      source: null,
      effect: null,
      source_effect: null,
      values: {
        source: "-battlerservice",
        action: "player-1",
        remainingsecs: "0",
        deadline: "1784498900",
        done: "",
      },
    };

    const mockBattleState: BattleState = {
      turn: 1,
      phase: "play",
      ui_log: [[doneTimerEntry]],
    } as unknown as BattleState;

    store.dispatch(
      battleStateUpdated({
        battleId,
        state: mockBattleState,
      }),
    );

    const state = store.getState().battles;
    const battle = state.battles[battleId];
    expect(battle).toBeDefined();

    const timers = battle.activeTimers;
    expect(timers).toBeDefined();
    expect(timers?.["action:player-1"]).toEqual({
      type: "action",
      playerId: "player-1",
      remainingSecs: 0,
      deadlineSecs: 1784498900,
      isDone: true,
      isInactive: false,
    });
  });

  it("should parse and retain player bank timer when player is on entry.player", () => {
    const store = configureStore({
      reducer: {
        battles: battlesReducer,
      },
    });

    const battleId = "15cf2863-792b-4afc-8852-3aa6481m268e";
    store.dispatch(battleSessionCreated(battleId));

    // Entry as produced by battler-state WASM parser: player is top-level on entry
    const playerTimerEntry: UiLogEntry = {
      title: "timer",
      side: null,
      slot: null,
      player: "player-1",
      target: null,
      source: null,
      effect: null,
      source_effect: null,
      values: {
        source: "-battlerservice",
        remainingsecs: "420",
        deadline: "1784498900",
      },
    };

    const mockBattleState: BattleState = {
      turn: 1,
      phase: "play",
      ui_log: [[playerTimerEntry]],
    } as unknown as BattleState;

    store.dispatch(
      battleStateUpdated({
        battleId,
        state: mockBattleState,
      }),
    );

    const state = store.getState().battles;
    const battle = state.battles[battleId];
    expect(battle).toBeDefined();

    const timers = battle.activeTimers;
    expect(timers).toBeDefined();
    expect(timers?.["player:player-1"]).toEqual({
      type: "player",
      playerId: "player-1",
      remainingSecs: 420,
      deadlineSecs: 1784498900,
      isDone: false,
      isInactive: false,
    });
  });

  it("should delete teampreview timer when it is cleared", () => {
    const store = configureStore({
      reducer: {
        battles: battlesReducer,
      },
    });

    const battleId = "15cf2863-792b-4afc-8852-3aa6481m268e";
    store.dispatch(battleSessionCreated(battleId));

    const clearTimerEntry: UiLogEntry = {
      title: "timer",
      side: null,
      slot: null,
      player: null,
      target: null,
      source: null,
      effect: null,
      source_effect: null,
      values: {
        source: "-battlerservice",
        teampreview: "",
        remainingsecs: "48",
        deadline: "1784498900",
        clear: "",
      },
    };

    const mockBattleState: BattleState = {
      turn: 0,
      phase: "play",
      ui_log: [[clearTimerEntry]],
    } as unknown as BattleState;

    store.dispatch(
      battleStateUpdated({
        battleId,
        state: mockBattleState,
      }),
    );

    const state = store.getState().battles;
    const battle = state.battles[battleId];
    expect(battle).toBeDefined();

    const timers = battle.activeTimers;
    expect(timers).toBeDefined();
    expect(timers?.["teampreview"]).toBeUndefined();
  });

  it("should delete teampreview timer when it is done", () => {
    const store = configureStore({
      reducer: {
        battles: battlesReducer,
      },
    });

    const battleId = "15cf2863-792b-4afc-8852-3aa6481m268e";
    store.dispatch(battleSessionCreated(battleId));

    const doneTimerEntry: UiLogEntry = {
      title: "timer",
      side: null,
      slot: null,
      player: null,
      target: null,
      source: null,
      effect: null,
      source_effect: null,
      values: {
        source: "-battlerservice",
        teampreview: "",
        remainingsecs: "0",
        deadline: "1784498900",
        done: "",
      },
    };

    const mockBattleState: BattleState = {
      turn: 0,
      phase: "play",
      ui_log: [[doneTimerEntry]],
    } as unknown as BattleState;

    store.dispatch(
      battleStateUpdated({
        battleId,
        state: mockBattleState,
      }),
    );

    const state = store.getState().battles;
    const battle = state.battles[battleId];
    expect(battle).toBeDefined();

    const timers = battle.activeTimers;
    expect(timers).toBeDefined();
    expect(timers?.["teampreview"]).toBeUndefined();
  });

  it("should mark battle as deleted and set default error when clearBattleState is dispatched", () => {
    const store = configureStore({
      reducer: {
        battles: battlesReducer,
      },
    });

    const battleId = "15cf2863-792b-4afc-8852-3aa6481m268e";
    store.dispatch(battleSessionCreated(battleId));

    store.dispatch({
      type: "battles/clearBattleState",
      payload: battleId,
    });

    const state = store.getState().battles;
    const battle = state.battles[battleId];
    expect(battle).toBeDefined();
    expect(battle?.isDeleted).toBe(true);
    expect(battle?.error).toBe("Battle no longer exists");
  });

  it("should not prematurely clear choiceSubmitted when battleState turn increments", () => {
    const store = configureStore({
      reducer: {
        battles: battlesReducer,
      },
    });

    const battleId = "15cf2863-792b-4afc-8852-3aa6481m268e";
    store.dispatch(battleSessionCreated(battleId));
    store.dispatch(setChoiceSubmitted({ battleId, submitted: true }));

    expect(store.getState().battles.battles[battleId]?.choiceSubmitted).toBe(true);

    // When a new turn state arrives mid-stream (turn increments from 1 to 2)
    const mockBattleState: BattleState = {
      turn: 2,
      phase: "play",
      ui_log: [],
    } as unknown as BattleState;

    store.dispatch(
      battleStateUpdated({
        battleId,
        state: mockBattleState,
      }),
    );

    // choiceSubmitted must remain true so choice selection UI does not prematurely pop up
    expect(store.getState().battles.battles[battleId]?.choiceSubmitted).toBe(true);

    // When the new turn's request actually arrives, choiceSubmitted is cleared
    store.dispatch(
      setBattleRequest({
        battleId,
        request: { type: "turn", active: [] } as any,
      }),
    );

    expect(store.getState().battles.battles[battleId]?.choiceSubmitted).toBe(false);
  });
});
