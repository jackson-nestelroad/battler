import { describe, expect, it } from "vitest";
import type { Battle, BattlePreview } from "battler-service-client";
import type { BattleState, UiLogEntry } from "battler-state";
import type { ProposedBattleWithDetails } from "../store/proposalsSlice";
import {
  formatDeletionReason,
  getBattleSessionTitle,
  getBattleTitle,
  getRuleBadgeClass,
  parseTimerLog,
} from "./battle";

describe("getBattleTitle", () => {
  it("uses battleState side names when available", () => {
    const battleState = {
      field: {
        sides: [
          { name: "Ash" },
          { name: "Gary" },
        ],
      },
    } as unknown as BattleState;

    expect(getBattleTitle(battleState)).toBe("Ash vs Gary");
  });

  it("uses serviceBattle side names when battleState is not available", () => {
    const serviceBattle = {
      sides: [
        { name: "Red", players: [] },
        { name: "Blue", players: [] },
      ],
    } as unknown as Battle;

    expect(getBattleTitle(null, serviceBattle)).toBe("Red vs Blue");
  });

  it("uses proposal side names when neither battleState nor serviceBattle is available", () => {
    const proposal = {
      sides: [
        { name: "Alice", players: [] },
        { name: "Bob", players: [] },
      ],
    } as unknown as ProposedBattleWithDetails;

    expect(getBattleTitle(null, null, proposal)).toBe("Alice vs Bob");
  });

  it("uses preview side names when available during session restoration", () => {
    const preview: BattlePreview = {
      uuid: "12345678-1234-1234-1234-123456789abc",
      sides: [
        {
          name: "Brock",
          players: [{ id: "p1", name: "Player 1" }],
        },
        {
          name: "Misty",
          players: [{ id: "p2", name: "Player 2" }],
        },
      ],
      battle_type: "Singles",
      state: "active",
      turn: 1n,
    };

    expect(getBattleTitle(null, null, null, false, preview)).toBe("Brock vs Misty");
  });

  it("falls back to preview player names when preview side names are missing", () => {
    const preview = {
      uuid: "12345678-1234-1234-1234-123456789abc",
      sides: [
        {
          name: "",
          players: [{ id: "p1", name: "Player 1" }],
        },
        {
          name: "",
          players: [{ id: "p2", name: "Player 2" }],
        },
      ],
      battle_type: "Singles",
      state: "active",
      turn: 1n,
    } as unknown as BattlePreview;

    expect(getBattleTitle(null, null, null, false, preview)).toBe("Player 1 vs Player 2");
  });

  it("returns Deleted Battle if isDeleted is true and side names are absent", () => {
    expect(getBattleTitle(null, null, null, true)).toBe("Deleted Battle");
  });

  it("falls back to Side 1 vs Side 2 when no data is provided", () => {
    expect(getBattleTitle()).toBe("Side 1 vs Side 2");
  });
});

describe("getBattleSessionTitle", () => {
  it("resolves title from session object", () => {
    const session = {
      battleState: {
        field: {
          sides: [{ name: "Red" }, { name: "Blue" }],
        },
      } as any,
    };
    expect(getBattleSessionTitle(session)).toBe("Red vs Blue");
  });

  it("resolves title from preview object", () => {
    const session = {
      preview: {
        sides: [
          { name: "Brock", players: [] },
          { name: "Misty", players: [] },
        ],
      } as any,
    };
    expect(getBattleSessionTitle(session)).toBe("Brock vs Misty");
  });

  it("respects isDeletedOverride", () => {
    expect(getBattleSessionTitle(null, null, true)).toBe("Deleted Battle");
  });
});

describe("formatDeletionReason", () => {
  it("formats null or undefined as Declined", () => {
    expect(formatDeletionReason(null)).toBe("Declined");
    expect(formatDeletionReason(undefined)).toBe("Declined");
  });

  it("formats deleted as Deleted", () => {
    expect(formatDeletionReason("deleted")).toBe("Deleted");
  });

  it("capitalizes custom reason", () => {
    expect(formatDeletionReason("timeout")).toBe("Timeout");
  });
});

describe("getRuleBadgeClass", () => {
  it("returns badge-danger for negative rules", () => {
    expect(getRuleBadgeClass("-Dynamax")).toBe("badge-danger");
  });

  it("returns badge-success for positive rules", () => {
    expect(getRuleBadgeClass("+Uber")).toBe("badge-success");
  });

  it("returns badge-warning for negated rules", () => {
    expect(getRuleBadgeClass("! Team Preview")).toBe("badge-warning");
  });

  it("returns badge-secondary for rules containing =", () => {
    expect(getRuleBadgeClass("minlevel=50")).toBe("badge-secondary");
  });

  it("returns badge-primary for other rules", () => {
    expect(getRuleBadgeClass("Standard")).toBe("badge-primary");
  });
});

describe("parseTimerLog", () => {
  it("parses battle timer start log", () => {
    const parsed = parseTimerLog({
      title: "timer",
      values: {
        battle: "",
        remainingsecs: "100",
        total: "300",
        deadline: "1700000000",
      },
    } as unknown as UiLogEntry);
    expect(parsed).toEqual({
      type: "battle",
      remainingSecs: 100,
      deadlineSecs: 1700000000,
      isWarning: false,
      isDone: false,
      isInactive: false,
      isClear: false,
    });
  });

  it("parses clear action timer log", () => {
    const parsed = parseTimerLog({
      title: "timer",
      values: {
        action: "player-1",
        remainingsecs: "0",
        clear: "true",
      },
    } as unknown as UiLogEntry);
    expect(parsed).toEqual({
      type: "action",
      playerId: "player-1",
      remainingSecs: 0,
      deadlineSecs: 0,
      isWarning: false,
      isDone: true,
      isInactive: false,
      isClear: true,
    });
  });
});
