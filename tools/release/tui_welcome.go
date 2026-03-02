package main

import (
	"fmt"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
)

func (m model) updateWelcome(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	switch msg.String() {
	case "q":
		m.cancelCmds()
		m.exitErr = fmt.Errorf("aborted by user")
		return m, tea.Quit
	case "enter":
		if !m.preflightOK {
			return m, nil
		}
		opts, err := deriveVersionOptions(m.ctx.CurrentVersion)
		if err != nil {
			m.versionErr = err.Error()
			m.versionOptions = VersionOptions{Current: m.ctx.CurrentVersion}
		} else {
			m.versionOptions = opts
			m.versionErr = ""
		}
		m.screen = screenVersion
		return m, nil
	}
	return m, nil
}

func (m model) viewWelcome() string {
	var body strings.Builder

	if m.preflightErr != nil {
		body.WriteString(errStyle.Render("Failed to load preflight: " + m.preflightErr.Error()))
		return m.renderFrame("Preflight", body.String(), "q quit")
	}

	for _, check := range m.checks {
		var icon string
		switch {
		case check.OK:
			icon = okStyle.Render("✓")
		case check.Skippable && m.allowDirty:
			icon = warnStyle.Render("⚠")
		default:
			icon = errStyle.Render("✗")
		}
		body.WriteString(fmt.Sprintf("  %s  %-28s %s\n", icon, check.Name, dimStyle.Render(check.Detail)))
	}

	body.WriteString("\n")
	body.WriteString(fmt.Sprintf("  Repository   %s\n", dimStyle.Render(m.ctx.RepoRoot)))
	body.WriteString(fmt.Sprintf("  Version      %s\n", m.ctx.CurrentVersion))
	body.WriteString(fmt.Sprintf("  Branch       %s\n", m.ctx.StartBranch))

	if !m.preflightOK {
		body.WriteString("\n" + errStyle.Render("  Preflight failed. Fix issues before releasing."))
		return m.renderFrame("Preflight", body.String(), "q quit")
	}

	if m.allowDirty {
		body.WriteString("\n" + warnStyle.Render("  --dirty: uncommitted changes will be included in release."))
	}

	return m.renderFrame("Preflight", body.String(), "enter continue  q quit")
}
