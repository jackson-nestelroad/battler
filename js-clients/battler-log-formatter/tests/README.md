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
3. Parses every `key:value` pair and `flag` from the extracted strings.
4. Groups them by log title (e.g., `move`, `faint`, `activate`).
5. Algorithmically selects a minimal subset of strings for each title to guarantee that **every single optional key and flag** ever outputted by the Rust engine is covered in at least one test case!

### Updating the Matrix
Whenever new log types, fields, or flags are added to the Rust engine and tested in the Rust integration tests, you should regenerate the matrix to ensure the formatter supports them.

To update the matrix, simply run:
```bash
npx tsx tests/scraper.ts
```
This will overwrite `logs-matrix.json`. You can then run `vitest -u` to update the snapshots for the newly discovered logs!
