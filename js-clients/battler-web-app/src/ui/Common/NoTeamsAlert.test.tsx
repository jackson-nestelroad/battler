import { configureStore } from "@reduxjs/toolkit";
import { renderToStaticMarkup } from "react-dom/server";
import { Provider } from "react-redux";
import { describe, expect, it } from "vitest";
import battlesReducer from "../../store/battlesSlice";
import NoTeamsAlert from "./NoTeamsAlert";

describe("NoTeamsAlert", () => {
  it("renders warning alert with message and button", () => {
    const store = configureStore({
      reducer: {
        battles: battlesReducer,
      },
    });

    const html = renderToStaticMarkup(
      <Provider store={store}>
        <NoTeamsAlert />
      </Provider>,
    );

    expect(html).toContain('role="alert"');
    expect(html).toContain("alert alert-warning");
    expect(html).toContain("No teams configured");
    expect(html).toContain("Go to Teams");
    expect(html).toContain("btn btn-secondary btn-sm");
  });

  it("applies custom className if provided", () => {
    const store = configureStore({
      reducer: {
        battles: battlesReducer,
      },
    });

    const html = renderToStaticMarkup(
      <Provider store={store}>
        <NoTeamsAlert className="custom-test-class" />
      </Provider>,
    );

    expect(html).toContain("custom-test-class");
  });
});
