import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
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
});
