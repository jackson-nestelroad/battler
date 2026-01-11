---
trigger: model_decision
description: Applies when writing effect tests for battler
---

# battler Effect Test Rules

This rule describes rules that MUST be followed when writing effect tests for battler. Exceptions to these rules ALWAYS require ADDITIONAL user approval.

1. Test cases MUST be based on the specialized behavior of the effect itself. Focus on testing what makes the effect unique. Focus on edge cases that are identifiable in the fxlang code (JSON) itself. Do not introduce unnecessary complexity unrelated to the test itself.
2. NEVER use other effects (e.g., moves, abilities, items) unless necessary (for example, using a simple damaging move when a Mon needs to take damage). ALWAYS use `No Ability` when the ability of a Mon does not matter. NEVER attach items, IVs, or EVs to Mons if they have no effect on the test.
3. Test battles MUST only go as long as required for testing, which is often only one turn.
4. ALWAYS use `pass` actions as much as possible. DO NOT use moves unless absolutely necessary. ALWAYS scrutinize moves to ensure they are necessary for the test.
5. ALWAYS minimize the number of turns to test. If multiple actions can happen in the same turn, they MUST be combined.
6. ALWAYS verify battle state ONLY using the public battle log. NEVER attempt to access battle state directly in tests. NEVER use negative assertions for battle logs.
7. ALWAYS verify the error message of a `Result::Err`, ESPECIALLY if it is an expected error of `make_choice`.
8. ALWAYS use `Vec<LogMatch>` to verify the battle log.
9. ALWAYS combine log matches as much as possible. NEVER run multiple turns and verify logs in between. You MUST run all turns back-to-back and verify all logs at the end.
10. NEVER focus on low-level intricacies, such as precise damage calculations. You MUST trust that the core battle engine is correct for core features unrelated to the special effect being tested. ALWAYS focus on the high-level effect under test. DO NOT estimate damage calculations yourself.
11. ALWAYS heal Mons (e.g., with Max Potion) to prevent fainting if the damage calculation is the critical portion of the test.
12. ALWAYS reuse MINIMAL teams. Use as few teams and Mons as possible. ALWAYS use a mirror match, unless a different Mon is EXPLICITLY REQUIRED and APPROVED by the user.
13. ALWAYS use at least one Pokémon that was introduced in the same generation as the effect being tested for variety. This Pokémon SHOULD make logical sense for the effect under test (e.g., a Pokémon with the same type as the move, or a Pokémon that naturally has the ability).
14. ALWAYS use a set seed (often `0`) to prevent minor RNG differences from failing tests. You MAY use controlled RNG for highly complex scenarios.
15. ALWAYS use `player-1` and `player-2` for test battles. ALWAYS define a `make_battle` method that uses TestBattleBuilder internally.
