import { describe, expect, it } from "vitest";
import {
  getAllyPlayerIds,
  getBattleStateLabel,
  isMonActiveOnField,
  isMonDynamaxedInState,
  isMonFaintedInState,
} from "./battleState";
import type { BattleState } from "battler-state";
import type { PlayerBattleData } from "battler-types";

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

describe("isMonFaintedInState", () => {
  it("returns false when both battleState and playerData are null/undefined", () => {
    expect(isMonFaintedInState(null, 0, 0, null)).toBe(false);
    expect(isMonFaintedInState(undefined, 1, 0, undefined)).toBe(false);
  });

  it("checks playerData correctly for side 0 (ally/self)", () => {
    const playerData: PlayerBattleData = {
      name: "Ash",
      side: 0,
      mons: [
        {
          name: "Pikachu",
          species: "Pikachu",
          hp: 100,
          max_hp: 100,
          active: true,
          player_active_position: 0,
        },
        {
          name: "Charizard",
          species: "Charizard",
          hp: 0,
          max_hp: 100,
          active: true,
          player_active_position: 1,
        },
        {
          name: "Bulbasaur",
          species: "Bulbasaur",
          hp: 50,
          max_hp: 100,
          active: false,
          player_active_position: null,
        },
      ],
    } as unknown as PlayerBattleData;

    // Slot 0 has healthy active Pikachu
    expect(isMonFaintedInState(null, 0, 0, playerData)).toBe(false);
    // Slot 1 has fainted active Charizard (hp === 0)
    expect(isMonFaintedInState(null, 0, 1, playerData)).toBe(true);
    // Slot 2 has no active mon
    expect(isMonFaintedInState(null, 0, 2, playerData)).toBe(true);
  });

  it("checks battleState correctly for foes (side 1) and allies (side 0)", () => {
    const mockState = {
      field: {
        sides: [
          {
            id: 0,
            players: {
              "player-1": {
                mons: [
                  { fainted: false, physical_appearance: { name: "Pikachu" } },
                  { fainted: true, physical_appearance: { name: "Charizard" } },
                ],
              },
            },
            active: [
              { player: "player-1", mon_index: 0, battle_appearance_index: 0 },
              { player: "player-1", mon_index: 1, battle_appearance_index: 0 },
            ],
          },
          {
            id: 1,
            players: {
              "player-2": {
                mons: [
                  { fainted: false, physical_appearance: { name: "Gengar" } },
                  { fainted: true, physical_appearance: { name: "Dragonite" } },
                ],
              },
            },
            active: [
              { player: "player-2", mon_index: 0, battle_appearance_index: 0 },
              { player: "player-2", mon_index: 1, battle_appearance_index: 0 },
            ],
          },
        ],
      },
    } as unknown as BattleState;

    // Foe slot 0 (Gengar): alive
    expect(isMonFaintedInState(mockState, 1, 0)).toBe(false);
    // Foe slot 1 (Dragonite): fainted
    expect(isMonFaintedInState(mockState, 1, 1)).toBe(true);
    // Foe slot 2: empty
    expect(isMonFaintedInState(mockState, 1, 2)).toBe(true);

    // Ally slot 0 (Pikachu): alive
    expect(isMonFaintedInState(mockState, 0, 0)).toBe(false);
    // Ally slot 1 (Charizard): fainted
    expect(isMonFaintedInState(mockState, 0, 1)).toBe(true);
  });
});

describe("isMonActiveOnField", () => {
  it("returns fallbackActive when battleState is null or undefined", () => {
    expect(isMonActiveOnField(null, 0, "player-1", 0, true)).toBe(true);
    expect(isMonActiveOnField(null, 0, "player-1", 0, false)).toBe(false);
    expect(isMonActiveOnField(undefined, 0, "player-1", 0, undefined)).toBe(false);
  });

  it("returns false if player has left the battle", () => {
    const mockState = {
      field: {
        sides: [
          {
            id: 0,
            players: {
              "player-1": {
                id: "player-1",
                left_battle: true,
              },
            },
            active: [{ player: "player-1", mon_index: 0 }],
          },
        ],
      },
    } as unknown as BattleState;

    expect(isMonActiveOnField(mockState, 0, "player-1", 0)).toBe(false);
  });

  it("returns true if mon is in side.active, even when battle phase is finished", () => {
    const mockState = {
      phase: "finished",
      field: {
        sides: [
          {
            id: 0,
            players: {
              "player-1": {
                id: "player-1",
                left_battle: false,
              },
            },
            active: [
              { player: "player-1", mon_index: 0 },
              null,
            ],
          },
        ],
      },
    } as unknown as BattleState;

    expect(isMonActiveOnField(mockState, 0, "player-1", 0)).toBe(true);
    expect(isMonActiveOnField(mockState, 0, "player-1", 1)).toBe(false);
  });
});

describe("getAllyPlayerIds", () => {
  it("returns empty array when battleState or playerId is null or undefined", () => {
    expect(getAllyPlayerIds(null, "p1")).toEqual([]);
    expect(getAllyPlayerIds(undefined, "p1")).toEqual([]);
    expect(getAllyPlayerIds({} as unknown as BattleState, null)).toEqual([]);
    expect(getAllyPlayerIds({} as unknown as BattleState, undefined)).toEqual([]);
  });

  it("returns empty array when side has only one player", () => {
    const mockState = {
      field: {
        sides: [
          {
            players: {
              "player-1": { id: "player-1" },
            },
          },
          {
            players: {
              "player-2": { id: "player-2" },
            },
          },
        ],
      },
    } as unknown as BattleState;

    expect(getAllyPlayerIds(mockState, "player-1")).toEqual([]);
    expect(getAllyPlayerIds(mockState, "player-2")).toEqual([]);
  });

  it("returns ally player IDs on the same side and excludes current player and opponents", () => {
    const mockState = {
      field: {
        sides: [
          {
            players: {
              "player-1": { id: "player-1" },
              "player-2": { id: "player-2" },
              "player-3": { id: "player-3" },
            },
          },
          {
            players: {
              "player-4": { id: "player-4" },
              "player-5": { id: "player-5" },
            },
          },
        ],
      },
    } as unknown as BattleState;

    expect(getAllyPlayerIds(mockState, "player-1")).toEqual(["player-2", "player-3"]);
    expect(getAllyPlayerIds(mockState, "player-2")).toEqual(["player-1", "player-3"]);
    expect(getAllyPlayerIds(mockState, "player-4")).toEqual(["player-5"]);
  });

  it("returns empty array when player is not found on any side", () => {
    const mockState = {
      field: {
        sides: [
          {
            players: {
              "player-1": { id: "player-1" },
            },
          },
        ],
      },
    } as unknown as BattleState;

    expect(getAllyPlayerIds(mockState, "player-999")).toEqual([]);
  });
});
