# Workspace Agent Instructions

When working in this repository—specifically inside the `battler-web-app` project directory (`js-clients/battler-web-app/`)—all agents must strictly follow and enforce the layout, styling, and DRY rules outlined in the style guide:

👉 **[STYLEGUIDE.md](file:///Users/jackson/Code/GitHub/pokemon/js-clients/battler-web-app/STYLEGUIDE.md)**

### Key Directives for Styling Tasks
1.  **Strict Variable Binding**: No raw hex values, sizes in pixels, or hardcoded font sizes are allowed in custom stylesheets. Use CSS properties defined in `index.scss`.
2.  **Encapsulation vs. Layout Utilities**: 
    *   Place flex and gap declarations inside `.module.scss` stylesheets *only* for self-contained components (e.g. cards, status rows, inputs).
    *   Use global layout utility classes (`flex-col`, `flex-row`, `gap-*`, etc.) in TSX markup for simple wrappers, listings, and form structures.
3.  **Refactoring & Cleaning**: Always clean up unused classes, variables, and properties when refactoring.
