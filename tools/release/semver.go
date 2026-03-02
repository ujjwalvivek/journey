package main

import (
	"fmt"
	"strconv"
	"strings"

	"golang.org/x/mod/semver"
)

type VersionOptions struct {
	Current string
	Patch   string
	Minor   string
	Major   string
}

func normalizeVersionInput(input string) (version string, tag string, err error) {
	v := strings.TrimSpace(input)
	if v == "" {
		return "", "", fmt.Errorf("version is required")
	}
	if !strings.HasPrefix(v, "v") {
		v = "v" + v
	}
	if !semver.IsValid(v) {
		return "", "", fmt.Errorf("invalid semver: %q", input)
	}

	normalized := strings.TrimPrefix(v, "v")
	if _, _, _, err := parseCoreSemver(normalized); err != nil {
		return "", "", fmt.Errorf("version must include major.minor.patch: %w", err)
	}
	return normalized, v, nil
}

func deriveVersionOptions(current string) (VersionOptions, error) {
	normalized, _, err := normalizeVersionInput(current)
	if err != nil {
		return VersionOptions{}, err
	}

	major, minor, patch, err := parseCoreSemver(normalized)
	if err != nil {
		return VersionOptions{}, err
	}

	return VersionOptions{
		Current: normalized,
		Patch:   fmt.Sprintf("%d.%d.%d", major, minor, patch+1),
		Minor:   fmt.Sprintf("%d.%d.0", major, minor+1),
		Major:   fmt.Sprintf("%d.0.0", major+1),
	}, nil
}

func parseCoreSemver(version string) (major int, minor int, patch int, err error) {
	core := strings.TrimSpace(version)
	core = strings.TrimPrefix(core, "v")
	if idx := strings.IndexAny(core, "-+"); idx >= 0 {
		core = core[:idx]
	}

	parts := strings.Split(core, ".")
	if len(parts) != 3 {
		return 0, 0, 0, fmt.Errorf("invalid core semver: %q", version)
	}

	maj, err := strconv.Atoi(parts[0])
	if err != nil {
		return 0, 0, 0, fmt.Errorf("invalid major version %q: %w", parts[0], err)
	}
	min, err := strconv.Atoi(parts[1])
	if err != nil {
		return 0, 0, 0, fmt.Errorf("invalid minor version %q: %w", parts[1], err)
	}
	pat, err := strconv.Atoi(parts[2])
	if err != nil {
		return 0, 0, 0, fmt.Errorf("invalid patch version %q: %w", parts[2], err)
	}
	return maj, min, pat, nil
}
