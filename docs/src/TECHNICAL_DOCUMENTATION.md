# Journey Engine Technical Documentation

> **Version:** 1.1.2 · **Edition:** Rust 2024 · **License:** MIT
> **Repository:** [github.com/ujjwalvivek/journey](https://github.com/ujjwalvivek/journey)

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Workspace Layout](#workspace-layout)
4. [Build Targets](#build-targets)
5. [Core Engine Systems](#core-engine-systems)
6. [Game Layer](#game-layer)
7. [Game Loop and Timing Model](#game-loop-and-timing-model)
8. [Rendering Pipeline](#rendering-pipeline)
9. [Physics and Collision](#physics-and-collision)
10. [Input System](#input-system)
11. [Audio System](#audio-system)
12. [Animation System](#animation-system)
13. [Combat System](#combat-system)
14. [Level System](#level-system)
15. [Configuration and Tuning](#configuration-and-tuning)
16. [Cross-Platform Strategy](#cross-platform-strategy)
17. [Performance Budget](#performance-budget)
18. [Dependency Map](#dependency-map)

---

## Overview

Journey Engine is a custom high-performance 2D game engine written in Rust and wGPU. It targets both native desktop (Vulkan/Metal/DX12) and WebAssembly (WebGL/WebGPU) from a single codebase. The engine is purpose-built for a fast-momentum Metroidvania tech demo that prioritizes tight, deterministic, arcade-style physics over realistic simulation.

The project ships as a Cargo workspace with two crates, a reusable `journey-engine` library (published as `journey-engine` on crates.io) and a `game` binary, plus a Vite-based web project for WASM distribution. This separation enforces a clean API boundary: the engine owns rendering, input infrastructure, audio, physics primitives, and the game loop; the game owns content, gameplay logic, level design, input action definitions, and asset definitions.

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    game (binary crate)                  │
│  Player · Enemies · Combat FSM · Levels · Assets · UI   │
├─────────────────────────────────────────────────────────┤
│                  engine (library crate)                 │
│  GameApp trait · Context · Renderer · Input · Physics   │
│  Audio · Animation · Camera · Texture · Time · Atmosphere│
├─────────────────────────────────────────────────────────┤
│              Platform Abstraction Layer                 │
│  wgpu (GPU) · winit (Windowing) · kira (Audio)          │
│  gilrs (Gamepad, native-only) · egui (Debug UI)         │
└─────────────────────────────────────────────────────────┘
```

**Design philosophy:** Data-oriented, trait-driven, and deliberately narrow. Games implement the `GameApp` trait to hook into the engine's lifecycle. The engine calls `init → fixed_update → update → render → ui` each frame. All mutable engine access flows through a single `Context` struct.

---

## Workspace Layout

```
Journey/
├── Cargo.toml              # Workspace root (members: engine, game)
├── engine/                  # Reusable library - the product
│   ├── Cargo.toml           # wgpu, winit, kira, egui, glam, bytemuck
│   ├── assets/shaders/      # WGSL shaders (sprite, atmosphere)
│   └── src/
│       ├── lib.rs           # Public API surface, GameApp trait, re-exports
│       ├── runtime.rs       # Event loop, wGPU init, render orchestration
│       ├── context.rs       # Context struct passed to GameApp methods
│       ├── input.rs         # Action-based input (keyboard, mouse, gamepad)
│       ├── physics.rs       # AABB, swept collision, collision layers
│       ├── sprite.rs        # Instanced sprite renderer, blend modes
│       ├── texture.rs       # GPU texture loading
│       ├── texture_manager.rs # Texture asset manager with handles
│       ├── camera.rs        # Orthographic camera, screen shake
│       ├── audio.rs         # Kira wrapper, sub-tracks, events
│       ├── animation.rs     # Asset-agnostic animation state machine
│       ├── time.rs          # Fixed-timestep accumulator
│       ├── math.rs          # Shared math helpers (move_towards)
│       └── atmosphere.rs    # Sky gradient and Perlin fog atmosphere buffers
├── game/                    # Executable - the content
│   ├── Cargo.toml           # Depends on journey-engine (aliased as engine)
│   ├── assets/              # Sprites, audio, levels
│   ├── pkg/                 # WASM build artifacts (wasm-pack output)
│   └── src/
│       ├── main.rs          # Native entry point
│       ├── lib.rs           # JourneyGame: GameApp implementation, game states
│       ├── input.rs         # JourneyAction enum and key/gamepad bindings
│       ├── player.rs        # Player controller FSM, input handling
│       ├── enemy.rs         # Enemy AI (Grunt, Sniper, Ronin)
│       ├── entity.rs        # Shared Entity struct (physics + combat)
│       ├── combat/          # Frame-data FSM, input buffer, move database
│       ├── projectile.rs    # Projectile pool and collision
│       ├── level.rs         # ASCII-based level generation
│       ├── level_editor.rs  # In-game level editor (F12)
│       ├── anim.rs          # Spritesheet animation definitions
│       ├── assets.rs        # Player animation grid mappings
│       ├── audio.rs         # Audio asset loading and dispatch
│       ├── config.rs        # Physics constants and tuning
│       ├── scene.rs         # egui debug overlay
│       └── start_sequence.rs # Splash, menus, options UI
└── web/                     # Vite project for WASM distribution
    ├── src/                 # JavaScript glue code
    ├── public/              # Static assets (favicon)
    └── vite.config.ts       # WASM bundling config
```

---

## Build Targets

### Native (Desktop)

```bash
cargo run --bin journey              # Debug
cargo run --bin journey --release    # Release
```

Uses `pollster` to block on async wGPU initialization. Gamepad support via `gilrs`. Window title and icon are configured by the game's `GameApp` implementation (Journey loads its icon from `web/public/favicon.png` via `GameApp::window_icon()`).

### WebAssembly (Browser)

```bash
wasm-pack build game --target web    # Build WASM artifacts to game/pkg/
cd web && npm install && npm run dev  # Start Vite dev server
```

Uses `wasm-bindgen-futures` for async GPU init. `console_error_panic_hook` pipes Rust panics to browser DevTools. Audio requires a user gesture to unlock the Web Audio API context.

### Internal Resolution

All gameplay renders to a configurable internal-resolution offscreen buffer (default **640×360**, set via `GameApp::internal_resolution()`), upscaled with nearest-neighbor filtering for a retro pixel aesthetic. The CPU atmosphere pass operates at a fixed **32×32** simulation resolution.

---

## Core Engine Systems

The engine exposes functionality through a small set of public modules, all re-exported from `engine::lib.rs` for ergonomic imports:

| Module                        | Purpose                                                                    |
| ----------------------------- | -------------------------------------------------------------------------- |
| `context`                     | `Context` struct - the single handle games use to interact with the engine |
| `input`                       | Action-based input mapping (keyboard, mouse, gamepad)                      |
| `physics`                     | AABB collision detection, swept CCD, collision layers                      |
| `sprite`                      | Instanced GPU sprite renderer with sheet and blend support                 |
| `texture` / `texture_manager` | GPU texture loading and handle-based management                            |
| `camera`                      | Orthographic camera with screen shake                                      |
| `audio`                       | Cross-platform audio (Kira) with sub-tracks and events                     |
| `animation`                   | Generic animation state machine (asset-agnostic)                           |
| `time`                        | Fixed-timestep accumulator with freeze support                             |
| `math`                        | Shared math utilities                                                      |
| `atmosphere`                  | Sky gradient and Perlin fog atmosphere rendering                           |

---

## Game Layer

The `game` crate implements `GameApp` for `JourneyGame` and owns all content:

### Game State Machine

```rust
enum GameState {
    Splash { timer: f32 },
    StartMenu { animation_progress: f32 },
    Options { return_state, tab },
    LevelEditor { return_state },
    Benchmark,
    InGame,
    Paused,
}
```

State transitions drive music changes, input routing, and UI visibility. The splash screen is skipped on WASM builds.

### Shared Entity Model

Journey uses direct composition. Actors such as the player and enemies share an `Entity` struct containing:

- Position and velocity (`Vec2`)
- Facing direction
- Pushbox and hurtbox sizes (`AABB`)
- Combat state (`CombatState`)
- Health

System functions like `fixed_update_physics()` and `integrate_and_collide()` operate on `&mut Entity` generically. This keeps the hot gameplay path explicit and simple while still avoiding duplicated movement, collision, and combat state across actor types.

### Player Controller

The player is a state-machine-driven controller with states: `Idle`, `Run`, `Jump`, `Fall`, `Dash`, `AirDash`, `Parry`, `AttackHorizontal`, `AttackUp`, `AttackDown`, `WallGrab`, `WallSlide`, `GrapplePull`, `GrappleSlingshot`, `Death`. Each state handles its own movement, transitions, coyote timing, jump buffering, and animation selection.

Combat input is sampled every render frame but consumed in `fixed_update()` via a tick-stamped `CombatInputBuffer` for deterministic FSM synchronization.

### Enemy System

Three enemy types (`Grunt`, `Sniper`, `Ronin`) share an FSM but differ via a data-driven `EnemyConfig` table. Core mechanic chain: Enemy shoots → Player parries → Enemy staggers → Player grapples → Execute.

---

## Game Loop and Timing Model

The engine uses a **fixed-timestep with interpolation** model:

```
Each Frame:
  1. Measure wall-clock delta time (capped at 100ms)
  2. Feed delta into FixedTime accumulator
  3. Run N fixed_update() steps at fixed_dt (default 1/60s)
     - Cap at MAX_STEPS=5 to prevent spiral of death
  4. Compute interpolation_alpha for render smoothing
  5. Call update() with variable delta_time
  6. Call render() for sprite submission
  7. Call ui() for egui overlay
  8. GPU: upload sprites, draw background + sprites + egui
```

### FixedTime

- **`tick`**: Monotonic 64-bit counter, incremented once per fixed step.
- **`fixed_dt`**: Duration of one fixed step (default `1/60s`).
- **`accumulator`**: Leftover wall-clock time carried between frames.
- **`freeze_frames`**: When > 0, accumulator is not fed and no fixed steps run. Decrements once per render frame. Used for hitstop and impact freeze.

### Interpolation

`interpolation_alpha = accumulator / fixed_dt` gives a 0.0–1.0 fraction for render-time smoothing between the previous and current physics positions. This ensures silky-smooth visuals even at high refresh rates while keeping physics deterministic.

---

## Rendering Pipeline

```
┌─────────────────────────────────────────────┐
│       CPU Atmosphere Pass (32×32)           │
│   Sky gradient texture + fog overlay texture│
├─────────────────────────────────────────────┤
│        Full-Screen Quad (background)        │
│   Linear sky sample + nearest fog sample    │
├─────────────────────────────────────────────┤
│     Instanced Sprite Pass (640×360)         │
│   RenderLayer → Alpha → Additive            │
│   Batched by texture_id, for 65,536 sprites │
├─────────────────────────────────────────────┤
│       Bloom Composite / Final Blit          │
│   Bright-pass neighborhood glow + upscale   │
├─────────────────────────────────────────────┤
│           egui Overlay Pass                 │
│   Debug UI, menus, HUD                      │
└─────────────────────────────────────────────┘
```

### Atmosphere Rendering

The background is not a scene graph or entity system. It is a small CPU-generated atmosphere pass built from `SceneParams`:

- `SkyParams` produces the base sky gradient with top, horizon, and bottom colors.
- Fog is generated into a separate Perlin overlay texture where RGB stores fog color and alpha stores coverage.
- Both textures stay at **32×32**.
- The fullscreen shader samples the sky texture with linear filtering, then samples the fog texture with nearest filtering and composites `mix(sky.rgb, fog.rgb, fog.a)`.

This split keeps the sky blended while preserving the blocky legacy fog style. Gameplay sprites still render later into the internal 640×360 scene and remain nearest-upscaled.

### Sprite Rendering

Sprites are submitted via `ctx.draw_sprite()`, `ctx.draw_rect()`, and `ctx.draw_sprite_from_sheet()` during `render()`. They are collected into a `Vec<Sprite>`, partitioned by coarse `RenderLayer`, then by `BlendMode`, converted to `SpriteInstance` GPU data, batched by texture ID, and rendered with instanced draw calls.

The sprite layer contract is intentionally small:

- **Background**: optional sprite-backed background elements above the atmosphere pass
- **World**: default gameplay sprites and solid geometry
- **Effects**: glows, trails, particles, and additive rectangles
- **Debug**: collision boxes and diagnostic overlays

This is a render-order contract only. It is not a scene graph, ownership model, or ECS substitute.

Two render pipelines exist within the same render pass:

- **Alpha blend** (default): Standard transparency
- **Additive blend**: For effects like hit flashes, trails, projectile glow, and neon rectangles

Additive blending brightens existing pixels. Bloom is the separate post-process that makes those bright pixels bleed into neighboring pixels. The engine exposes this through `BloomSettings`, either persistently through `ctx.bloom` or for a single frame with `ctx.override_bloom(...)`.

The current bloom pass is intentionally small: it runs during the final blit from the internal render target to the window surface, keeps the source image crisp with `textureLoad`, extracts bright pixels above a threshold, samples a compact neighborhood, and composites the glow back over the scene. It is designed for neon/pixel-art styling, not physically accurate lighting.

Horizontal sprite flipping is done in UV-space (not scale-space) to eliminate anchor-offset artifacts.

### Camera

An orthographic camera maps pixel coordinates to NDC space. It supports:

- Horizontal and vertical panning (camera follow)
- Additive screen shakes with exponential decay and Lissajous-like orbits
- Resize handling for window/canvas changes

---

## Physics and Collision

### AABB (Axis-Aligned Bounding Box)

The primary collision primitive. Constructed from center + size or top-left + size:

```rust
let box_a = AABB::new(center, size);
let box_b = AABB::from_top_left(top_left, size);
```

Operations:

- `check_collision()`: Boolean overlap test
- `get_overlap()`: Overlap amount per axis
- `resolve_collision()`: Minimum translation vector (MTV) to separate two boxes
- `swept_collision()`: Continuous collision detection via Minkowski-expanded ray cast

### Collision Layers

```rust
enum CollisionLayer { Pushbox, Hurtbox, Hitbox, Parrybox }
```

`BoxVolume` pairs a `CollisionLayer` with a local offset and size, and generates world-space AABBs that auto-flip based on facing direction.

### Integration

Physics integration uses decoupled X/Y swept passes to prevent the "floor catch" bug where combined diagonal displacement enters a tile from the wrong axis. One-way platforms only resolve downward collisions.

A skin-width constant prevents the floor the player stands on from being detected as a side wall during the X-sweep.

---

## Input System

### Action-Based Design

The engine provides a generic `GameAction` trait. Each game defines its own action enum and registers key/gamepad bindings at init time, the engine contains no hardcoded actions or default bindings:

```rust
//? Game-side action definition (e.g. game/src/input.rs)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum JourneyAction {
    MoveLeft, MoveRight, MoveUp, MoveDown,
    Jump, Attack, Block, Dash, Grapple,
}

impl engine::GameAction for JourneyAction {}
```

Bindings are registered in `GameApp::init()` via `InputMap<A>`:

```rust
ctx.input.map.bind_key(Key::ArrowLeft, JourneyAction::MoveLeft);
ctx.input.map.bind_key(Key::Space, JourneyAction::Jump);
//? Gamepad (native only)
ctx.input.map.bind_gamepad_button(gilrs::Button::South, JourneyAction::Jump);
ctx.input.map.bind_gamepad_axis_negative(gilrs::Axis::LeftStickX, JourneyAction::MoveLeft);
```

Game logic queries actions through the generic `InputState<A>`, never raw keys:

```rust
if ctx.input.is_action_just_pressed(JourneyAction::Jump) { ... }
if ctx.input.was_action_pressed_buffered(JourneyAction::Jump, buffer_window) { ... }
let move_x = ctx.input.get_move_x(); //* -1.0 to 1.0
```

### Input Buffering

Both movement and combat inputs support buffering. Movement uses `was_action_pressed_buffered()` with a configurable time window. Combat uses a dedicated `CombatInputBuffer` with tick-stamped entries consumed by the FSM.

### Platform Support

- **Native**: Keyboard + mouse + gamepad (via `gilrs`)
- **WASM**: Keyboard + mouse only (gamepad planned)

---

## Audio System

### Architecture

The audio pipeline supports two distinct paradigms: **Static Audio** (sample playback) and **Procedural Audio** (generative synthesis).

#### Static Audio (Kira)

For traditional `.wav`/`.ogg` playback, the engine wraps `kira` with lazy initialization (required on WASM for Web Audio API gesture requirements):

```
AudioManager
├── Music track     (looping, crossfade)
├── Ambience track  (looping, crossfade)
├── SFX track       (one-shot + looping)
└── UI track        (one-shot)
```

Each track has independent volume control. Music and ambience use handle tracking to prevent overlapping loops. All sounds are embedded via `include_bytes!()` and decoded at init time for cross-platform compatibility.

#### Procedural Audio (Resonance & Cadence)

To support dynamic, endless, and generative audio without inflating binary size with megabytes of static files, the ecosystem includes a custom, zero-allocation procedural audio stack built as independent crates:

1. **Resonance (`no_std` DSP Primitive)**: A pure-math audio synthesis primitive. It generates waveforms (Sine, Square, Triangle, Sawtooth) via compile-time generated LUTs (via `build.rs`), eliminating the need for `std::f32::sin()` and enabling bare-metal usage. It features a programmable ADSR envelope system and handles PCM buffer filling entirely using fixed-point `u32` phase accumulators.
2. **Cadence (`no_std` Sequencer)**: The mathematical logic core sitting above Resonance. It tracks time accurately via discrete audio sample counting (guaranteeing zero drift indefinitely) and executes deterministic algorithms like Euclidean Rhythms (Bjorklund's algorithm) and Markov Chains (via an LFSR) to trigger real-time events.

**Decoupled Sinks:** Both Resonance and Cadence are pure state machines completely unaware of their audio sink. In native builds, they can be fed to a `cpal` audio driver. On the web, they compile to a tiny `wasm32-unknown-unknown` blob running directly inside an `AudioWorkletNode`, remaining perfectly phase-locked and executing completely off the main thread.

### Event-Driven SFX

The engine provides `UiAudioEvent` for UI sounds (hover, click, checkbox, tab change), managed via `ctx.pending_ui_audio` and deduplicated per frame. Game-specific audio events (jump, dash, hit, etc.) are defined and queued entirely by the game crate, keeping the engine free of game-specific coupling.

```rust
//? Engine UI events (automatic via AudioResponse trait on egui widgets)
ui.button("Attack").with_ui_sound(&mut ctx.pending_ui_audio);

//? Game events (game-defined enum, game-managed queue)
self.pending_game_audio.push(AudioEvent::Jump);
```

---

## Animation System

### Two-Layer Design

1. **Engine layer** (`engine::animation`): Asset-agnostic `AnimationDef` + `AnimationState` runtime. Handles frame timing, playback, looping, and state transitions. Does not know about textures.

2. **Game layer** (`game::anim`): `Animation` struct that wraps `AnimationDef` with spritesheet-specific logic (grid-based `get_frame_rect()`). `AnimationState` wraps the engine's state machine and adds frame rectangle calculation.

### Spritesheet Layout

The player spritesheet uses a 5-column × 11-row grid at 100×100px per cell:

| Row | Frames | Animation                       |
| --- | ------ | ------------------------------- |
| 0   | 0–3    | Idle                            |
| 1   | 5–8    | Run                             |
| 2   | 10–14  | Jump (10–12 ascend, 13–14 fall) |
| 3   | 15–18  | Death                           |
| 4   | 20–23  | Parry                           |
| 5   | 25–28  | Attack Horizontal               |
| 6   | 30–33  | Attack Up                       |
| 7   | 35–38  | Attack Down                     |
| 8   | 40–43  | Dash                            |
| 9   | 45–47  | Wall Grab/Slide                 |
| 10  | 50–51  | Grapple                         |

Combat animation durations are dynamically derived from FSM frame data so visual playback stays locked to combat timing.

---

## Combat System

### Frame-Data FSM

All combat timing uses integer tick counts at the fixed tick rate (default 60Hz). Each move is defined by three phases:

```
Startup → Active → Recovery
```

`MoveData` stores frame counts for each phase, plus damage, knockback, hitbox geometry, and a cancel window percentage. The FSM advances one tick per `fixed_update()` call and auto-transitions between phases.

### Moves

```rust
enum MoveId {
    AttackHorizontal, AttackUp, AttackDown,
    Parry, Dash, Grapple,
}
```

`MoveDatabase` holds all move definitions and supports runtime tick-rate scaling. Frame counts are proportionally adjusted so wall-clock timing stays consistent across different tick rates.

### Cancel Windows

The last N% of recovery allows cancelling into another move, enabling combo chains. The cancel window percentage is per-move configurable.

### Combat Input Buffer

Bridges per-frame input sampling and fixed-rate FSM updates. Inputs are queued with tick stamps and consumed within a configurable frame window (default 20 ticks ≈ 333ms at 60Hz), ensuring rapid inputs are never lost between fixed steps.

### Dash Invincibility

Dash grants i-frames during the first half of its active phase, handled within the combat FSM.

---

## Level System

### ASCII-Based Level Definition

Levels are defined as ASCII strings where each character maps to a game element:

| Char | Element          |
| ---- | ---------------- |
| `#`  | Solid platform   |
| `=`  | One-way platform |
| `\|` | Wall             |
| `@`  | Player spawn     |
| `G`  | Grunt enemy      |
| `S`  | Sniper enemy     |
| `R`  | Ronin enemy      |
| `O`  | Grapple node     |
| `X`  | Exit             |

### Level Editor

Press `F12` to toggle a full-screen dual-mode editor operating on a canonical ASCII string. Features:

- Live minimap with color-coded elements
- Validation pass warning on missing elements
- Universal persistence: `world.txt` on native, `localStorage` on WASM

---

## Configuration and Tuning

All physics constants are centralized in `game/src/config.rs` using a `PIXELS_PER_UNIT` base (16px) for dimensional consistency:

| Category     | Key Constants                                                                 |
| ------------ | ----------------------------------------------------------------------------- |
| **Movement** | `MOVEMENT_SPEED` (300px/s), `MAX_SPEED` (300px/s)                             |
| **Jump**     | `JUMP_POWER` (600px/s), `COYOTE_TICKS` (6), `JUMP_BUFFER_TICKS` (8)           |
| **Gravity**  | `GRAVITY` (1760px/s²), `MAX_FALL_SPEED` (640px/s)                             |
| **Dash**     | `DASH_SPEED` (800px/s), `DASH_DURATION_TICKS` (8), `DASH_COOLDOWN_TICKS` (10) |
| **Wall**     | `WALL_SLIDE_SPEED` (120px/s), `WALL_GRAB_TIMEOUT_TICKS` (10)                  |
| **Grapple**  | `GRAPPLE_PULL_SPEED` (400px/s), `GRAPPLE_DETECT_RANGE` (144px)                |

A runtime-tunable `PhysicsConfig` struct mirrors these constants and can be modified via the egui debug UI during development.

---

## Cross-Platform Strategy

### Conditional Compilation

```rust
#[cfg(not(target_arch = "wasm32"))]  //* Native-only code
#[cfg(target_arch = "wasm32")]        //* WASM-only code
```

Key platform differences:

| Concern           | Native                      | WASM                                        |
| ----------------- | --------------------------- | ------------------------------------------- |
| GPU init          | `pollster::block_on` (sync) | `wasm_bindgen_futures::spawn_local` (async) |
| Gamepad           | `gilrs`                     | Not available                               |
| Audio init        | Immediate at startup        | Deferred until user gesture                 |
| Logging           | `env_logger`                | `console_log` + `console_error_panic_hook`  |
| Fullscreen        | `winit::Fullscreen`         | CSS `100vw × 100vh`                         |
| Visual FPS cap    | Native sleep/WaitUntil cap  | Browser RAF/vsync                           |
| Level persistence | File I/O (`world.txt`)      | `web_sys::Storage` (localStorage)           |
| Splash screen     | 2s timer                    | Skipped                                     |
| Clipboard         | OS clipboard via `arboard`  | Not available                               |

### WASM Dependencies

- `wasm-bindgen` / `wasm-bindgen-futures`: JS interop and async support
- `web-sys`: DOM manipulation (canvas, storage, window)
- `console_error_panic_hook`: Panic messages in DevTools
- `console_log`: Rust `log` facade to browser console
- `getrandom` with `js` feature: Crypto RNG for `rand`

---

## Performance Budget

| Metric              | Target                                                                 |
| ------------------- | ---------------------------------------------------------------------- |
| Frame rate          | 60 FPS stable (native + browser)                                       |
| WASM binary         | < 5 MB (gzip)                                                          |
| Sprite cap          | 65,536 instanced sprites per frame                                     |
| Physics steps       | Max 5 per frame (spiral-of-death cap)                                  |
| Atmosphere resolution | 32×32 CPU sky/fog pass at ~60Hz                                      |
| Internal resolution | Default 640×360 (configurable via `GameApp`), nearest-neighbor upscale |

---

## Dependency Map

### Engine Crate

| Dependency                               | Purpose                        |
| ---------------------------------------- | ------------------------------ |
| `wgpu` 27                                | Cross-platform GPU abstraction |
| `winit` 0.30                             | Windowing and event loop       |
| `egui` 0.33 / `egui-wgpu` / `egui-winit` | Immediate-mode debug UI        |
| `kira` 0.12                              | Cross-platform audio engine    |
| `glam` 0.29                              | Fast math (Vec2, Mat4)         |
| `bytemuck` 1                             | Safe GPU data casting          |
| `image` 0.25                             | PNG texture loading            |
| `noise` 0.8                              | Perlin noise generation        |
| `web-time` 1                             | Cross-platform `Instant`       |
| `gilrs` 0.11                             | Gamepad input (native only)    |
| `pollster` 0.4                           | Async blocking (native only)   |

### Game Crate

| Dependency                                   | Purpose                  |
| -------------------------------------------- | ------------------------ |
| `journey-engine` (path, aliased as `engine`) | The engine library       |
| `rand` 0.8                                   | Random number generation |
| `instant` 0.1                                | WASM-compatible timing   |
| `log` 0.4                                    | Logging facade           |
