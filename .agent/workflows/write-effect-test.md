---
description: Write integration tests for a specific battle effect
---

# Write Effect Test

## Goal

Your goal is to write integration tests for a specific battle effect, as instructed by the user.

A battle effect is anything that impacts some part of a battle, such as a move, ability, item, status, weather, field effect, and more.

If a battle effect implements an existing Pokémon mechanic, you SHOULD assume it implements the same behavior as the most recent Pokémon generation. DO NOT consider earlier generations.

The tests run against the core battle engine, `battler`, written in Rust. All battle effects are implemented in fxlang in the JSON files defined in `battle-data`.

### Test Structure

All integration tests are defined in `battler/tests`. There is a directory for each type of effect (e.g., abilities, items, moves, etc.), and a sub-directory for each generation, which corresponds to where the effect is defined.

Each effect has its own test file consisting of multiple integration tests. Your goal is to create a new test file, consisting of one or more integration tests, for your specific effect. Each test file MUST be registered in `battler/tests/tests.rs`.

Your tests should match the structure of existing tests as closely as possible:

1. Define team(s) to be used.
2. Create a battle with `TestBattleBuilder`.
3. Create Rust tests that trigger the effect being tested and verify the battle log output.

It is acceptable and encouraged to use a test file for an existing effect as a _template_ for your test. You MUST ensure style consistency with other effect integration tests.

Individual test cases should be as simple as possible and only test the required effect. Other simple effects can be used when required (for example, using a simple damaging move when a Mon needs to take damage). A battle should ONLY go as long as required for testing, which is often only one turn.

You MUST use `pass` actions as much as possible. For example, in a Singles battle, if only one Mon needs to move, the other player should use `pass`. In a Doubles battle, if only one Mon needs to move for a single player, the second Mon's action should be `pass`.

You MUST minimize the number of turns required to test. If multiple actions can happen on the same turn, they MUST be combined. For example:

1. Turn 1 - Mon A uses move A.
2. Turn 2 - Mon B uses move B.

if Mon A always moves before Mon B (i.e., Mon A has same or higher speed than Mon B), then the two turns can be combined into one.

### Battle Verification with Battle Log

Verifying battle state MUST use the public battle log. NEVER attempt to access battle state directly in tests.

Battle logs are matched with `Vec<LogMatch>`. You MUST combine log matches as much as possible. DO NOT run multiple turns and verify logs in between. You MUST run all turns back-to-back and verify all logs at the end.

It is often infeasible to generate the expected battle log output ahead of time. It is reasonable to write a test, run it, view the battle log output, and then update the test to match the actual output. You MUST verify that the battle log output is correct after updating the test.

NOTE: `LogMatch` supports substring matches by placing strings in an array. When viewing log diffs in test failures, substring matches will always appear to not be matched (a limitation of the string differ). When focusing on concrete log differences, ignore substring failures unless they appear legitimately incorrect.

DO NOT focus on low-level intricacies, such as precise damage calculations. You SHOULD trust that the core battle engine is correct for core features unrelated to the special effect being tested. ALWAYS focus on the high-level effect under test. DO NOT estimate damage calculations yourself.

### Teams

A single test file MUST reuse MINIMAL teams as much as possible: use as few teams and Mons as possible. It is HIGHLY encouraged for the same team to face one another. You are only allowed to use multiple teams by exception, if Mon diversity is required.

ALWAYS use Pokémon that make logical sense for the effect being tested. For example, if testing a move or ability, you SHOULD use a Pokémon that can naturally use that move/ability, introduced in the same generation.

### RNG

Every test MUST use a set seed to prevent minor RNG differences from failing tests. In most cases, using a set seed is acceptable to control an outcome.

If you are tempted to generate a large number of turns to verify an RNG-affected condition, consider using the `with_controlled_rng` option instead. This option allows you to call `insert_fake_values_relative_to_sequence_count` to control the RNG values used through the battle. Understanding the correct RNG values WILL require you to run the test multiple times to find the correct injections. Every RNG injection MUST be understood EXACTLY and explained with a comment.

### Test Cases

Test cases MUST be based on the specialized behavior of the effect itself. Focus on testing what makes the effect unique. Focus on edge cases that are identifiable in the fxlang code (JSON) itself.

Below are some examples of test cases:

- For an effect that modifies move power or damage under some condtion, use the move before and after that condition is true. The damage output should be noticeably different (DO NOT assume what the exact damage values should be; use logical approximations on if the modified damage output looks correct).
- For an effect that prevents a Mon from doing something, verify the interaction.
- For an effect that does not affect a Mon under some condition, verify this behavior is true.
- For an effect that removes some other effect, verify the interaction is correct.

### Tips

If testing damage calculation modifiers, it is often beneficial to:

1. Use a move before the effect/condition.
2. Apply the effect/condition that adds the modifier.
3. Use the same move after the effect/condition.
4. Observe the difference.

To avoid a Mon from fainting from being hit repeatedly, you can either a) use Recover on the target or b) enable infinite bag items and use a Max Potion on the target.

Damage calculations are randomized early in the process. For more precise control of calculations by removing this randomness, you can change `with_base_damage_randomization` on the battle builder to `Max`.

## Completing Effect Implementations and Fixing Bugs in Non-Test Code

You may be instructed to complete some implementation aspect of the effect. This may be in the fxlang code (JSON) or `battler` itself (Rust). For example, a function used by the effect fxlang code may not be implemented in `battler`, or there may be an incorrect implementation in the fxlang code.

You SHOULD make such changes and fix such bugs by including the changes in your plan artifact and waiting for user approval.

You MUST carefully scrutinize all changes you are making to the core battle engine. If you discover a new code requirement, you MUST update your plan artifact and receive additional user approval before making the change. NEVER proceed with unrelated changes without additional user approval on the specifics of the change.

If you are repeatedly stuck on solving a bug in the core battle engine, STOP AND ASK FOR HELP FROM THE USER. DO NOT continually guess and try to fix the bug yourself.

## Process

### Step 1: Gather Requirements

1. Ask the user for the following:
   1. The specific effect that must be tested.
   2. Any specific tests scenarios that should be included.
   3. Ask for any additional code that must be completed for the effect to work.

### Step 2: Understand the Effect

1. Locate the JSON effect definition in `battle-data`.
2. Read the effect definition. Pay particular attention to the fxlang code for the effect and any condition. Understand the triggering events and any edge conditions.
3. If necessary, consult background knowledge and online resources such as Bulbapedia for the effect.
4. Identify similar effects that have already been implemented and tested. Use their tests as a reference for how to test this effect. Focus on testing special behavior; common functionality does not need to be explicitly tested on special effects.

### Step 3: Plan the Test

1. Create a plan artifact. It MUST contain all of the following for the effect:
   1. A summary of your understanding of the effect.
   2. The test cases to be written, based on your understanding of the effect (collected in Step 2).
   3. (Optional) Existing tests used as a template, if any.
   4. The team (or in the exceptional case, teams) that will be used for the test. Include the Pokémon and moves that will be used.
2. Wait for user approval of the plan.

### Step 4: Implement

IMPORTANT: If any new requirement arises, you MUST update your plan artifact and receive additional user approval before proceeding. NEVER make rogue code changes to the core battle engine or effect data (JSON) without additional user approval.

1. Write the test and make changes following your plan.
2. Ensure the test passes using `cargo test` with `--no-default-features`.

### Step 5: Complete

1. Verify code follows the Rust coding style (`.rules/rust.md`).
