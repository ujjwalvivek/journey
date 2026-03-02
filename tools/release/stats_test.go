package main

import (
	"testing"
	"time"
)

func TestCalculateStats(t *testing.T) {
	start := time.Now()
	preflight := start.Add(2 * time.Second)
	pipeline := start.Add(10 * time.Second)
	end := start.Add(15 * time.Second)

	results := []StepResult{
		{Status: StepStatusSuccess},
		{Status: StepStatusFailed},
		{Status: StepStatusSkipped},
	}
	ci := WorkflowRunInfo{
		Jobs: []WorkflowJobInfo{
			{Name: "validate", Conclusion: "success"},
			{Name: "publish", Conclusion: "failure"},
		},
	}

	stats := calculateStats(results, ci, 2, 1, start, preflight, pipeline, end)

	if stats.PlannedCommands != 3 {
		t.Fatalf("expected planned=3, got %d", stats.PlannedCommands)
	}
	if stats.ExecutedCommands != 2 {
		t.Fatalf("expected executed=2, got %d", stats.ExecutedCommands)
	}
	if stats.FailedCommands != 1 {
		t.Fatalf("expected failed=1, got %d", stats.FailedCommands)
	}
	if stats.SkippedCommands != 1 {
		t.Fatalf("expected skipped=1, got %d", stats.SkippedCommands)
	}
	if stats.ManualApprovals != 2 || stats.AutoApprovals != 1 {
		t.Fatalf("unexpected approvals manual=%d auto=%d", stats.ManualApprovals, stats.AutoApprovals)
	}
	if stats.WorkflowJobsTotal != 2 || stats.WorkflowJobsPassed != 1 || stats.WorkflowJobsFailed != 1 {
		t.Fatalf("unexpected workflow counts: total=%d passed=%d failed=%d", stats.WorkflowJobsTotal, stats.WorkflowJobsPassed, stats.WorkflowJobsFailed)
	}
	if stats.TotalDuration <= 0 {
		t.Fatalf("expected positive total duration")
	}
}

func TestOverallReleaseSuccess(t *testing.T) {
	results := []StepResult{
		{Status: StepStatusSuccess},
		{Status: StepStatusSuccess},
	}
	ci := WorkflowRunInfo{Conclusion: "success"}
	if !overallReleaseSuccess(results, ci, nil) {
		t.Fatalf("expected release success")
	}
}
