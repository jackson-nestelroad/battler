import { configureStore } from "@reduxjs/toolkit";
import { Provider } from "react-redux";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import battlesReducer from "../../store/battlesSlice";
import Resources from "./Resources";
import ResourcesHome from "./ResourcesHome";
import TypeChartScreen from "./TypeChartScreen";
import * as typeChartHook from "../../hooks/useTypeChart";

describe("Resources", () => {
  it("renders ResourcesHome with category cards", () => {
    const html = renderToStaticMarkup(
      <ResourcesHome onSelectResource={vi.fn()} />,
    );

    expect(html).toContain("Resources");
    expect(html).toContain("Type Chart");
    expect(html).toContain("Effectiveness matrix and matchup calculator.");
    expect(html).not.toContain("Coming soon");
  });

  it("renders TypeChartScreen with back button, title, and type chart grid", () => {
    vi.spyOn(typeChartHook, "useTypeChart").mockReturnValue({
      typeChart: { types: {} },
      loading: false,
      error: null,
    });

    const html = renderToStaticMarkup(<TypeChartScreen onBack={vi.fn()} />);

    expect(html).toContain("← Resources");
    expect(html).toContain("Type Chart");
    expect(html).toContain("gridWrapper");
  });

  it("renders Resources wrapper routing to ResourcesHome when view is resources", () => {
    const store = configureStore({
      reducer: {
        battles: battlesReducer,
      },
      preloadedState: {
        battles: {
          battles: {},
          activeBattleId: null,
          currentView: "resources" as const,
          activeResource: null,
          spectatingBattleIds: [],
        },
      },
    });

    const html = renderToStaticMarkup(
      <Provider store={store}>
        <Resources />
      </Provider>,
    );
    expect(html).toContain("Resources");
    expect(html).toContain("Type Chart");
  });

  it("renders Resources wrapper routing to TypeChartScreen when activeResource is type-chart", () => {
    vi.spyOn(typeChartHook, "useTypeChart").mockReturnValue({
      typeChart: { types: {} },
      loading: false,
      error: null,
    });

    const store = configureStore({
      reducer: {
        battles: battlesReducer,
      },
      preloadedState: {
        battles: {
          battles: {},
          activeBattleId: null,
          currentView: "resources" as const,
          activeResource: "type-chart",
          spectatingBattleIds: [],
        },
      },
    });

    const html = renderToStaticMarkup(
      <Provider store={store}>
        <Resources />
      </Provider>,
    );
    expect(html).toContain("← Resources");
    expect(html).toContain("Type Chart");
    expect(html).toContain("gridWrapper");
  });
});
