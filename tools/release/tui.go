package main

import (
	"context"
	"fmt"
	"time"

	"github.com/charmbracelet/bubbles/spinner"
	"github.com/charmbracelet/bubbles/textinput"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
)

type screen int

const (
	screenLoading screen = iota
	screenWelcome
	screenVersion
	screenMode
	screenExecution
	screenCI
	screenSummary
)

const (
	appTitle           = "Release Terminal v0.1.0"
	maxPipelineLogs    = 1200
	maxCILogs          = 3000
	pollRunnerInterval = 120 * time.Millisecond
	pollCIInterval     = 200 * time.Millisecond
)

type preflightLoadedMsg struct {
	ctx    ReleaseContext
	checks []PreflightCheck
	err    error
}

type tagCheckedMsg struct {
	version string
	tag     string
	exists  bool
	err     error
}

type executionTickMsg struct{}
type pollRunnerMsg struct{}
type pollCIMsg struct{}

type runnerLogEvent struct{ line string }
type runnerDoneEvent struct {
	output string
	err    error
	ended  time.Time
}
type ciLogEvent struct{ line string }
type ciDoneEvent struct {
	info WorkflowRunInfo
	err  error
}

var (
	accentColor  = lipgloss.Color("39")
	successColor = lipgloss.Color("42")
	errorColor   = lipgloss.Color("196")
	warnColor    = lipgloss.Color("214")
	dimColor     = lipgloss.Color("244")
	mutedColor   = lipgloss.Color("241")

	titleStyle    = lipgloss.NewStyle().Bold(true).Foreground(accentColor)
	subtitleStyle = lipgloss.NewStyle().Bold(true).Foreground(mutedColor)
	okStyle       = lipgloss.NewStyle().Foreground(successColor)
	errStyle      = lipgloss.NewStyle().Foreground(errorColor)
	warnStyle     = lipgloss.NewStyle().Foreground(warnColor)
	dimStyle      = lipgloss.NewStyle().Foreground(dimColor)
	selectedStyle = lipgloss.NewStyle().Bold(true).Foreground(accentColor)
	boxStyle      = lipgloss.NewStyle().Border(lipgloss.RoundedBorder()).Padding(1, 2).Margin(0, 0)
	logBoxStyle   = lipgloss.NewStyle().Border(lipgloss.RoundedBorder()).BorderForeground(dimColor).Padding(0, 1)
	sectionStyle  = lipgloss.NewStyle().Border(lipgloss.RoundedBorder()).BorderForeground(mutedColor).Padding(0, 2)
)

type model struct {
	screen     screen
	width      int
	height     int
	ciTimeout  time.Duration
	executor   CommandExecutor
	cmdCtx     context.Context
	cancelCmds context.CancelFunc
	appStarted time.Time
	spinner    spinner.Model

	dryRun     bool
	skipCI     bool
	allowDirty bool

	ctx          ReleaseContext
	checks       []PreflightCheck
	preflightOK  bool
	preflightErr error

	preflightDoneAt  time.Time
	releaseStartedAt time.Time
	pipelineDoneAt   time.Time
	finishedAt       time.Time

	versionOptions  VersionOptions
	versionIndex    int
	versionErr      string
	checkingTag     bool
	selectedVersion string
	selectedTag     string
	customInput     textinput.Model
	enteringCustom  bool

	modeIndex int

	steps            []CommandStep
	results          []StepResult
	currentStep      int
	awaitingApproval bool
	runningStep      bool
	manualApprovals  int
	autoApprovals    int
	logLines         []string
	runnerEvents     chan any

	ciRunning  bool
	ciInfo     WorkflowRunInfo
	ciErr      error
	ciLogLines []string
	ciEvents   chan any

	summaryStats ReleaseStats
	summaryReady bool
	exitErr      error
}

func newModel(ciTimeout time.Duration) model {
	ti := textinput.New()
	ti.Placeholder = "e.g. 1.0.0"
	ti.CharLimit = 64
	ti.Prompt = "  version: "

	s := spinner.New()
	s.Spinner = spinner.Dot
	s.Style = lipgloss.NewStyle().Foreground(accentColor)

	cmdCtx, cancel := context.WithCancel(context.Background())
	return model{
		screen:       screenLoading,
		ciTimeout:    ciTimeout,
		executor:     NewRealExecutor(),
		cmdCtx:       cmdCtx,
		cancelCmds:   cancel,
		appStarted:   time.Now(),
		spinner:      s,
		versionIndex: 0,
		modeIndex:    0,
		customInput:  ti,
		logLines:     make([]string, 0, 512),
		ciLogLines:   make([]string, 0, 2048),
	}
}

func (m model) Init() tea.Cmd {
	return tea.Batch(m.spinner.Tick, loadPreflightCmd(m.executor, m.ciTimeout))
}

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height
		return m, nil

	case spinner.TickMsg:
		if m.screen == screenLoading {
			var cmd tea.Cmd
			m.spinner, cmd = m.spinner.Update(msg)
			return m, cmd
		}
		return m, nil

	case preflightLoadedMsg:
		m.ctx = msg.ctx
		m.checks = msg.checks
		m.preflightErr = msg.err
		m.preflightOK = msg.err == nil && preflightOKWith(msg.checks, m.allowDirty)
		m.preflightDoneAt = time.Now()
		m.screen = screenWelcome
		return m, nil

	case tagCheckedMsg:
		m.checkingTag = false
		if msg.err != nil {
			m.versionErr = msg.err.Error()
			return m, nil
		}
		if msg.exists {
			m.versionErr = "tag already exists: " + msg.tag
			return m, nil
		}
		m.versionErr = ""
		m.selectedVersion = msg.version
		m.selectedTag = msg.tag
		m.ctx.SelectedVersion = msg.version
		m.ctx.Tag = msg.tag
		m.screen = screenMode
		return m, nil

	case executionTickMsg:
		if m.screen != screenExecution {
			return m, nil
		}
		if m.runningStep || m.awaitingApproval {
			return m, nil
		}
		if m.currentStep >= len(m.steps) {
			if m.pipelineDoneAt.IsZero() {
				m.pipelineDoneAt = time.Now()
			}
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

		if m.dryRun {
			res := m.results[m.currentStep]
			res.Status = StepStatusSkipped
			res.Approved = false
			m.results[m.currentStep] = res
			m.currentStep++
			return m, executionTickCmd()
		}

		if m.ctx.Mode == ApprovalPrompt {
			m.awaitingApproval = true
			res := m.results[m.currentStep]
			res.Status = StepStatusWaitingApproval
			m.results[m.currentStep] = res
			return m, nil
		}

		m.autoApprovals++
		return m, m.beginStep()

	case pollRunnerMsg:
		return m.handleRunnerEvents()

	case pollCIMsg:
		return m.handleCIEvents()

	case tea.KeyMsg:
		if msg.String() == "ctrl+c" {
			m.cancelCmds()
			if m.screen != screenSummary && m.exitErr == nil {
				m.exitErr = fmt.Errorf("aborted by user")
			}
			return m, tea.Quit
		}

		switch m.screen {
		case screenWelcome:
			return m.updateWelcome(msg)
		case screenVersion:
			return m.updateVersion(msg)
		case screenMode:
			return m.updateMode(msg)
		case screenExecution:
			return m.updateExecution(msg)
		case screenCI:
			return m.updateCI(msg)
		case screenSummary:
			return m.updateSummary(msg)
		}
	}

	return m, nil
}

func (m model) View() string {
	switch m.screen {
	case screenLoading:
		return m.viewLoading()
	case screenWelcome:
		return m.viewWelcome()
	case screenVersion:
		return m.viewVersion()
	case screenMode:
		return m.viewMode()
	case screenExecution:
		return m.viewExecution()
	case screenCI:
		return m.viewCI()
	case screenSummary:
		return m.viewSummary()
	default:
		return "unknown screen"
	}
}
