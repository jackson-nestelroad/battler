import { Provider } from "react-redux";
import App from "./App";
import { connectWamp } from "./core/wamp";
import { hydrateStore, store } from "./store/store";

let hasHydrated = false;

function initMainApp() {
  if (hasHydrated) return;
  hasHydrated = true;

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
}

// Initiate hydration and autoconnect as soon as the MainApp module is evaluated
initMainApp();

export default function MainApp() {
  return (
    <Provider store={store}>
      <App />
    </Provider>
  );
}
