# Resonance & Cadence

This document explains how to actually use the procedural audio stack in the Journey Engine ecosystem. The stack consists of **Resonance** (the DSP physics core) and **Cadence** (the procedural sequencer logic).

Both systems are `no_std`, perform zero heap allocations, and are completely agnostic to how you output the audio. They generate and manipulate raw PCM data, which you then pass to a sink like `cpal` (for native builds) or an `AudioWorkletNode` (for the web).

---

## 1. Using Resonance (DSP Primitive)

Resonance provides raw oscillators, ADSR envelopes, and pre-configured "patches" (like Kick, Snare, Laser). It generates PCM audio frame by frame.

### Playing a Pre-Configured Patch

The easiest way to generate sound is to use `PatchVoice`, which manages its own ADSR state and oscillator math.

```rust
use resonance::patch::{Patch, PatchVoice};

// 1. Initialize a voice with the target sample rate (e.g., 44,100 Hz)
let mut voice = PatchVoice::new(44_100);

// 2. Trigger a specific sound
voice.trigger(Patch::Kick);

// 3. Inside your audio thread, poll the voice for samples
let mut buffer = [0i16; 512];
for i in 0..buffer.len() {
    if voice.is_active() {
        buffer[i] = voice.next_sample();
    } else {
        buffer[i] = 0;
    }
}
// Now pass `buffer` to your audio hardware sink
```

### Mixing Multiple Voices

Because Resonance is pure math, mixing is just adding integers together and dividing by the number of voices (or clamping/compressing).

```rust
let mut kick = PatchVoice::new(44_100);
let mut snare = PatchVoice::new(44_100);

kick.trigger(Patch::Kick);
snare.trigger(Patch::Snare);

// Mixing in a loop
let sample_k = kick.next_sample() as i32;
let sample_s = snare.next_sample() as i32;

// Mix and clamp back to i16
let mixed = (sample_k + sample_s).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
```

---

## 2. Using Cadence (Procedural Sequencer)

Cadence is a logic layer. It doesn't output audio; it tells you *when* to trigger audio based on a BPM and mathematical rulesets.

### The Transport Clock

The `Transport` is the source of truth for time. It converts human-readable BPM into discrete audio samples, ensuring zero timing drift.

```rust
use cadence::Transport;

// 120 BPM at 44,100 Hz
let mut transport = Transport::new(44_100, 120.0);

// In your audio loop, tick the transport forward one sample
if transport.tick() {
    // This block executes exactly when a 16th-note boundary is crossed
    let current_step = transport.current_step();
    println!("Step {} fired!", current_step);
}
```

### Euclidean Rhythms

Cadence uses Bjorklund's algorithm (`EuclideanPattern`) to distribute beats evenly. You map these patterns to patches.

```rust
use cadence::EuclideanPattern;

// Distribute 4 beats evenly across 16 steps (a standard 4-on-the-floor kick)
// E(pulses, steps, offset)
let kick_pattern = EuclideanPattern::<16>::new(4, 16, 0);

// Distribute 2 beats across 16 steps, offset by 4 (backbeat snare)
let snare_pattern = EuclideanPattern::<16>::new(2, 16, 4);

if transport.tick() {
    let step = transport.current_step() as usize;

    if kick_pattern.is_active(step) {
        kick_voice.trigger(Patch::Kick);
    }
    
    if snare_pattern.is_active(step) {
        snare_voice.trigger(Patch::Snare);
    }
}
```

### Markov Chains for Melody

For generative melodies, use a `MarkovChain` to pick the next note from a frequency array based on probability weights.

```rust
use cadence::MarkovChain;

// Pentatonic scale (A3 to D5)
let scale = [220.0, 261.63, 293.66, 329.63, 392.00, 440.0, 523.25, 587.33];

// 8x8 Probability matrix. Higher numbers mean higher probability of transition.
let matrix = [
    [0, 40, 10,  5,  0,  0,  0,  5], // A3 mostly goes to C4
    [20, 0, 35, 10,  5,  0,  0,  0], // C4 mostly goes to D4
    // ...
];

let mut markov = MarkovChain::new(matrix, 0 /* starting state index */);

// To get the next note, pass in a random seed (or use the built-in LFSR)
let next_state_index = markov.next(0xBEEF);
let frequency_hz = scale[next_state_index];

// Trigger your melodic synthesizer with `frequency_hz`
```

---

## 3. Putting It All Together

Here is what a complete, standalone procedural audio thread looks like using both primitives:

```rust
use cadence::{Transport, EuclideanPattern};
use resonance::patch::{Patch, PatchVoice};

pub fn audio_thread_loop(sample_rate: u32, output_buffer: &mut [i16]) {
    // 1. Initialize State
    let mut transport = Transport::new(sample_rate, 120.0);
    
    let kick_pattern = EuclideanPattern::<16>::new(4, 16, 0);
    let snare_pattern = EuclideanPattern::<16>::new(2, 16, 4);
    
    let mut kick = PatchVoice::new(sample_rate);
    let mut snare = PatchVoice::new(sample_rate);

    // 2. Fill the audio buffer
    for i in 0..output_buffer.len() {
        
        // A. Tick Logic
        if transport.tick() {
            let step = transport.current_step() as usize;
            
            if kick_pattern.is_active(step) {
                kick.trigger(Patch::Kick);
            }
            if snare_pattern.is_active(step) {
                snare.trigger(Patch::Snare);
            }
        }
        
        // B. Tick Physics
        let k_sample = kick.next_sample() as i32;
        let s_sample = snare.next_sample() as i32;
        
        // C. Mix
        output_buffer[i] = (k_sample + s_sample).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
    }
}
```

This code allocates no memory, pulls zero files from disk, and runs indefinitely. Because it operates entirely on arrays and integer math, it will effortlessly compile directly into an `AudioWorkletProcessor` for WebAssembly or stream directly into `cpal` for desktop.
