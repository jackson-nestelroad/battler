import { describe, it, expect } from "vitest";
import { precomputeReplayKeyframes, resolveReplayTurnState } from "./replay";

describe("replay utilities", () => {
  const mockLogs = [
    "info|battletype:singles",
    "side|id:0|name:Side 1",
    "side|id:1|name:Side 2",
    "maxsidelength|length:1",
    "player|id:player-1|name:Player 1|side:0|position:0",
    "player|id:player-2|name:Player 2|side:1|position:0",
    "teamsize|player:player-1|size:3",
    "teamsize|player:player-2|size:3",
    "battlestart",
    "switch|player:player-1|position:1|name:Pikachu|health:100/100|species:Pikachu|level:5",
    "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5",
    "turn|turn:1",
    "move|mon:Pikachu,player-1,1|name:Thunderbolt",
    "turn|turn:2",
    "move|mon:Charmander,player-2,1|name:Ember",
  ];

  it("should precompute keyframes starting at battlestart log", () => {
    const { keyframes, maxTurn } = precomputeReplayKeyframes(mockLogs);
    expect(maxTurn).toBe(2);
    expect(keyframes.length).toBe(2); // turn 0 and final step (maxTurn + 1 = 3)
    const turn0 = keyframes.find((k) => k.turn === 0);
    expect(turn0).toBeDefined();
    expect(turn0?.state.phase).toBe("battle");
    const side0 = turn0?.state.field.sides[0];
    expect(side0?.active[0]).toBeUndefined();
  });

  it("should resolve subsequent turns with correct switch-ins and turns applied", () => {
    const { keyframes, maxTurn } = precomputeReplayKeyframes(mockLogs);
    // Steps are: 0 (start), 1 (turn 1 start), 2 (turn 2 start), 3 (end)
    const maxStep = maxTurn + 1;
    const session = {
      replayStates: new Array(maxStep + 1),
      replayEngineLogs: mockLogs,
      replayKeyframes: keyframes,
    };

    // Pre-fill keyframes into the cache
    for (const kf of keyframes) {
      session.replayStates[kf.turn] = kf.state;
    }

    // Step 0: Start state
    const step0State = resolveReplayTurnState(session, 0);
    expect(step0State.field.sides[0].active[0]).toBeUndefined();

    // Step 1: Turn 1 start (after switches, before turn 1 moves)
    const step1State = resolveReplayTurnState(session, 1);
    expect(step1State.turn).toBe(0); // turn has not advanced to 1 yet in state
    const side0 = step1State.field.sides[0];
    const activeRef = side0?.active[0];
    expect(activeRef).toBeDefined();
    expect(activeRef).not.toBeNull();
    const activeMon = side0?.players[activeRef!.player]?.mons[activeRef!.mon_index];
    expect(activeMon?.physical_appearance.name).toBe("Pikachu");

    // Step 2: Turn 2 start (after turn 1 moves, before turn 2 moves)
    const step2State = resolveReplayTurnState(session, 2);
    expect(step2State.turn).toBe(1);

    // Step 3: End state (after turn 2 moves)
    const step3State = resolveReplayTurnState(session, 3);
    expect(step3State.turn).toBe(2);
  });
});
