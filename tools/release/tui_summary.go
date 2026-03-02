package main

import (
	"fmt"
	"strings"
	"time"

	tea "github.com/charmbracelet/bubbletea"
)

func (m model) updateSummary(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	switch msg.String() {
	case "q", "enter":
		m.cancelCmds()
		return m, tea.Quit
	}
	return m, nil
}

func (m *model) finalizeSummary() {
	if m.summaryReady {
		return
	}

	if m.pipelineDoneAt.IsZero() {
		m.pipelineDoneAt = time.Now()
	}
	if m.finishedAt.IsZero() {
		m.finishedAt = time.Now()
	}
	if m.releaseStartedAt.IsZero() {
		m.releaseStartedAt = m.appStarted
	}

	branch, err := currentBranch(m.cmdCtx, m.executor, m.ctx.RepoRoot)
	if err == nil && strings.TrimSpace(branch) != "" {
		m.ctx.FinalBranch = strings.TrimSpace(branch)
	}

	if m.ctx.ReleaseCommitSHA != "" {
		files, insertions, deletions, diffErr := collectDiffStats(m.cmdCtx, m.executor, m.ctx.RepoRoot, m.ctx.ReleaseCommitSHA)
		if diffErr == nil {
			m.summaryStats.ChangedFiles = files
			m.summaryStats.Insertions = insertions
			m.summaryStats.Deletions = deletions
		}
	}

	stats := calculateStats(
		m.results,
		m.ciInfo,
		m.manualApprovals,
		m.autoApprovals,
		m.releaseStartedAt,
		m.preflightDoneAt,
		m.pipelineDoneAt,
		m.finishedAt,
	)

	if m.summaryStats.ChangedFiles > 0 || m.summaryStats.Insertions > 0 || m.summaryStats.Deletions > 0 {
		stats.ChangedFiles = m.summaryStats.ChangedFiles
		stats.Insertions = m.summaryStats.Insertions
		stats.Deletions = m.summaryStats.Deletions
	}

	m.summaryStats = stats
	m.summaryReady = true
}

func (m model) viewSummary() string {
	w := m.frameWidth()
	var b strings.Builder

	success := overallReleaseSuccess(m.results, m.ciInfo, m.ciErr)
	badge := okStyle.Render("✓ SUCCESS")
	if !success {
		badge = errStyle.Render("✗ FAILED")
	}
	b.WriteString("  " + badge + "\n\n")

	var release strings.Builder
	release.WriteString(fmt.Sprintf("  tag       %s\n", m.ctx.Tag))
	release.WriteString(fmt.Sprintf("  version   %s\n", m.ctx.SelectedVersion))
	release.WriteString(fmt.Sprintf("  branch    %s → %s\n", m.ctx.StartBranch, m.ctx.FinalBranch))
	if m.ctx.ReleaseCommitSHA != "" {
		short := m.ctx.ReleaseCommitSHA
		if len(short) > 8 {
			short = short[:8]
		}
		release.WriteString(fmt.Sprintf("  commit    %s\n", short))
	}
	b.WriteString(renderSection("Release", release.String(), w))
	b.WriteString("\n")

	s := m.summaryStats
	var timing strings.Builder
	timing.WriteString(fmt.Sprintf("  total      %s\n", s.TotalDuration.Truncate(time.Millisecond)))
	timing.WriteString(fmt.Sprintf("  preflight  %s\n", s.PreflightDuration.Truncate(time.Millisecond)))
	timing.WriteString(fmt.Sprintf("  pipeline   %s\n", s.PipelineDuration.Truncate(time.Millisecond)))
	timing.WriteString(fmt.Sprintf("  ci watch   %s\n", s.CIDuration.Truncate(time.Millisecond)))
	b.WriteString(renderSection("Timing", timing.String(), w))
	b.WriteString("\n")

	var pipeline strings.Builder
	pipeline.WriteString(fmt.Sprintf("  planned    %d\n", s.PlannedCommands))
	pipeline.WriteString(fmt.Sprintf("  executed   %d\n", s.ExecutedCommands))
	pipeline.WriteString(fmt.Sprintf("  failed     %d\n", s.FailedCommands))
	pipeline.WriteString(fmt.Sprintf("  skipped    %d\n", s.SkippedCommands))
	pipeline.WriteString(fmt.Sprintf("  approvals  %d manual  %d auto\n", s.ManualApprovals, s.AutoApprovals))
	if s.ChangedFiles > 0 || s.Insertions > 0 || s.Deletions > 0 {
		pipeline.WriteString(fmt.Sprintf("  diff       %d files  +%d  -%d\n", s.ChangedFiles, s.Insertions, s.Deletions))
	}
	b.WriteString(renderSection("Pipeline", pipeline.String(), w))
	b.WriteString("\n")

	if s.WorkflowJobsTotal > 0 || m.ciInfo.URL != "" {
		var ci strings.Builder
		ci.WriteString(fmt.Sprintf("  jobs       %d total  %d passed  %d failed\n", s.WorkflowJobsTotal, s.WorkflowJobsPassed, s.WorkflowJobsFailed))
		if m.ciInfo.URL != "" {
			ci.WriteString(fmt.Sprintf("  url        %s\n", m.ciInfo.URL))
		}
		b.WriteString(renderSection("CI", ci.String(), w))
		b.WriteString("\n")
	}

	if tagURL := releaseTagURL(m.ctx.RepoWebURL, m.ctx.Tag); tagURL != "" {
		b.WriteString("  " + dimStyle.Render("release ") + tagURL + "\n")
	}

	if m.exitErr != nil {
		b.WriteString("\n  " + errStyle.Render("error: "+m.exitErr.Error()) + "\n")
	}

	return m.renderFrame("Summary", b.String(), "enter/q exit")
}
