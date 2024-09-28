# battler

**battler** is battle engine and simulator based on the Pokémon games, written in Rust.

The battle engine is designed off of a few principles:

1. Self-contained solution for controlling battles. A battle can easily be run with the engine and required effect data.
1. Separation between battle engine and battle clients. This engine is focused on generating a battle log, which can be easily displayed and replayed by battle clients.
1. Support for high level of customization for all sorts of effects through an interpreted language written directly on effect data.

All moves, abilities, and items through **Generation 3** have been implemented and validated to work on the battle engine.

## Features

- Battle types.
  - Single battles.
  - Double battles.
  - Triple battles.
  - Multi battles.
- Team validation.
- Team Preview.
- Switching.
- Moves.
  - Priority and speed ordering.
  - PP checking and deduction.
  - Damage calculation and modifiers.
  - Type effectiveness and immunity.
  - Critical hits.
  - Evasion and accuracy checks.
  - OHKO.
  - Self-destruct.
  - Self switch (including Baton Pass).
  - Recoil.
  - Multi-hit.
  - User and target effects.
  - Stat boosts.
  - Healing.
  - Draining.
  - Force switching.
  - Secondary effects against user and target.
  - Two-turn moves (e.g., Fly, Dig).
  - Multi-turn moves (e.g., Bide).
  - Locked moves (e.g., Thrash, Petal Dance).
  - Custom damage calculations (e.g., Low Kick, Psywave).
  - Using moves within moves (e.g., Mimic, Mirror Move, Metronome).
  - Custom move volatile conditions.
  - Move disabling.
  - Transformation.
  - Substitute.
  - Protection.
- Abilities.
- Items.
  - Held items.
  - Berries.
  - Gems.
- Forme changes.
- Status conditions (burn, paralysis, sleep, freeze, poison, bad poison).
- Volatile conditions (e.g., confusion, partially-trapped, flinch, recharge, and more).
- Side conditions.
- Entry hazards.
- Slot conditions (e.g., Future Sight).
- Weather.
- Pseudo-weather.
- Terrains.
- Battle environments.
- Single-player mechanics.
  - Experience.
  - Level up and move learning.
  - Fleeing.
  - Affection.
  - Disobedience.
  - Bag items.
  - Catching.
- Horde battle support.
