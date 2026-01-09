---
trigger: model_decision
description: Applies when working with low-level battle effects directly
---

# fxlang Context

**fxlang** is the JSON-based interpreted language used in `battler` for all battle effects.

## Language Documentation

fxlang's full documentation can be read at `fxlang.md`.

All battle events that trigger battle effects are documented in `events.md`, which MUST be kept up to date if any new event is added to the core battle engine (requires user approval).

## Code Locations

All fxlang code is at `battler/src/effect/fxlang`:

- `eval.rs` - The evaluator, including 
- `functions.rs` - All available functions and their implementation.
- `tree.rs` - The syntax tree.
- `variable.rs` - All available data members.

We can define new functions and data members as required, which requires user approval.