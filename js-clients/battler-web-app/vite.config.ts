import { defineConfig } from "vite";
import type { PluginOption } from "vite";
import react from "@vitejs/plugin-react";
import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";
import { nodePolyfills } from "vite-plugin-node-polyfills";
import path from "node:path";

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    react(),
    ((wasm as unknown) as () => PluginOption)(),
    ((topLevelAwait as unknown) as () => PluginOption)(),
    nodePolyfills(),
  ],
  resolve: {
    alias: {
      "battler-choice-wasm": path.resolve(
        __dirname,
        "../../battler-choice/battler-choice-wasm/pkg/bundler"
      ),
      "battler-state": path.resolve(
        __dirname,
        "../../battler-state/pkg/bundler"
      ),
    },
  },
  build: {
    target: "esnext",
  },
});
