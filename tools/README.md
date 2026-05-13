# Journey Engine Tools

A collection of internal utilities for the Journey Engine, including Go tooling for release automation and Rust tools for procedural audio.

## Quick Start

From the repo root:

```makefile
make release           # TUI interactive release
make release-dry       # dry-run: show plan, no execution
make release-skip-ci   # skip CI monitoring after pipeline
make release-headless  # full headless (CI mode)
make version           # bump Cargo.toml version
make audio             # build & run audio tools
```

Or directly:

```go
go run ./tools/release [flags]
go run ./tools/versioning
```

## tools/release

Full release pipeline: preflight checks → version selection → approval mode → git operations → CI monitoring → summary.

| Flag           | Default | Description                                                      |
| -------------- | ------- | ---------------------------------------------------------------- |
| `--headless`   | `false` | Run without TUI (CI/scripting)                                   |
| `--version`    | auto    | Semver to publish (headless only, defaults to patch bump)        |
| `--mode`       | `zen`   | Approval mode: `zen` (auto-approve) or `prompt` (step-by-step)   |
| `--ci-timeout` | `10m`   | Max wait for GitHub Actions workflow                             |
| `--dry-run`    | `false` | Show pipeline plan without executing any commands                |
| `--skip-ci`    | `false` | Skip CI monitoring after pipeline completes                      |
| `--dirty`      | `false` | Allow release from a dirty working tree (skips clean-tree check) |

```cmd
cd tools/release && go test ./... -count=1
```

Tests use `stubExecutor` (see `testutil_test.go`) to mock all shell commands. No network, no git repo, no filesystem access in tests.

Coverage areas: orchestrator flow, pipeline construction, semver parsing, stats calculation, CI JSON parsing, headless mode.

## tools/versioning

Standalone tool. Reads `Cargo.toml`, prompts for bump type (major/minor/patch), writes the new version back.

```cmd
cd tools/versioning && go test ./... -count=1
```

## tools/audio

- **WebAssembly (`web.rs`)**: Exports WASM bindings so the web frontend can load the procedural audio engine into an `AudioWorkletNode`. Built via `wasm-pack`.
- **Terminal UI (`cli.rs`)**: A `ratatui` + `cpal` based native application (`resonance-cli`) for real-time, zero-dependency audio synthesis and sequencing directly in the terminal.

```bash
# Run the terminal synthesizer natively
cargo run -p journey-audio
```

## Dependencies

### release

- `github.com/charmbracelet/bubbletea`: TUI framework
- `github.com/charmbracelet/bubbles`: spinner, textinput components
- `github.com/charmbracelet/lipgloss`: terminal styling
- `golang.org/x/mod/semver`: semver validation

### versioning

- `github.com/pelletier/go-toml/v2`: TOML read/write

### audio (Rust)

- `resonance`, `cadence`: Core primitive crates
- `cpal`: Native cross-platform audio driver
- `ratatui`, `crossterm`: Terminal UI framework
- `wasm-bindgen`: WebAssembly interface

## License

MIT License. See [LICENSE](LICENSE) for details.
