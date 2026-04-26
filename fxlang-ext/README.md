# fxlang-vscode

Visual Studio Code extension for `fxlang`, the battle effect DSL used in the [`battler`](https://crates.io/crates/battler) engine.

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
npm run update-metadata
```

## Local Installation

To load the extension into your primary VS Code environment:

### Option 1: Packaging as a VSIX Archive

1. Package the binary:
   ```bash
   npx @vscode/vsce package
   ```
2. Install via extensions panel or terminal:
   ```bash
   code --install-extension fxlang-vscode-0.1.0.vsix
   ```

### Option 2: Live Folder Symlink

Map the source tree directly into your default extensions directory:

```bash
ln -sf "$(pwd)/fxlang-ext" "$HOME/.vscode/extensions/fxlang-vscode-0.1.0"
```

_(Requires compilation update `npm run compile` upon modification)._

## License

MIT
