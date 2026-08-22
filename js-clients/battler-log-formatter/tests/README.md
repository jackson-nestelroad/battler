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

The list of 1,500+ logs is defined in `tests/data/logs-matrix.json`. This file should **not** be edited manually!

### How the Matrix is Generated
Because maintaining 1,500+ string permutations by hand is impossible, we algorithmically scrape them directly from the Rust integration tests!

The script `scraper.ts` performs the following steps:
1. Recursively scans all 600+ `.rs` files in the `battler/tests` directory.
2. Extracts every raw string literal that looks like a log.
3. Groups them by log title (e.g., `move`, `faint`, `activate`).
4. Algorithmically selects a minimal subset of strings for each title to guarantee that **every single optional key and flag** ever outputted by the Rust engine is covered in at least one test case!

When `scraper.ts` is run, it also inspects `locales/en.ts` to see if all discovered log patterns are supported. If it finds new combinations that aren't mapped, it automatically injects a key into `locales/en.ts` with the value `"[UNHANDLED]"`.

When you subsequently run the tests, these `[UNHANDLED]` values will appear in the generated `message` fields within the `.snap` file, immediately alerting you that a new Rust log output needs a localized translation.

**Important**: At runtime, if the `LogFormatter` encounters a key in `en.ts` whose value is exactly `"[UNHANDLED]"`, it will **ignore** that key and continue searching for a more generic fallback. This allows the scraper to be noisy in `en.ts` without breaking the game!

### Scraper Configuration (`scraper-config.json`)

To prevent combinatorial explosions of automatically generated fallbacks, you can collapse specific dimensions using `tests/data/scraper-config.json`.

For example, the `Forewarn` ability triggers on every single move the opponent has, which would generate hundreds of specific `activate__ability_forewarn__move_x` logs. We can prevent this by conditionally collapsing the `move` tag into a wildcard:

```json
{
  "collapseDimensions": [
    { "match": { "title": "activate", "ability": "Forewarn" }, "collapse": ["move"] }
  ]
}
```

With this configuration, any log matching `activate` and `Forewarn` will have its specific move thrown away and replaced with `move:*` before permutations are even calculated. This naturally funnels all of them into a single `activate__ability_forewarn__move_any` key, without you having to explicitly ignore anything!

You can also use `scraper-config.json` to configure the base scraper engine logic using `excludeTags` and `keepSpecificTags`, or use `injectDimensions` to forcibly generate variations for certain log titles (e.g. injecting `battletype:singles` and `battletype:doubles` for `crit` logs) even if those variations don't explicitly appear in the Rust unit test outputs.

### Stale Key Detection
As the Rust engine evolves, old translation keys in `en.ts` may become obsolete (dead code).
When the scraper runs, it generates a master list of **every possible** dynamically and statically generated fallback. Any key currently sitting in `en.ts` that is not in this master list is considered "stale".

The scraper outputs all orphaned keys to `tests/data/stale-keys.txt`. You can periodically review this file and delete those dead keys from `en.ts`.

### Updating the Matrix
Whenever new log types, fields, or flags are added to the Rust engine and tested in the Rust integration tests, you should regenerate the matrix to ensure the formatter supports them.

To update the matrix and log patterns, simply run:
```bash
npx tsx tests/scraper.ts
```
This will overwrite `tests/data/logs-matrix.json` and automatically inject any missing keys into `locales/en.ts`.

Afterward, you must manually review `locales/en.ts`:
1. Search for `"[UNHANDLED]"`
2. Replace them with the desired string template. 
3. **Important**: If you want a specific permutation to fall back to a generic template (like `weather__weather_any`), do **not** just delete the key (the scraper will re-inject it). Instead, explicitly set its value to `undefined`. This acts as an explicit marker that you've reviewed the key and chosen to fall back!
4. Run `npm run test -- -u` to update the snapshots with your new translations!
