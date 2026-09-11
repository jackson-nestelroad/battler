import type { BattleState } from "battler-state";
import { describe, expect, it } from "vitest";
import {
  extractAllBattleConditions,
  extractFieldConditions,
  extractSideConditions,
  formatCondition,
  resolveSideLabels,
} from "./conditionData";

function createMockBattleState(overrides: Partial<BattleState> = {}): BattleState {
  return {
    phase: "battle",
    turn: 1,
    winning_side: null,
    last_log_index: 0,
    battle_type: "Singles",
    field: {
      sides: [
        {
          name: "Player 1",
          id: 0,
          players: {
            p1: {
              name: "Player 1",
              id: "p1",
              position: 0,
              team_size: 6,
              mons: [],
              left_battle: false,
              wild: false,
            },
          },
          conditions: {},
          slot_conditions: [],
          active: [],
        },
        {
          name: "Player 2",
          id: 1,
          players: {
            p2: {
              name: "Player 2",
              id: "p2",
              position: 1,
              team_size: 6,
              mons: [],
              left_battle: false,
              wild: false,
            },
          },
          conditions: {},
          slot_conditions: [],
          active: [],
        },
      ],
      environment: null,
      time: null,
      weather: null,
      conditions: {},
      rules: [],
      max_side_length: 6,
    },
    ui_log: [],
    ...overrides,
  };
}

describe("conditionData", () => {
  it("formats condition item", () => {
    const spikesItem = formatCondition("Spikes", {
      since_turn: 2,
      data: { layers: "2" },
    });
    expect(spikesItem.name).toBe("Spikes");
    expect(spikesItem.displayText).toBe("Spikes");

    const simpleItem = formatCondition("Stealth Rock", {
      since_turn: 1,
      data: {},
    });
    expect(simpleItem.name).toBe("Stealth Rock");
    expect(simpleItem.displayText).toBe("Stealth Rock");
  });

  it("extracts field conditions with empty state and weather/terrain summary", () => {
    const emptyField = extractFieldConditions(null);
    expect(emptyField.allCount).toBe(0);
    expect(emptyField.summaryText).toBe("Clear");
    expect(emptyField.weather).toBeNull();
    expect(emptyField.terrain).toBeNull();

    const mockState = createMockBattleState({
      turn: 3,
      last_log_index: 10,
      field: {
        sides: [],
        environment: null,
        time: null,
        weather: "Rain",
        conditions: {
          "Electric Terrain": { since_turn: 1, data: {} },
          "Trick Room": { since_turn: 2, data: {} },
        },
        rules: [],
        max_side_length: 6,
      },
    });

    const field = extractFieldConditions(mockState);
    expect(field.weather).toBe("Rain");
    expect(field.terrain).toBe("Electric Terrain");
    expect(field.otherConditions).toHaveLength(1);
    expect(field.otherConditions[0].name).toBe("Trick Room");
    expect(field.allCount).toBe(3);
    expect(field.summaryText).toBe("Rain / Electric Terrain (+1)");

    // Only terrain active + multiple effects (Gravity, Trick Room)
    const terrainOnlyState: BattleState = {
      ...mockState,
      field: {
        ...mockState.field!,
        weather: null,
        conditions: {
          "Misty Terrain": { since_turn: 1, data: {} },
          Gravity: { since_turn: 1, data: {} },
          "Trick Room": { since_turn: 2, data: {} },
        },
      },
    };
    const terrainField = extractFieldConditions(terrainOnlyState);
    expect(terrainField.summaryText).toBe("Misty Terrain (+2)");
    expect(terrainField.summaryText).not.toContain("Gravity");
    expect(terrainField.summaryText).not.toContain("Trick Room");
  });

  it("extracts side and slot conditions with clean counts without favoring one condition", () => {
    const mockState = createMockBattleState({
      turn: 3,
      last_log_index: 10,
      field: {
        sides: [
          {
            name: "Player 1",
            id: 0,
            players: {
              "player-1": {
                name: "Player 1",
                id: "player-1",
                position: 0,
                team_size: 6,
                mons: [],
                left_battle: false,
                wild: false,
              },
            },
            conditions: {
              "Stealth Rock": { since_turn: 1, data: {} },
              Reflect: { since_turn: 2, data: {} },
            },
            slot_conditions: [
              {
                Wish: { since_turn: 3, data: {} },
              },
            ],
            active: [],
          },
          {
            name: "Opponent",
            id: 1,
            players: {
              "player-2": {
                name: "Opponent",
                id: "player-2",
                position: 1,
                team_size: 6,
                mons: [],
                left_battle: false,
                wild: false,
              },
            },
            conditions: {
              Spikes: { since_turn: 1, data: { layers: "2" } },
            },
            slot_conditions: [],
            active: [],
          },
        ],
        environment: null,
        time: null,
        weather: null,
        conditions: {},
        rules: [],
        max_side_length: 6,
      },
    });

    const side0 = extractSideConditions(mockState, 0, "Your Side");
    expect(side0.conditions).toHaveLength(2);
    expect(side0.slotConditions).toHaveLength(1);
    expect(side0.slotConditions[0].name).toBe("Wish");
    expect(side0.slotConditions[0].slotLabel).toBe("Slot 1");
    expect(side0.allCount).toBe(3);

    const side1 = extractSideConditions(mockState, 1, "Foe Side");
    expect(side1.conditions).toHaveLength(1);
    expect(side1.conditions[0].displayText).toBe("Spikes");
    expect(side1.allCount).toBe(1);
  });

  it("resolves side labels for active player vs spectator/replay", () => {
    const mockState = createMockBattleState();

    const activePlayer = resolveSideLabels(mockState, "p1");
    expect(activePlayer.playerSideLabel).toBe("Your Side");
    expect(activePlayer.foeSideLabel).toBe("Foe Side");
    expect(activePlayer.playerSideIndex).toBe(0);
    expect(activePlayer.foeSideIndex).toBe(1);
    expect(activePlayer.isSpectatorOrReplay).toBe(false);

    const spectator = resolveSideLabels(mockState, "spectator-id");
    expect(spectator.playerSideLabel).toBe("Side 1");
    expect(spectator.foeSideLabel).toBe("Side 2");
    expect(spectator.playerSideSubtitle).toBe("Player 1");
    expect(spectator.foeSideSubtitle).toBe("Player 2");
    expect(spectator.isSpectatorOrReplay).toBe(true);
  });

  it("extracts all battle conditions into a unified view model", () => {
    const baseState = createMockBattleState();
    const mockState: BattleState = {
      ...baseState,
      turn: 2,
      last_log_index: 4,
      field: {
        ...baseState.field!,
        weather: "Sun",
        sides: [
          {
            ...baseState.field!.sides[0],
            conditions: {
              "Stealth Rock": { since_turn: 1, data: {} },
            },
          },
          baseState.field!.sides[1],
        ],
      },
    };

    const allConditions = extractAllBattleConditions(mockState, "p1");
    expect(allConditions.playerSideLabel).toBe("Your Side");
    expect(allConditions.foeSideLabel).toBe("Foe Side");
    expect(allConditions.fieldData.weather).toBe("Sun");
    expect(allConditions.fieldData.summaryText).toBe("Sun");
    expect(allConditions.playerData.allCount).toBe(1);
    expect(allConditions.playerData.conditions[0].name).toBe("Stealth Rock");
    expect(allConditions.foeData.allCount).toBe(0);
  });
});
