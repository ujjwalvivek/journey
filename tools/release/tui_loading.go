package main

import (
	"context"
	"time"

	tea "github.com/charmbracelet/bubbletea"
)

func loadPreflightCmd(executor CommandExecutor, ciTimeout time.Duration) tea.Cmd {
	return func() tea.Msg {
		ctx, cancel := context.WithTimeout(context.Background(), preflightCheckTimeout)
		defer cancel()

		releaseCtx, checks, err := runPreflight(ctx, executor, ciTimeout)
		return preflightLoadedMsg{
			ctx:    releaseCtx,
			checks: checks,
			err:    err,
		}
	}
}

func (m model) viewLoading() string {
	body := m.spinner.View() + " Loading repository context..."
	return m.renderFrame("", body, "")
}
