package main

import (
	"context"
	"flag"
	"fmt"
	"os"
	"time"

	tea "github.com/charmbracelet/bubbletea"
)

func run() error {
	ciTimeout := flag.Duration("ci-timeout", defaultCITimeout, "max wait for CI workflow")
	headless := flag.Bool("headless", false, "run without TUI")
	version := flag.String("version", "", "version to publish (semver)")
	modeFlag := flag.String("mode", "zen", "approval mode: prompt or zen")
	dryRun := flag.Bool("dry-run", false, "show pipeline plan without executing")
	skipCI := flag.Bool("skip-ci", false, "skip CI monitoring after pipeline")
	dirty := flag.Bool("dirty", false, "allow release from a dirty working tree")
	flag.Parse()

	if *headless {
		if *version == "" {
			ctx, _, err := runPreflight(context.Background(), NewRealExecutor(), *ciTimeout)
			if err != nil {
				return err
			}
			opts, err := deriveVersionOptions(ctx.CurrentVersion)
			if err != nil {
				return err
			}
			*version = opts.Patch
		}
		mode := ApprovalZen
		if *modeFlag == string(ApprovalPrompt) {
			mode = ApprovalPrompt
		}
		return runHeadless(*version, mode, *ciTimeout, *dryRun, *skipCI, *dirty)
	}

	if flag.NArg() != 0 {
		return fmt.Errorf("usage: go run ./tools/release [flags]")
	}

	m := newModel(*ciTimeout)
	m.skipCI = *skipCI
	m.dryRun = *dryRun
	m.allowDirty = *dirty
	program := tea.NewProgram(m, tea.WithAltScreen())
	finalModel, err := program.Run()
	if err != nil {
		return err
	}

	mm, ok := finalModel.(model)
	if !ok {
		return fmt.Errorf("unexpected model type")
	}
	if mm.exitErr != nil {
		return mm.exitErr
	}
	return nil
}

func runHeadlessWithExecutor(exec CommandExecutor, version string, mode ApprovalMode, ciTimeout time.Duration, dryRun, skipCI, allowDirty bool) error {
	ctx, checks, err := runPreflight(context.Background(), exec, ciTimeout)
	if err != nil {
		return err
	}
	if !preflightOKWith(checks, allowDirty) {
		for _, c := range checks {
			if !c.OK {
				fmt.Fprintf(os.Stderr, "preflight failed: %s - %s\n", c.Name, c.Detail)
			}
		}
		return fmt.Errorf("preflight failed")
	}

	normalized, tag, err := normalizeVersionInput(version)
	if err != nil {
		return err
	}

	steps, err := buildCommandPipeline(ctx.StartBranch, normalized, tag)
	if err != nil {
		return err
	}

	if dryRun {
		fmt.Printf("dry run: %s (%s) from %s\n\n", normalized, tag, ctx.StartBranch)
		for i, step := range steps {
			fmt.Printf("  %2d. %-35s %s\n", i+1, step.Name, step.Display())
		}
		fmt.Printf("\n%d steps planned. no commands executed.\n", len(steps))
		return nil
	}

	results, pipelineErr := executePipeline(context.Background(), PipelineOptions{
		RepoRoot: ctx.RepoRoot,
		Steps:    steps,
		Mode:     mode,
		Executor: exec,
	})
	if pipelineErr != nil {
		return fmt.Errorf("pipeline failed: %w", pipelineErr)
	}

	fmt.Printf("pipeline: %d steps completed\n", len(results))

	if skipCI {
		fmt.Println("skipping CI monitoring (--skip-ci)")
		return nil
	}

	ciInfo, ciErr := monitorGitHubActions(context.Background(), exec, ctx.RepoRoot, tag, ciTimeout, defaultCIWatchPollInterval, nil, nil)
	if ciErr != nil {
		fmt.Fprintf(os.Stderr, "ci: %v\n", ciErr)
	}

	if ciInfo.ID != 0 {
		fmt.Printf("ci: %s conclusion=%s url=%s\n", ciInfo.Name, ciInfo.Conclusion, ciInfo.URL)
	}

	if ciErr != nil && !ciInfo.TimedOut {
		return fmt.Errorf("ci failed: %w", ciErr)
	}
	if ciErr != nil && ciInfo.TimedOut {
		fmt.Fprintf(os.Stderr, "warning: CI monitoring timed out; pipeline completed successfully\n")
	}

	return nil
}

func runHeadless(version string, mode ApprovalMode, ciTimeout time.Duration, dryRun, skipCI, allowDirty bool) error {
	return runHeadlessWithExecutor(NewRealExecutor(), version, mode, ciTimeout, dryRun, skipCI, allowDirty)
}

func main() {
	if err := run(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
