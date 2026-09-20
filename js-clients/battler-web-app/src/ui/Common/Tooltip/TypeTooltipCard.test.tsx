import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import * as typeChartHook from "../../../hooks/useTypeChart";
import TypeTooltipCard from "./TypeTooltipCard";

const MOCK_CHART = {
  types: {
    Fire: {
      Grass: 2,
      Bug: 2,
      Ice: 2,
      Steel: 2,
      Water: 0.5,
      Fire: 0.5,
      Rock: 0.5,
      Dragon: 0.5,
    },
    Water: {
      Fire: 2,
    },
    Ground: {
      Fire: 2,
    },
    Rock: {
      Fire: 2,
    },
    Normal: {
      Ghost: 0,
    },
    Ghost: {
      Normal: 0,
    },
  },
};

describe("TypeTooltipCard", () => {
  it("renders loading state", () => {
    vi.spyOn(typeChartHook, "useTypeChart").mockReturnValue({
      typeChart: null,
      loading: true,
      error: null,
    });

    const html = renderToStaticMarkup(<TypeTooltipCard type="Fire" />);
    expect(html).toContain("Loading...");
    expect(html).toContain("spinner");
  });

  it("renders error state", () => {
    vi.spyOn(typeChartHook, "useTypeChart").mockReturnValue({
      typeChart: null,
      loading: false,
      error: "Failed to fetch type chart",
    });

    const html = renderToStaticMarkup(<TypeTooltipCard type="Fire" />);
    expect(html).toContain("Failed to fetch type chart");
  });

  it("renders offensive and defensive effectiveness for a type", () => {
    vi.spyOn(typeChartHook, "useTypeChart").mockReturnValue({
      typeChart: MOCK_CHART,
      loading: false,
      error: null,
    });

    const html = renderToStaticMarkup(<TypeTooltipCard type="Fire" />);
    // Title, Effect button, subtitle, and sections
    expect(html).toContain("Fire");
    expect(html).toContain("Effect");
    expect(html).toContain("Type");
    expect(html).toContain("Offense");
    expect(html).toContain("Defense");

    // Offense sections
    expect(html).toContain("Super effective");
    expect(html).toContain("2×");
    expect(html).toContain("Grass");
    expect(html).toContain("Bug");
    expect(html).toContain("Not very effective");
    expect(html).toContain("½×");
    expect(html).toContain("Water");

    // Defense sections
    expect(html).toContain("Weaknesses");
    expect(html).toContain("2×");
    expect(html).toContain("Water");
    expect(html).toContain("Ground");
    expect(html).toContain("Rock");
  });

  it("renders immunities when present", () => {
    vi.spyOn(typeChartHook, "useTypeChart").mockReturnValue({
      typeChart: MOCK_CHART,
      loading: false,
      error: null,
    });

    const html = renderToStaticMarkup(<TypeTooltipCard type="Ghost" />);
    expect(html).toContain("Ghost");
    expect(html).toContain("No effect");
    expect(html).toContain("Immunities");
    expect(html).toContain("0×");
    expect(html).toContain("Normal");
  });

  it("handles empty / none list cleanly", () => {
    vi.spyOn(typeChartHook, "useTypeChart").mockReturnValue({
      typeChart: { types: {} },
      loading: false,
      error: null,
    });

    const html = renderToStaticMarkup(<TypeTooltipCard type="Normal" />);
    expect(html).toContain("None");
  });
});
