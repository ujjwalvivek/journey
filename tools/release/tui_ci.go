package main

import (
	"fmt"
	"strings"
	"time"

	tea "github.com/charmbracelet/bubbletea"
)

func pollCICmd() tea.Cmd {
	return tea.Tick(pollCIInterval, func(time.Time) tea.Msg {
		return pollCIMsg{}
	})
}

func (m model) updateCI(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	switch msg.String() {
	case "q":
		m.cancelCmds()
		m.exitErr = fmt.Errorf("aborted by user")
		return m, tea.Quit
	case "c":
		m.ciJobsExpanded = !m.ciJobsExpanded
		return m, nil
	case "s":
		m.ciRunning = false
		m.finishedAt = time.Now()
		m.finalizeSummary()
		m.screen = screenSummary
		return m, nil
	}
	return m, nil
}

//? runCIWatcher runs in a goroutine. Channel is created by the caller
//? to avoid the value-receiver mutation bug.
func (m model) runCIWatcher(ch chan any) {
	info, err := monitorGitHubActions(
		m.cmdCtx,
		m.executor,
		m.ctx.RepoRoot,
		m.ctx.Tag,
		m.ctx.CITimeout,
		defaultCIWatchPollInterval,
		func(line string) {
			ch <- ciLogEvent{line: line}
		},
		func(jobs []WorkflowJobInfo) {
			ch <- ciJobsUpdatedEvent{jobs: jobs}
		},
	)
	ch <- ciDoneEvent{info: info, err: err}
	close(ch)
}

func (m model) handleCIEvents() (tea.Model, tea.Cmd) {
	if m.ciEvents == nil {
		return m, nil
	}

	for {
		select {
		case ev, ok := <-m.ciEvents:
			if !ok {
				m.ciEvents = nil
				goto drained
			}

			switch event := ev.(type) {
			case ciLogEvent:
				appendLimited(&m.ciLogLines, event.line, maxCILogs)
			case ciJobsUpdatedEvent:
				m.ciJobs = event.jobs
			case ciDoneEvent:
				m.ciRunning = false
				m.ciInfo = event.info
				m.ciErr = event.err
				if event.err != nil {
					m.exitErr = event.err
				}
				m.finishedAt = time.Now()
				m.finalizeSummary()
				m.screen = screenSummary
				return m, nil
			}
		default:
			goto drained
		}
	}

drained:
	if m.ciRunning || m.ciEvents != nil {
		return m, pollCICmd()
	}
	return m, nil
}

func (m model) viewCI() string {
	w := m.frameWidth()
	var b strings.Builder

	status := okStyle.Render("● running")
	if !m.ciRunning {
		status = dimStyle.Render("○ finished")
	}
	b.WriteString(fmt.Sprintf("  %s  %s\n\n", m.ctx.Tag, status))

	if len(m.ciJobs) > 0 {
		if m.ciJobsExpanded {
			b.WriteString(dimStyle.Render("  Jobs") + dimStyle.Render("  c collapse") + "\n")
			b.WriteString(dimStyle.Render("  "+strings.Repeat("─", w-4)) + "\n")
			for _, job := range m.ciJobs {
				icon := ciJobIcon(job)
				name := job.Name
				if len(name) > w-8 {
					name = name[:w-11] + "..."
				}
				b.WriteString(fmt.Sprintf("  %s  %s\n", icon, dimStyle.Render(name)))
			}
			b.WriteString("\n")
		} else {
			b.WriteString(dimStyle.Render(fmt.Sprintf("  Jobs (%d)", len(m.ciJobs))) +
				dimStyle.Render("  c expand") + "\n\n")
		}
	}

	if m.ciInfo.URL != "" {
		b.WriteString("  " + dimStyle.Render("url ") + m.ciInfo.URL + "\n\n")
	}

	elapsed := time.Since(m.pipelineDoneAt).Truncate(time.Second)
	b.WriteString("  " + dimStyle.Render("elapsed ") + elapsed.String() + "\n\n")

	b.WriteString(renderLogBox(m.ciLogLines, m.logHeight(), w))

	return m.renderFrame("CI Watch", b.String(), "c toggle jobs  s skip  q quit")
}

func ciJobIcon(job WorkflowJobInfo) string {
	switch {
	case job.Status == "completed" && job.Conclusion == "success":
		return okStyle.Render("✓")
	case job.Status == "completed" && (job.Conclusion == "failure" || job.Conclusion == "cancelled" || job.Conclusion == "timed_out"):
		return errStyle.Render("✗")
	case job.Status == "completed" && job.Conclusion == "skipped":
		return dimStyle.Render("⊘")
	case job.Status == "in_progress":
		return titleStyle.Render("●")
	case job.Status == "queued":
		return warnStyle.Render("○")
	default:
		return dimStyle.Render("○")
	}
}
