package main

import (
	"context"
	"strconv"
	"strings"
	"time"
)

func calculateStats(
	results []StepResult,
	ci WorkflowRunInfo,
	manualApprovals int,
	autoApprovals int,
	startedAt time.Time,
	preflightDoneAt time.Time,
	pipelineDoneAt time.Time,
	endedAt time.Time,
) ReleaseStats {
	stats := ReleaseStats{
		StartedAt:       startedAt,
		EndedAt:         endedAt,
		PlannedCommands: len(results),
		ManualApprovals: manualApprovals,
		AutoApprovals:   autoApprovals,
	}

	if !startedAt.IsZero() && !endedAt.IsZero() && endedAt.After(startedAt) {
		stats.TotalDuration = endedAt.Sub(startedAt)
	}
	if !startedAt.IsZero() && !preflightDoneAt.IsZero() && preflightDoneAt.After(startedAt) {
		stats.PreflightDuration = preflightDoneAt.Sub(startedAt)
	}
	if !startedAt.IsZero() && !pipelineDoneAt.IsZero() && pipelineDoneAt.After(startedAt) {
		stats.PipelineDuration = pipelineDoneAt.Sub(startedAt)
	}
	if !pipelineDoneAt.IsZero() && !endedAt.IsZero() && endedAt.After(pipelineDoneAt) {
		stats.CIDuration = endedAt.Sub(pipelineDoneAt)
	}

	for _, result := range results {
		switch result.Status {
		case StepStatusSuccess:
			stats.ExecutedCommands++
		case StepStatusFailed:
			stats.ExecutedCommands++
			stats.FailedCommands++
		case StepStatusSkipped:
			stats.SkippedCommands++
		}
	}

	stats.WorkflowJobsTotal, stats.WorkflowJobsPassed, stats.WorkflowJobsFailed = workflowJobCounts(ci)
	return stats
}

func workflowJobCounts(ci WorkflowRunInfo) (total int, passed int, failed int) {
	for _, job := range ci.Jobs {
		total++
		conclusion := strings.ToLower(strings.TrimSpace(job.Conclusion))
		switch conclusion {
		case "success":
			passed++
		case "failure", "cancelled", "timed_out", "action_required", "startup_failure":
			failed++
		}
	}
	return total, passed, failed
}

func collectDiffStats(ctx context.Context, executor CommandExecutor, repoRoot string, commitSHA string) (files int, insertions int, deletions int, err error) {
	if strings.TrimSpace(commitSHA) == "" {
		return 0, 0, 0, nil
	}

	out, err := executor.Output(ctx, repoRoot, "git", []string{"show", "--numstat", "--format=", commitSHA})
	if err != nil {
		return 0, 0, 0, err
	}

	for _, line := range strings.Split(out, "\n") {
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}
		parts := strings.Split(line, "\t")
		if len(parts) < 3 {
			continue
		}
		files++
		if parts[0] != "-" {
			if n, convErr := strconv.Atoi(parts[0]); convErr == nil {
				insertions += n
			}
		}
		if parts[1] != "-" {
			if n, convErr := strconv.Atoi(parts[1]); convErr == nil {
				deletions += n
			}
		}
	}
	return files, insertions, deletions, nil
}

func releaseTagURL(repoWebURL, tag string) string {
	if strings.TrimSpace(repoWebURL) == "" || strings.TrimSpace(tag) == "" {
		return ""
	}
	return strings.TrimSuffix(repoWebURL, "/") + "/releases/tag/" + tag
}

func overallReleaseSuccess(results []StepResult, ci WorkflowRunInfo, ciErr error) bool {
	for _, result := range results {
		if result.Status == StepStatusFailed || result.Status == StepStatusSkipped {
			return false
		}
	}
	if ciErr != nil {
		return false
	}
	conclusion := strings.ToLower(strings.TrimSpace(ci.Conclusion))
	if conclusion == "" {
		return true
	}
	return conclusion == "success" || conclusion == "neutral"
}
