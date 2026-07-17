import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./index.scss";
import App from "./App.tsx";

import { Provider } from "react-redux";
import { store, hydrateStore } from "./store/store.ts";

// Dispatch hydration immediately on startup
store.dispatch(hydrateStore());

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Provider store={store}>
      <App />
    </Provider>
  </StrictMode>,
);
