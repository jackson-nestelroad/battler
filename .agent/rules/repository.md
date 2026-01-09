---
trigger: always_on
---

# Repository Context

This repository implements a robust Pokémon battle engine in Rust.

It contains several Rust crates for modularization. Most crates contain their own README for context. There are a few high-level categories of crates in this repository:

- `battler` - The core battle engine.
- `battler-choice`, `battler-data`, `battler-prng` - Utility crates for the core battle engine.
- `battler-ai`, `battler-calc`, `battler-client`, `battler-state` - Client-side libraries for battler.
- `battler-service-*` - Service for managing multiple battles on battler.
- `battler-multiplayer-service-*` - Service for managing multiplayer battles on battler.
- `battler-wamp-*` - WAMP implementation for Rust, for a battle server.