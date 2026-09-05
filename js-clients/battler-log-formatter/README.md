# `battler-log-formatter`

A TypeScript library for formatting battle logs emitted by the [`battler`](../../battler/) engine into localized, human-readable combat messages and structured UI notice badges.

---

## Overview

The `battler` battle engine generates machine-readable, event-based battle logs. `battler-log-formatter` translates these logs into rich, localized text messages suitable for battle replays, live battle spectators, and active players.

Along with formatted messages, it extracts **UI Notice Badges** (`UiNotice[]`) representing ability activations, item triggers, damage, and healing.

---

## Features

- **Pattern Matching**: Matches log signatures against predefined pattern keys (e.g. `move`, `damage|from:item:*` -> `damage__from_item_any`, `boost|by:2` -> `boost__by_2`).
- **I18n & Localization**: Built on `i18next` with localization tables for combat dialogue, stats, pronouns, and system hints.
- **Perspective-Aware Formatting**: Automatically resolves Pokémon ownership and pronouns based on viewer context:
  - **Active Player**: `"Pikachu used Thunderbolt!"`
  - **Opponent**: `"The opposing Charizard used Flamethrower!"`
  - **Spectator**: `"Player 1's Pikachu used Thunderbolt!"`
- **Notice Badge Extraction**: Declaratively extracts badges for UI displays (`[The opposing Gyarados's Intimidate]`, `(Pikachu lost 12% HP)`).
- **Display Item Normalization**: Formats turn markers, continue/residual dividers, and message ordering via `formatEntry` or `formatUiLogEntry`.
- **Structured Tokens & Entity Binding**: Emits tokenized messages (`FormattedUiLog`) associating variables (`monRef`) with live game entities for tooltips and highlights, or flattens directly to strings via `stringifyLog`.
- **Exhaustive Matrix Validation**: Tested against 1,310 real battle engine scenario logs with snapshot regression protection.

---

## Architecture

```
UiLogEntry ──────────────────┐
                             ▼
BattleState (optional) ──► ┌──────────────┐
(from battler-state)       │  mapper.ts   │ ──► Normalizes logs into candidate patterns,
                           └──────┬───────┘     context map, and participant metadata.
                                  │
                                  ▼
                           ┌──────────────┐
                           │ formatter.ts │ ──► Evaluates notice-rules.json, queries i18next,
                           └──────┬───────┘     and produces FormattedLogEvent or DisplayItems.
                                  │
                                  ▼
                    FormattedLogEvent { messages, notices }
                    or FormattedLogDisplayItem[]
```

### Key Modules

- **[`src/index.ts`](src/index.ts)**: Package entry point exporting the core classes, formatting helpers, and TypeScript types.
- **[`src/mapper.ts`](src/mapper.ts)**: Extracts participants (`mon`, `target`, `of`, `source`, `prev_mon`), maps context variables (`MON`, `MOVE`, `ITEM`, `ABILITY`), and handles stat boosts and damage fractions.
- **[`src/formatter.ts`](src/formatter.ts)**: Core `LogFormatter` class and helpers (`formatUiLogEntry`, `stringifyLog`, `formatNoticeText`, `formatContextValue`). Matches templates, applies capitalization, and evaluates notice rules.
- **[`src/engine.ts`](src/engine.ts)**: AST tokenizer (`parseTemplateToTokens`) parsing template variables into structured `LogToken[]`.
- **[`src/types.ts`](src/types.ts)**: Core type definitions (`FormattedLogEvent`, `FormattedLogDisplayItem`, `UiNotice`, `MapperOptions`, `LogCategory`).
- **[`src/i18n.ts`](src/i18n.ts)**: Initializes and configures the underlying `i18next` instance with locale tables.
- **[`src/pattern.ts`](src/pattern.ts)**: Pattern parsing, serialization, and conversion to translation lookup keys.
- **[`locales/en.ts`](locales/en.ts)**: Primary English localization catalog containing combat dialogue templates, vocabulary, pronoun resolutions, and system hints.
- **[`src/config/category-rules.json`](src/config/category-rules.json)**: Declarative rules classifying log events into `LogCategory` (`primary`, `secondary`, `hint`).
- **[`src/config/notice-rules.json`](src/config/notice-rules.json)**: Declarative rules controlling which log events emit notice badges.
- **[`src/config/mapper-rules.json`](src/config/mapper-rules.json)**: Configuration for omittable tags, excluded tags, and numeric bucketing.

---

## Installation & Usage

### Basic Example

```typescript
import { LogFormatter, stringifyLog } from "battler-log-formatter";
import type { UiLogEntry } from "battler-state";

// Create formatter from the perspective of player "p1"
const formatter = new LogFormatter({ localPlayerId: "p1" });

const entry: UiLogEntry = {
  title: "abilitystart",
  side: 1,
  slot: 0,
  player: "p2",
  target: null,
  source: null,
  effect: null,
  source_effect: null,
  values: {
    mon: { Active: { side: 1, position: 0, name: "Gyarados", player: "p2" } },
    ability: "Intimidate",
  },
};

const result = formatter.format(entry);

console.log(stringifyLog(result!.messages[0]));
// "The opposing Gyarados acquired Intimidate!"

console.log(result?.notices);
// [
//   {
//     type: "Ability",
//     name: "Intimidate",
//     mon: "The opposing Gyarados's",
//     monRef: { Active: { side: 1, position: 0, name: "Gyarados", player: "p2" } }
//   }
// ]
```

### UI Integration Example (`formatUiLogEntry` / `formatEntry`)

For UI components (such as log panels), `formatUiLogEntry` flattens turns, dividers, notices, and messages into an ordered sequence of display items (`FormattedLogDisplayItem[]`). Pre-notices (`Ability`, `Item`) are placed before action messages, while post-notices (`Damage`, `Heal`) appear directly after.

`formatUiLogEntry` is a standalone convenience wrapper around `new LogFormatter(options).formatEntry(entry, state)`:

```typescript
import { formatNoticeText, formatUiLogEntry, stringifyLog } from "battler-log-formatter";
import type { BattleState, UiLogEntry } from "battler-state";

// Note: If BattleState is omitted when passing options, pass undefined as the second parameter
const items = formatUiLogEntry(entry, battleState, { localPlayerId: "p1" });

for (const item of items) {
  switch (item.kind) {
    case "turn":
      console.log(`--- Turn ${item.turn} ---`);
      break;
    case "divider":
      console.log(`--- Divider (${item.subtype}) ---`); // "continue" or "residual"
      break;
    case "notice":
      console.log(formatNoticeText(item.notice)); // e.g. "[The opposing Gyarados's Intimidate]" or "(Pikachu lost 12% HP)"
      break;
    case "message":
      console.log(`[${item.category}] ${stringifyLog(item.message)}`);
      break;
  }
}
```

### Options (`MapperOptions`)

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `localPlayerId` | `string` | `undefined` | ID of the local player for 1st-person perspective formatting. |
| `isSpectator` | `boolean` | `!localPlayerId` | When `true`, formats with spectator-style player attribution. Defaults to `true` if `localPlayerId` is omitted, `false` otherwise. |
| `healthFormat` | `"fraction" \| "percentage"` | `"fraction"` (`"percentage"` in `formatUiLogEntry`) | Format for damage and healing numbers. |
| `forceTemplateKey` | `string` | `undefined` | Explicitly overrides template key lookup within `logs.*` (provided without the `logs.` prefix, e.g. `"move"` or `"boost__by_2"`). |

*Notes:*
- The optional `state: BattleState` parameter is passed directly to `formatter.format(entry, state)`, `formatter.formatEntry(entry, state)`, or `formatUiLogEntry(entry, state, options)` rather than to the constructor.
- `formatUiLogEntry` accepts `options?: MapperOptions | string` where a string shorthand sets `localPlayerId` directly (with `healthFormat` defaulting to `"percentage"`). When omitting `state` while providing `options`, explicitly pass `undefined` for `state` (e.g. `formatUiLogEntry(entry, undefined, "p1")`).

---

## Notice Rules & State Attribution

For the complete specification of how ability and item start/end logs are processed across different battle scenarios (such as Trace, Thief, Skill Swap, Neutralizing Gas, and Bug Bite/Pluck), see the repository root specification:

👉 **[`notices.md`](../../notices.md)**

---

## Scripts & Validation

```bash
# Build the package
npm run build

# Run unit tests and matrix snapshot tests
npm test

# Type-check TypeScript code, validate locale syntax, and verify test matrix logs
npm run lint

# Validate that all templates in en.ts have valid syntax and variables
npm run lint:locales

# Validate all 1,310 test matrix logs against the formatter
npm run validate:logs

# Scrape new battle logs from the Rust engine test suite into the test matrix
npm run extract-logs
```

For detailed documentation on the test matrix, golden master snapshots, scraper pipeline, and stale key deletion policy, see [`tests/README.md`](tests/README.md).
