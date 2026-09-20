import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import TypeChartStandaloneApp from "./TypeChartStandaloneApp";

describe("TypeChartStandaloneApp", () => {
  it("renders the standalone layout with header, title, back link, and grid", () => {
    const html = renderToStaticMarkup(<TypeChartStandaloneApp />);

    expect(html).toContain("Type Chart");
    expect(html).toContain("← Battler");
    expect(html).toContain("favicon.svg");
    expect(html).toContain("DEF →");
    expect(html).toContain("ATK ↓");
    expect(html).toContain("multSuper");
    expect(html).toContain("multResist");
    expect(html).toContain("multImmune");
  });
});
