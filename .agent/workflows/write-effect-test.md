---
description: Write an integration test for a specific battle effect
---

# Write Effect Test

## Goal

Your goal is to write an integration test for a specific battle effect, as instructed by the user.

A battle effect is anything that impacts some part of a battle, such as a move, ability, item, status, weather, field effect, and more.

If a battle effect implements an existing Pokémon mechanic, you SHOULD assume it implements the same behavior as the most recent Pokémon generation. DO NOT consider earlier generations.

The test runs against the core battle engine, `battler`, written in Rust. All battle effects are implemented in fxlang in the JSON files defined in `battle-data`.

### Test Structure

All integration tests are defined in `battler/tests`. There is a directory for each type of effect (e.g., abilities, items, moves, etc.), and a sub-directory for each generation, which corresponds to where the effect is defined.

Each effect has its own test file. Each test MUST be registered in `battler/tests/tests.rs`.

Your tests should match the structure of existing tests as closely as possible:

1. Define team(s) to be used.
2. Create a battle with `TestBattleBuilder`.
3. Create Rust tests that trigger the effect being tested and verify the battle log output.

In fact, it is acceptable and encouraged to use a test file for an existing effect as a *template* for your test.

Tests should be as simple as possible and only test the required effect. Other simple effects can be used when required (for example, using a simple damaging move when a Mon needs to take damage). A battle should only go as long as reasonably required for testing, which is often only one turn.

You SHOULD use `pass` actions as much as reasonably possible. For example, in a Singles battle, if only one Mon needs to move, the other player should use `pass`. In a Doubles battle, if only one Mon needs to move for a single player, the second Mon's action should be `pass`.

### Battle Log

It is often infeasible to generate the expected battle log output ahead of time. It is reasonable to write a test, run it, view the battle log output, and then update the test to match the actual output. You MUST verify that the battle log output is correct after updating the test.

NOTE: When viewing log diffs in test failures, the `time` log will always appear to not be matched. This is because the time is dynamic based on when the test runs. The `time` log should always be matched with `["time"]` in the Rust test. If this is true, you SHOULD ignore the `time` diff.

### Teams

Tests SHOULD use Pokémon that make logical sense for the effect being tested. For example, if testing a Water-type move introduced in Generation 5, use a Water-type Pokémon from Generation 5. This requirement helps ensure a wide variety of Pokémon are used in tests.

### Tips

Every test MUST use a set seed to prevent minor RNG differences from failing tests. In most cases, using a set seed is acceptable to control an outcome.

If you are tempted to generate a large number of turns to verify an RNG-affected condition, consider using the `with_controlled_rng` option instead. This option allows you to call `insert_fake_values_relative_to_sequence_count` to control the RNG values used through the battle. Understanding the correct RNG values WILL require you to run the test multiple times to find the correct injections. Every RNG injection MUST be understood EXACTLY and explained with a comment.

## Fixing Bugs in Non-Test Code

In your testing, you may find a bug in either the fxlang code (JSON) or `battler` itself (Rust). For example, a function used by the effect fxlang code may not be implemented in `battler`.

You MAY fix such bugs by creating a plan artifact to fix the bug and waiting for user approval.

## Step 1: Gather Requirements

1. Ask the user for the following:
    1. The specific effect that must be tested.
    2. Any specific tests scenarios that should be included.

## Step 2: Understand the Effect

1. Locate the JSON effect definition in `battle-data`.
2. Read the effect definition. Pay particular attention to the fxlang effect and any condition. Understand the triggering events and any edge conditions.
3. If necessary and if the effect is an existing Pokémon mechanic, consult background knowledge and online resources such as Bulbapedia for the effect.
4. Identify similar effects that have already been implemented and tested. Use their tests as a reference for how to test this effect.

## Step 3: Plan the Test

1. Create a plan artifact with the following:
    1. A short summary of your understanding of the effect.
    2. The test cases to be written. You SHOULD consider your understanding of the effect (collected in Step 2) in generating test cases.
    3. Pokémon that will be used for the teams in the test.
    4. (Optional) Existing tests used as a template, if any.
2. Wait for user approval of the plan.

## Step 4: Write the Test

1. Write the test following your plan.
2. Ensure the test passes using `cargo test`.

## Step 5: Verify Code

1. Verify your code is clean and follows Rust best practices.
2. Remove any discussion comments from development.
3. Remove any comments that are obvious based on the surrounding code.