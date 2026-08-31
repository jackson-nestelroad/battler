# Battler Log Formatter Tests

This directory contains the testing suite for the log formatting library. Because the formatter sits directly on top of the Rust `battler-state` engine, testing it requires a structured setup to guarantee complete coverage without duplicating the engine's parsing logic.

## The Exhaustive Snapshot Test

The crown jewel of this test suite is `exhaustive.test.ts`. It runs over **1,300+ unique log permutations** to guarantee that our UI string formatting doesn't break or look grammatically incorrect for *any* possible log output from the engine.

Instead of asserting against manually constructed objects, this test uses **Golden Master Snapshot Testing**. It feeds every log through the parser and snapshots the exact data contract produced by the formatter into `__snapshots__/exhaustive.test.ts.snap`.

This snapshot file acts as a human-readable dictionary. For each log, it asserts:
1. **The Context**: The extracted context variables (e.g., `MON`, `STAT`, `MOVE`).
2. **The Key**: The specific combinatorial enum key the formatter resolved to based on `locales/en.ts` (e.g., `ability__ability_neutralizinggas`).
3. **The Formatted Output**: The final `category` and interpolated `message` string (e.g., `"Neutralizing gas filled the arena!"`).

Any change to the parsing logic, translation strings, or tokenization engine will cause this static data contract to fail, ensuring 100% correct parser validation!

### The "Master Setup State"
In order to parse raw strings like `damage|mon:Pikachu,player-1,1|health:120/312`, the `battler-state` engine enforces strict validation. If it doesn't know who `player-1` is, it will crash.

To bypass this without needing to construct a unique battle state for every single test case, `exhaustive.test.ts` initializes a massive **Master Setup State** before running the tests. This mock state registers generic Player IDs and Pokemon for every slot, so that the engine never crashes on initialization.

## The Test Matrix & Scraper Pipeline

The list of test logs is defined in `tests/data/logs-matrix.json`. **This file is generated automatically and should never be edited manually.**

### How the Scraper Works (`tests/scraper.ts`)

Because maintaining thousands of string permutations by hand is impossible, `tests/scraper.ts` algorithmically discovers, normalizes, and validates logs from across the codebase:

1. **Rust Test Suite Scraping**: Recursively scans all `.rs` files in `battler/tests` and extracts all raw string literals matching log structures.
2. **Fxlang Program Analysis**: Analyzes effect programs in `battle-data/data/` (abilities, items, moves, conditions, clauses) to find programmatic log calls (such as `log_*`, `add_side_condition`, `forme_change`) and delegates.
3. **Log Masking & Normalization**: Normalizes logs into canonical patterns using `tests/data/scraper-config.json` (tag exclusions, numeric bucketing, rule-based tag/flag stripping, collapsing wildcards, and dimension injection).
4. **Permutation & Combinatorics Generation**: Computes all fallback combinations for every discovered pattern using `generateCombinatorics()`.
5. **Locale Key Synchronization**: Automatically injects any newly discovered keys into `locales/en.ts` initialized to `null`.
6. **Stale Key Detection**: Compares all keys in `locales/en.ts` against the master list of generated fallbacks.

### Scraper Output Files

Running `npm run extract-logs` (or `npx tsx tests/scraper.ts`) generates and updates the following artifacts:

| Output File | Purpose |
|---|---|
| `tests/data/logs-matrix.json` | Sample of raw log strings (up to 3 per unique pattern) used by `exhaustive.test.ts`. |
| `tests/data/unique-log-patterns.txt` | Master list of all unique masked log patterns extracted from the codebase. |
| `tests/data/stale-keys.txt` | List of orphaned/dead translation keys currently in `locales/en.ts` that have no corresponding log patterns. |
| `locales/en.ts` | Injects missing keys set to `null` (alphabetically sorted). |

---

## Stale Key Lifecycle & Deletion Policy

> [!IMPORTANT]
> **Never delete translation keys from `locales/en.ts` manually.**
>
> Deleting an active key manually causes it to be immediately re-injected as `null` the next time `npm run extract-logs` is run.
> Keys should **strictly and exclusively** be deleted only if they appear in `tests/data/stale-keys.txt`.

### How Stale Keys Occur
As the Rust battle engine evolves (e.g. log tags are renamed, unused flags are removed, or scraping rules change in `scraper-config.json`), old translation keys in `locales/en.ts` become dead code.

When the scraper runs, it generates the master set `allGeneratedFallbacks`. Any key in `locales/en.ts` that does not exist in `allGeneratedFallbacks` is written to `tests/data/stale-keys.txt`.

### Handling Stale Keys
When `tests/data/stale-keys.txt` contains entries:
1. **If the key is truly obsolete**: Delete the listed keys from `locales/en.ts`.
2. **If the key should NOT be stale (e.g. it is a valid mechanic)**: Do NOT delete it! Check `tests/data/scraper-config.json` to ensure the relevant title, tag, flag, or effect is not accidentally being stripped, ignored, or excluded.

---

## Scraper Configuration (`scraper-config.json`)

To prevent combinatorial explosion of permutations and fine-tune pattern generation, configure `tests/data/scraper-config.json`:

- **`rules`**: Define transformations for specific log patterns:
  - `strip`: Removes unnecessary tags or flags (e.g. stripping `noanim`).
  - `collapse`: Collapses specific tag values into wildcards `*` (e.g. collapsing `move` for `Forewarn` into `move:*` so it generates `activate__ability_forewarn__move_any`).
  - `inject`: Forcibly generates variations (e.g. injecting `battletype:singles` for `crit` or `supereffective` logs).
- **`excludeTags`**: Globally ignored metadata tags (e.g. `health`, `gender`, `level`).
- **`allowKeys`**: Explicitly allows certain fxlang patterns that don't directly occur in unit tests.
- **`ignoreKeys`**: Explicitly ignores specific patterns.
- **`manualLogs`**: Injects explicit log patterns that should be included in the test matrix.

---

## How Fallback (`null`) Keys Work in `en.ts`

In `locales/en.ts`:
- Setting a key to a string template (e.g. `"{{MON}} used {{MOVE}}!"`) provides an explicit message for that permutation.
- Setting a key to `null` instructs the `LogFormatter` to **skip** this specific permutation and continue down the fallback chain to a more generic template (e.g. `move__from_move_metronome__zpower: null` falls back to `move__zpower` or `move`).

If you review an auto-injected key and want it to inherit from a generic template, leave its value as `null`.

---

## Standard Workflow

Whenever log generation logic or battle engine mechanics change:

1. **Extract logs and synchronize keys**:
   ```bash
   npm run extract-logs
   ```
2. **Review output**:
   - Check `tests/data/stale-keys.txt`. If dead keys are found, delete only those keys from `locales/en.ts` (or fix `scraper-config.json` if they should not be stale).
   - Check `locales/en.ts` for newly added keys set to `null`. If a custom translation string is desired, replace `null` with the translation template.
3. **Run validation and tests**:
   ```bash
   npm run lint:locales
   npm run test
   ```
4. **Update test snapshots if translations changed**:
   ```bash
   npm run test -- -u
   ```
