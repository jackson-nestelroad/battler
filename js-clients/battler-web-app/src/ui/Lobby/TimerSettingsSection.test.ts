import { describe, it, expect } from "vitest";
import { TIMER_PRESETS } from "./TimerSettingsSection";

describe("TimerSettingsSection", () => {
  describe("TIMER_PRESETS", () => {
    it("should have default warnings configured for blitz preset", () => {
      expect(TIMER_PRESETS.blitz.actionWarnings).toEqual([5n]);
      expect(TIMER_PRESETS.blitz.teamPreviewWarnings).toEqual([5n]);
    });

    it("should have default warnings configured for standard preset", () => {
      expect(TIMER_PRESETS.standard.actionWarnings).toEqual([15n, 5n]);
      expect(TIMER_PRESETS.standard.teamPreviewWarnings).toEqual([30n, 10n]);
      expect(TIMER_PRESETS.standard.playerWarnings).toEqual([300n, 180n, 60n, 30n, 10n]);
      expect(TIMER_PRESETS.standard.battleWarnings).toEqual([600n, 300n, 60n]);
    });
  });
});
