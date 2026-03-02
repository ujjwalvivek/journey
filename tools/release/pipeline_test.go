package main

import "testing"

func TestBuildCommandPipelineStaging(t *testing.T) {
	steps, err := buildCommandPipeline("staging", "0.3.3", "v0.3.3")
	if err != nil {
		t.Fatalf("buildCommandPipeline returned error: %v", err)
	}

	if len(steps) != 10 {
		t.Fatalf("expected 10 steps for staging flow, got %d", len(steps))
	}

	if steps[5].Display() != "git checkout main" {
		t.Fatalf("unexpected step 6: %s", steps[5].Display())
	}
	if steps[6].Display() != "git merge --ff-only staging" {
		t.Fatalf("unexpected step 7: %s", steps[6].Display())
	}
	if steps[8].Display() != "git push origin main staging" {
		t.Fatalf("unexpected step 9: %s", steps[8].Display())
	}
}

func TestBuildCommandPipelineMain(t *testing.T) {
	steps, err := buildCommandPipeline("main", "0.3.3", "v0.3.3")
	if err != nil {
		t.Fatalf("buildCommandPipeline returned error: %v", err)
	}

	if len(steps) != 8 {
		t.Fatalf("expected 8 steps for main flow, got %d", len(steps))
	}
	if steps[6].Display() != "git push origin main" {
		t.Fatalf("unexpected step 7: %s", steps[6].Display())
	}
}
