import fs from "node:fs";
import path from "node:path";
import { execSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// 1. Regenerate TS bindings from Rust types
console.log("Generating TypeScript bindings via cargo test...");
execSync("cargo test -p battler-choice --features typescript export_types", {
  cwd: __dirname,
  stdio: "inherit",
});

// 2. Re-link npm workspaces to expose the newly built package and its types
if (!process.env.npm_lifecycle_event) {
  console.log("Re-linking npm workspaces...");
  execSync("npm install", { cwd: path.resolve(__dirname, ".."), stdio: "inherit" });
} else {
  console.log("Skipping npm install in post-build (already running inside npm lifecycle event).");
}
