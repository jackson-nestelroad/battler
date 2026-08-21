# Battler Log Formatter Tests

This directory contains the testing suite for the log formatting library. Because the formatter sits directly on top of the Rust `battler-state` engine, testing it requires a slightly complex setup to guarantee complete coverage without duplicating the engine's parsing logic.

## The Exhaustive Snapshot Test

The crown jewel of this test suite is `exhaustive.test.ts`. It runs over **1,500+ unique log permutations** to guarantee that our UI string formatting doesn't break or look grammatically incorrect for *any* possible log output from the engine.

Instead of asserting against manually constructed objects, this test uses **Golden Master Snapshot Testing**. It feeds every log through the parser and snapshots the exact data contract produced by the formatter into `__snapshots__/exhaustive.test.ts.snap`. 

This snapshot file acts as a human-readable dictionary. For each log, it asserts:
1. **The Context**: The extracted context variables (e.g., `MON`, `STAT`).
2. **The Key**: The specific combinatorial enum key the formatter resolved to based on `locales/en.ts` (e.g., `ability__ability_neutralizinggas`).
3. **The Formatted Output**: The final `category` and interpolated `message` string (e.g., `"Neutralizing gas filled the arena!"`).

Any change to the parsing logic, translation strings, or tokenization engine will cause this static data contract to fail, ensuring 100% correct parser validation!

### The "Master Setup State"
In order to parse raw strings like `damage|mon:Pikachu,player-1,1|health:120/312`, the `battler-state` engine enforces strict validation. If it doesn't know who `player-1` is, it will crash.

To bypass this without needing to construct a unique battle state for every single test case, `exhaustive.test.ts` initializes a massive **Master Setup State** before running the tests. This mock state registers generic Player IDs and Pokemon for every slot, so that the engine never crashes on initialization.

## The Test Matrix

The list of 1,500+ logs is defined in `logs-matrix.json`. This file should **not** be edited manually!

### How the Matrix is Generated
Because maintaining 1,500+ string permutations by hand is impossible, we algorithmically scrape them directly from the Rust integration tests!

The script `scraper.ts` performs the following steps:
1. Recursively scans all 600+ `.rs` files in the `battler/tests` directory.
2. Extracts every raw string literal that looks like a log.
3. Groups them by log title (e.g., `move`, `faint`, `activate`).
4. Algorithmically selects a minimal subset of strings for each title to guarantee that **every single optional key and flag** ever outputted by the Rust engine is covered in at least one test case!

### The "[UNHANDLED]" Pipeline
When `scraper.ts` is run, it also inspects `locales/en.ts` to see if all discovered log patterns are supported. If it finds new combinations that aren't mapped, it automatically injects a key into `locales/en.ts` with the value `"[UNHANDLED]"`.

When you subsequently run the tests, these `[UNHANDLED]` values will appear in the generated `message` fields within the `.snap` file, immediately alerting you that a new Rust log output needs a localized translation.

### Updating the Matrix
Whenever new log types, fields, or flags are added to the Rust engine and tested in the Rust integration tests, you should regenerate the matrix to ensure the formatter supports them.

To update the matrix and log patterns, simply run:
```bash
npx tsx tests/scraper.ts
```
This will overwrite `logs-matrix.json` and automatically inject any missing keys into `locales/en.ts`.

Afterward, you must manually review `locales/en.ts`:
1. Search for `"[UNHANDLED]"`
2. Replace them with the desired string template (e.g., `{{MON}}'s {{STAT}} fell drastically!`). You can also safely delete them if you want the combinatorial engine to implicitly fallback to a generic template (e.g., `{{MON}}'s {{STAT}} fell!`).
3. Run `npm test -- -u` to update the snapshots with your new translations!
