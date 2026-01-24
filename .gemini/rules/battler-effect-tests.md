This section describes rules that ALWAYS must be followed when writing effect tests for battler. Exceptions to these rules ALWAYS require ADDITIONAL user approval.

1. ALWAYS match the style and organization of existing tests. Function order MUST be: all team functions, battle builder functions, test cases.
2. ALWAYS put as much common functionality into the battle builder function (`make_battle`) as possible. The battle builder function MUST return `Result<PublicCoreBattle<'static>>`. This function MUST take in teams as input. You MAY optionally pass battle type and seed ONLY if they are dynamic across multiple tests (otherwise, they should be hard-coded).
3. Tests always end in `_test.rs` and are organized according to the existing module structure. NEVER deviate from the existing module structure for new test files.
4. Test cases are ALWAYS based on the specialized behavior of the effect itself. Focus on testing what makes the effect unique. Focus on edge cases that are identifiable in the fxlang code (JSON) itself. Do not introduce unnecessary complexity unrelated to the test itself.
5. NEVER access battle engine internals such as context objects.
6. NEVER use or include other effects (e.g., moves, abilities, items) unless necessary (for example, using a simple damaging move when a Mon needs to take damage). ALWAYS use `No Ability` when the ability of a Mon does not matter. ALWAYS use an empty moves array when the Mon does not need to use any moves. NEVER attach items, IVs, EVs, or other properties to Mons if they have no effect on the test. ALWAYS remove unused effects (especially moves and abilities). If a move is in the team but not used in the test, it MUST be removed. It is OK for moves list to be empty.
7. When using a mirror match, it is OK for some moves to go unused on some Mons, as long as OTHER Mons use those moves. For example, Mon A and B have two moves; it is acceptable for Mon A to only use the first move and Mon B to only use the second move.
8. Test battles ALWAYS go only as long as required for testing, which is often only one turn.
9. ALWAYS use `pass` actions as much as possible (passes must be explicitly allowed on the `TestBattleBuilder`). DO NOT use moves unless absolutely necessary. ALWAYS scrutinize moves to ensure they are necessary for the test. NEVER use Splash to pass a turn.
10. ALWAYS minimize the number of turns to test. If multiple actions can happen in the same turn, they are ALWAYS combined.
11. ALWAYS verify battle state ONLY using the public battle log. NEVER attempt to access battle state directly in tests. NEVER use negative assertions for battle logs.
12. NEVER verify player data or request options unless the user explicitly instructs you to. If you think such verification is needed, you ALWAYS require additional user approval.
13. ALWAYS verify the error message of a `Result::Err`, ESPECIALLY if it is an expected error of `make_choice`.
14. ALWAYS use `assert_logs_since_turn_eq` and `Vec<LogMatch>` to verify the battle log. NEVER use custom log matching solutions. NEVER use regex for log matching. ALWAYS define an intermediate variable `expected_logs` for the `Vec<LogMatch>` passed into the log assertion. ALWAYS use `serde_json` to define `expected_logs` with a JSON array (reference other tests as an example).
15. NEVER use substring matches, except for "switch" logs.
16. ALWAYS define teams, log expectations, and other complex battle objects with `serde_json::from_str`. ALWAYS qualify `serde_json::from_str`. String formatting and indentation should match that of other effect tests: `r#"{` and `}"#` must be on their own indented lines, and inner JSON contents must be additionally indented.
17. ALWAYS combine log matches into a single assertion at the end of the test. NEVER run multiple turns and verify logs in between. ALWAYS run all turns back-to-back and verify all logs at the end.
18. NEVER focus on low-level intricacies, such as precise damage calculations. ALWAYS trust that the core battle engine is correct for core features unrelated to the special effect being tested. ALWAYS focus on the high-level effect under test. DO NOT estimate damage calculations yourself.
19. NEVER estimate damage calculations. ALWAYS visibly verify damage calculations with the battle log.
20. NEVER heal a Mon unnecessarily if you are not testing damage calculations or if the Mon is at no risk of fainting during the test.
21. When testing damage calculations, ALWAYS heal Mons (e.g., with the move Recover, or item Max Potion) to prevent fainting if the damage calculation is the critical portion of the test. NEVER allow a Mon to faint when testing damage calculation modifiers, unless the faint is caused by the very last damage calculation in the test.
22. NEVER include a Mon solely for the purpose of surviving multiple hits. You SHOULD use healing effects instead (see above).
23. NEVER include EVs or IVs only for the purpose of making a Mon survive hits. Use a different species instead.
24. ALWAYS reuse MINIMAL teams. Use as few teams and Mons as possible, where ALL Mons play an active and important role in the battle. ALWAYS use a mirror match, unless a different Mon is EXPLICITLY REQUIRED and APPROVED by the user. ALWAYS combine mirror matches into a single team for simplicity.
25. If a specific move, item, or ability must be variable, ALWAYS use a single team and just set the field dynamically on the team JSON before starting the battle. KEEP THINGS SIMPLE. Just set the member directly on the mutable TeamData variable.
26. ALWAYS use one function per team.
27. You do not need to adhere to team validation. You MAY turn team validation off.
28. ALWAYS use at least one Pokémon that was introduced in the same generation as the effect being tested for variety. This Pokémon SHOULD make logical sense for the effect under test (e.g., a Pokémon with the same type as the move, or a Pokémon that naturally has the ability).
29. ALWAYS name Mons with the exact same name as their species. NEVER use unique names.
30. ALWAYS use a set seed (often `0`) to prevent minor RNG differences from failing tests. You MAY use controlled RNG for highly complex scenarios. When using a set seed, you DO NOT need to set base damage randomization if damage values are visibly different.
31. ALWAYS use `player-1` and `player-2` for test battles. ALWAYS define a `make_battle` method that uses TestBattleBuilder internally.
32. ALWAYS use properly capitalized names for effects, such as moves, abilities, and items, in team definitions.

## Team Verification Checklist

Before finalizing any plan or code involving team creation, you MUST explicitly verify the following for EACH Mon in the team:

1.  **Moves:** Is every move listed actually used in the test? (If no, delete it).
2.  **Abilities:** Is the ability `No Ability`? If not, is it essential for the test? (If not essential, change to `No Ability`).
3.  **Items:** Is there an item? Is it essential? (If no, delete it).
4.  **EVs/IVs:** Are there EVs or IVs? (If yes, DELETE THEM. If survival is an issue, change the Mon species or level).
5.  **Species:** Is the species from the correct generation?
