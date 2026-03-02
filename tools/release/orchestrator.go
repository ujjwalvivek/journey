package main

import (
	"context"
	"errors"
	"fmt"
	"time"
)

var ErrCommandDenied = errors.New("command approval denied")

func shouldRunStep(mode ApprovalMode, userApproved bool) (bool, error) {
	switch mode {
	case ApprovalZen:
		return true, nil
	case ApprovalPrompt:
		if userApproved {
			return true, nil
		}
		return false, ErrCommandDenied
	default:
		return false, fmt.Errorf("unsupported approval mode: %q", mode)
	}
}

type PipelineOptions struct {
	RepoRoot  string
	Steps     []CommandStep
	Mode      ApprovalMode
	ApproveFn func(CommandStep) bool
	Executor  CommandExecutor
	OnOutput  func(CommandStep, string)
}

func executePipeline(ctx context.Context, opts PipelineOptions) ([]StepResult, error) {
	if opts.Executor == nil {
		return nil, fmt.Errorf("executor is required")
	}

	results := make([]StepResult, len(opts.Steps))

	for i, step := range opts.Steps {
		res := StepResult{
			Step:   step,
			Status: StepStatusPending,
		}

		approved := true
		if step.RequiresApproval {
			userApproved := true
			if opts.Mode == ApprovalPrompt {
				if opts.ApproveFn == nil {
					userApproved = false
				} else {
					userApproved = opts.ApproveFn(step)
				}
			}

			shouldRun, approvalErr := shouldRunStep(opts.Mode, userApproved)
			if approvalErr != nil {
				res.Status = StepStatusSkipped
				res.Approved = false
				res.Err = approvalErr
				results[i] = res
				return results, approvalErr
			}
			approved = shouldRun
		}

		res.Approved = approved
		res.Status = StepStatusRunning
		res.StartedAt = time.Now()

		output, err := opts.Executor.RunStream(ctx, opts.RepoRoot, step.Command, step.Args, func(line string) {
			if opts.OnOutput != nil {
				opts.OnOutput(step, line)
			}
		})

		res.EndedAt = time.Now()
		res.Duration = res.EndedAt.Sub(res.StartedAt)
		res.Output = output

		if err != nil {
			res.Status = StepStatusFailed
			res.Err = err
			results[i] = res
			return results, err
		}

		res.Status = StepStatusSuccess
		results[i] = res
	}

	return results, nil
}
