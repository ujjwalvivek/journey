package main

import (
	"fmt"
	"strings"
	"time"

	tea "github.com/charmbracelet/bubbletea"
)

func (m model) updateMode(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	switch msg.String() {
	case "q":
		m.cancelCmds()
		m.exitErr = fmt.Errorf("aborted by user")
		return m, tea.Quit
	case "up", "k":
		if m.modeIndex > 0 {
			m.modeIndex--
		}
		return m, nil
	case "down", "j":
		if m.modeIndex < 1 {
			m.modeIndex++
		}
		return m, nil
	case "enter":
		if m.modeIndex == 0 {
			m.ctx.Mode = ApprovalPrompt
		} else {
			m.ctx.Mode = ApprovalZen
		}

		steps, err := buildCommandPipeline(m.ctx.StartBranch, m.ctx.SelectedVersion, m.ctx.Tag)
		if err != nil {
			m.exitErr = err
			m.finishedAt = time.Now()
			m.finalizeSummary()
			m.screen = screenSummary
			return m, nil
		}

		m.steps = steps
		m.results = make([]StepResult, len(steps))
		for i, step := range steps {
			m.results[i] = StepResult{
				Step:   step,
				Status: StepStatusPending,
			}
		}

		m.releaseStartedAt = time.Now()
		m.screen = screenExecution
		return m, executionTickCmd()
	}

	return m, nil
}

func (m model) viewMode() string {
	var b strings.Builder

	b.WriteString(fmt.Sprintf("  Version: %s (%s)    Branch: %s\n\n", m.ctx.SelectedVersion, m.ctx.Tag, m.ctx.StartBranch))

	modes := []struct {
		label string
		desc  string
	}{
		{"Prompt", "ask before every command"},
		{"Zen", "auto-approve all commands"},
	}

	for i, mode := range modes {
		prefix := "    "
		style := dimStyle
		if m.modeIndex == i {
			prefix = "  › "
			style = selectedStyle
		}
		b.WriteString(fmt.Sprintf("%s%-10s %s\n", prefix, style.Render(mode.label), dimStyle.Render(mode.desc)))
	}

	return m.renderFrame("Approval Mode", b.String(), "↑/↓ navigate  enter select  q quit")
}
