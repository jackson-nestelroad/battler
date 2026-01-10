---
trigger: always_on
---

# Rust

This rule describes the REQUIRED code style for Rust.

## Tests

Use `assert_matches::assert_matches!` as much as possible for method calls that return a `Result`. Only use `unwrap` when the value must be stored for later parts of the test.

## Comments

Keep code comments as short as reasonably possible. If the code can reasonably be understood WITHOUT comments, then the comments SHOULD be removed. Complex details about a test SHOULD be commented.

You MAY use long, explanatory comments in iterative development, but these comments MUST be removed before completing the task.
