package main

import (
	"context"
	"testing"
	"time"
)

func TestRunHeadlessWithStubExecutor(t *testing.T) {
	stub := &stubExecutor{}
	stub.outputFn = func(ctx context.Context, dir, name string, args []string) (string, error) {
		switch name {
		case "git":
			if len(args) > 0 {
				switch args[0] {
				case "rev-parse":
					return "main", nil
				case "status":
					return "", nil
				case "tag", "checkout", "merge", "push", "add", "commit":
					return "", nil
				case "rev-list":
					return "abc123", nil
				}
			}
		case "gh":
			if len(args) > 0 {
				switch args[0] {
				case "--version", "auth":
					return "ok", nil
				case "run":
					//? First list call includes a run matching the tag SHA so the
					//? monitor exits immediately without polling.
					if len(args) > 1 && args[1] == "list" {
						return `[{"databaseId":1,"headSha":"abc123","url":"http://example.com","name":"publish","status":"completed","conclusion":"success","createdAt":"","updatedAt":""}]`, nil
					}
					return "[]", nil
				}
			}
		}
		return "", nil
	}
	stub.runFn = func(ctx context.Context, dir, name string, args []string, onOutput func(string)) (string, error) {
		if onOutput != nil {
			onOutput("ok")
		}
		return "ok", nil
	}

	err := runHeadlessWithExecutor(stub, "0.4.0", ApprovalZen, 1*time.Minute, false, false, false)
	if err != nil {
		t.Fatalf("headless run failed: %v", err)
	}
}
