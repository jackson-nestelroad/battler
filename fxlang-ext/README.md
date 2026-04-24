# fxlang-vscode

Visual Studio Code extension for `fxlang`, the battle effect DSL used in the `battler` engine.

## Features

- **Syntax Highlighting**: High-precision, context-aware syntax highlighting for `fxlang` code.
- **Precision Injection**: Automatically identifies `fxlang` code blocks within JSON, while isolating metadata fields (like `priority` or `order`) to maintain theme-native JSON highlighting.
- **IntelliSense**: Auto-completion for functions, variables, and members.
- **Hovers**: View documentation for functions and members directly from the source code.
- **Always Up-to-Date**: Includes a scraper tool to sync metadata with the Rust source code.

## Requirements

- VS Code 1.80.0 or higher.

## Development

To build the extension:

1. `cd fxlang-ext`
2. `npm install`
3. `npm run compile`

To run the metadata scraper:

```bash
python3 tools/scrape-metadata.py
```

## License

MIT
