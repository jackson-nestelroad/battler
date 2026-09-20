import { configureStore } from "@reduxjs/toolkit";
import { renderToStaticMarkup } from "react-dom/server";
import { Provider } from "react-redux";
import { describe, expect, it } from "vitest";
import battlesReducer from "../../store/battlesSlice";
import connectionReducer from "../../store/connectionSlice";
import proposalsReducer from "../../store/proposalsSlice";
import teamsReducer from "../../store/teamsSlice";
import ProposalForm from "./ProposalForm";

describe("ProposalForm", () => {
  const createMockStore = () =>
    configureStore({
      reducer: {
        connection: connectionReducer,
        proposals: proposalsReducer,
        battles: battlesReducer,
        teams: teamsReducer,
      },
    });

  it("renders Standard battle explanation and tab tooltips by default", () => {
    const store = createMockStore();
    const html = renderToStaticMarkup(
      <Provider store={store}>
        <ProposalForm />
      </Provider>,
    );

    // Header & Tabs
    expect(html).toContain("New Battle Proposal");
    expect(html).toContain('title="Standard battle with configured teams"');
    expect(html).toContain('title="Chaos battle with randomly generated legitimate Pokémon"');

    // Standard battle explanation
    expect(html).toContain(
      "Standard: Traditional Pokémon battles using your configured teams with customizable formats and rules.",
    );
  });

  it("renders Chaos battle explanation highlighting legitimate Pokémon vs True Chaos illegal Pokémon", () => {
    const store = createMockStore();
    const html = renderToStaticMarkup(
      <Provider store={store}>
        <ProposalForm initialCategory="chaos" />
      </Provider>,
    );

    // Chaos battle explanation
    expect(html).toContain(
      "Chaos: Battles with randomly generated teams of legitimate Pokémon. True Chaos uses fully random, illegal Pokémon.",
    );
    expect(html).toContain('title="True Chaos uses fully random, illegal Pokémon"');
  });
});
