package main

import (
	"bufio"
	"context"
	"fmt"
	"io"
	"os/exec"
	"strings"
	"sync"
)

type CommandExecutor interface {
	RunStream(ctx context.Context, dir, name string, args []string, onOutput func(string)) (string, error)
	Output(ctx context.Context, dir, name string, args []string) (string, error)
}

type RealExecutor struct{}

func NewRealExecutor() *RealExecutor {
	return &RealExecutor{}
}

func (e *RealExecutor) RunStream(ctx context.Context, dir, name string, args []string, onOutput func(string)) (string, error) {
	cmd := exec.CommandContext(ctx, name, args...)
	cmd.Dir = dir

	stdoutPipe, err := cmd.StdoutPipe()
	if err != nil {
		return "", fmt.Errorf("stdout pipe: %w", err)
	}
	stderrPipe, err := cmd.StderrPipe()
	if err != nil {
		return "", fmt.Errorf("stderr pipe: %w", err)
	}

	if err := cmd.Start(); err != nil {
		return "", fmt.Errorf("starting command %s %s: %w", name, strings.Join(args, " "), err)
	}

	var (
		builder strings.Builder
		mu      sync.Mutex
		wg      sync.WaitGroup
	)

	appendLine := func(line string) {
		mu.Lock()
		builder.WriteString(line)
		builder.WriteByte('\n')
		mu.Unlock()
		if onOutput != nil {
			onOutput(line)
		}
	}

	readPipe := func(r io.Reader) {
		defer wg.Done()
		scanner := bufio.NewScanner(r)
		buf := make([]byte, 0, 64*1024)
		scanner.Buffer(buf, 1024*1024)
		for scanner.Scan() {
			appendLine(scanner.Text())
		}
		if scanErr := scanner.Err(); scanErr != nil {
			appendLine("stream read error: " + scanErr.Error())
		}
	}

	wg.Add(2)
	go readPipe(stdoutPipe)
	go readPipe(stderrPipe)

	waitErr := cmd.Wait()
	wg.Wait()

	output := strings.TrimRight(builder.String(), "\n")
	if waitErr != nil {
		return output, fmt.Errorf("%s %s: %w", name, strings.Join(args, " "), waitErr)
	}
	return output, nil
}

func (e *RealExecutor) Output(ctx context.Context, dir, name string, args []string) (string, error) {
	cmd := exec.CommandContext(ctx, name, args...)
	cmd.Dir = dir
	out, err := cmd.CombinedOutput()
	output := strings.TrimRight(string(out), "\n")
	if err != nil {
		if output == "" {
			return "", fmt.Errorf("%s %s: %w", name, strings.Join(args, " "), err)
		}
		return output, fmt.Errorf("%s %s: %w: %s", name, strings.Join(args, " "), err, output)
	}
	return output, nil
}
