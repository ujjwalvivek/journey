package main

import (
	"strings"
	"time"
)

type ApprovalMode string

const (
	ApprovalPrompt ApprovalMode = "prompt"
	ApprovalZen    ApprovalMode = "zen"
)

type StepStatus string

const (
	StepStatusPending         StepStatus = "pending"
	StepStatusWaitingApproval StepStatus = "waiting_approval"
	StepStatusRunning         StepStatus = "running"
	StepStatusSuccess         StepStatus = "success"
	StepStatusFailed          StepStatus = "failed"
	StepStatusSkipped         StepStatus = "skipped"
)

type CommandStep struct {
	Name             string
	Command          string
	Args             []string
	RequiresApproval bool
}

func (s CommandStep) Display() string {
	joined := strings.Join(s.Args, " ")
	if joined == "" {
		return s.Command
	}
	return s.Command + " " + joined
}

type StepResult struct {
	Step      CommandStep
	Status    StepStatus
	StartedAt time.Time
	EndedAt   time.Time
	Duration  time.Duration
	Approved  bool
	Output    string
	Err       error
}

type ReleaseContext struct {
	RepoRoot         string
	RepoWebURL       string
	CurrentVersion   string
	StartBranch      string
	SelectedVersion  string
	Tag              string
	Mode             ApprovalMode
	CITimeout        time.Duration
	ReleaseCommitSHA string
	FinalBranch      string
}

type WorkflowJobInfo struct {
	Name       string
	Status     string
	Conclusion string
}

type WorkflowRunInfo struct {
	ID         int64
	Name       string
	URL        string
	Status     string
	Conclusion string
	CreatedAt  time.Time
	UpdatedAt  time.Time
	Jobs       []WorkflowJobInfo
	TimedOut   bool
}

type ReleaseStats struct {
	StartedAt         time.Time
	EndedAt           time.Time
	TotalDuration     time.Duration
	PreflightDuration time.Duration
	PipelineDuration  time.Duration
	CIDuration        time.Duration

	PlannedCommands  int
	ExecutedCommands int
	FailedCommands   int
	SkippedCommands  int

	ManualApprovals int
	AutoApprovals   int

	ChangedFiles int
	Insertions   int
	Deletions    int

	WorkflowJobsTotal  int
	WorkflowJobsPassed int
	WorkflowJobsFailed int
}

type PreflightCheck struct {
	Name      string
	OK        bool
	Skippable bool
	Detail    string
}
