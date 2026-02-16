# Development Workflow

`Awaiting updates... Read more in docd/journey.md`

## Native Development

1. Make Rust changes in `engine/` or `game/`
2. Press `Ctrl+Shift+B` to build + run
3. Window opens automatically

## Web Development

**Full rebuild (Rust + TS changes):**

1. Make changes in `engine/` or `web/src/`
2. Run task "Run Web Dev Server"
3. Vite will rebuild WASM + serve at localhost:5173

**Quick iteration (TS/HTML only):**

1. Make changes in `web/src/` (no Rust changes)
2. Run task "Quick Web Dev (skip WASM rebuild)"
3. Vite hot-reloads instantly

## Tips

- Rust Analyzer will run clippy automatically on save.
- Format on save is enabled for both Rust and TypeScript.
- `target/`, `node_modules/`, `dist/`, and `Cargo.lock` are hidden from file explorer.
- The default task (`Ctrl+Shift+B`) is "Run Native (Debug)" which is fastest for iteration.

## Cloning to Windows

1. Clone the repo
2. Install Rust, Node.js, and wasm-pack
3. Press `Ctrl+Shift+B` → "Run Native (Release)"
4. Enjoy!
