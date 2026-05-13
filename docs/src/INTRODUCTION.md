# Journey Engine

A custom 2D game engine built with Rust and wGPU, designed for tight, expressive platformers. Powers [Journey](https://journey.ujjwalvivek.com).

## What's Here

| Section                                               | What you'll find                                                                                          |
| ----------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| [Engine API](ENGINE_API.md)                           | Public API reference with usage examples: `GameApp`, `Context`, input, physics, sprites, audio, animation |
| [Technical Documentation](TECHNICAL_DOCUMENTATION.md) | Architecture internals, game loop model, rendering pipeline, cross-platform strategy, dependency map      |
| [Procedural Audio](AUDIO.md)                          | Guides and examples for using the custom Resonance (`no_std` DSP) and Cadence (Sequencer) audio stack     |

## Quick Links

- **Play** → [journey.ujjwalvivek.com](https://journey.ujjwalvivek.com)
- **Engine Crate** → [crates.io/crates/journey-engine](https://crates.io/crates/journey-engine)
- **Audio Crate** → [crates.io/crates/journey-audio](https://crates.io/crates/journey-audio)
- **Synth Crate** → [crates.io/crates/journey-synthesizer](https://crates.io/crates/journey-synthesizer)
- **Source** → [github.com/ujjwalvivek/journey](https://github.com/ujjwalvivek/journey)

## At a Glance

```toml
[dependencies]
journey-engine = "1.2.0"
journey-audio = "1.2.0"
journey-synthesizer = "1.2.0"
```

```rust
use engine::{Context, GameAction, GameApp};

struct MyGame;

impl GameApp for MyGame {
    type Action = MyAction;
    fn init(ctx: &mut Context<MyAction>) -> Self { MyGame }
    fn update(&mut self, ctx: &mut Context<MyAction>) {}
    fn fixed_update(&mut self, ctx: &mut Context<MyAction>) {}
    fn render(&mut self, ctx: &mut Context<MyAction>) {}
}
```

See [Engine API → Quick Start](ENGINE_API.md#quick-start) for the full minimal example.
