package main

import (
	"context"
	"fmt"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"golang.org/x/mod/semver"
)

func checkTagCmd(executor CommandExecutor, repoRoot string, selected string) tea.Cmd {
	return func() tea.Msg {
		version, tag, err := normalizeVersionInput(selected)
		if err != nil {
			return tagCheckedMsg{err: err}
		}
		exists, err := tagExists(context.Background(), executor, repoRoot, tag)
		return tagCheckedMsg{
			version: version,
			tag:     tag,
			exists:  exists,
			err:     err,
		}
	}
}

//? quickSemverCheck validates without hitting the network.
func quickSemverCheck(input string) string {
	v := strings.TrimSpace(input)
	if v == "" {
		return "version is required"
	}
	if !strings.HasPrefix(v, "v") {
		v = "v" + v
	}
	if !semver.IsValid(v) {
		return "invalid semver format"
	}
	return ""
}

func (m model) updateVersion(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	if m.checkingTag {
		return m, nil
	}

	if m.enteringCustom {
		var inputCmd tea.Cmd
		m.customInput, inputCmd = m.customInput.Update(msg)

		switch msg.String() {
		case "esc":
			m.enteringCustom = false
			m.versionErr = ""
			m.customInput.Blur()
			return m, nil
		case "enter":
			selected := strings.TrimSpace(m.customInput.Value())
			if errMsg := quickSemverCheck(selected); errMsg != "" {
				m.versionErr = errMsg
				return m, nil
			}
			m.checkingTag = true
			m.versionErr = ""
			return m, checkTagCmd(m.executor, m.ctx.RepoRoot, selected)
		default:
			//? Live validation feedback as user types.
			val := strings.TrimSpace(m.customInput.Value())
			if val != "" {
				m.versionErr = quickSemverCheck(val)
			} else {
				m.versionErr = ""
			}
		}
		return m, inputCmd
	}

	switch msg.String() {
	case "q":
		m.cancelCmds()
		m.exitErr = fmt.Errorf("aborted by user")
		return m, tea.Quit
	case "up", "k":
		if m.versionIndex > 0 {
			m.versionIndex--
		}
		return m, nil
	case "down", "j":
		if m.versionIndex < 3 {
			m.versionIndex++
		}
		return m, nil
	case "enter":
		if m.versionIndex == 3 {
			m.enteringCustom = true
			m.customInput.SetValue("")
			m.customInput.Focus()
			return m, nil
		}

		var selected string
		switch m.versionIndex {
		case 0:
			selected = m.versionOptions.Patch
		case 1:
			selected = m.versionOptions.Minor
		case 2:
			selected = m.versionOptions.Major
		}

		m.checkingTag = true
		m.versionErr = ""
		return m, checkTagCmd(m.executor, m.ctx.RepoRoot, selected)
	}

	return m, nil
}

func (m model) viewVersion() string {
	var b strings.Builder

	b.WriteString(fmt.Sprintf("  Current: %s    Branch: %s\n\n", m.versionOptions.Current, m.ctx.StartBranch))

	options := []struct {
		label   string
		version string
	}{
		{"Patch", m.versionOptions.Patch},
		{"Minor", m.versionOptions.Minor},
		{"Major", m.versionOptions.Major},
		{"Custom", ""},
	}

	for i, opt := range options {
		prefix := "    "
		style := dimStyle
		if m.versionIndex == i && !m.enteringCustom {
			prefix = "  › "
			style = selectedStyle
		}
		if opt.version != "" {
			b.WriteString(fmt.Sprintf("%s%-8s %s\n", prefix, style.Render(opt.label), opt.version))
		} else {
			b.WriteString(fmt.Sprintf("%s%s\n", prefix, style.Render(opt.label)))
		}
	}

	if m.enteringCustom {
		b.WriteString("\n" + m.customInput.View() + "\n")
	}

	if m.checkingTag {
		b.WriteString("\n  " + dimStyle.Render("checking tag..."))
	}

	if m.versionErr != "" {
		b.WriteString("\n  " + errStyle.Render(m.versionErr))
	}

	hints := "↑/↓ navigate  enter select  q quit"
	if m.enteringCustom {
		hints = "enter validate  esc cancel"
	}

	return m.renderFrame("Version", b.String(), hints)
}
