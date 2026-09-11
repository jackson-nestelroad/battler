import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import TypeChartGrid from "./TypeChartGrid";
import * as typeChartHook from "../../hooks/useTypeChart";

const MOCK_CHART = {
  types: {
    Grass: {
      Water: 2,
      Ground: 2,
    },
    Water: {
      Fire: 2,
    },
  },
};

describe("TypeChartGrid", () => {
  it("renders loading state", () => {
    vi.spyOn(typeChartHook, "useTypeChart").mockReturnValue({
      typeChart: null,
      loading: true,
      error: null,
    });

    const html = renderToStaticMarkup(<TypeChartGrid />);
    expect(html).toContain("Loading...");
    expect(html).toContain("spinner");
  });

  it("renders error state", () => {
    vi.spyOn(typeChartHook, "useTypeChart").mockReturnValue({
      typeChart: null,
      loading: false,
      error: "Network failure",
    });

    const html = renderToStaticMarkup(<TypeChartGrid />);
    expect(html).toContain("Network failure");
    expect(html).toContain("alert-danger");
  });

  it("renders full 18x18 matrix by default", () => {
    vi.spyOn(typeChartHook, "useTypeChart").mockReturnValue({
      typeChart: MOCK_CHART,
      loading: false,
      error: null,
    });

    const html = renderToStaticMarkup(<TypeChartGrid />);
    expect(html).toContain("DEF");
    expect(html).toContain("ATK");
    expect(html).toContain("typeSquare");
    expect(html).toContain("Grass");
    expect(html).toContain("Water");
    expect(html).toContain("2");
    expect(html).toContain('aria-label="Grass attacking Water: 2×"');
  });

  it("renders merged 2-width column when 2 defendingTypes are specified", () => {
    vi.spyOn(typeChartHook, "useTypeChart").mockReturnValue({
      typeChart: MOCK_CHART,
      loading: false,
      error: null,
    });

    const html = renderToStaticMarkup(
      <TypeChartGrid defendingTypes={["Water", "Ground"]} />,
    );
    expect(html).toContain("selectedDefenderBtn");
    expect(html).toContain('colSpan="2"');
    expect(html).toContain("cellCombined");
    expect(html).toContain("Reset");
    // Grass hits Water (2x) and Ground (2x) -> 4x in combined column
    expect(html).toContain("4");
    expect(html).toContain('aria-label="Grass attacking Water and Ground: 4×"');
  });

  it("renders merged 3-width column when 3 defendingTypes are specified", () => {
    vi.spyOn(typeChartHook, "useTypeChart").mockReturnValue({
      typeChart: MOCK_CHART,
      loading: false,
      error: null,
    });

    const html = renderToStaticMarkup(
      <TypeChartGrid defendingTypes={["Water", "Ground", "Fire"]} />,
    );
    expect(html).toContain('colSpan="3"');
    expect(html).toContain("cellCombined");
  });

  it("renders single defender selected with faded other columns", () => {
    vi.spyOn(typeChartHook, "useTypeChart").mockReturnValue({
      typeChart: MOCK_CHART,
      loading: false,
      error: null,
    });

    const html = renderToStaticMarkup(
      <TypeChartGrid defendingTypes={["Water"]} />,
    );
    expect(html).toContain("selectedDefenderBtn");
    expect(html).toContain('colSpan="1"');
    expect(html).toContain("cellFaded");
    expect(html).toContain("headerFaded");
    expect(html).toContain("Reset");
  });
});
