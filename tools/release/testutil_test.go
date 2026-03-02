package main

import (
	"context"
	"sync"
)

type stubExecutor struct {
	mu sync.Mutex

	runFn    func(ctx context.Context, dir, name string, args []string, onOutput func(string)) (string, error)
	outputFn func(ctx context.Context, dir, name string, args []string) (string, error)
}

func (s *stubExecutor) RunStream(ctx context.Context, dir, name string, args []string, onOutput func(string)) (string, error) {
	s.mu.Lock()
	fn := s.runFn
	s.mu.Unlock()
	if fn == nil {
		return "", nil
	}
	return fn(ctx, dir, name, args, onOutput)
}

func (s *stubExecutor) Output(ctx context.Context, dir, name string, args []string) (string, error) {
	s.mu.Lock()
	fn := s.outputFn
	s.mu.Unlock()
	if fn == nil {
		return "", nil
	}
	return fn(ctx, dir, name, args)
}
