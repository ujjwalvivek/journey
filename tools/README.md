# Release and Versioning Automation Tool

Go tooling for release automation and version management.
Two independent modules. No shared code between them.

## Quick Start

From the repo root:

```makefile
make release           # TUI interactive release
make release-dry       # dry-run: show plan, execute nothing
make release-skip-ci   # skip CI monitoring after pipeline
make release-headless  # full headless (CI mode)
make version           # bump Cargo.toml version
```

Or directly:

```go
go run ./tools/release [flags]
go run ./tools/versioning
```

## tools/release

Full release pipeline: preflight checks → version selection → approval mode → git operations → CI monitoring → summary.

### Flags

| Flag           | Default | Description                                                      |
| -------------- | ------- | ---------------------------------------------------------------- |
| `--headless`   | `false` | Run without TUI (CI/scripting)                                   |
| `--version`    | auto    | Semver to publish (headless only, defaults to patch bump)        |
| `--mode`       | `zen`   | Approval mode: `zen` (auto-approve) or `prompt` (step-by-step)   |
| `--ci-timeout` | `10m`   | Max wait for GitHub Actions workflow                             |
| `--dry-run`    | `false` | Show pipeline plan without executing any commands                |
| `--skip-ci`    | `false` | Skip CI monitoring after pipeline completes                      |
| `--dirty`      | `false` | Allow release from a dirty working tree (skips clean-tree check) |

### Architecture

Elm architecture via [Bubble Tea](https://github.com/charmbracelet/bubbletea). Single `model` struct, message-driven state machine.

**Screens**: Loading → Welcome → Version → Mode → Execution → CI → Summary

**Key files** (19 source, 7 test):

| File              | Purpose                                                |
| ----------------- | ------------------------------------------------------ |
| `main.go`         | Entrypoint, flag parsing, headless path                |
| `tui.go`          | Model definition, `Init`/`Update`/`View` dispatcher    |
| `tui_*.go`        | Per-screen update/view logic (8 files)                 |
| `types.go`        | Domain types: steps, results, contexts, stats          |
| `constants.go`    | Named constants (timeouts, branch names, limits)       |
| `executor.go`     | `CommandExecutor` interface + `RealExecutor` (os/exec) |
| `orchestrator.go` | `executePipeline` (headless runner), `shouldRunStep`   |
| `pipeline.go`     | `buildCommandPipeline` (git/cargo step definitions)    |
| `repo.go`         | Git/Cargo.toml operations, preflight checks            |
| `semver.go`       | Version parsing, normalization, option derivation      |
| `actions.go`      | `monitorGitHubActions` (gh CLI polling)                |
| `stats.go`        | Post-release stats calculation, diff collection        |

### Receiver Mutation Pattern

Bubble Tea passes `model` by value. Goroutines that modify model state (channel creation, flags) must have those mutations done in `Update()` before launching the goroutine. The goroutine receives channels as parameters, never reads/writes model fields.

```go
//? Correct: mutations in Update, goroutine gets channel param
m.ciEvents = make(chan any, 1024)
m.ciRunning = true
go m.runCIWatcher(m.ciEvents)

//? Wrong: mutations inside a method called as tea.Cmd
func (m model) startCIWatcher() tea.Cmd {
    m.ciRunning = true  //? lost, m is a copy
}
```

### Testing

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

Single file (`main.go`, 137 lines) + tests (`main_test.go`, 75 lines). Uses `pelletier/go-toml/v2` for TOML parsing.

## Dependencies

### release

- `github.com/charmbracelet/bubbletea`: TUI framework
- `github.com/charmbracelet/bubbles`: spinner, textinput components
- `github.com/charmbracelet/lipgloss`: terminal styling
- `golang.org/x/mod/semver`: semver validation

### versioning

- `github.com/pelletier/go-toml/v2`: TOML read/write

### External CLI tools (runtime)

- `git`: all repository operations
- `cargo`: workspace metadata
- `gh`: GitHub Actions monitoring (authenticated)
- `make`: for running from the root Makefile, not a hard dependency

## Module Structure

```bash
tools/
├── release/          # go module: release
│   ├── go.mod
│   ├── *.go          (19 source files, ~2100 lines)
│   └── *_test.go     (7 test files, ~400 lines)
├── versioning/       # go module: versioning
│   ├── go.mod
│   ├── main.go
│   └── main_test.go
└── README.md
```

## License

MIT License. See [LICENSE](LICENSE) for details.
