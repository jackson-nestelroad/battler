import { describe, expect, it } from "vitest";
import { getBattleStateLabel } from "./battleState";

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
