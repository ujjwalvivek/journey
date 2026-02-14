# Quick Start

### Running the Engine

**Native (Desktop)**
- Press `Ctrl+Shift+B` (or `Cmd+Shift+B` on Mac) → Select "Run Native (Debug)"
- For maximum performance: Run task "Run Native (Release)"

**Web (Browser)**
- Run task "Run Web Dev Server" (builds WASM, starts Vite)
- Open browser to `http://localhost:5173`
- For faster iteration (skip WASM rebuild): "Quick Web Dev (skip WASM rebuild)"

### Building

- **Full rebuild**: Run task "Build All (Native + WASM + Web)"
- **Native only**: `Ctrl+Shift+B` → "Build Native"
- **WASM only**: Run task "Build WASM"

### Testing & Quality

- **Run tests**: Run task "Run Tests"
- **Clippy lints**: Run task "Clippy (All)"

## Keyboard Shortcuts

1. **Run Tasks**: `Ctrl+Shift+P` → type "Tasks: Run Task"
2. **Default Build Task**: `Ctrl+Shift+B` (runs native debug by default)
3. **Debug**: `F5` (launches debugger for native binary)