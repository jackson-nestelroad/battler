import { describe, it, expect } from "vitest";
import { LogFormatter } from "../src/formatter.js";
import { LogCategory } from "../src/types.js";

describe("LogFormatter", () => {
  it("should format string log correctly", () => {
    const formatter = new LogFormatter();
    // Simulate UiLogEntry enum using any
    const entry: any = "TurnLimit";
    const result = formatter.format(entry);
    
    expect(result).not.toBeNull();
    expect(result?.category).toBe(LogCategory.Primary);
    expect(result?.tokens).toEqual([
      { type: "text", value: "The turn limit has been reached." }
    ]);
  });

  it("should format complex Move log", () => {
    const formatter = new LogFormatter({ localPlayerId: "p1" });
    const entry: any = {
      Move: {
        name: "Thunderbolt",
        mon: {
          Active: { side: 0, position: 0, name: "Pikachu", player: "p2" }
        }
      }
    };
    
    // Mock state
    const state: any = {
      field: {
        sides: [
          {
            active: [{ player: "p2", mon_index: 0 }],
            players: { p2: { id: "p2", mons: [{ physical_appearance: { name: "Pikachu" } }] } }
          }
        ]
      }
    };
    
    const result = formatter.format(entry, state);
    expect(result).not.toBeNull();
    expect(result?.category).toBe(LogCategory.Primary);
    
    // Because localPlayerId is p1, and the mon belongs to p2, it should format as a foe.
    // wait, we mocked the state to have player id 'p2', which is not 'p1'.
    // so it should use mon.foe -> "The opposing Pikachu"
    expect(result?.context.MON).toEqual({ text: "the opposing Pikachu", id: "p2-active-0", noAutoCapitalize: false });
    expect(result?.context.MOVE).toBe("Thunderbolt");
    
    expect(result?.tokens).toEqual([
      { type: "variable", value: "__CAPITALIZED_MON" },
      { type: "text", value: " used " },
      { type: "variable", value: "MOVE" },
      { type: "text", value: "!" }
    ]);
  });
});
