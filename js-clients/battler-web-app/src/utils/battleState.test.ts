import { describe, expect, it } from "vitest";
import { getBattleStateLabel, isMonDynamaxedInState } from "./battleState";
import type { BattleState } from "battler-state";

describe("getBattleStateLabel", () => {
  it("returns Preparing when state is preparing", () => {
    expect(getBattleStateLabel({ state: "preparing" })).toBe("Preparing");
  });

  it("returns Finished when state or phase is finished", () => {
    expect(getBattleStateLabel({ state: "finished" })).toBe("Finished");
    expect(getBattleStateLabel({ phase: "finished" })).toBe("Finished");
  });

  it("returns Preview when phase is pre_battle or turn is 0", () => {
    expect(getBattleStateLabel({ state: "active", turn: 0 })).toBe("Preview");
    expect(getBattleStateLabel({ phase: "pre_battle", turn: 1 })).toBe("Preview");
  });

  it("returns Turn X when turn is greater than 0", () => {
    expect(getBattleStateLabel({ state: "active", turn: 1 })).toBe("Turn 1");
    expect(getBattleStateLabel({ state: "active", turn: 14 })).toBe("Turn 14");
  });
});

describe("isMonDynamaxedInState", () => {
  it("returns false for null or undefined battleState", () => {
    expect(isMonDynamaxedInState(null, 0, 0)).toBe(false);
    expect(isMonDynamaxedInState(undefined, 0, 0)).toBe(false);
  });

  it("returns true when the active mon at the given field position has Dynamax condition", () => {
    const mockState = {
      field: {
        sides: [
          {
            id: 0,
            name: "Side 1",
            players: {
              "player-1": {
                id: "player-1",
                name: "Player 1",
                mons: [
                  {
                    physical_appearance: { name: "Pikachu", species: "Pikachu", gender: "M" },
                    volatile_data: {
                      conditions: {
                        Dynamax: { since_turn: 1, data: {} },
                      },
                    },
                  },
                  {
                    physical_appearance: { name: "Charizard", species: "Charizard", gender: "M" },
                    volatile_data: {
                      conditions: {},
                    },
                  },
                ],
              },
            },
            active: [
              { player: "player-1", mon_index: 0, battle_appearance_index: 0 },
              { player: "player-1", mon_index: 1, battle_appearance_index: 0 },
            ],
          },
        ],
      },
    } as unknown as BattleState;

    expect(isMonDynamaxedInState(mockState, 0, 0)).toBe(true);
    expect(isMonDynamaxedInState(mockState, 0, 1)).toBe(false);
    expect(isMonDynamaxedInState(mockState, 0, 2)).toBe(false); // empty slot
  });
});
