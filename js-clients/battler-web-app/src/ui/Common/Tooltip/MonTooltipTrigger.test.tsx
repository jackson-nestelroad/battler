import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import MonTooltipTrigger from "./MonTooltipTrigger";
import type { MonBattleData } from "battler-types";

function createMockMonBattleData(overrides: Partial<MonBattleData> = {}): MonBattleData {
  return {
    species: "Pikachu",
    summary: { name: "Pikachu", level: 50 },
    hp: 100,
    max_hp: 100,
    status: null,
    moves: [],
    ...overrides,
  } as unknown as MonBattleData;
}

describe("MonTooltipTrigger", () => {
  it("renders plain children when no mon data is provided", () => {
    const html = renderToStaticMarkup(
      <MonTooltipTrigger>
        <span className="trigger-child">No Data</span>
      </MonTooltipTrigger>,
    );
    expect(html).toContain("trigger-child");
    expect(html).toContain("No Data");
  });

  it("renders trigger element with mon battle data", () => {
    const mon = createMockMonBattleData();
    const html = renderToStaticMarkup(
      <MonTooltipTrigger mon={mon} as="div" className="mon-card-trigger">
        <span>Hover Me</span>
      </MonTooltipTrigger>,
    );
    expect(html).toContain("mon-card-trigger");
    expect(html).toContain("Hover Me");
  });

  it("supports as span and as div", () => {
    const mon = createMockMonBattleData();
    const spanHtml = renderToStaticMarkup(
      <MonTooltipTrigger mon={mon} as="span">
        <span>Span Trigger</span>
      </MonTooltipTrigger>,
    );
    expect(spanHtml).toContain("<span");
    expect(spanHtml).toContain("Span Trigger");

    const divHtml = renderToStaticMarkup(
      <MonTooltipTrigger mon={mon} as="div">
        <span>Div Trigger</span>
      </MonTooltipTrigger>,
    );
    expect(divHtml).toContain("<div");
    expect(divHtml).toContain("Div Trigger");
  });
});
