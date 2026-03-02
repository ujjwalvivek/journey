package main

import (
	"context"
	"errors"
	"testing"
)

func TestShouldRunStepPromptApproved(t *testing.T) {
	run, err := shouldRunStep(ApprovalPrompt, true)
	if err != nil {
		t.Fatalf("shouldRunStep returned error: %v", err)
	}
	if !run {
		t.Fatalf("expected command to run")
	}
}

func TestShouldRunStepPromptDenied(t *testing.T) {
	run, err := shouldRunStep(ApprovalPrompt, false)
	if err == nil {
		t.Fatalf("expected denial error")
	}
	if err != ErrCommandDenied {
		t.Fatalf("expected ErrCommandDenied, got %v", err)
	}
	if run {
		t.Fatalf("expected command not to run")
	}
}

func TestShouldRunStepZen(t *testing.T) {
	run, err := shouldRunStep(ApprovalZen, false)
	if err != nil {
		t.Fatalf("shouldRunStep returned error: %v", err)
	}
	if !run {
		t.Fatalf("expected zen mode to auto-approve")
	}
}

func TestExecutePipelineHappyPathStaging(t *testing.T) {
	steps, err := buildCommandPipeline("staging", "0.3.3", "v0.3.3")
	if err != nil {
		t.Fatalf("buildCommandPipeline error: %v", err)
	}

	stub := &stubExecutor{
		runFn: func(ctx context.Context, dir, name string, args []string, onOutput func(string)) (string, error) {
			if onOutput != nil {
				onOutput("ok")
			}
			return "ok", nil
		},
	}

	results, runErr := executePipeline(context.Background(), PipelineOptions{
		RepoRoot: "repo",
		Steps:    steps,
		Mode:     ApprovalZen,
		Executor: stub,
	})
	if runErr != nil {
		t.Fatalf("executePipeline error: %v", runErr)
	}
	if len(results) != len(steps) {
		t.Fatalf("expected %d results, got %d", len(steps), len(results))
	}
	for _, result := range results {
		if result.Status != StepStatusSuccess {
			t.Fatalf("expected success status, got %s", result.Status)
		}
	}
}

func TestExecutePipelineHappyPathMainPrompt(t *testing.T) {
	steps, err := buildCommandPipeline("main", "0.3.3", "v0.3.3")
	if err != nil {
		t.Fatalf("buildCommandPipeline error: %v", err)
	}

	stub := &stubExecutor{
		runFn: func(ctx context.Context, dir, name string, args []string, onOutput func(string)) (string, error) {
			return "ok", nil
		},
	}

	results, runErr := executePipeline(context.Background(), PipelineOptions{
		RepoRoot: "repo",
		Steps:    steps,
		Mode:     ApprovalPrompt,
		ApproveFn: func(CommandStep) bool {
			return true
		},
		Executor: stub,
	})
	if runErr != nil {
		t.Fatalf("executePipeline error: %v", runErr)
	}
	for _, result := range results {
		if result.Status != StepStatusSuccess {
			t.Fatalf("expected success status, got %s", result.Status)
		}
	}
}

func TestExecutePipelineCommandFailureAborts(t *testing.T) {
	steps, err := buildCommandPipeline("staging", "0.3.3", "v0.3.3")
	if err != nil {
		t.Fatalf("buildCommandPipeline error: %v", err)
	}

	stub := &stubExecutor{
		runFn: func(ctx context.Context, dir, name string, args []string, onOutput func(string)) (string, error) {
			if name == "git" && len(args) == 3 && args[0] == "merge" && args[1] == "--ff-only" {
				return "", errors.New("ff-only failed")
			}
			return "ok", nil
		},
	}

	results, runErr := executePipeline(context.Background(), PipelineOptions{
		RepoRoot: "repo",
		Steps:    steps,
		Mode:     ApprovalZen,
		Executor: stub,
	})
	if runErr == nil {
		t.Fatalf("expected pipeline failure")
	}

	foundFailed := false
	for _, result := range results {
		if result.Status == StepStatusFailed {
			foundFailed = true
		}
	}
	if !foundFailed {
		t.Fatalf("expected at least one failed step")
	}
}

func TestExecutePipelineDeniedApprovalAborts(t *testing.T) {
	steps, err := buildCommandPipeline("main", "0.3.3", "v0.3.3")
	if err != nil {
		t.Fatalf("buildCommandPipeline error: %v", err)
	}

	stub := &stubExecutor{
		runFn: func(ctx context.Context, dir, name string, args []string, onOutput func(string)) (string, error) {
			return "ok", nil
		},
	}

	results, runErr := executePipeline(context.Background(), PipelineOptions{
		RepoRoot: "repo",
		Steps:    steps,
		Mode:     ApprovalPrompt,
		ApproveFn: func(CommandStep) bool {
			return false
		},
		Executor: stub,
	})
	if runErr == nil {
		t.Fatalf("expected denial error")
	}
	if !errors.Is(runErr, ErrCommandDenied) {
		t.Fatalf("expected ErrCommandDenied, got %v", runErr)
	}
	if len(results) == 0 || results[0].Status != StepStatusSkipped {
		t.Fatalf("expected first result to be skipped")
	}
}
