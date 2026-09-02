import type { MonMoveSlotData, Request } from "battler-types";
import { describe, expect, it } from "vitest";
import {
  canSlotShift,
  canSlotSwitch,
  getMonDisplayName,
  getMonForSlot,
  getMonTeamPosition,
  getRequestSlotCount,
  getSlotLabel,
} from "./monHelpers";

describe("monHelpers", () => {
  it("returns mon display name correctly", () => {
    expect(getMonDisplayName(null)).toBe("");
    expect(getMonDisplayName({ species: "Pikachu" })).toBe("Pikachu");
    expect(getMonDisplayName({ summary: { name: "Sparky" }, species: "Pikachu" })).toBe("Sparky");
  });

  it("returns mon team position with fallbacks", () => {
    expect(getMonTeamPosition(null, 2)).toBe(2);
    expect(getMonTeamPosition({ player_team_position: 1 }, 0)).toBe(1);
    expect(getMonTeamPosition({ team_position: 3 }, 0)).toBe(3);
  });

  it("formats slot label correctly", () => {
    expect(getSlotLabel(1, "Pikachu")).toBe("Slot 1: Pikachu");
    expect(getSlotLabel(2, null)).toBe("Slot 2");
    expect(getSlotLabel(3, "")).toBe("Slot 3");
  });

  it("calculates request slot count correctly", () => {
    expect(getRequestSlotCount(null)).toBe(0);
    const turnReq = {
      type: "turn",
      active: [{ team_position: 0 }, { team_position: 1 }],
    } as unknown as Request;
    expect(getRequestSlotCount(turnReq)).toBe(2);

    const switchReq = {
      type: "switch",
      needs_switch: [0, 1, 2],
    } as unknown as Request;
    expect(getRequestSlotCount(switchReq)).toBe(3);
  });

  it("resolves mon for slot index correctly", () => {
    const playerData = {
      mons: [
        { species: "Charizard", player_team_position: 0, hp: 0 },
        { species: "Pikachu", player_team_position: 1, hp: 100, player_active_position: 0 },
        { species: "Zarude", player_team_position: 5, hp: 0 },
      ],
    };

    const turnReq = {
      type: "turn",
      active: [{ team_position: 1 }],
    } as unknown as Request;

    expect(getMonForSlot(playerData, turnReq, 0)?.species).toBe("Pikachu");

    const uturnSwitchReq = {
      type: "switch",
      needs_switch: [0],
    } as unknown as Request;

    expect(getMonForSlot(playerData, uturnSwitchReq, 0)?.species).toBe("Pikachu");

    const faintSwitchReq = {
      type: "switch",
      needs_switch: [1],
    } as unknown as Request;

    expect(getMonForSlot(playerData, faintSwitchReq, 0)).toBeNull();
  });

  it("determines canSlotShift dynamically for any active slot count", () => {
    // Singles & Doubles (<= 2)
    expect(canSlotShift(0, 1)).toBe(false);
    expect(canSlotShift(0, 2)).toBe(false);
    expect(canSlotShift(1, 2)).toBe(false);

    // Triples (3): Center is slot index 1
    expect(canSlotShift(0, 3)).toBe(true);
    expect(canSlotShift(1, 3)).toBe(false); // center cannot shift
    expect(canSlotShift(2, 3)).toBe(true);

    // Quintuples (5): Center is slot index 2
    expect(canSlotShift(0, 5)).toBe(true);
    expect(canSlotShift(1, 5)).toBe(true);
    expect(canSlotShift(2, 5)).toBe(false); // center cannot shift
    expect(canSlotShift(3, 5)).toBe(true);
    expect(canSlotShift(4, 5)).toBe(true);

    // Trapped mon cannot shift
    expect(canSlotShift(0, 3, true)).toBe(false);
  });

  it("determines canSlotSwitch correctly including trapped mon attempts", () => {
    expect(canSlotSwitch(null, 0, null)).toBe(false);

    // Turn request without trapped
    const normalTurnReq = {
      type: "turn",
      active: [{ team_position: 0, trapped: false }],
    } as unknown as Request;
    expect(canSlotSwitch(normalTurnReq, 0, null)).toBe(true);
    // When a move is selected, cannot switch
    expect(
      canSlotSwitch(normalTurnReq, 0, {
        id: "thunderbolt",
        name: "Thunderbolt",
        pp: 15,
        max_pp: 15,
        disabled: false,
      } as MonMoveSlotData),
    ).toBe(false);

    // Turn request WITH trapped mon: should return true to allow user to try switching
    const trappedTurnReq = {
      type: "turn",
      active: [{ team_position: 0, trapped: true }],
    } as unknown as Request;
    expect(canSlotSwitch(trappedTurnReq, 0, null)).toBe(true);

    // Switch request
    const switchReq = {
      type: "switch",
      needs_switch: [0],
    } as unknown as Request;
    expect(canSlotSwitch(switchReq, 0, null)).toBe(true);
    expect(canSlotSwitch(switchReq, 1, null)).toBe(false);
  });
});
