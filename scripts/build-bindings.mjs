import fs from "node:fs";
import path from "node:path";
import { execSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, "..");

console.log("=== Starting TypeScript Bindings Build Pipeline ===");

// 1. Generate TypeScript bindings from Rust crates
console.log("Generating types from Rust crates...");
const crates = ["battler-choice", "battler", "battler-service", "battler-multiplayer-service"];

for (const crate of crates) {
  console.log(`Running cargo test for ${crate}...`);
  execSync(`cargo test -p ${crate} --features typescript export_types`, {
    cwd: rootDir,
    stdio: "inherit",
  });
}

// 2. Discover all core engine types dynamically
const coreEngineTypes = new Set();
const choiceFiles = fs
  .readdirSync(path.resolve(rootDir, "battler-choice/bindings"))
  .filter((f) => f.endsWith(".ts"));
const battlerFiles = fs
  .readdirSync(path.resolve(rootDir, "battler/bindings"))
  .filter((f) => f.endsWith(".ts"));
for (const f of [...choiceFiles, ...battlerFiles]) {
  coreEngineTypes.add(path.basename(f, ".ts"));
}
console.log(`Discovered ${coreEngineTypes.size} core engine types.`);

// 3. Clear and set up destination directories
const battlerTypesDir = path.resolve(rootDir, "js-clients/battler-types/src/bindings");
const serviceClientBindingsDir = path.resolve(
  rootDir,
  "js-clients/battler-service-client/src/bindings",
);
const multiplayerClientBindingsDir = path.resolve(
  rootDir,
  "js-clients/battler-multiplayer-service-client/src/bindings",
);

for (const dir of [battlerTypesDir, serviceClientBindingsDir, multiplayerClientBindingsDir]) {
  if (fs.existsSync(dir)) {
    fs.rmSync(dir, { recursive: true, force: true });
  }
  fs.mkdirSync(dir, { recursive: true });
}

// 4. Copy bindings files
console.log("Copying generated binding files to JS client directories...");

// Helper function to copy files matching pattern
function copyPattern(srcDir, destDir, pattern = /\.ts$/) {
  const files = fs.readdirSync(srcDir);
  for (const file of files) {
    if (pattern.test(file)) {
      fs.copyFileSync(path.resolve(srcDir, file), path.resolve(destDir, file));
    }
  }
}

// Core types package gets choice and battler bindings
copyPattern(path.resolve(rootDir, "battler-choice/bindings"), battlerTypesDir);
copyPattern(path.resolve(rootDir, "battler/bindings"), battlerTypesDir);

// Service Client gets service specific bindings only
copyPattern(path.resolve(rootDir, "battler-service/bindings"), serviceClientBindingsDir);

// Multiplayer Client gets multiplayer service specific bindings and copy of specific service options
copyPattern(
  path.resolve(rootDir, "battler-multiplayer-service/bindings"),
  multiplayerClientBindingsDir,
);
fs.copyFileSync(
  path.resolve(rootDir, "battler-service/bindings/BattleServiceOptions.ts"),
  path.resolve(multiplayerClientBindingsDir, "BattleServiceOptions.ts"),
);
fs.copyFileSync(
  path.resolve(rootDir, "battler-service/bindings/Timers.ts"),
  path.resolve(multiplayerClientBindingsDir, "Timers.ts"),
);
fs.copyFileSync(
  path.resolve(rootDir, "battler-service/bindings/Timer.ts"),
  path.resolve(multiplayerClientBindingsDir, "Timer.ts"),
);

// 5. Post-process to fix relative ESM import extensions & rewrite cross-package dependencies
console.log("Post-processing generated bindings...");

const directories = [
  { dir: battlerTypesDir, isCore: true },
  { dir: serviceClientBindingsDir, isCore: false },
  { dir: multiplayerClientBindingsDir, isCore: false },
];

for (const { dir, isCore } of directories) {
  const files = fs.readdirSync(dir).filter((f) => f.endsWith(".ts"));

  for (const file of files) {
    const filePath = path.resolve(dir, file);
    let content = fs.readFileSync(filePath, "utf8");

    // Add missing Stat import to MonBattleData.ts since it has custom Record<Stat, number> type
    if (file === "MonBattleData.ts") {
      content = 'import type { Stat } from "./Stat.js";\n' + content;
    }

    // Rewrite imports: from "./Side" -> from "./Side.js"
    content = content.replace(/(from\s+["']\.\/[^"'\.]+)["']/g, '$1.js"');

    // Rewrite relative core imports in client packages to point to the NPM "battler-types" package instead
    if (!isCore) {
      content = content.replace(/from\s+["']\.\/([^"'\.]+)\.js["']/g, (match, importName) => {
        if (coreEngineTypes.has(importName)) {
          return `from "battler-types"`;
        }
        return match;
      });
    }

    // Make generated properties readonly
    content = content.replace(/(^\s*)([a-zA-Z0-9_]+(\?)?:)/gm, "$1readonly $2");

    fs.writeFileSync(filePath, content, "utf8");
  }

  // Generate bindings/index.ts that re-exports all binding types in that folder
  const exportedFiles = fs.readdirSync(dir).filter((f) => f.endsWith(".ts") && f !== "index.ts");
  const indexContent =
    exportedFiles.map((f) => `export * from "./${path.basename(f, ".ts")}.js";`).join("\n") + "\n";

  fs.writeFileSync(path.resolve(dir, "index.ts"), indexContent, "utf8");
  console.log(`Generated bindings index for: ${path.relative(rootDir, dir)}`);
}

// 6. Build WASM battler-state and battler-choice types
console.log("Building WebAssembly and state selectors bindings...");
execSync("npm run build:battler-state", { cwd: rootDir, stdio: "inherit" });
execSync("npm run build:battler-choice", { cwd: rootDir, stdio: "inherit" });

console.log("=== TS Bindings Build Pipeline Completed Successfully ===");
