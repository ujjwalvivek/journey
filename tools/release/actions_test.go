package main

import (
	"context"
	"strings"
	"testing"
	"time"
)

func TestMonitorGitHubActionsTimeoutReturnsPartial(t *testing.T) {
	stub := &stubExecutor{
		outputFn: func(ctx context.Context, dir, name string, args []string) (string, error) {
			if name == "git" && len(args) == 4 && args[0] == "rev-list" {
				return "abc123", nil
			}
			if name == "gh" && len(args) >= 3 && args[0] == "run" && args[1] == "list" {
				return "[]", nil
			}
			return "", nil
		},
		runFn: func(ctx context.Context, dir, name string, args []string, onOutput func(string)) (string, error) {
			return "", nil
		},
	}

	info, err := monitorGitHubActions(
		context.Background(),
		stub,
		"repo",
		"v0.3.3",
		40*time.Millisecond,
		5*time.Millisecond,
		nil,
	)
	if err == nil {
		t.Fatalf("expected timeout error")
	}
	if !strings.Contains(err.Error(), "timed out") {
		t.Fatalf("expected timeout error message, got %v", err)
	}
	if !info.TimedOut {
		t.Fatalf("expected TimedOut=true")
	}
}
