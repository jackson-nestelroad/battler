import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./index.scss";
import App from "./App.tsx";

import { Provider } from "react-redux";
import { store, hydrateStore } from "./store/store.ts";
import { connectWamp } from "./core/wamp.ts";

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
      <App />
    </Provider>
  </StrictMode>,
);
