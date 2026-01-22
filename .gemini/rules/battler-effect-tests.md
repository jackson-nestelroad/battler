This section describes rules that ALWAYS must be followed when writing effect tests for battler. Exceptions to these rules ALWAYS require ADDITIONAL user approval.

1. ALWAYS match the style and organization of existing tests. Function order MUST be: all team functions, battle builder functions, test cases.
2. Tests always end in `_test.rs` and are organized according to the existing module structure. NEVER deviate from the existing module structure for new test files.
3. Test cases are ALWAYS based on the specialized behavior of the effect itself. Focus on testing what makes the effect unique. Focus on edge cases that are identifiable in the fxlang code (JSON) itself. Do not introduce unnecessary complexity unrelated to the test itself.
4. NEVER use or include other effects (e.g., moves, abilities, items) unless necessary (for example, using a simple damaging move when a Mon needs to take damage). ALWAYS use `No Ability` when the ability of a Mon does not matter. NEVER attach items, IVs, EVs, or other properties to Mons if they have no effect on the test. ALWAYS remove unused effects (especially moves and abilities).
5. When using a mirror match, it is OK for some moves to go unused on some Mons, as long as OTHER Mons use those moves. For example, Mon A and B have two moves; it is acceptable for Mon A to only use the first move and Mon B to only use the second move.
6. Test battles ALWAYS go only as long as required for testing, which is often only one turn.
7. ALWAYS use `pass` actions as much as possible (passes must be explicitly allowed on the `TestBattleBuilder`). DO NOT use moves unless absolutely necessary. ALWAYS scrutinize moves to ensure they are necessary for the test. NEVER use Splash to pass a turn.
8. ALWAYS minimize the number of turns to test. If multiple actions can happen in the same turn, they are ALWAYS combined.
9. ALWAYS verify battle state ONLY using the public battle log. NEVER attempt to access battle state directly in tests. NEVER use negative assertions for battle logs.
10. NEVER verify player data or request options unless the user explicitly instructs you to. If you think such verification is needed, you ALWAYS require additional user approval.
11. ALWAYS verify the error message of a `Result::Err`, ESPECIALLY if it is an expected error of `make_choice`.
12. ALWAYS use `assert_logs_since_turn_eq` and `Vec<LogMatch>` to verify the battle log. NEVER use custom log matching solutions. NEVER use regex for log matching. ALWAYS define an intermediate variable `expected_logs` for the `Vec<LogMatch>` passed into the log assertion. ALWAYS use `serde_json` to define `expected_logs` with a JSON array (reference other tests as an example).
13. ALWAYS define teams, log expectations, and other complex battle objects with `serde_json::from_str`. ALWAYS qualify `serde_json::from_str`. String formatting and indentation should match that of other effect tests: `r#"{` and `}"#` must be on their own indented lines, and inner JSON contents must be additionally indented.
14. ALWAYS combine log matches into a single assertion at the end of the test. NEVER run multiple turns and verify logs in between. ALWAYS run all turns back-to-back and verify all logs at the end.
15. NEVER focus on low-level intricacies, such as precise damage calculations. ALWAYS trust that the core battle engine is correct for core features unrelated to the special effect being tested. ALWAYS focus on the high-level effect under test. DO NOT estimate damage calculations yourself.
16. NEVER heal a Mon unnecessarily if you are not testing damage calculations or if the Mon is at no risk of fainting during the test.
17. When testing damage calculations, ALWAYS heal Mons (e.g., with the move Recover, or item Max Potion) to prevent fainting if the damage calculation is the critical portion of the test. NEVER allow a Mon to faint when testing damage calculation modifiers, unless the faint is caused by the very last damage calculation in the test.
18. NEVER include a Mon solely for the purpose of surviving multiple hits. You SHOULD use healing effects instead (see above).
19. NEVER include EVs or IVs only for the purpose of making a Mon survive hits. Use a different species instead.
20. ALWAYS reuse MINIMAL teams. Use as few teams and Mons as possible, where ALL Mons play an active and important role in the battle. ALWAYS use a mirror match, unless a different Mon is EXPLICITLY REQUIRED and APPROVED by the user. ALWAYS combine mirror matches into a single team for simplicity.
21. ALWAYS use one function per team.
22. You do not need to adhere to team validation. You MAY turn team validation off.
23. ALWAYS use at least one Pokémon that was introduced in the same generation as the effect being tested for variety. This Pokémon SHOULD make logical sense for the effect under test (e.g., a Pokémon with the same type as the move, or a Pokémon that naturally has the ability).
24. ALWAYS name Mons with the exact same name as their species. NEVER use unique names.
25. ALWAYS use a set seed (often `0`) to prevent minor RNG differences from failing tests. You MAY use controlled RNG for highly complex scenarios. You MAY use max or min base damage randomization (you do not need to if damage numbers do not matter in the test).
26. ALWAYS use `player-1` and `player-2` for test battles. ALWAYS define a `make_battle` method that uses TestBattleBuilder internally.
27. ALWAYS use properly capitalized names for effects, such as moves, abilities, and items, in team definitions.
