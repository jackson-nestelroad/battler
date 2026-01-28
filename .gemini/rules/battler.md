`battler` is the core battle engine based on the Pokémon games, written in Rust.

The battle engine is designed off of a few principles:

1. Self-contained solution for controlling battles. A battle can easily be run with the engine and required effect data.
2. Separation between battle engine and battle clients. This engine is focused on generating a battle log, which can be easily displayed and replayed by battle clients.
3. Support for high level of customization for all sorts of effects through an interpreted language written directly on effect data (fxlang).

## API

Battle options are passed in to create a battle (configuring the type of battle, field, players, teams, etc.). As the battle progresses, requests are generated for players (e.g., make a move, switch out) and players must respond to each request to progress the battle.

The state of the battle is recorded in the battle log. All visible effects simulated by the battle engine are added to the battle log.

## Data

All data for the battle engine is defined in JSON in the `battle-data` directory.

The battle engine is designed to be as generic as possible. For specialized behavior, specific effects (e.g., moves, abilities, items, conditions, etc.) define dynamic callbacks directly within their JSON data.

All effects are grouped by generation. Each generation-specific file is a hash map of data, keyed by ID (lowercase name with no punctuation).

## Dependencies

Within this repository, `battler` only depends on a few utility crates that should rarely change. Otherwise, `battler` operates as the base layer for all battle-related code in this repository.

## Dependents

1. Service libraries (e.g., `battler-service`) provide an API layer on top of the battle engine for managing multiple battles at once.
2. Client libraries (e.g., `battler-client`) provide an API for interacting with a single battle.

The most important dependency of `battler` is `battler-state`, which is responsible for reading the battle log and mutating client-side state accordingly, effectively keeping track of the battle from a single player's point of view. If a new log type is ever added, it MUST be properly handled in `battler-state`. However, adding new logs SHOULD BE RARE and requires user approval.

## Testing

`battler` is a `no_std` crate. As such, tests should be run with `--no-default-features` as much as possible.
