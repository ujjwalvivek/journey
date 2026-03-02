package main

import (
	"context"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"

	toml "github.com/pelletier/go-toml/v2"
)

type cargoDoc struct {
	Workspace struct {
		Package struct {
			Version string `toml:"version"`
		} `toml:"package"`
	} `toml:"workspace"`
}

func findRepoRoot(start string) (string, error) {
	dir := start
	for {
		cargoPath := filepath.Join(dir, "Cargo.toml")
		if st, err := os.Stat(cargoPath); err == nil && !st.IsDir() {
			return dir, nil
		}

		parent := filepath.Dir(dir)
		if parent == dir {
			break
		}
		dir = parent
	}
	return "", errors.New("could not find repo root (Cargo.toml)")
}

func readWorkspaceVersion(cargoPath string) (string, error) {
	b, err := os.ReadFile(cargoPath)
	if err != nil {
		return "", err
	}
	var doc cargoDoc
	if err := toml.Unmarshal(b, &doc); err != nil {
		return "", err
	}
	if doc.Workspace.Package.Version == "" {
		return "", fmt.Errorf("workspace.package.version not found in %s", cargoPath)
	}
	return strings.TrimSpace(doc.Workspace.Package.Version), nil
}

func runPreflight(ctx context.Context, executor CommandExecutor, ciTimeout time.Duration) (ReleaseContext, []PreflightCheck, error) {
	cwd, err := os.Getwd()
	if err != nil {
		return ReleaseContext{}, nil, err
	}

	repoRoot, err := findRepoRoot(cwd)
	if err != nil {
		return ReleaseContext{}, nil, err
	}

	version, err := readWorkspaceVersion(filepath.Join(repoRoot, "Cargo.toml"))
	if err != nil {
		return ReleaseContext{}, nil, err
	}

	branchOut, err := executor.Output(ctx, repoRoot, "git", []string{"rev-parse", "--abbrev-ref", "HEAD"})
	if err != nil {
		return ReleaseContext{}, nil, err
	}
	branch := strings.TrimSpace(branchOut)

	checks := make([]PreflightCheck, 0, 5)

	statusOut, statusErr := executor.Output(ctx, repoRoot, "git", []string{"status", "--porcelain"})
	if statusErr != nil {
		checks = append(checks, PreflightCheck{
			Name:      "Working tree clean",
			OK:        false,
			Skippable: true,
			Detail:    statusErr.Error(),
		})
	} else if strings.TrimSpace(statusOut) != "" {
		checks = append(checks, PreflightCheck{
			Name:      "Working tree clean",
			OK:        false,
			Skippable: true,
			Detail:    "repository has uncommitted changes",
		})
	} else {
		checks = append(checks, PreflightCheck{
			Name:   "Working tree clean",
			OK:     true,
			Detail: "no pending changes",
		})
	}

	allowed := branch == branchStaging || branch == branchMain
	if allowed {
		checks = append(checks, PreflightCheck{
			Name:   "Release branch",
			OK:     true,
			Detail: branch,
		})
	} else {
		checks = append(checks, PreflightCheck{
			Name:   "Release branch",
			OK:     false,
			Detail: fmt.Sprintf("expected %s or %s, got %s", branchStaging, branchMain, branch),
		})
	}

	if _, ghVersionErr := executor.Output(ctx, repoRoot, "gh", []string{"--version"}); ghVersionErr != nil {
		checks = append(checks, PreflightCheck{
			Name:   "GitHub CLI available",
			OK:     false,
			Detail: ghVersionErr.Error(),
		})
	} else {
		checks = append(checks, PreflightCheck{
			Name:   "GitHub CLI available",
			OK:     true,
			Detail: "gh detected",
		})
	}

	if _, ghAuthErr := executor.Output(ctx, repoRoot, "gh", []string{"auth", "status"}); ghAuthErr != nil {
		checks = append(checks, PreflightCheck{
			Name:   "GitHub CLI authenticated",
			OK:     false,
			Detail: ghAuthErr.Error(),
		})
	} else {
		checks = append(checks, PreflightCheck{
			Name:   "GitHub CLI authenticated",
			OK:     true,
			Detail: "authenticated",
		})
	}

	if ciTimeout > 0 {
		checks = append(checks, PreflightCheck{
			Name:   "CI timeout",
			OK:     true,
			Detail: ciTimeout.String(),
		})
	}

	repoWebURL, _ := resolveRepositoryWebURL(ctx, executor, repoRoot)

	return ReleaseContext{
		RepoRoot:         repoRoot,
		RepoWebURL:       repoWebURL,
		CurrentVersion:   version,
		StartBranch:      branch,
		CITimeout:        ciTimeout,
		SelectedVersion:  "",
		Tag:              "",
		Mode:             "",
		ReleaseCommitSHA: "",
		FinalBranch:      branch,
	}, checks, nil
}

func preflightPassed(checks []PreflightCheck) bool {
	return preflightOKWith(checks, false)
}

func preflightOKWith(checks []PreflightCheck, allowDirty bool) bool {
	for _, check := range checks {
		if !check.OK && !(allowDirty && check.Skippable) {
			return false
		}
	}
	return true
}

func tagExists(ctx context.Context, executor CommandExecutor, repoRoot, tag string) (bool, error) {
	out, err := executor.Output(ctx, repoRoot, "git", []string{"tag", "--list", tag})
	if err != nil {
		return false, err
	}

	for _, line := range strings.Split(out, "\n") {
		if strings.TrimSpace(line) == tag {
			return true, nil
		}
	}
	return false, nil
}

func currentBranch(ctx context.Context, executor CommandExecutor, repoRoot string) (string, error) {
	out, err := executor.Output(ctx, repoRoot, "git", []string{"rev-parse", "--abbrev-ref", "HEAD"})
	if err != nil {
		return "", err
	}
	return strings.TrimSpace(out), nil
}

func resolveRepositoryWebURL(ctx context.Context, executor CommandExecutor, repoRoot string) (string, error) {
	out, err := executor.Output(ctx, repoRoot, "git", []string{"remote", "get-url", "origin"})
	if err != nil {
		return "", err
	}

	origin := strings.TrimSpace(out)
	switch {
	case strings.HasPrefix(origin, "git@github.com:"):
		path := strings.TrimPrefix(origin, "git@github.com:")
		path = strings.TrimSuffix(path, ".git")
		return "https://github.com/" + path, nil
	case strings.HasPrefix(origin, "https://github.com/"):
		return strings.TrimSuffix(origin, ".git"), nil
	case strings.HasPrefix(origin, "http://github.com/"):
		url := strings.TrimPrefix(origin, "http://")
		url = strings.TrimSuffix(url, ".git")
		return "https://" + url, nil
	default:
		return "", fmt.Errorf("unsupported remote URL format: %s", origin)
	}
}
