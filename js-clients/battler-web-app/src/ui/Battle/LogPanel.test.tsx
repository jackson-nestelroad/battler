import { LogCategory } from "battler-log-formatter";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { FormattedLogDisplayItem } from "../../utils/logFormatter";
import LogPanel from "./LogPanel";

describe("LogPanel", () => {
  it("renders Text, Players, and Engine tabs without JSON tab", () => {
    const html = renderToStaticMarkup(
      <LogPanel
        visibleLogs={[]}
        uiLogs={[]}
        engineLogs={[]}
        battleState={null}
      />,
    );

    expect(html).toContain("Text");
    expect(html).toContain("Players");
    expect(html).toContain("Engine");
    expect(html).not.toContain("JSON");
  });

  it("renders move names with DataTooltipTrigger in log messages", () => {
    const moveLog: FormattedLogDisplayItem = {
      kind: "message",
      category: LogCategory.Primary,
      message: {
        category: LogCategory.Primary,
        tokens: [
          { type: "variable", value: "MON" },
          { type: "text", value: " used " },
          { type: "variable", value: "MOVE" },
          { type: "text", value: "!" },
        ],
        context: {
          MON: "Metagross",
          MOVE: "Heart Stamp",
        },
      },
    };

    const html = renderToStaticMarkup(
      <LogPanel visibleLogs={[moveLog]} />,
    );

    expect(html).toContain("Heart Stamp");
    expect(html).toContain('role="button"');
  });

  it("renders split triggers for ability notices with mon and ability", () => {
    const abilityNotice: FormattedLogDisplayItem = {
      kind: "notice",
      notice: {
        type: "ability",
        name: "Intimidate",
        mon: "The opposing Gyarados's",
        monRef: { Active: { name: "Gyarados", player: "p2", side: 1, position: 0 } },
      },
    };

    const html = renderToStaticMarkup(
      <LogPanel visibleLogs={[abilityNotice]} />,
    );

    expect(html).toContain("[");
    expect(html).toContain("The opposing Gyarados");
    expect(html).toContain("Intimidate");
    expect(html).toContain("]");
    expect(html).toContain('role="button"');
  });

  it("renders split triggers for item notices with mon and item", () => {
    const itemNotice: FormattedLogDisplayItem = {
      kind: "notice",
      notice: {
        type: "item",
        name: "Leftovers",
        mon: "Snorlax's",
        monRef: { Active: { name: "Snorlax", player: "p1", side: 0, position: 0 } },
      },
    };

    const html = renderToStaticMarkup(
      <LogPanel visibleLogs={[itemNotice]} />,
    );

    expect(html).toContain("[");
    expect(html).toContain("Snorlax");
    expect(html).toContain("Leftovers");
    expect(html).toContain("]");
    expect(html).toContain('role="button"');
  });

  it("renders weather and terrain tokens with DataTooltipTrigger in log messages", () => {
    const weatherLog: FormattedLogDisplayItem = {
      kind: "message",
      category: LogCategory.Primary,
      message: {
        category: LogCategory.Primary,
        tokens: [
          { type: "text", value: "The sunlight turned harsh! (" },
          { type: "variable", value: "WEATHER" },
          { type: "text", value: ")" },
        ],
        context: {
          WEATHER: "Sunny Day",
        },
      },
    };

    const html = renderToStaticMarkup(
      <LogPanel visibleLogs={[weatherLog]} />,
    );

    expect(html).toContain("Sunny Day");
    expect(html).toContain('role="button"');
  });

  it("renders species tokens with DataTooltipTrigger in log messages", () => {
    const speciesLog: FormattedLogDisplayItem = {
      kind: "message",
      category: LogCategory.Primary,
      message: {
        category: LogCategory.Primary,
        tokens: [
          { type: "variable", value: "MON" },
          { type: "text", value: " transformed into " },
          { type: "variable", value: "SPECIES" },
          { type: "text", value: "!" },
        ],
        context: {
          MON: "Ditto",
          SPECIES: "Mew",
        },
      },
    };

    const html = renderToStaticMarkup(
      <LogPanel visibleLogs={[speciesLog]} />,
    );

    expect(html).toContain("Mew");
    expect(html).toContain('role="button"');
  });

  it("renders Mon hover trigger when context variable contains monRef", () => {
    const monLog: FormattedLogDisplayItem = {
      kind: "message",
      category: LogCategory.Primary,
      message: {
        category: LogCategory.Primary,
        tokens: [
          { type: "variable", value: "MON" },
          { type: "text", value: " used " },
          { type: "variable", value: "MOVE" },
          { type: "text", value: "!" },
        ],
        context: {
          MON: {
            text: "Gyarados",
            monRef: { Active: { side: 1, position: 0, name: "Gyarados", player: "p2" } },
          },
          MOVE: "Waterfall",
        },
      },
    };

    const html = renderToStaticMarkup(
      <LogPanel visibleLogs={[monLog]} />,
    );

    expect(html).toContain("Gyarados");
    expect(html).toContain("Waterfall");
  });
});
