import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import EngineLogViewer from "./EngineLogViewer";

describe("EngineLogViewer", () => {
  const sampleLogs = [
    "-battlerservice:started",
    "info|battletype:Doubles",
    "turn|turn:1",
    "-battlerservice:timer|battle|remainingsecs:1200|deadline:1788787219",
    "move|mon:Baxcalibur,ash,2|name:Fire Spin|target:Stantler,ai-random-1,1",
    "damage|mon:Stantler,ai-random-1,1|health:88/100",
    "faint|mon:Turtwig,ash,1",
  ];

  it("renders all engine logs with proper styling classes", () => {
    const html = renderToStaticMarkup(<EngineLogViewer engineLogs={sampleLogs} />);

    expect(html).toContain("turn|turn:1");
    expect(html).toContain("faint|mon:Turtwig,ash,1");
    expect(html).toContain("landmark");
    expect(html).toContain("faint");
    expect(html).toContain("action");
    expect(html).toContain("event");
    expect(html).toContain("noise");
    expect(html).toContain("Hide timers");
  });

  it("renders empty state correctly", () => {
    const html = renderToStaticMarkup(<EngineLogViewer engineLogs={[]} />);
    expect(html).toContain("None");
  });

  it("renders without controls when showControls is false", () => {
    const html = renderToStaticMarkup(
      <EngineLogViewer engineLogs={sampleLogs} showControls={false} />
    );
    expect(html).not.toContain("Hide timers");
    expect(html).toContain("turn|turn:1");
  });
});
