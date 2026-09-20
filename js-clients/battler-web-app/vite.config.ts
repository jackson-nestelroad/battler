import react from "@vitejs/plugin-react";
import path from "node:path";
import type { PluginOption } from "vite";
import { defineConfig } from "vite";
import { nodePolyfills } from "vite-plugin-node-polyfills";
import { VitePWA } from "vite-plugin-pwa";
import topLevelAwait from "vite-plugin-top-level-await";
import wasm from "vite-plugin-wasm";

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    react(),
    (wasm as unknown as () => PluginOption)(),
    (topLevelAwait as unknown as () => PluginOption)(),
    nodePolyfills(),
    VitePWA({
      registerType: "autoUpdate",
      devOptions: {
        enabled: false,
      },
      manifest: {
        name: "Battler",
        short_name: "Battler",
        description: "Battle Simulator",
        theme_color: "#1e1e2e",
        background_color: "#1e1e2e",
        display: "standalone",
        start_url: "/",
        scope: "/",
        icons: [
          {
            src: "favicon.svg",
            sizes: "any",
            type: "image/svg+xml",
            purpose: "any maskable",
          },
        ],
      },
      workbox: {
        maximumFileSizeToCacheInBytes: 5 * 1024 * 1024,
        globPatterns: ["**/*.{js,css,html,svg,png,wasm,json}"],
        globIgnores: ["**/assets/mons/**", "**/assets/items/**"],
        navigateFallback: "index.html",
        navigateFallbackAllowlist: [/^\/.*/],
        navigateFallbackDenylist: [/^\/assets\/.*/],
        clientsClaim: true,
        skipWaiting: true,
        runtimeCaching: [
          {
            urlPattern: /\/assets\/(?:mons|items)\/.+/i,
            handler: "CacheFirst",
            options: {
              cacheName: "battle-assets-cache",
              expiration: {
                maxEntries: 500,
                maxAgeSeconds: 60 * 60 * 24 * 30,
                purgeOnQuotaError: true,
              },
              cacheableResponse: {
                statuses: [0, 200],
              },
            },
          },
          {
            urlPattern: /^https:\/\/fonts\.googleapis\.com\/.*/i,
            handler: "StaleWhileRevalidate",
            options: {
              cacheName: "google-fonts-stylesheets",
            },
          },
          {
            urlPattern: /^https:\/\/fonts\.gstatic\.com\/.*/i,
            handler: "CacheFirst",
            options: {
              cacheName: "google-fonts-webfonts",
              expiration: {
                maxEntries: 20,
                maxAgeSeconds: 60 * 60 * 24 * 365,
                purgeOnQuotaError: true,
              },
              cacheableResponse: {
                statuses: [0, 200],
              },
            },
          },
        ],
      },
    }),
  ],
  resolve: {
    alias: {
      "battler-choice-wasm": path.resolve(
        __dirname,
        "../../battler-choice/battler-choice-wasm/pkg/bundler",
      ),
      "battler-state": path.resolve(__dirname, "../../battler-state/pkg/bundler"),
    },
  },
  build: {
    target: "esnext",
    modulePreload: false,
  },
});
