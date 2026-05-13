# Cadence

A mathematically driven, zero-allocation procedural sequencing primitive in Rust.

Cadence does not synthesize audio or parse MIDI files. It is a `no_std` state machine that counts audio samples and executes deterministic algorithms (Euclidean rhythms, Markov chains, LFSRs) to trigger events in real-time.

It is designed to sit directly on top of audio synthesis primitives like [Resonance](../resonance).

## Architecture

- **Zero Allocation**: No `Vec`, no heap usage. All sequences, transition matrices, and state tracking use `const` generics and fixed-size stack arrays.
- **Sample-Accurate Time**: Time is tracked by counting discrete audio samples (`Transport`), ensuring mathematically zero drift indefinitely.
- **Deterministic Output**: Given the same seed, the LFSR and Markov chains generate the exact same sequences on every platform.
- **Rhythm & Melody**: Implements Bjorklund's algorithm (`EuclideanPattern`) for rhythms and weighted `MarkovChain` states for melodies.

## Integration

Cadence exposes a state machine polled by your audio thread.

```rust
use cadence::{Transport, EuclideanPattern};

let mut transport = Transport::new(44_100, 120.0);
let kick_rhythm = EuclideanPattern::<16>::new(4, 16, 0); // 4-on-the-floor

// Inside your core audio rendering loop
for i in 0..BUFFER_SIZE {
    // Tick the sequencer clock
    if transport.tick() {
        let step = transport.current_step();
        
        // Query the mathematical rulesets
        if kick_rhythm.is_active(step) {
            // Trigger audio patch here
        }
    }
}
```

Deployed alongside Resonance in the Journey Engine Web Build and TUI environments.
