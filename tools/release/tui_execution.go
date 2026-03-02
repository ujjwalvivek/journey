package main

import (
	"fmt"
	"strings"
	"time"

	tea "github.com/charmbracelet/bubbletea"
)

func executionTickCmd() tea.Cmd {
	return func() tea.Msg { return executionTickMsg{} }
}

func pollRunnerCmd() tea.Cmd {
	return tea.Tick(pollRunnerInterval, func(time.Time) tea.Msg {
		return pollRunnerMsg{}
	})
}

func (m model) updateExecution(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	switch msg.String() {
	case "q":
		m.cancelCmds()
		m.exitErr = fmt.Errorf("aborted by user")
		return m, tea.Quit
	case "y", "enter":
		if m.awaitingApproval {
			m.manualApprovals++
			return m, m.beginStep()
		}
	case "n":
		if m.awaitingApproval {
			res := m.results[m.currentStep]
			res.Status = StepStatusSkipped
			res.Approved = false
			res.Err = ErrCommandDenied
			m.results[m.currentStep] = res
			m.awaitingApproval = false
			m.pipelineDoneAt = time.Now()
			m.exitErr = ErrCommandDenied
			m.finishedAt = time.Now()
			m.finalizeSummary()
			m.screen = screenSummary
			return m, nil
		}
	}
	return m, nil
}

//? beginStep sets up state and launches the goroutine for the current step.
//? Mutations happen on the caller's model (in Update), fixing the value-receiver issue.
func (m *model) beginStep() tea.Cmd {
	if m.currentStep >= len(m.steps) {
		return nil
	}

	step := m.steps[m.currentStep]
	result := m.results[m.currentStep]
	result.Status = StepStatusRunning
	result.StartedAt = time.Now()
	result.Approved = true
	m.results[m.currentStep] = result

	m.awaitingApproval = false
	m.runningStep = true
	m.runnerEvents = make(chan any, 512)

	appendLimited(&m.logLines, fmt.Sprintf("› %s", step.Display()), maxPipelineLogs)

	ch := m.runnerEvents
	go func() {
		output, err := m.executor.RunStream(m.cmdCtx, m.ctx.RepoRoot, step.Command, step.Args, func(line string) {
			ch <- runnerLogEvent{line: line}
		})
		ch <- runnerDoneEvent{output: output, err: err, ended: time.Now()}
		close(ch)
	}()

	return pollRunnerCmd()
}

func (m model) handleRunnerEvents() (tea.Model, tea.Cmd) {
	if m.runnerEvents == nil {
		return m, nil
	}

	for {
		select {
		case ev, ok := <-m.runnerEvents:
			if !ok {
				m.runnerEvents = nil
				goto drained
			}

			switch event := ev.(type) {
			case runnerLogEvent:
				appendLimited(&m.logLines, event.line, maxPipelineLogs)
			case runnerDoneEvent:
				m.runningStep = false
				if m.currentStep >= len(m.results) {
					continue
				}

				res := m.results[m.currentStep]
				res.EndedAt = event.ended
				res.Duration = res.EndedAt.Sub(res.StartedAt)
				res.Output = event.output
				res.Err = event.err
				if event.err != nil {
					res.Status = StepStatusFailed
					m.results[m.currentStep] = res
					m.pipelineDoneAt = time.Now()
					m.exitErr = event.err
					m.finishedAt = time.Now()
					m.finalizeSummary()
					m.screen = screenSummary
					return m, nil
				}
				res.Status = StepStatusSuccess
				m.results[m.currentStep] = res

				step := m.steps[m.currentStep]
				if step.Name == stepNameCommitVersion {
					shaOut, shaErr := m.executor.Output(m.cmdCtx, m.ctx.RepoRoot, "git", []string{"rev-parse", "HEAD"})
					if shaErr == nil {
						m.ctx.ReleaseCommitSHA = strings.TrimSpace(shaOut)
					}
				}
				if step.Command == "git" && len(step.Args) >= 2 && step.Args[0] == "checkout" {
					m.ctx.FinalBranch = step.Args[1]
				}

				m.currentStep++
				if m.currentStep >= len(m.steps) {
					m.pipelineDoneAt = time.Now()
					if m.skipCI {
						m.finishedAt = time.Now()
						m.finalizeSummary()
						m.screen = screenSummary
						return m, nil
					}
					m.screen = screenCI
					m.ciRunning = true
					m.ciEvents = make(chan any, 1024)
					appendLimited(&m.ciLogLines, "Starting GitHub Actions watcher...", maxCILogs)
					go m.runCIWatcher(m.ciEvents)
					return m, pollCICmd()
				}
				return m, executionTickCmd()
			}
		default:
			goto drained
		}
	}

drained:
	if m.runningStep || m.runnerEvents != nil {
		return m, pollRunnerCmd()
	}
	return m, nil
}

func (m model) viewExecution() string {
	w := m.frameWidth()
	var b strings.Builder

	modeLabel := string(m.ctx.Mode)
	if m.dryRun {
		modeLabel = "dry-run"
	}
	b.WriteString(fmt.Sprintf("  %s  %s\n\n", m.ctx.Tag, dimStyle.Render(modeLabel)))
	b.WriteString("  " + progressBar(m.currentStep, len(m.steps), w-16) + "\n\n")

	for i, step := range m.steps {
		icon := statusIcon(m.results[i].Status)
		marker := " "
		if i == m.currentStep && m.currentStep < len(m.steps) {
			marker = "›"
		}
		name := dimStyle.Render(step.Name)
		if m.results[i].Status == StepStatusSuccess {
			name = step.Name
		} else if i == m.currentStep {
			name = selectedStyle.Render(step.Name)
		}
		b.WriteString(fmt.Sprintf("  %s %s  %s\n", marker, icon, name))
	}

	if m.awaitingApproval && m.currentStep < len(m.steps) {
		step := m.steps[m.currentStep]
		b.WriteString("\n  " + warnStyle.Render("Approve: ") + step.Display())
	}

	b.WriteString("\n")
	b.WriteString(renderLogBox(m.logLines, m.logHeight(), w))

	hints := "q quit"
	if m.awaitingApproval {
		hints = "y approve  n deny  q quit"
	}
	return m.renderFrame("Pipeline", b.String(), hints)
}
