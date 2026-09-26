# Workspace Agent Instructions

When working in this repository—specifically inside the `battler-web-app` project directory (`js-clients/battler-web-app/`)—all agents must strictly follow and enforce the layout, styling, and DRY rules outlined in the style guide:

👉 **[STYLEGUIDE.md](file:///Users/jackson/Code/GitHub/pokemon/js-clients/battler-web-app/STYLEGUIDE.md)**

### Key Directives for Styling Tasks
1.  **Strict Variable Binding**: No raw hex values, sizes in pixels, or hardcoded font sizes are allowed in custom stylesheets. Use CSS properties defined in `index.scss`.
2.  **Encapsulation vs. Layout Utilities**: 
    *   Place flex and gap declarations inside `.module.scss` stylesheets *only* for self-contained components (e.g. cards, status rows, inputs).
    *   Use global layout utility classes (`flex-col`, `flex-row`, `gap-*`, etc.) in TSX markup for simple wrappers, listings, and form structures.
3.  **Refactoring & Cleaning**: Always clean up unused classes, variables, and properties when refactoring.

### Key Directives for Engine, Calculator & Trainer AI Tasks
1.  **Repository Terminology**: Strictly use "Mon" or "Mons". Never use "Pokémon" or "Pokemon" in code, comments, docstrings, or test descriptions.
2.  **Phase Isolation & Separation of Concerns**: Schema crates are pure schemas (types, enums, structs, traits, serde only). Never add unnecessary code or logic that belongs to subsequent phases or is unused.
3.  **No Loose Booleans**: Avoid boolean sprawl across structs. Use typed flag enums (`HashSet<...Flag>`) for extensibility and consistency with `battler-data::MoveFlag`. Unify identical domain concepts across schemas (e.g. `type_immunities` across items and abilities).
4.  **Accurate Domain Naming**: Ensure files and symbols accurately reflect their domain responsibility (e.g. `manifest_store.rs` rather than `data_store.rs`).
5.  **Strict Alphabetical Ordering**: Keep all `Cargo.toml` dependencies, workspace members, `mod` statements, and `pub use` statements in strict alphabetical order.
6.  **Explicit Review Gates**: Stop immediately after finishing a phase and do not proceed to the next phase without explicit user approval.

