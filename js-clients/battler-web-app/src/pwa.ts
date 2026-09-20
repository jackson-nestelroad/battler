import { registerSW } from "virtual:pwa-register";

export function initPWA(): () => void {
  const updateSW = registerSW({
    onOfflineReady() {
      console.log("Battler web app is ready for offline use.");
    },
  });

  return updateSW;
}
