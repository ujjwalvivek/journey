package main

import "fmt"

const stepNameCommitVersion = "Commit version bump"

func buildCommandPipeline(startBranch, version, tag string) ([]CommandStep, error) {
	if startBranch != branchStaging && startBranch != branchMain {
		return nil, fmt.Errorf("release must start from %s or %s, got %q", branchStaging, branchMain, startBranch)
	}

	steps := []CommandStep{
		{
			Name:             "Set workspace version",
			Command:          "cargo",
			Args:             []string{"set-version", "--workspace", version},
			RequiresApproval: true,
		},
		{
			Name:             "Sync web/package.json version",
			Command:          "go",
			Args:             []string{"-C", "tools/versioning", "run", "."},
			RequiresApproval: true,
		},
		{
			Name:             "Verify version sync",
			Command:          "go",
			Args:             []string{"-C", "tools/versioning", "run", ".", "-check"},
			RequiresApproval: true,
		},
		{
			Name:             "Stage release changes",
			Command:          "git",
			Args:             []string{"add", "."},
			RequiresApproval: true,
		},
		{
			Name:             stepNameCommitVersion,
			Command:          "git",
			Args:             []string{"commit", "-m", fmt.Sprintf(versionCommitMsgFmt, version)},
			RequiresApproval: true,
		},
	}

	if startBranch == branchStaging {
		steps = append(steps, CommandStep{
			Name:             "Switch to main",
			Command:          "git",
			Args:             []string{"checkout", branchMain},
			RequiresApproval: true,
		})
		steps = append(steps, CommandStep{
			Name:             "Fast-forward main from staging",
			Command:          "git",
			Args:             []string{"merge", "--ff-only", branchStaging},
			RequiresApproval: true,
		})
	}

	steps = append(steps, CommandStep{
		Name:             "Create release tag",
		Command:          "git",
		Args:             []string{"tag", tag},
		RequiresApproval: true,
	})

	if startBranch == branchStaging {
		steps = append(steps, CommandStep{
			Name:             "Push main and staging",
			Command:          "git",
			Args:             []string{"push", "origin", branchMain, branchStaging},
			RequiresApproval: true,
		})
	} else {
		steps = append(steps, CommandStep{
			Name:             "Push main",
			Command:          "git",
			Args:             []string{"push", "origin", branchMain},
			RequiresApproval: true,
		})
	}

	steps = append(steps, CommandStep{
		Name:             "Push release tag",
		Command:          "git",
		Args:             []string{"push", "origin", tag},
		RequiresApproval: true,
	})

	return steps, nil
}
