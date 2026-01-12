---
trigger: always_on
---

# Rust

This rule describes the REQUIRED code style for Rust.

## Variables

ALWAYS remove intermediate variables that are defined once and used once, EXCEPT in cases where it improves readability (i.e., the variable is a multi-line definition). ALWAYS inline intermediate variables with a single line definition.

## Tests

Use `assert_matches::assert_matches!` as much as possible for method calls that return a `Result`. Only use `unwrap` when the value must be stored for later parts of the test. NEVER import `assert_matches::assert_matches!`. ALWAYS fully qualify `assert_matches::assert_matches!`.

ALWAYS fully qualify `pretty_assertions::assert_eq!`.

## Comments

Comments ALWAYS explain WHY not WHAT.

All public methods ALWAYS have a rustdoc comment.

Code comments are ALWAYS as short as reasonably possible. If the code can reasonably be understood WITHOUT comments, then the comments MUST be removed. Complex details about a test SHOULD be commented.

If you make code changes, ALWAYS verify surrounding comments are still valid. ALWAYS update comments, ensuring they are true.
