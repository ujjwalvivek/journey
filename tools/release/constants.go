package main

import "time"

const (
	//* branchStaging and branchMain are the only allowed release starting branches.
	branchStaging = "staging"
	branchMain    = "main"

	//* defaultCITimeout is the default maximum time to wait for the publish workflow.
	defaultCITimeout = 45 * time.Minute

	//* defaultCIWatchPollInterval is the default delay between workflow status polls.
	defaultCIWatchPollInterval = 10 * time.Second

	ciWorkflowName = "publish.yml"

	//* ciRunListLimit caps the number of recent workflow runs fetched per poll.
	ciRunListLimit = "50"

	//* versionCommitMsgFmt is the commit message format used for version bump commits.
	versionCommitMsgFmt = "chore: bump engine version %s"

	//* preflightCheckTimeout is the maximum time allowed for preflight checks.
	preflightCheckTimeout = 30 * time.Second
)
