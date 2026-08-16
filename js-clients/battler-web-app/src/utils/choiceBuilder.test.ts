import { describe, expect, it } from "vitest";
import { ChoiceBuilder } from "./choiceBuilder";

describe("ChoiceBuilder utility", () => {
  it("builds simple move choice", () => {
    expect(ChoiceBuilder.move(0)).toBe("move 0");
  });

  it("builds move choice with target and modifiers", () => {
    expect(ChoiceBuilder.move(1, 2, { mega: true })).toBe("move 1, 2, mega");
    expect(ChoiceBuilder.move(0, -1, { tera: true, dyna: true })).toBe("move 0, -1, dyna, tera");
  });

  it("builds switch choice", () => {
    expect(ChoiceBuilder.switch(3)).toBe("switch 3");
  });

  it("builds shift choice", () => {
    expect(ChoiceBuilder.shift()).toBe("shift");
  });

  it("builds pass choice", () => {
    expect(ChoiceBuilder.pass()).toBe("pass");
  });

  it("builds team choice", () => {
    expect(ChoiceBuilder.team([1, 2, 3])).toBe("team 1, 2, 3");
  });
});
