import type { BattleState } from "battler-state";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import BattleConditionPopover from "./BattleConditionPopover";
import BattleConditionsBar from "./BattleConditionsBar";

function createMockBattleState(overrides: Partial<BattleState> = {}): BattleState {
  return {
    phase: "battle",
    turn: 3,
    winning_side: null,
    last_log_index: 10,
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
          name: "Rival",
          id: 1,
          players: {
            p2: {
              name: "Rival",
              id: "p2",
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
      weather: "Rain",
      conditions: {
        "Electric Terrain": { since_turn: 1, data: {} },
        "Trick Room": { since_turn: 2, data: {} },
      },
      rules: [],
      max_side_length: 6,
    },
    ui_log: [],
    ...overrides,
  };
}

describe("BattleConditions", () => {
  describe("BattleConditionPopover", () => {
    it("renders Field tab with weather, terrain, and other conditions", () => {
      const state = createMockBattleState();
      const html = renderToStaticMarkup(
        <BattleConditionPopover
          battleState={state}
          playerId="p1"
          activeTab="field"
          onTabChange={() => {}}
        />,
      );

      expect(html).toContain("Field Conditions");
      expect(html).toContain("Weather:");
      expect(html).toContain("Rain");
      expect(html).toContain("Terrain:");
      expect(html).toContain("Electric Terrain");
      expect(html).toContain("Effects:");
      expect(html).toContain("Trick Room");
      // Check NO emojis in rendered markup
      expect(html).not.toMatch(/[\u{1F300}-\u{1F9FF}]/u);
      // Check NO fake "PINNED" badge
      expect(html).not.toContain("PINNED");
    });

    it("renders Your Side tab with side conditions and slot conditions", () => {
      const state = createMockBattleState();
      const html = renderToStaticMarkup(
        <BattleConditionPopover
          battleState={state}
          playerId="p1"
          activeTab="player"
          onTabChange={() => {}}
        />,
      );

      expect(html).toContain("Your Side Conditions");
      expect(html).toContain("Stealth Rock");
      expect(html).toContain("Reflect");
      expect(html).toContain("Slot 1:");
      expect(html).toContain("Wish");
      // Check NO emojis in rendered markup
      expect(html).not.toMatch(/[\u{1F300}-\u{1F9FF}]/u);
    });

    it("renders Foe Side tab with layers information", () => {
      const state = createMockBattleState();
      const html = renderToStaticMarkup(
        <BattleConditionPopover
          battleState={state}
          playerId="p1"
          activeTab="foe"
          onTabChange={() => {}}
        />,
      );

      expect(html).toContain("Foe Side Conditions");
      expect(html).toContain("Spikes (2 layers)");
      expect(html).toContain("None"); // slot conditions is empty
      // Check NO emojis in rendered markup
      expect(html).not.toMatch(/[\u{1F300}-\u{1F9FF}]/u);
    });

    it("renders clean None state when no conditions are active", () => {
      const emptyState: BattleState = {
        phase: "battle",
        turn: 1,
        winning_side: null,
        last_log_index: 0,
        battle_type: "Singles",
        field: {
          sides: [
            {
              name: "Side 1",
              id: 0,
              players: {},
              conditions: {},
              slot_conditions: [],
              active: [],
            },
            {
              name: "Side 2",
              id: 1,
              players: {},
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
      };

      const fieldHtml = renderToStaticMarkup(
        <BattleConditionPopover
          battleState={emptyState}
          activeTab="field"
          onTabChange={() => {}}
        />,
      );
      expect(fieldHtml).toContain("Clear");
      expect(fieldHtml).toContain("None");

      const sideHtml = renderToStaticMarkup(
        <BattleConditionPopover
          battleState={emptyState}
          activeTab="player"
          onTabChange={() => {}}
        />,
      );
      expect(sideHtml).toContain("None");
    });
  });

  describe("BattleConditionsBar", () => {
    it("renders 3 compact chips for active player", () => {
      const state = createMockBattleState();
      const html = renderToStaticMarkup(<BattleConditionsBar battleState={state} playerId="p1" />);

      expect(html).toContain("Field:");
      expect(html).toContain("Rain / Electric Terrain (+1)");

      expect(html).toContain("Your Side");
      expect(html).toContain("3"); // 3 active conditions: Rocks, Reflect, Wish

      expect(html).toContain("Foe Side");
      expect(html).toContain("1"); // 1 active condition: Spikes

      // Check NO emojis
      expect(html).not.toMatch(/[\u{1F300}-\u{1F9FF}]/u);
    });

    it("renders fallback labels for spectator or replay mode", () => {
      const state = createMockBattleState();
      const html = renderToStaticMarkup(
        <BattleConditionsBar battleState={state} playerId="spectator" />,
      );

      expect(html).toContain("Field:");
      expect(html).toContain("Player 1");
      expect(html).toContain("Rival");
    });
  });
});
