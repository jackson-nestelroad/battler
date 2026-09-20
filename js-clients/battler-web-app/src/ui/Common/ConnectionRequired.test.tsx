import { configureStore } from "@reduxjs/toolkit";
import { renderToStaticMarkup } from "react-dom/server";
import { Provider } from "react-redux";
import { describe, expect, it } from "vitest";
import connectionReducer, {
  setConnectionStatus,
  setIsHydrated,
} from "../../store/connectionSlice";
import ConnectionRequired from "./ConnectionRequired";

describe("ConnectionRequired", () => {
  it("renders children when connected", () => {
    const store = configureStore({
      reducer: { connection: connectionReducer },
    });
    store.dispatch(setIsHydrated(true));
    store.dispatch(setConnectionStatus("connected"));

    const html = renderToStaticMarkup(
      <Provider store={store}>
        <ConnectionRequired>
          <div data-testid="child-content">Main Content</div>
        </ConnectionRequired>
      </Provider>,
    );

    expect(html).toContain("Main Content");
    expect(html).not.toContain("modalOverlay");
    expect(html).not.toContain("Offline");
  });

  it("renders children when bypass is true even if disconnected", () => {
    const store = configureStore({
      reducer: { connection: connectionReducer },
    });
    store.dispatch(setIsHydrated(true));
    store.dispatch(setConnectionStatus("disconnected"));

    const html = renderToStaticMarkup(
      <Provider store={store}>
        <ConnectionRequired bypass={true}>
          <div data-testid="child-content">Bypassed Content</div>
        </ConnectionRequired>
      </Provider>,
    );

    expect(html).toContain("Bypassed Content");
  });

  it("renders ConnectForm when completely disconnected", () => {
    const store = configureStore({
      reducer: { connection: connectionReducer },
    });
    store.dispatch(setIsHydrated(true));
    store.dispatch(setConnectionStatus("disconnected"));

    const html = renderToStaticMarkup(
      <Provider store={store}>
        <ConnectionRequired>
          <div data-testid="child-content">Protected Content</div>
        </ConnectionRequired>
      </Provider>,
    );

    expect(html).not.toContain("Protected Content");
    expect(html).toContain("<h2>Connect</h2>");
  });

  it("does not render modal initially during reconnect before debounce timer", () => {
    const store = configureStore({
      reducer: { connection: connectionReducer },
    });
    store.dispatch(setIsHydrated(true));
    store.dispatch(setConnectionStatus("connected"));
    store.dispatch(setConnectionStatus("connecting"));

    const html = renderToStaticMarkup(
      <Provider store={store}>
        <ConnectionRequired>
          <div data-testid="child-content">Active Screen</div>
        </ConnectionRequired>
      </Provider>,
    );

    // Initial render before debounce has showModal = false, so no modal overlay is rendered
    expect(html).toContain("Active Screen");
    expect(html).not.toContain("modalOverlay");
    expect(html).not.toContain("Offline");
  });
});

