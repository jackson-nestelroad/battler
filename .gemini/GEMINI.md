# battler Repository

This repository implements a robust Pokémon battle engine in Rust.

It contains several Rust crates for modularization. Most crates contain their own README for context. There are a few high-level categories of crates in this repository:

- `battler` - The core battle engine.
- `battler-choice`, `battler-data`, `battler-prng` - Utility crates for the core battle engine.
- `battler-ai`, `battler-calc`, `battler-client`, `battler-state` - Client-side libraries for battler.
- `battler-service-*` - Service for managing multiple battles on battler.
- `battler-multiplayer-service-*` - Service for managing multiplayer battles on battler.
- `battler-wamp-*` - WAMP implementation for Rust, for a battle server.

Due to the high modularization, implementation tasks typically only span a few crates. For example, changing the core battle engine does not require changing `battler-service`, and changing `battler-service` does not require knowing the internals of `battler-wamp`.

# Artifact Working Directory

Any artifact you produce that is not code for the project, such as plan artifacts, MUST live in `.gemini/work`.

# Rules

You MUST follow all rules below. You ALWAYS require explicit user approval when writing code that disobeys a rule.

When faced with an unexpected requirement or bug in the code, you ALWAYS plan your fix and receive user approval before implementing. You NEVER continuously attempt to fix the same bug. When you are stuck, you ALWAYS stop and ask the user for help.

# Rust

@../.gemini/rules/rust.md

# battler (Core Battle Engine)

@../.gemini/rules/battler.md

# Battle Log

@../.gemini/rules/battle-log.md

# Battle Participation

@../.gemini/rules/battle-participation.md

# battler Effect Tests

@../.gemini/rules/battler-effect-tests.md

# fxlang

@../.gemini/rules/fxlang.md
