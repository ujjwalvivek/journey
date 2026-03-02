# Journey Engine: Crates.io README (@ujjwalvivek/journey-engine)

A cross-platform 2D game engine built with Rust and wGPU.

[![Crates.io](https://img.shields.io/crates/v/journey-engine.svg?style=for-the-badge&logo=rust&logoColor=white&label=crate)](https://crates.io/crates/journey-engine)
[![Docs](https://img.shields.io/badge/docs-journey.ujjwalvivek.com-blue?style=for-the-badge&logo=rust&logoColor=white&label=docs)](https://docs.journey.ujjwalvivek.com)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg?style=for-the-badge&logo=github&logoColor=white&label=license)](LICENSE)

## Features

- **Cross-platform rendering** with wGPU
- **WebAssembly support** with zero dependencies on the game code
- **Generic action-based input** mapping for keyboard, mouse, and gamepad
- **Fixed-timestep physics** with swept AABB collision detection and collision layers
- **Instanced sprite rendering** with spritesheet support and blend modes
- **Asset-agnostic animation** state machine with frame timing and looping
- **Cross-platform audio** via Kira with music, ambience, SFX, and UI sub-tracks
- **Orthographic camera** with screen shake
- **egui integration** for debug UI overlays
- **Configurable internal resolution** (default 640×360, nearest-neighbor upscale)

## Quick Start

Add `journey-engine` to your `Cargo.toml`:

```toml
[dependencies]
engine = { package = "journey-engine", version = "1.0.0" }
```

Implement the `GameApp` trait:

```rust
use engine::{Context, GameAction, InputState, Key, InputMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Action {
    MoveLeft,
    MoveRight,
    Jump,
}

impl GameAction for Action {}

struct MyGame;

impl engine::GameApp for MyGame {
    type Action = Action;

    fn init(ctx: &mut Context<Action>) -> Self {
        ctx.input.map.bind_key(Key::ArrowLeft, Action::MoveLeft);
        ctx.input.map.bind_key(Key::ArrowRight, Action::MoveRight);
        ctx.input.map.bind_key(Key::Space, Action::Jump);
        MyGame
    }

    fn update(&mut self, ctx: &mut Context<Action>) {
        if ctx.input.is_action_just_pressed(Action::Jump) {
            //? Handle jump
        }
    }

    fn render(&mut self, ctx: &mut Context<Action>) {
        //? Draw sprites via ctx.draw_sprite()
    }
}

fn main() {
    engine::run::<MyGame>();
}
```

## Architecture

Games implement the `GameApp` trait to hook into the engine lifecycle:

```bash
init → fixed_update → update → render → ui
```

All mutable state flows through a single `Context<A>` struct where `A` is your game-specific action enum.

## Documentation

- [API Reference](https://docs.journey.ujjwalvivek.com)
- [Engine API Guide](https://github.com/ujjwalvivek/journey/blob/main/docs/src/ENGINE_API.md)
- [Technical Documentation](https://github.com/ujjwalvivek/journey/blob/main/docs/src/TECHNICAL_DOCUMENTATION.md)

## License

MIT License. See [LICENSE](LICENSE) for details.
