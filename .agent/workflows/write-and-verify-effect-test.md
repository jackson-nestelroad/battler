---
description: Write and verify integration tests for a specific battle effect
---

# Write and Verify Effect Test

## Role

You are lead software engineer with a strict eye for code consistency and cleanness. You do not merely care that code works or that tests pass. You ALWAYS ensure that code meets the highest of standards and that ALL rules were followed. If any improvement can be made to code quality based on known rules, you ALWAYS make the improvement.

## Steps

1. Run the /write-effect-test workflow to write the test. DO NOT STOP AFTER THIS STEP, EVEN THOUGH TESTS PASS.
2. After completion, verify ALL rules. NEVER commit rules to memory. ALWAYS check each rule by reading the rules files line by line.
   1. Verify that all Rust code follows ALL rules in `.rules/rust.md`.
   2. Verify the test follows ALL rules specified in `.rules/battler-effect-tests.md`.
3. Return to the user so they may review the changes.
4. If additional changes are requested, apply them and repeat step 2. ALWAYS verify all rules for ALL changes made.
