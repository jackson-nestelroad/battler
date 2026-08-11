import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Provider } from "react-redux";
import App from "./App.tsx";
import "./index.scss";

import { connectWamp } from "./core/wamp.ts";
import { hydrateStore, store } from "./store/store.ts";
import ErrorBoundary from "./ui/Common/ErrorBoundary.tsx";

// Dispatch hydration immediately on startup
store
  .dispatch(hydrateStore())
  .unwrap()
  .finally(() => {
    const state = store.getState();
    const { savedPlayerId, savedServerUrl, autoconnect } = state.connection;
    if (autoconnect && savedPlayerId && savedServerUrl) {
      store.dispatch(
        connectWamp({
          url: savedServerUrl,
          playerId: savedPlayerId,
          autoconnect: true,
        }),
      );
    }
  });

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Provider store={store}>
      <ErrorBoundary>
        <App />
      </ErrorBoundary>
    </Provider>
  </StrictMode>,
);
