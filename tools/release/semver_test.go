package main

import "testing"

func TestDeriveVersionOptions(t *testing.T) {
	opts, err := deriveVersionOptions("0.3.2")
	if err != nil {
		t.Fatalf("deriveVersionOptions returned error: %v", err)
	}

	if opts.Patch != "0.3.3" {
		t.Fatalf("expected patch 0.3.3, got %s", opts.Patch)
	}
	if opts.Minor != "0.4.0" {
		t.Fatalf("expected minor 0.4.0, got %s", opts.Minor)
	}
	if opts.Major != "1.0.0" {
		t.Fatalf("expected major 1.0.0, got %s", opts.Major)
	}
}

func TestNormalizeVersionInput(t *testing.T) {
	version, tag, err := normalizeVersionInput("v1.2.3")
	if err != nil {
		t.Fatalf("normalizeVersionInput returned error: %v", err)
	}
	if version != "1.2.3" || tag != "v1.2.3" {
		t.Fatalf("unexpected normalization output: version=%s tag=%s", version, tag)
	}

	version, tag, err = normalizeVersionInput("1.2.4")
	if err != nil {
		t.Fatalf("normalizeVersionInput returned error: %v", err)
	}
	if version != "1.2.4" || tag != "v1.2.4" {
		t.Fatalf("unexpected normalization output: version=%s tag=%s", version, tag)
	}

	if _, _, err := normalizeVersionInput("1.2"); err == nil {
		t.Fatalf("expected invalid semver error")
	}
}
