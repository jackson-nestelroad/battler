# Battler Log Formatter Tests

This directory contains the testing suite for the log formatting library. Because the formatter sits directly on top of the Rust `battler-state` engine, testing it requires a slightly complex setup to guarantee complete coverage without duplicating the engine's parsing logic.

## The Exhaustive Snapshot Test

The crown jewel of this test suite is `exhaustive.test.ts`. It runs over **200+ unique log permutations** to guarantee that our UI string formatting doesn't break or look grammatically incorrect for *any* possible log output from the engine.

Instead of asserting against manually constructed objects, this test uses **Snapshot Testing**. It formats every log, stringifies the tokens, and writes them to `__snapshots__/exhaustive.test.ts.snap`. This snapshot file acts as a human-readable dictionary of how every single log type is translated into English!

### The "Master Setup State"
In order to parse raw strings like `damage|mon:Pikachu,player-1,1|health:120/312`, the `battler-state` engine enforces strict validation. If it doesn't know who `player-1` is, it will crash.

To bypass this without needing to construct a unique battle state for every single test case, `exhaustive.test.ts` initializes a massive **Master Setup State** before running the tests. This mock state registers every possible generic Player ID (`player-1`, `protagonist`, `wild`, etc.) and a generic Pokemon for every slot, so that the engine never crashes on initialization.

## The Test Matrix

The list of 200+ logs is defined in `logs-matrix.json`. This file should **not** be edited manually!

### How the Matrix is Generated
Because maintaining 200+ string permutations by hand is impossible, we algorithmically scrape them directly from the Rust integration tests!

The script `scraper.ts` performs the following steps:
1. Recursively scans all 600+ `.rs` files in the `battler/tests` directory.
2. Extracts every raw string literal that looks like a log.
3. Uses `maskLog` from `utils.ts` to convert the string literal into a masked pattern (e.g., `boost|by:*|mon:*|stat:*`).
4. Compiles a master list of all unique masked patterns across the codebase into `unique-log-patterns.txt`.
5. Checks if these patterns have a defined fallback mapping in `locales/en.ts` via the `pattern_reconstructor.ts` API.
6. If a pattern is completely unmapped, it automatically injects an `[UNHANDLED]` placeholder into `locales/en.ts`.
7. Parses every `key:value` pair and `flag` from the extracted strings.
8. Groups them by log title (e.g., `move`, `faint`, `activate`).
9. Algorithmically selects a minimal subset of strings for each title to guarantee that **every single optional key and flag** ever outputted by the Rust engine is covered in at least one test case!

### The "[UNHANDLED]" Pipeline
The introduction of `[UNHANDLED]` guarantees that we never accidentally drop a log message without formatting it for the UI.
When `scraper.ts` is run, any newly discovered log pattern from Rust integration tests that isn't mapped to a generic string in `locales/en.ts` will have a key automatically generated for it, with the value `"[UNHANDLED]"`.

`exhaustive.test.ts` specifically asserts that no log string contains the word `"[UNHANDLED]"`. If a new log is found, the test will automatically fail until a developer manually reviews the `en.ts` file. 
The developer must either:
- Supply a specific string translation (e.g. `{{MON}}'s {{STAT}} fell drastically!`)
- Set the key to `undefined` in `en.ts`, telling the formatter to implicitly fallback to the more generic template version of that log (e.g. `{{MON}}'s {{STAT}} fell!`).

### Updating the Matrix
Whenever new log types, fields, or flags are added to the Rust engine and tested in the Rust integration tests, you should regenerate the matrix to ensure the formatter supports them.

To update the matrix and log patterns, simply run:
```bash
npx tsx tests/scraper.ts
```
This will overwrite `logs-matrix.json` and `unique-log-patterns.txt`. It will also inject any unhandled logs into `locales/en.ts`. 

If `vitest` fails due to `[UNHANDLED]` logs:
1. Open `locales/en.ts`
2. Search for `"[UNHANDLED]"`
3. Replace them with either the desired string template, or `undefined` to fallback to a generic template.
4. Run `npx vitest run -u tests/exhaustive.test.ts` to update the snapshot.
