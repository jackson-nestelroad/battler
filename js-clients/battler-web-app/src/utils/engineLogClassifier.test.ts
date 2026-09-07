import { describe, expect, it } from "vitest";
import { classifyEngineLog } from "./engineLogClassifier";

describe("engineLogClassifier", () => {
  it("classifies landmark commands", () => {
    expect(classifyEngineLog("turn|turn:1")).toBe("landmark");
    expect(classifyEngineLog("battlestart")).toBe("landmark");
    expect(classifyEngineLog("win|side:0")).toBe("landmark");
    expect(classifyEngineLog("tie")).toBe("landmark");
  });

  it("classifies faint command specifically", () => {
    expect(classifyEngineLog("faint|mon:Turtwig,ash,1")).toBe("faint");
  });

  it("classifies core actions and transformations", () => {
    expect(
      classifyEngineLog(
        "move|mon:Baxcalibur,ash,2|name:Fire Spin|target:Stantler,ai-random-1,1"
      )
    ).toBe("action");
    expect(
      classifyEngineLog(
        "switch|player:ash|position:1|name:Turtwig|health:130/130|species:Turtwig|level:50|gender:F"
      )
    ).toBe("action");
    expect(classifyEngineLog("switchout|mon:Turtwig,ash,1")).toBe("action");
    expect(classifyEngineLog("drag|mon:Pikachu,p1,1")).toBe("action");
    expect(classifyEngineLog("dynamax|mon:Baxcalibur,ash,2")).toBe("action");
    expect(classifyEngineLog("gigantamax|mon:Pikachu,p1,1")).toBe("action");
    expect(classifyEngineLog("mega|mon:Charizard,p1,1")).toBe("action");
    expect(classifyEngineLog("formechange|mon:Castform,p1,1")).toBe("action");
  });

  it("classifies revert logs as standard events", () => {
    expect(classifyEngineLog("revertdynamax|mon:Baxcalibur,ash,2")).toBe("event");
    expect(classifyEngineLog("revertmega|mon:Charizard,p1,1")).toBe("event");
    expect(classifyEngineLog("revertgigantamax|mon:Pikachu,p1,1")).toBe("event");
  });

  it("classifies standard events", () => {
    expect(
      classifyEngineLog("damage|mon:Stantler,ai-random-1,1|health:88/100")
    ).toBe("event");
    expect(classifyEngineLog("heal|mon:Stantler,ai-random-1,1|health:100/100")).toBe(
      "event"
    );
    expect(
      classifyEngineLog(
        "boost|mon:Swampert,ai-random-1,2|stat:atk|by:2|from:ability:Defiant"
      )
    ).toBe("event");
    expect(
      classifyEngineLog("unboost|mon:Swampert,ai-random-1,2|stat:atk|by:2")
    ).toBe("event");
    expect(classifyEngineLog("status|mon:Pikachu,p1,1|status:brn")).toBe("event");
    expect(classifyEngineLog("curestatus|mon:Pikachu,p1,1")).toBe("event");
    expect(classifyEngineLog("weather|weather:Harsh Sunlight")).toBe("event");
    expect(classifyEngineLog("weather|weather:Harsh Sunlight|residual")).toBe(
      "event"
    );
    expect(classifyEngineLog("residual")).toBe("event");
    expect(
      classifyEngineLog("activate|mon:Stantler,ai-random-1,1|move:Fire Spin")
    ).toBe("event");
    expect(classifyEngineLog("resisted|mon:Swampert,ai-random-1,2")).toBe("event");
    expect(classifyEngineLog("crit|mon:Swampert,ai-random-1,2")).toBe("event");
    expect(classifyEngineLog("start|mon:Baxcalibur,ash,2|move:Telekinesis")).toBe(
      "event"
    );
  });

  it("classifies noise and metadata commands", () => {
    expect(classifyEngineLog("info|battletype:Doubles")).toBe("noise");
    expect(classifyEngineLog("info|environment:Normal|time:Day")).toBe("noise");
    expect(classifyEngineLog("side|id:0|name:ash")).toBe("noise");
    expect(
      classifyEngineLog("player|id:ash|name:ash|side:0|position:0")
    ).toBe("noise");
    expect(classifyEngineLog("teamsize|player:ash|size:4")).toBe("noise");
    expect(classifyEngineLog("time|value:1788786019701")).toBe("noise");
    expect(classifyEngineLog("teampreviewstart")).toBe("noise");
    expect(classifyEngineLog("teampreview")).toBe("noise");
    expect(classifyEngineLog("debug|event:test|error:none")).toBe("noise");
    expect(classifyEngineLog("-battlerservice:started")).toBe("noise");
    expect(classifyEngineLog("-battlerservice:request")).toBe("noise");
    expect(
      classifyEngineLog(
        "-battlerservice:timer|battle|remainingsecs:1200|deadline:1788787219"
      )
    ).toBe("noise");
    expect(
      classifyEngineLog(
        "-battlerservice:timer|player:ai-random-1|inactive|remainingsecs:419|deadline:1788786439"
      )
    ).toBe("noise");
  });

  it("classifies all timer logs as noise", () => {
    expect(
      classifyEngineLog(
        "-battlerservice:timer|action:ash|warning|remainingsecs:15|deadline:1788786064"
      )
    ).toBe("noise");
    expect(
      classifyEngineLog(
        "-battlerservice:timer|action:ash|done|remainingsecs:0|deadline:1788786064"
      )
    ).toBe("noise");
  });
});
