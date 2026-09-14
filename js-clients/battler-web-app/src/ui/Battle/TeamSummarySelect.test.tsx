import type { PlayerBattleData, Request } from "battler-types";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import TeamSummary from "./TeamSummary";

describe("TeamSummary select request handling", () => {
  const mockPlayerData = {
    side: 0,
    player_index: 0,
    position: 0,
    mons: [
      {
        species: "Pawmot",
        summary: { name: "Pawmot", level: 100 },
        player_active_position: 0,
        player_team_position: 0,
        hp: 250,
        max_hp: 250,
        status: null,
        active: true,
      },
      {
        species: "Quaxly",
        summary: { name: "Quaxly", level: 50 },
        player_team_position: 1,
        hp: 0,
        max_hp: 115,
        status: "fnt",
        active: false,
      },
      {
        species: "Sprigatito",
        summary: { name: "Sprigatito", level: 50 },
        player_team_position: 2,
        hp: 100,
        max_hp: 100,
        status: null,
        active: false,
      },
    ],
  } as unknown as PlayerBattleData;

  const mockSelectRequest: Request = {
    type: "select",
    positions: [{ position: 0, reason: "Revive" }],
  };

  it("renders Reviving badge on the active mon and makes only fainted benched mons clickable", () => {
    const html = renderToStaticMarkup(
      <TeamSummary
        playerData={mockPlayerData}
        request={mockSelectRequest}
        currentSlotIndex={0}
        selectedMove={null}
        isMeReady={false}
        playbackPending={false}
        isLoading={false}
        onSwitch={() => {}}
        onSelect={() => {}}
        activeMonTeamPosition={0}
        actingBadgeText="Reviving"
      />,
    );

    // Pawmot has the Reviving badge
    expect(html).toContain("Reviving");
    expect(html).toContain("Pawmot");

    // Quaxly is fainted (hp: 0) and benched -> must have clickableSummaryCard
    expect(html).toContain("Quaxly");
    // Sprigatito is healthy (hp: 100) and benched -> must NOT be clickable in Revive mode
    expect(html).toContain("Sprigatito");

    expect(html).toMatch(/Quaxly[\s\S]*?FNT[\s\S]*?0\/115/);
  });

  it("triggers onSelect callback when clicking a fainted Mon card", () => {
    let selectedPos = -1;
    let selectedSlots = -1;

    const element = TeamSummary({
      playerData: mockPlayerData,
      request: mockSelectRequest,
      currentSlotIndex: 0,
      selectedMove: null,
      isMeReady: false,
      playbackPending: false,
      isLoading: false,
      onSwitch: () => {},
      onSelect: (pos, slots) => {
        selectedPos = pos;
        selectedSlots = slots;
      },
      activeMonTeamPosition: 0,
      actingBadgeText: "Reviving",
    });

    // The grid children are MonCards
    const grid = element?.props.children[1];
    const monCards = grid.props.children;

    // Pawmot (active): not clickable
    expect(monCards[0].props.isClickable).toBe(false);
    expect(monCards[0].props.onClick).toBeUndefined();

    // Quaxly (fainted, position 1): clickable
    expect(monCards[1].props.isClickable).toBe(true);
    expect(monCards[1].props.onClick).toBeDefined();

    // Sprigatito (healthy, position 2): not clickable in Revive select
    expect(monCards[2].props.isClickable).toBe(false);
    expect(monCards[2].props.onClick).toBeUndefined();

    // Click Quaxly
    monCards[1].props.onClick();
    expect(selectedPos).toBe(1);
    expect(selectedSlots).toBe(1);
  });
});
