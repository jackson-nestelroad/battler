import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import DataTooltipTrigger from "./DataTooltipTrigger";

describe("DataTooltipTrigger", () => {
  it("renders children cleanly", () => {
    const html = renderToStaticMarkup(
      <DataTooltipTrigger resourceType="move" name="Thunderbolt">
        <span>Thunderbolt</span>
      </DataTooltipTrigger>,
    );
    expect(html).toContain("Thunderbolt");
  });

  it("renders as simple element if name is empty", () => {
    const html = renderToStaticMarkup(
      <DataTooltipTrigger resourceType="move" name="">
        <span>Empty</span>
      </DataTooltipTrigger>,
    );
    expect(html).toContain("Empty");
  });

  it("supports as button", () => {
    const html = renderToStaticMarkup(
      <DataTooltipTrigger resourceType="ability" name="Levitate" as="button">
        Levitate
      </DataTooltipTrigger>,
    );
    expect(html).toContain("<button");
    expect(html).toContain("Levitate");
  });

  it("renders nested triggers cleanly", () => {
    const html = renderToStaticMarkup(
      <DataTooltipTrigger resourceType="species" name="Pikachu">
        <div>
          <span>Pikachu</span>
          <DataTooltipTrigger resourceType="ability" name="Static">
            <span>Static</span>
          </DataTooltipTrigger>
        </div>
      </DataTooltipTrigger>,
    );
    expect(html).toContain("Pikachu");
    expect(html).toContain("Static");
  });
});
