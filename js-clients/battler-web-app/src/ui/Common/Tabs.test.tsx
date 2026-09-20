import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import Tabs from "./Tabs";

describe("Tabs", () => {
  it("renders tab options with active class and optional title attributes", () => {
    const options = [
      { value: "standard", label: "Standard", title: "Standard battle mode" },
      { value: "chaos", label: "Chaos", title: "Chaos battle mode" },
    ];

    const html = renderToStaticMarkup(
      <Tabs options={options} active="standard" onChange={vi.fn()} />,
    );

    expect(html).toContain('class="tabs-row"');
    expect(html).toContain('class="tab-btn active"');
    expect(html).toContain('title="Standard battle mode"');
    expect(html).toContain("Standard");
    expect(html).toContain('class="tab-btn "');
    expect(html).toContain('title="Chaos battle mode"');
    expect(html).toContain("Chaos");
  });
});
