import { describe, it, expect } from "vitest";
import { configureStore } from "@reduxjs/toolkit";
import battlesReducer, {
  battleSessionCreated,
  battleStateUpdated,
} from "./battlesSlice";
import type { BattleState, UiLogEntry } from "battler-state";

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
      Extension: {
        source: "-battlerservice",
        title: "timer",
        values: {
          action: "player-1",
          remainingsecs: "0",
          deadline: "1784498900",
          done: "",
        },
      },
    } as unknown as UiLogEntry;

    const mockBattleState: BattleState = {
      turn: 1,
      phase: "play",
      ui_log: [
        [doneTimerEntry],
      ],
    } as unknown as BattleState;

    store.dispatch(
      battleStateUpdated({
        battleId,
        state: mockBattleState,
      })
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
});
