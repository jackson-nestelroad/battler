import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { BattleState } from "battler-state";
import type { MonMoveSlotData } from "battler-types";
import { setCachedTypeChartForTesting } from "../../hooks/useTypeChart";
import TargetSelector from "./TargetSelector";

const mockTypeChart = {
  types: {
    Water: {
      Fire: 2,
      Grass: 0.5,
      Water: 0.5,
    },
    Thunderbolt: {
      Water: 2,
    },
    StatusMove: {
      Fire: 2,
    },
  },
};

describe("TargetSelector", () => {
  it("renders non-choosable move target with Confirm button", () => {
    const html = renderToStaticMarkup(
      <TargetSelector
        selectedMoveTarget="AllAdjacent"
        dynamicTargets={[]}
        isLoading={false}
        onConfirmMove={() => {}}
      />,
    );

    expect(html).toContain("Confirm");
    expect(html).not.toContain("targetGrid");
  });

  it("does not show effectiveness badges for Status moves", () => {
    setCachedTypeChartForTesting(mockTypeChart);

    const statusMove: MonMoveSlotData = {
      id: "willowisp",
      name: "Will-O-Wisp",
      category: "Status",
      type: "Fire",
      pp: 15,
      max_pp: 15,
      target: "Normal",
      disabled: false,
    };

    const targets = [
      {
        value: 1,
        monName: "Charizard",
        label: "Charizard (Foe)",
        subText: "Foe",
        type: "foe" as const,
        position: 0,
      },
    ];

    const html = renderToStaticMarkup(
      <TargetSelector
        selectedMoveTarget="Normal"
        selectedMove={statusMove}
        dynamicTargets={targets}
        isLoading={false}
        onConfirmMove={() => {}}
      />,
    );

    expect(html).toContain("Charizard");
    expect(html).toContain("Foe");
    expect(html).not.toContain("effectivenessBadge");
  });

  it("shows effectiveness badge for Physical / Special damaging moves", () => {
    setCachedTypeChartForTesting(mockTypeChart);

    const waterMove: MonMoveSlotData = {
      id: "surf",
      name: "Surf",
      category: "Special",
      type: "Water",
      pp: 15,
      max_pp: 15,
      target: "Normal",
      disabled: false,
    };

    const mockBattleState = {
      field: {
        sides: [
          { active: [] },
          {
            active: [{ player: "p2", mon_index: 0, battle_appearance_index: 0 }],
            players: {
              p2: {
                mons: [
                  {
                    types: ["Fire"],
                    physical_appearance: { species: "Charizard" },
                  },
                ],
              },
            },
          },
        ],
      },
    } as unknown as BattleState;

    const targets = [
      {
        value: 1,
        monName: "Charizard",
        label: "Charizard (Foe)",
        subText: "Foe",
        type: "foe" as const,
        position: 0,
      },
    ];

    const html = renderToStaticMarkup(
      <TargetSelector
        selectedMoveTarget="Normal"
        selectedMove={waterMove}
        dynamicTargets={targets}
        isLoading={false}
        battleState={mockBattleState}
        playerData={{ side: 0 } as any}
        currentSlotIndex={0}
        onConfirmMove={() => {}}
      />,
    );

    expect(html).toContain("Charizard");
    expect(html).toContain("effectivenessBadge");
    expect(html).toContain("2×");
    expect(html).toContain("Super effective");
  });

  it("shows back button when onBack prop is supplied", () => {
    const html = renderToStaticMarkup(
      <TargetSelector
        selectedMoveTarget="Normal"
        dynamicTargets={[]}
        isLoading={false}
        onConfirmMove={() => {}}
        onBack={() => {}}
      />,
    );

    expect(html).toContain("← Back");
  });
});
