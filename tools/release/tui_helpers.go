package main

import (
	"fmt"
	"strings"
)

func (m model) renderFrame(screenTitle, body, hints string) string {
	w := m.frameWidth()

	var b strings.Builder

	header := titleStyle.Render("◆ " + appTitle)
	if m.ctx.StartBranch != "" {
		header += dimStyle.Render("  " + m.ctx.StartBranch)
	}
	if m.ctx.CurrentVersion != "" {
		header += dimStyle.Render(" • v" + m.ctx.CurrentVersion)
	}
	b.WriteString(header + "\n")
	b.WriteString(dimStyle.Render(strings.Repeat("─", w)) + "\n\n")

	if screenTitle != "" {
		b.WriteString(subtitleStyle.Render(screenTitle) + "\n\n")
	}

	b.WriteString(body)
	b.WriteString("\n\n")
	b.WriteString(dimStyle.Render(strings.Repeat("─", w)) + "\n")
	b.WriteString(dimStyle.Render(hints))

	return b.String()
}

func (m model) frameWidth() int {
	if m.width > 4 {
		return m.width - 4
	}
	return 76
}

func appendLimited(lines *[]string, line string, limit int) {
	trimmed := strings.TrimRight(line, "\r\n")
	*lines = append(*lines, trimmed)
	if len(*lines) > limit {
		*lines = (*lines)[len(*lines)-limit:]
	}
}

func statusIcon(status StepStatus) string {
	switch status {
	case StepStatusPending:
		return dimStyle.Render("○")
	case StepStatusWaitingApproval:
		return warnStyle.Render("◆")
	case StepStatusRunning:
		return titleStyle.Render("●")
	case StepStatusSuccess:
		return okStyle.Render("✓")
	case StepStatusFailed:
		return errStyle.Render("✗")
	case StepStatusSkipped:
		return dimStyle.Render("⊘")
	default:
		return dimStyle.Render("○")
	}
}

func progressBar(current, total, width int) string {
	if total == 0 || width < 10 {
		return ""
	}
	filled := 0
	if total > 0 {
		filled = (current * width) / total
	}
	if filled > width {
		filled = width
	}
	empty := width - filled
	bar := okStyle.Render(strings.Repeat("━", filled)) + dimStyle.Render(strings.Repeat("░", empty))
	return fmt.Sprintf("Step %d of %d  %s", current, total, bar)
}

func (m model) logHeight() int {
	if m.height <= 0 {
		return 15
	}
	h := m.height - 24
	if h < 6 {
		return 6
	}
	return h
}

func tailLines(lines []string, count int) []string {
	if count <= 0 || len(lines) <= count {
		return lines
	}
	return lines[len(lines)-count:]
}

func renderLogBox(lines []string, height, width int) string {
	visible := tailLines(lines, height)
	content := strings.Join(visible, "\n")
	if content == "" {
		content = dimStyle.Render("(waiting for output)")
	}
	style := logBoxStyle
	if width > 6 {
		style = style.Width(width - 6)
	}
	return style.Render(content)
}

func renderSection(title, content string, width int) string {
	style := sectionStyle
	if width > 6 {
		style = style.Width(width - 6)
	}
	header := subtitleStyle.Render(title)
	return header + "\n" + style.Render(content)
}
