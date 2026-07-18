# Battler UI Development & Styling Guide

This guide establishes the rules and conventions for styling the `battler-web-app` UI prototype. All human contributors and AI agents must follow these rules to ensure consistency, simplicity, maintainability, and clean code separation.

---

## 1. Variable-Driven Styling (No Hardcoding)

*   **No Hex Colors in Modules**: Never hardcode colors (e.g., `#ffffff`, `#0a0d14`, `red`) inside component modules (`*.module.scss`). All colors must use global custom properties defined in [index.scss](src/index.scss):
    *   *Correct*: `background-color: var(--bg-card);`
    *   *Incorrect*: `background-color: #161c2b;`
*   **Spacing and Padding**: Use the unified rem-based tokens for all margins, padding, and layout offsets:
    *   Tokens: `var(--spacing-xxs)` (2px), `var(--spacing-xs)` (4px), `var(--spacing-s)` (8px), `var(--spacing-m)` (16px), `var(--spacing-l)` (20px), `var(--spacing-xl)` (24px), `var(--spacing-xxl)` (32px).
*   **Font Sizes**: Use standard typography tokens, which automatically resize responsively at small viewport widths:
    *   Tokens: `var(--font-size-xs)` (12px), `var(--font-size-s)` (14px), `var(--font-size-m)` (16px), `var(--font-size-l)` (18px), `var(--font-size-xl)` (20px), `var(--font-size-xxl)` (28px).
*   **Border Radius**: Use standard border radius values:
    *   Tokens: `var(--border-radius-xs)` (0.25rem), `var(--border-radius-s)` (0.375rem), `var(--border-radius-m)` (0.5rem), `var(--border-radius-l)` (0.75rem), `var(--border-radius-round)` (50%).

---

## 2. Encapsulation vs. Layout Utilities

To keep styles DRY (Don't Repeat Yourself) and KISS (Keep It Simple, Stupid), balance local stylesheets and global utility classes:

### Reusable/Self-Contained Components
For standalone UI widgets (like `MonCard`, `HpBar`, or `JsonEditor`), all interior structure (gaps, flex alignment, borders) must live inside the component's `.module.scss` file:
*   This makes components "plug-and-play" without requiring class list concatenation in parent views.
*   Example:
    ```scss
    /* MonCard.module.scss */
    .teamSummaryCard {
      display: flex;
      flex-direction: column;
      gap: var(--spacing-xs);
    }
    ```

### Generic Structural Layouts
For simple flex alignment, columns, rows, button lines, lists, or form layout fields, avoid writing empty wrappers in SCSS modules. Use the global utility helper classes in your JSX:
*   Classes: `flex-col`, `flex-row`, `gap-xs`, `gap-s`, `gap-m`, `gap-l`, `gap-xl`, `align-center`, `justify-between`, `w-full`, `flex-1`.
*   Example:
    ```tsx
    <div className="flex-row gap-m align-center">
      <button className="btn btn-primary">Confirm</button>
      <button className="btn btn-secondary">Cancel</button>
    </div>
    ```

### Responsive Utility Classes
To avoid duplicate media query blocks, use responsive layout utilities:
*   `.flex-tablet-col` / `.flex-tablet-row` (switches direction at/below `64rem` tablet width).
*   `.flex-mobile-col` / `.flex-mobile-row` (switches direction at/below `48rem` mobile width).

---

## 3. Standard Design Elements

Leverage global classes from [index.scss](src/index.scss) to keep components standardized:
*   **Containers**: Wrap panels in `.card` and use `.card-header` for standard header layouts.
*   **Buttons**: Always use `.btn` paired with modifiers (`.btn-primary`, `.btn-secondary`, `.btn-success`, `.btn-danger`, `.btn-sm`).
*   **Banners**: Standard alert states use `.alert` paired with types (`.alert-danger`, `.alert-warning`, `.alert-success`, `.alert-info`).
*   **Status Badges**: Mon status conditions (SLP, BRN, PSN, etc.) should use `.status-badge` coupled with status classes (`.brn`, `.par`, `.psn`).

---

## 4. Maintenance Rule: Dead Code Elimination
Never leave unused stylesheets, class declarations, or old variables behind when refactoring. Clean them up proactively to prevent styling bloat.
