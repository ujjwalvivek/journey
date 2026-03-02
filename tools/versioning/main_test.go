package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

const testCargoTOML = `
[workspace]
members = ["engine", "game"]

[workspace.package]
version = "0.3.2"
edition = "2024"
`

func TestParseWorkspaceVersion(t *testing.T) {
	version, err := parseWorkspaceVersion([]byte(testCargoTOML))
	if err != nil {
		t.Fatalf("parseWorkspaceVersion error: %v", err)
	}
	if version != "0.3.2" {
		t.Fatalf("expected 0.3.2, got %s", version)
	}
}

func TestParseWorkspaceVersionMissing(t *testing.T) {
	_, err := parseWorkspaceVersion([]byte(`[workspace]`))
	if err == nil {
		t.Fatalf("expected error for missing workspace.package.version")
	}
}

func TestReadWritePackageVersion(t *testing.T) {
	dir := t.TempDir()
	pkgPath := filepath.Join(dir, "package.json")

	initial := map[string]any{
		"name":    "my-app",
		"version": "0.1.0",
		"private": true,
	}
	b, _ := json.MarshalIndent(initial, "", "  ")
	if err := os.WriteFile(pkgPath, append(b, '\n'), 0644); err != nil {
		t.Fatalf("write setup: %v", err)
	}

	if err := writePackageVersion(pkgPath, "0.3.2"); err != nil {
		t.Fatalf("writePackageVersion error: %v", err)
	}

	got, err := readPackageVersion(pkgPath)
	if err != nil {
		t.Fatalf("readPackageVersion error: %v", err)
	}
	if got != "0.3.2" {
		t.Fatalf("expected 0.3.2, got %s", got)
	}

	//? Verify unrelated fields are preserved.
	raw, _ := os.ReadFile(pkgPath)
	var out map[string]any
	if err := json.Unmarshal(raw, &out); err != nil {
		t.Fatalf("unmarshal after write: %v", err)
	}
	if out["name"] != "my-app" {
		t.Fatalf("unexpected name field after write: %v", out["name"])
	}
}

func TestReadWorkspaceVersionFromFile(t *testing.T) {
	dir := t.TempDir()
	cargoPath := filepath.Join(dir, "Cargo.toml")
	if err := os.WriteFile(cargoPath, []byte(testCargoTOML), 0644); err != nil {
		t.Fatalf("write: %v", err)
	}

	version, err := readWorkspaceVersion(cargoPath)
	if err != nil {
		t.Fatalf("readWorkspaceVersion error: %v", err)
	}
	if version != "0.3.2" {
		t.Fatalf("expected 0.3.2, got %s", version)
	}
}
