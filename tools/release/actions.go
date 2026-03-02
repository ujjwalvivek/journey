package main

import (
	"context"
	"encoding/json"
	"fmt"
	"strconv"
	"strings"
	"time"
)

type ghRunListItem struct {
	DatabaseID int64  `json:"databaseId"`
	HeadSHA    string `json:"headSha"`
	URL        string `json:"url"`
	Name       string `json:"name"`
	Status     string `json:"status"`
	Conclusion string `json:"conclusion"`
	CreatedAt  string `json:"createdAt"`
	UpdatedAt  string `json:"updatedAt"`
}

type ghRunView struct {
	DatabaseID int64  `json:"databaseId"`
	URL        string `json:"url"`
	Name       string `json:"name"`
	Status     string `json:"status"`
	Conclusion string `json:"conclusion"`
	CreatedAt  string `json:"createdAt"`
	UpdatedAt  string `json:"updatedAt"`
	Jobs       []struct {
		Name       string `json:"name"`
		Status     string `json:"status"`
		Conclusion string `json:"conclusion"`
	} `json:"jobs"`
}

func monitorGitHubActions(
	ctx context.Context,
	executor CommandExecutor,
	repoRoot string,
	tag string,
	timeout time.Duration,
	pollInterval time.Duration,
	onLog func(string),
) (WorkflowRunInfo, error) {
	emit := func(line string) {
		if onLog != nil {
			onLog(line)
		}
	}

	if timeout <= 0 {
		timeout = defaultCITimeout
	}
	if pollInterval <= 0 {
		pollInterval = defaultCIWatchPollInterval
	}

	watchCtx, cancel := context.WithTimeout(ctx, timeout)
	defer cancel()

	var result WorkflowRunInfo

	shaOut, err := executor.Output(watchCtx, repoRoot, "git", []string{"rev-list", "-n", "1", tag})
	if err != nil {
		return result, fmt.Errorf("failed to resolve tag commit SHA: %w", err)
	}
	sha := strings.TrimSpace(shaOut)
	if sha == "" {
		return result, fmt.Errorf("empty commit SHA for tag %s", tag)
	}

	emit("Searching for " + ciWorkflowName + " workflow run...")

	var found ghRunListItem
	for {
		select {
		case <-watchCtx.Done():
			result.TimedOut = watchCtx.Err() == context.DeadlineExceeded
			return result, fmt.Errorf("timed out waiting for workflow run: %w", watchCtx.Err())
		default:
		}

		out, listErr := executor.Output(
			watchCtx,
			repoRoot,
			"gh",
			[]string{"run", "list", "--workflow", ciWorkflowName, "--json", "databaseId,headSha,url,name,status,conclusion,createdAt,updatedAt", "--limit", ciRunListLimit},
		)
		if listErr != nil {
			return result, fmt.Errorf("failed to list workflow runs: %w", listErr)
		}

		var runs []ghRunListItem
		if unmarshalErr := json.Unmarshal([]byte(out), &runs); unmarshalErr != nil {
			return result, fmt.Errorf("failed to parse gh run list output: %w", unmarshalErr)
		}

		foundThisPass := false
		for _, candidate := range runs {
			if strings.EqualFold(strings.TrimSpace(candidate.HeadSHA), sha) {
				found = candidate
				foundThisPass = true
				break
			}
		}

		if foundThisPass {
			break
		}

		emit("Waiting for matching workflow run...")
		select {
		case <-watchCtx.Done():
			result.TimedOut = watchCtx.Err() == context.DeadlineExceeded
			return result, fmt.Errorf("timed out waiting for workflow run: %w", watchCtx.Err())
		case <-time.After(pollInterval):
		}
	}

	result.ID = found.DatabaseID
	result.Name = found.Name
	result.URL = found.URL
	result.Status = found.Status
	result.Conclusion = found.Conclusion
	result.CreatedAt = parseGitHubTime(found.CreatedAt)
	result.UpdatedAt = parseGitHubTime(found.UpdatedAt)

	runID := strconv.FormatInt(found.DatabaseID, 10)
	emit(fmt.Sprintf("Found run %s (%s)", runID, found.URL))

	_, watchErr := executor.RunStream(watchCtx, repoRoot, "gh", []string{"run", "watch", runID, "--exit-status"}, emit)
	if watchErr != nil {
		emit("gh run watch reported a non-success conclusion")
	}

	logOut, logErr := executor.Output(watchCtx, repoRoot, "gh", []string{"run", "view", runID, "--log"})
	if logErr == nil {
		for _, line := range strings.Split(logOut, "\n") {
			emit(line)
		}
	}

	viewOut, viewErr := executor.Output(watchCtx, repoRoot, "gh", []string{"run", "view", runID, "--json", "databaseId,url,status,conclusion,createdAt,updatedAt,jobs,name"})
	if viewErr == nil {
		var view ghRunView
		if unmarshalErr := json.Unmarshal([]byte(viewOut), &view); unmarshalErr == nil {
			result.ID = view.DatabaseID
			result.URL = view.URL
			result.Status = view.Status
			result.Conclusion = view.Conclusion
			result.Name = view.Name
			result.CreatedAt = parseGitHubTime(view.CreatedAt)
			result.UpdatedAt = parseGitHubTime(view.UpdatedAt)

			jobs := make([]WorkflowJobInfo, 0, len(view.Jobs))
			for _, job := range view.Jobs {
				jobs = append(jobs, WorkflowJobInfo{
					Name:       job.Name,
					Status:     job.Status,
					Conclusion: job.Conclusion,
				})
			}
			result.Jobs = jobs
		}
	}

	if watchCtx.Err() == context.DeadlineExceeded {
		result.TimedOut = true
		return result, fmt.Errorf("timed out while watching workflow run")
	}

	if watchErr != nil {
		return result, watchErr
	}
	if viewErr != nil {
		return result, viewErr
	}

	conclusion := strings.ToLower(strings.TrimSpace(result.Conclusion))
	if conclusion == "failure" || conclusion == "cancelled" || conclusion == "timed_out" {
		return result, fmt.Errorf("workflow concluded with %s", result.Conclusion)
	}

	return result, nil
}

func parseGitHubTime(value string) time.Time {
	if strings.TrimSpace(value) == "" {
		return time.Time{}
	}
	t, err := time.Parse(time.RFC3339, value)
	if err != nil {
		return time.Time{}
	}
	return t
}
