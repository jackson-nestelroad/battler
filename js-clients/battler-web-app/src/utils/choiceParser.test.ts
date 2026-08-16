import { describe, expect, it } from "vitest";
import { parseChoiceError } from "./choiceParser";

describe("choiceErrorParser utility", () => {
  it("returns null for empty or null error string", () => {
    expect(parseChoiceError(null)).toEqual({ failedSlotIndex: null, errorMessage: "" });
    expect(parseChoiceError(undefined)).toEqual({ failedSlotIndex: null, errorMessage: "" });
    expect(parseChoiceError("")).toEqual({ failedSlotIndex: null, errorMessage: "" });
  });

  it("parses invalid choice 0 error string", () => {
    const res = parseChoiceError("invalid choice 0: cannot move: invalid target for Draco Meteor");
    expect(res.failedSlotIndex).toBe(0);
    expect(res.errorMessage).toBe("cannot move: invalid target for Draco Meteor");
  });

  it("parses invalid choice 1 error string", () => {
    const res = parseChoiceError("invalid choice 1: cannot switch: the mon in slot 3 can only switch in once");
    expect(res.failedSlotIndex).toBe(1);
    expect(res.errorMessage).toBe("cannot switch: the mon in slot 3 can only switch in once");
  });

  it("handles generic non-matching error strings", () => {
    const res = parseChoiceError("Server connection timeout");
    expect(res.failedSlotIndex).toBeNull();
    expect(res.errorMessage).toBe("Server connection timeout");
  });
});
