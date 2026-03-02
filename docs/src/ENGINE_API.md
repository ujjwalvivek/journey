# Journey Engine API Reference

> Public API documentation for the `journey-engine` crate with usage examples from `game`.
>
> **Version:** 1.1.2 · **Crate type:** `rlib`

---

## Table of Contents

1. [Quick Start](#quick-start)
2. [GameApp Trait](#gameapp-trait)
3. [Context](#context)
4. [Input](#input)
5. [Physics](#physics)
6. [Sprite and Rendering](#sprite-and-rendering)
7. [Texture Management](#texture-management)
8. [Camera and Screen Shake](#camera-and-screen-shake)
9. [Audio](#audio)
10. [Animation](#animation)
11. [Time](#time)
12. [Math Utilities](#math-utilities)
13. [Noise and Scene](#noise-and-scene)
14. [Re-exports](#re-exports)

---

## Quick Start

### Minimal Game

```rust
use engine::{Context, GameAction, GameApp, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action { MoveLeft, MoveRight, Jump }

impl GameAction for Action {
    fn count() -> usize { 3 }
    fn index(&self) -> usize { *self as usize }
    fn from_index(i: usize) -> Option<Self> {
        match i { 0 => Some(Self::MoveLeft), 1 => Some(Self::MoveRight), 2 => Some(Self::Jump), _ => None }
    }
    fn move_negative_x() -> Option<Self> { Some(Self::MoveLeft) }
    fn move_positive_x() -> Option<Self> { Some(Self::MoveRight) }
}

struct MyGame;

impl GameApp for MyGame {
    type Action = Action;

    fn init(ctx: &mut Context<Action>) -> Self {
        MyGame
    }

    fn update(&mut self, ctx: &mut Context<Action>) {
        //? Variable-rate update logic
    }

    fn render(&mut self, ctx: &mut Context<Action>) {
        ctx.draw_rect(Vec2::new(100.0, 100.0), Vec2::new(32.0, 32.0), [1.0, 0.0, 0.0, 1.0]);
    }
}

fn main() {
    engine::run::<MyGame>();
}
```

### WASM Entry Point

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn wasm_main() {
    engine::run_wasm::<MyGame>();
}
```

---

## GameApp Trait

The core contract between engine and game. Implement this trait to define your game's lifecycle.

**Module:** `engine` (root)

```rust
pub trait GameApp: 'static {
    type Action: GameAction;

    fn init(ctx: &mut Context<Self::Action>) -> Self;
    fn fixed_update(&mut self, ctx: &mut Context<Self::Action>, fixed_time: &FixedTime) {}
    fn update(&mut self, ctx: &mut Context<Self::Action>);
    fn render(&mut self, ctx: &mut Context<Self::Action>);
    fn ui(&mut self, egui_ctx: &egui::Context, ctx: &mut Context<Self::Action>, scene_params: &mut SceneParams) {}

    //? Optional overrides with defaults:
    fn window_title() -> &'static str { "Journey Engine" }
    fn window_icon() -> Option<&'static [u8]> { None }
    fn wasm_ready_event() -> Option<&'static str> { None }
    fn internal_resolution() -> (u32, u32) { (640, 360) }
}
```

### How `game` Uses It

```rust
impl GameApp for JourneyGame {
    type Action = JourneyAction;

    fn window_title() -> &'static str { "Journey" }
    fn window_icon() -> Option<&'static [u8]> {
        Some(include_bytes!("../../web/public/favicon.png"))
    }
    fn wasm_ready_event() -> Option<&'static str> { Some("journey:first-frame") }
    fn internal_resolution() -> (u32, u32) { (640, 360) }

    fn init(ctx: &mut Context<JourneyAction>) -> Self {
        //? Load textures during init and return a 1-based texture ID
        let tex_player = ctx.load_texture(
            include_bytes!("../assets/player/player.png"),
            "Player Spritesheet",
        );
        //? Set up input bindings
        input::setup_default_bindings(&mut ctx.input);

        let level = Level::new(ctx.screen_width, ctx.screen_height);
        let player = Player::new(level.player_spawn, anim_state);

        Self { player, level, /* ... */ }
    }

    fn fixed_update(&mut self, ctx: &mut Context<JourneyAction>, fixed_time: &FixedTime) {
        //? All physics and combat run here at a fixed rate (default 60Hz)
        self.player.fixed_update(ctx.delta_time, fixed_time.tick, fixed_time.tick_rate(), /* ... */);
    }

    fn update(&mut self, ctx: &mut Context<JourneyAction>) {
        //? Camera smoothing with interpolation alpha
        let alpha = ctx.interpolation_alpha;
        let cam_x = lerp(self.prev_camera_x, self.camera_x, alpha);
    }

    fn render(&mut self, ctx: &mut Context<JourneyAction>) {
        //? Draw player sprite from sheet
        ctx.draw_sprite_from_sheet(
            position, size, color, source_rect, flip_x, texture_id,
        );
    }
}
```

---

## Context

The single mutable handle games use to interact with the engine. Passed to every `GameApp` method.

**Module:** `engine::context`

### Public Fields

| Field                 | Type                | Description                                                                |
| --------------------- | ------------------- | -------------------------------------------------------------------------- |
| `input`               | `InputState<A>`     | Current keyboard, mouse, and gamepad state (generic over your action enum) |
| `delta_time`          | `f32`               | Frame delta (variable in `update`, fixed in `fixed_update`)                |
| `screen_width`        | `f32`               | Current viewport width in pixels                                           |
| `screen_height`       | `f32`               | Current viewport height in pixels                                          |
| `camera_offset_x`     | `f32`               | Horizontal camera pan offset                                               |
| `camera_offset_y`     | `f32`               | Vertical camera pan offset                                                 |
| `fps`                 | `f32`               | Current frames per second                                                  |
| `frame_time_ms`       | `f32`               | Last frame time in milliseconds                                            |
| `fixed_tick_rate`     | `u32`               | Fixed update rate in Hz (default: 60)                                      |
| `target_fps`          | `u32`               | Target display frame rate                                                  |
| `interpolation_alpha` | `f32`               | 0.0–1.0 fraction for render-time smoothing                                 |
| `freeze_frames`       | `u16`               | Remaining freeze frames (hitstop)                                          |
| `pending_shakes`      | `Vec<(f32, f32)>`   | Queued screen shakes: (intensity, duration)                                |
| `request_exit`        | `bool`              | Set to `true` to close the window                                          |
| `fullscreen_enabled`  | `bool`              | Whether fullscreen is currently enabled                                    |
| `hdr_enabled`         | `bool`              | Whether HDR output is active                                               |
| `audio`               | `AudioManager`      | Direct access to the audio system                                          |
| `pending_ui_audio`    | `Vec<UiAudioEvent>` | UI audio events queued this frame                                          |

### Methods

#### Texture Loading

```rust
pub fn load_texture(&mut self, bytes: &'static [u8], label: &str) -> usize
```

Queue a texture for loading during `init()`. Returns a 1-based texture ID (0 is the built-in white pixel). The engine processes these after `init` completes.

```rust
//? In GameApp::init()
let tex_player = ctx.load_texture(
    include_bytes!("../assets/player/player.png"),
    "Player Spritesheet",
);
//? tex_player == 1 (first loaded texture)
```

#### Drawing

```rust
//? Draw a colored rectangle
pub fn draw_rect(&mut self, position: Vec2, size: Vec2, color: [f32; 4])

//? Draw a sprite with optional horizontal flip
pub fn draw_sprite(&mut self, position: Vec2, size: Vec2, color: [f32; 4], flip_x: bool)

//? Draw a region from a sprite sheet
pub fn draw_sprite_from_sheet(
    &mut self,
    position: Vec2,    //? Top-left corner in screen space
    size: Vec2,        //? Width and height in pixels
    color: [f32; 4],   //? RGBA tint (0.0–1.0 each)
    source_rect: Rect, //? Sprite sheet region in pixel coordinates
    flip_x: bool,      //? Horizontal mirror
    texture_id: usize,  //? 1+ for loaded textures, 0 for white pixel
)

//? Same as above with additive blending (for effects/glow)
pub fn draw_sprite_from_sheet_additive(
    &mut self, position: Vec2, size: Vec2, color: [f32; 4],
    source_rect: Rect, flip_x: bool, texture_id: usize,
)
```

**Usage from `game`:**

```rust
//? Draw a solid red rectangle (health bar)
ctx.draw_rect(Vec2::new(10.0, 10.0), Vec2::new(100.0, 8.0), [1.0, 0.0, 0.0, 1.0]);

//? Draw the player from a spritesheet
let frame_rect = Rect::new(col as f32 * 100.0, row as f32 * 100.0, 100.0, 100.0);
ctx.draw_sprite_from_sheet(
    player_pos,
    Vec2::new(50.0, 50.0),
    [1.0, 1.0, 1.0, 1.0], //? White tint = original colors
    frame_rect,
    !player.facing_right(),
    1, //? texture ID from load_texture
);

//? Additive glow effect on a hit flash
ctx.draw_sprite_from_sheet_additive(
    position, size, [1.0, 0.8, 0.2, 0.6], source_rect, flip_x, texture_id,
);
```

#### Effects

```rust
//? Freeze physics and FSM for N render frames (hitstop)
pub fn trigger_freeze(&mut self, frames: u16)

//? Queue a screen shake with intensity and duration in seconds
pub fn trigger_shake(&mut self, intensity: f32, duration: f32)
```

**Usage from `game`:**

```rust
//? Parry hitstop: 3-frame freeze + screen shake
ctx.trigger_freeze(3);
ctx.trigger_shake(4.0, 0.15);
```

#### Audio

The engine only manages UI-level audio events (`UiAudioEvent`). Game-specific audio events should be defined and queued by the game crate. The runtime automatically deduplicates `pending_ui_audio` each frame.

```rust
//? UI audio is handled automatically via the AudioResponse trait on egui widgets.
//? Game-specific audio (jump, dash, etc.) is managed by the game crate's own event queue.
```

#### Utility

```rust
pub fn screen_center(&self) -> Vec2
pub fn set_fullscreen_enabled(&mut self, enabled: bool)
pub fn set_hdr_enabled(&mut self, enabled: bool)
```

---

## Input

Generic, action-based input system that decouples hardware keys from game intent. Each game defines its own action enum by implementing the `GameAction` trait.

**Module:** `engine::input`

### GameAction Trait

The engine's input system is generic over this trait. There are no hardcoded actions, each game defines its own.

```rust
pub trait GameAction: Copy + Eq + Debug + 'static {
    fn count() -> usize;
    fn index(&self) -> usize;
    fn from_index(index: usize) -> Option<Self>;

    //? Optional: wire these up for get_move_x() / get_move_y() axis helpers
    fn move_negative_x() -> Option<Self> { None }
    fn move_positive_x() -> Option<Self> { None }
    fn move_negative_y() -> Option<Self> { None }
    fn move_positive_y() -> Option<Self> { None }
}
```

### Defining Actions (Game-Side)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JourneyAction {
    MoveLeft, MoveRight, MoveUp, MoveDown,
    Jump, Attack, Block, Dash, Grapple,
}

impl GameAction for JourneyAction {
    fn count() -> usize { 9 }
    fn index(&self) -> usize { *self as usize }
    fn from_index(i: usize) -> Option<Self> {
        match i {
            0 => Some(Self::MoveLeft),  1 => Some(Self::MoveRight),
            2 => Some(Self::MoveUp),    3 => Some(Self::MoveDown),
            4 => Some(Self::Jump),      5 => Some(Self::Attack),
            6 => Some(Self::Block),     7 => Some(Self::Dash),
            8 => Some(Self::Grapple),   _ => None,
        }
    }
    fn move_negative_x() -> Option<Self> { Some(Self::MoveLeft) }
    fn move_positive_x() -> Option<Self> { Some(Self::MoveRight) }
    fn move_negative_y() -> Option<Self> { Some(Self::MoveUp) }
    fn move_positive_y() -> Option<Self> { Some(Self::MoveDown) }
}
```

### Key

```rust
pub enum Key {
    W, A, S, D, Space, Shift, Alt,
    Up, Down, Left, Right, F12, Escape,
}
```

### MouseBinding

```rust
pub enum MouseBinding {
    Left,
    Right,
    Middle,
}
```

### `InputState<A>` Methods

```rust
//? Is the action currently held down?
pub fn is_action_pressed(&self, action: A) -> bool

//? Was the action pressed this frame (rising edge)?
pub fn is_action_just_pressed(&self, action: A) -> bool

//? Was the action pressed within a recent time window? (for jump buffering)
pub fn was_action_pressed_buffered(&self, action: A, buffer_window: f32) -> bool

//? Raw key queries (for non-action keys like Escape)
pub fn is_key_pressed(&self, key: Key) -> bool
pub fn is_key_just_pressed(&self, key: Key) -> bool

//? Mouse button state
pub fn is_mouse_pressed(&self, button: MouseButton) -> bool

//? Axis values: -1.0 to 1.0 (combines keyboard + gamepad)
//? Requires move_negative_x/move_positive_x on your GameAction impl
pub fn get_move_x(&self) -> f32
pub fn get_move_y(&self) -> f32

//? Device detection
pub fn any_keyboard_or_mouse(&self) -> bool
pub fn any_gamepad(&self) -> bool

//? Access binding configuration
pub fn input_map_mut(&mut self) -> &mut InputMap<A>
```

### `InputMap<A>` (Custom Bindings)

The engine ships with **no default bindings**. Games register their own during `init()`:

```rust
pub fn bind_key(&mut self, key: Key, action: A)
pub fn bind_mouse(&mut self, button: MouseBinding, action: A)

//? Native only:
pub fn bind_button(&mut self, button: gilrs::Button, action: A)
```

### Setting Up Bindings (Game-Side)

```rust
fn init(ctx: &mut Context<JourneyAction>) -> Self {
    let map = ctx.input.input_map_mut();
    map.bind_key(Key::A, JourneyAction::MoveLeft);
    map.bind_key(Key::Left, JourneyAction::MoveLeft);
    map.bind_key(Key::D, JourneyAction::MoveRight);
    map.bind_key(Key::Right, JourneyAction::MoveRight);
    map.bind_key(Key::Space, JourneyAction::Jump);
    map.bind_mouse(MouseBinding::Left, JourneyAction::Attack);
    map.bind_mouse(MouseBinding::Right, JourneyAction::Block);
    //? ...
}
```

### Usage from `game`

```rust
fn fixed_update(&mut self, ctx: &mut Context<JourneyAction>, fixed_time: &FixedTime) {
    let move_x = ctx.input.get_move_x();

    //? Jump with buffering (8 ticks ≈ 133ms at 60Hz)
    let buffer_window = self.stats.jump_buffer_ticks as f32 / 60.0;
    if ctx.input.was_action_pressed_buffered(JourneyAction::Jump, buffer_window) {
        self.execute_jump();
    }

    //? Dash on just-pressed (no buffering)
    if ctx.input.is_action_just_pressed(JourneyAction::Dash) {
        self.start_dash(move_x);
    }
}
```

### Journey’s Key Bindings (Game-Side)

| Key     | Action   |     | Key     | Action    |
| ------- | -------- | --- | ------- | --------- |
| A / ←   | MoveLeft |     | D / →   | MoveRight |
| W / ↑   | MoveUp   |     | S / ↓   | MoveDown  |
| Space   | Jump     |     | Shift   | Dash      |
| Alt     | Grapple  |     | Mouse L | Attack    |
| Mouse R | Block    |     |         |           |

### Journey’s Gamepad Bindings (Game-Side, Native)

| Button             | Action  |
| ------------------ | ------- |
| South (A/Cross)    | Jump    |
| West (X/Square)    | Attack  |
| RightTrigger (RB)  | Block   |
| RightTrigger2 (RT) | Dash    |
| LeftTrigger2 (LT)  | Grapple |

---

## Physics

2D collision detection primitives.

**Module:** `engine::physics`

### AABB

Axis-Aligned Bounding Box: the primary collision shape.

```rust
pub struct AABB {
    pub center: Vec2,
    pub size: Vec2,
}
```

#### Constructors

```rust
//? From center position and size
pub fn new(center: Vec2, size: Vec2) -> Self

//? From top-left corner and size (common for sprites)
pub fn from_top_left(top_left: Vec2, size: Vec2) -> Self
```

#### Queries

```rust
pub fn min(&self) -> Vec2           //? Top-left corner
pub fn max(&self) -> Vec2           //? Bottom-right corner
pub fn top_left(&self) -> Vec2      //? Alias for min()
pub fn check_collision(&self, other: &AABB) -> bool
pub fn get_overlap(&self, other: &AABB) -> Vec2  //? Per-axis overlap
```

#### Collision Resolution

```rust
//? Minimum Translation Vector to push mover out of obstacle
//? Returns None if no overlap. Pushes along smallest-overlap axis.
//? Y-axis bias toward upward push (landing on platforms).
pub fn resolve_collision(mover: &AABB, obstacle: &AABB) -> Option<Vec2>
```

#### Swept Collision (CCD)

```rust
pub struct SweepResult {
    pub time: f32,      //? 0.0–1.0, fraction of displacement before hit
    pub normal: Vec2,   //? Surface normal at contact point
}

//? Continuous collision: move self along displacement, find earliest hit.
//? Uses Minkowski-expanded ray cast.
pub fn swept_collision(&self, displacement: Vec2, obstacle: &AABB) -> Option<SweepResult>
```

### CollisionLayer

```rust
pub enum CollisionLayer {
    Pushbox,    //? Physics body (blocks movement)
    Hurtbox,    //? Vulnerable area (takes damage)
    Hitbox,     //? Damaging area (deals damage)
    Parrybox,   //? Parry detection area
}
```

### BoxVolume

A positioned collision volume relative to an entity center. Auto-flips X offset based on facing direction.

```rust
pub struct BoxVolume {
    pub layer: CollisionLayer,
    pub local_offset: Vec2,
    pub size: Vec2,
    pub active: bool,
}

pub fn new(layer: CollisionLayer, offset: Vec2, size: Vec2) -> Self
pub fn world_aabb(&self, entity_pos: Vec2, facing_right: bool) -> AABB
```

### Usage from `game`

```rust
//? Player pushbox
let player_aabb = AABB::new(player.position, Vec2::new(PLAYER_WIDTH, PLAYER_HEIGHT));

//? Check collision with each platform
for platform in &platforms {
    if let Some(mtv) = AABB::resolve_collision(&player_aabb, &platform.aabb) {
        player.position += mtv;
    }
}

//? Swept collision for high-speed movement
let displacement = velocity * dt;
if let Some(hit) = player_aabb.swept_collision(displacement, &wall) {
    //? Move to contact point
    player.position += displacement * hit.time;
    //? Slide along surface
    let remaining = displacement * (1.0 - hit.time);
    let slide = remaining - hit.normal * remaining.dot(hit.normal);
    player.position += slide;
}

//? Hitbox that flips with facing direction
let hitbox = BoxVolume::new(
    CollisionLayer::Hitbox,
    Vec2::new(12.0, 0.0), //? 12px in front of center
    Vec2::new(20.0, 24.0),
);
let world_hitbox = hitbox.world_aabb(entity.position, entity.facing_right);
```

---

## Sprite and Rendering

Instanced 2D sprite rendering with sprite sheet and blend mode support.

**Module:** `engine::sprite`

### Rect

```rust
pub struct Rect {
    pub x: f32, pub y: f32,
    pub w: f32, pub h: f32,
}

pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self
pub fn from_pos_size(pos: Vec2, size: Vec2) -> Self
```

### BlendMode

```rust
pub enum BlendMode {
    Alpha,      //? Standard transparency (default)
    Additive,   //? Additive blending (effects, glow)
}
```

### Sprite (Internal)

The `Sprite` struct is not typically created directly by game code. Instead, use the `Context` draw methods which build sprites internally:

```rust
//? Solid colored rectangle
ctx.draw_rect(pos, size, color);

//? Textured sprite from sheet
ctx.draw_sprite_from_sheet(pos, size, color, source_rect, flip_x, texture_id);

//? Additive blended sprite
ctx.draw_sprite_from_sheet_additive(pos, size, color, source_rect, flip_x, texture_id);
```

### Rendering Details

- Up to **1024 sprites** per frame (instanced rendering)
- Sprites are batched by `texture_id` for minimal GPU state changes
- Two render pipelines: alpha blend (default) and additive blend
- Horizontal flipping is done in UV-space, avoiding anchor-offset artifacts
- `texture_id = 0` uses a built-in 1×1 white pixel (for colored rectangles)

---

## Texture Management

**Modules:** `engine::texture`, `engine::texture_manager`

### Texture

Low-level GPU texture. Not typically used directly, go through `Context::load_texture()` or `TextureManager`.

```rust
pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub width: u32,
    pub height: u32,
}

pub fn from_bytes(device, queue, bytes, label) -> Result<Self, ImageError>
pub fn from_image(device, queue, img, label) -> Result<Self, ImageError>
pub fn white_pixel(device, queue) -> Self
```

### TextureHandle

Opaque handle to a loaded texture in the `TextureManager`.

```rust
pub struct TextureHandle(pub(crate) usize);
```

### Loading Textures in `game`

The simplest path is `Context::load_texture()` during `init`:

```rust
fn init(ctx: &mut Context) -> Self {
    //? Returns a 1-based ID used with draw_sprite_from_sheet
    let tex_player = ctx.load_texture(
        include_bytes!("../assets/player/player.png"),
        "Player Spritesheet",
    );
    //? tex_player == 1

    let tex_effects = ctx.load_texture(
        include_bytes!("../assets/effects/particles.png"),
        "Effects Sheet",
    );
    //? tex_effects == 2
}
```

---

## Camera and Screen Shake

**Module:** `engine::camera`

### ScreenShake

Decaying sinusoidal screen shake with Lissajous-like orbits for organic feel.

```rust
pub struct ScreenShake {
    pub intensity: f32,
    pub duration: f32,
    pub frequency: f32,  //? Default: 40.0
    pub decay: f32,      //? Default: 8.0
    pub elapsed: f32,
}

pub fn new(intensity: f32, duration: f32) -> Self
pub fn sample(&self) -> Vec2     //? Current shake offset
pub fn update(&mut self, dt: f32) -> bool  //? Returns true if still active
pub fn is_active(&self) -> bool
```

### Triggering Shakes from `game`

```rust
//? Via Context (most common)
ctx.trigger_shake(4.0, 0.15);  //? intensity=4.0, duration=0.15s

//? Multiple shakes stack additively
ctx.trigger_shake(2.0, 0.1);  //? Light shake
ctx.trigger_shake(8.0, 0.3);  //? Heavy shake (both run simultaneously)
```

### Camera (Internal)

The camera is managed by the engine runtime. Game code interacts with it via `Context` fields:

```rust
//? Read camera state
let offset_x = ctx.camera_offset_x;
let offset_y = ctx.camera_offset_y;

//? Camera follow is typically done in update():
fn update(&mut self, ctx: &mut Context) {
    let target_x = (self.player.position().x - ctx.screen_width / 2.0).max(0.0);
    self.camera_x = lerp(self.prev_camera_x, target_x, 0.1);
    ctx.camera_offset_x = self.camera_x;
    ctx.camera_offset_y = self.camera_y;
}
```

---

## Audio

Cross-platform audio system wrapping Kira with lazy WASM initialization.

**Module:** `engine::audio`

### AudioTrack

```rust
pub enum AudioTrack {
    Music,     //? Looping background music
    Ambience,  //? Looping ambient sounds
    Sfx,       //? Sound effects (one-shot + looping)
    Ui,        //? UI sounds (one-shot)
}
```

### UiAudioEvent

Engine-level UI audio events (game-specific events are defined by the game crate):

```rust
pub enum UiAudioEvent {
    Hover,
    Click,
    CheckboxOn,
    CheckboxOff,
    TabChange,
}
```

### AudioManager Methods

#### Playback

```rust
//? One-shot sound on a specific track
pub fn play_oneshot(&mut self, data: &StaticSoundData, track: AudioTrack)

//? Looping music with crossfade
pub fn play_music(&mut self, data: &StaticSoundData, fade_in_secs: f32)
pub fn stop_music(&mut self, fade_out_secs: f32)

//? Looping ambience with crossfade
pub fn play_ambience(&mut self, data: &StaticSoundData, fade_in_secs: f32)
pub fn stop_ambience(&mut self, fade_out_secs: f32)

//? Looping SFX (e.g., footstep loop while running)
pub fn play_loop_sfx(&mut self, data: &StaticSoundData)
pub fn stop_loop_sfx(&mut self, fade_out_secs: f32)
```

#### Volume Control

```rust
//? Track-level volume (0.0–1.0)
pub fn set_master_volume(&mut self, volume: f64)
pub fn set_music_volume(&mut self, volume: f64)
pub fn set_ambience_volume(&mut self, volume: f64)
pub fn set_sfx_volume(&mut self, volume: f64)
pub fn set_ui_volume(&mut self, volume: f64)

//? Read current volume
pub fn master_volume(&self) -> f64
pub fn music_volume(&self) -> f64
//? ... (same pattern for other tracks)

//? Computed effective volume (master × track)
pub fn effective_volume(&self, track: AudioTrack) -> f64

//? Live ducking with fade
pub fn set_music_live_volume(&mut self, amp: f64, fade_secs: f32)
pub fn set_ambience_live_volume(&mut self, amp: f64, fade_secs: f32)
```

#### State Queries

```rust
pub fn has_active_music(&self) -> bool
pub fn has_active_ambience(&self) -> bool
pub fn notify_user_gesture(&mut self)  //? WASM: unlock Web Audio context
```

### Loading Sounds

```rust
//? Decode sound from embedded bytes (cross-platform)
pub fn load_sound_data(bytes: &'static [u8]) -> Option<StaticSoundData>
```

### AudioResponse Trait (egui Integration)

```rust
pub trait AudioResponse {
    fn with_ui_sound(self, pending: &mut Vec<UiAudioEvent>) -> Self;
    fn with_checkbox_sound(self, checked: bool, pending: &mut Vec<UiAudioEvent>) -> Self;
    fn with_tab_sound(self, pending: &mut Vec<UiAudioEvent>) -> Self;
}
```

### Usage from `game`

```rust
//? Loading audio assets at init
struct AudioAssets {
    sfx_jump: Option<StaticSoundData>,
    bg_music: Option<StaticSoundData>,
}

impl AudioAssets {
    fn load() -> Self {
        Self {
            sfx_jump: load_sound_data(include_bytes!("../assets/audio/sfx_jump.ogg")),
            bg_music: load_sound_data(include_bytes!("../assets/audio/bg_music.ogg")),
        }
    }
}

//? Playing music (in update)
if let Some(ref data) = self.audio_assets.bg_music {
    ctx.audio.play_music(data, 1.0); //? 1s fade-in
}

//? Game-specific audio: define your own enum and queue
pub enum AudioEvent { Jump, Dash, Hit, /* ... */ }
let mut pending_game_audio: Vec<AudioEvent> = Vec::new();
pending_game_audio.push(AudioEvent::Jump);

//? Dispatch game events (in update)
for event in pending_game_audio.drain(..) {
    self.audio_assets.dispatch(event, &mut ctx.audio);
}

//? Dispatch engine UI events
for ui_event in ctx.pending_ui_audio.drain(..) {
    self.audio_assets.dispatch_ui(ui_event, &mut ctx.audio);
}

//? Music ducking during menus
let ducked_vol = ctx.audio.effective_volume(AudioTrack::Music) * 0.3;
ctx.audio.set_music_live_volume(ducked_vol, 0.4);

//? egui UI sounds (push UiAudioEvent automatically)
ui.button("Start")
    .with_ui_sound(&mut ctx.pending_ui_audio)
    .clicked();

ui.checkbox(&mut show_fps, "Show FPS")
    .with_checkbox_sound(show_fps, &mut ctx.pending_ui_audio);
```

---

## Animation

Asset-agnostic animation runtime. The engine provides the state machine; the game maps frames to textures.

**Module:** `engine::animation`

### AnimationDef

```rust
pub struct AnimationDef {
    pub name: String,
    pub start_frame: usize,
    pub frame_count: usize,
    pub frame_duration: f32,  //? Seconds per frame
    pub looping: bool,
}

pub fn new(
    name: impl Into<String>,
    start_frame: usize,
    frame_count: usize,
    frame_duration: f32,
    looping: bool,
) -> Self
```

### AnimationState

```rust
pub struct AnimationState {
    pub current_anim: String,
    pub frame_index: usize,
    pub timer: f32,
}

pub fn new(animations: Vec<AnimationDef>, default_anim: &str) -> Self
pub fn update(&mut self, dt: f32)
pub fn current(&self) -> Option<(&AnimationDef, usize)>
pub fn play(&mut self, anim_name: &str)        //? Switch animation (resets frame/timer)
pub fn is_finished(&self) -> bool               //? Non-looping anim reached last frame?
pub fn current_animation_name(&self) -> Option<&str>
pub fn get_progress(&self) -> f32               //? 0.0–1.0 through current animation
```

### Usage from `game`

The game wraps the engine's `AnimationState` with spritesheet-specific logic:

```rust
//? Define animations with grid-based start frames
let animations = vec![
    Animation::new("Idle", 0, 4, 0.12, true),     //? Row 0, 4 frames, looping
    Animation::new("Run", 5, 4, 0.08, true),       //? Row 1, 4 frames, looping
    Animation::new("Jump", 10, 3, 0.1, false),     //? Row 2, 3 frames, one-shot
    Animation::new("Fall", 13, 2, 0.1, true),      //? Row 2 continued
    Animation::new("Death", 15, 4, 0.1, false),    //? Row 3, one-shot
];

let mut anim_state = AnimationState::new(animations, "Idle");

//? In fixed_update: switch based on player state
match player.state {
    PlayerState::Idle => anim_state.play("Idle"),
    PlayerState::Run => anim_state.play("Run"),
    PlayerState::Jump => anim_state.play("Jump"),
    PlayerState::Fall => anim_state.play("Fall"),
    _ => {}
}

//? In update: advance timer
anim_state.update(ctx.delta_time);

//? In render: get the current animation def and frame index
if let Some((anim_def, frame_idx)) = anim_state.current() {
    let sprite_frame = anim_def.start_frame + frame_idx;
    let col = sprite_frame % SHEET_COLS;
    let row = sprite_frame / SHEET_COLS;
    let frame_rect = Rect::new(col as f32 * FRAME_W, row as f32 * FRAME_H, FRAME_W, FRAME_H);
    ctx.draw_sprite_from_sheet(
        position, size, [1.0, 1.0, 1.0, 1.0],
        frame_rect, !facing_right, texture_id,
    );
}

//? Combat animations derive frame duration from FSM data
let move_data = move_db.get_base(MoveId::AttackHorizontal);
let frame_dur = move_data.anim_frame_duration(4); //? 4 sprite frames
Animation::new("AttackHorizontal", 25, 4, frame_dur, false);
```

---

## Time

Fixed-timestep timing system for deterministic game logic.

**Module:** `engine::time`

### Constants

```rust
pub const DEFAULT_FIXED_HZ: u32 = 60;
pub const MAX_STEPS: u32 = 5;  //? Spiral-of-death cap
```

### FixedTime

```rust
pub struct FixedTime {
    pub tick: u64,       //? Monotonic tick counter
    pub fixed_dt: f32,   //? Duration of one fixed step
}

pub fn new(hz: u32) -> Self
pub fn accumulate(&mut self, dt: f32) -> u32   //? Returns number of steps to run
pub fn advance(&mut self)                       //? Consume one step, increment tick
pub fn interpolation_alpha(&self) -> f32        //? Leftover fraction for smoothing
pub fn tick_rate(&self) -> u32
pub fn set_tick_rate(&mut self, hz: u32)
pub fn freeze(&mut self, frames: u16)           //? Pause physics for N frames
pub fn is_frozen(&self) -> bool
pub fn freeze_remaining(&self) -> u16
```

### Usage from `game`

`FixedTime` is managed by the engine and passed to `fixed_update()`:

```rust
fn fixed_update(&mut self, ctx: &mut Context, fixed_time: &FixedTime) {
    //? Use tick for deterministic frame-data combat
    let current_tick = fixed_time.tick;
    self.player.current_tick = current_tick;

    //? Use fixed_dt for physics integration
    entity.velocity.y += GRAVITY * fixed_time.fixed_dt;
    entity.position += entity.velocity * fixed_time.fixed_dt;

    //? Hitstop via freeze
    ctx.trigger_freeze(5); //? 5-frame pause on a heavy hit
}
```

---

## Math Utilities

**Module:** `engine::math`

### move_towards

```rust
pub fn move_towards(current: f32, target: f32, max_delta: f32) -> f32
```

Move `current` toward `target` by at most `max_delta`. Snaps exactly to `target` if within `max_delta`, avoiding overshooting. Mirrors Unity's `Mathf.MoveTowards`.

```rust
use engine::math::move_towards;

//? Smooth deceleration
velocity_x = move_towards(velocity_x, 0.0, GROUND_DECEL * dt);

//? Smooth acceleration toward target speed
velocity_x = move_towards(velocity_x, target_speed, ACCELERATION * dt);
```

---

## Noise and Scene

Procedural background rendering.

**Module:** `engine::noise`

### SceneParams

```rust
pub struct SceneParams {
    pub background_color: [f32; 3],  //? RGB (0.0–1.0)
    pub seed: u32,
    pub fog_enabled: bool,
    pub fog_density: f32,
    pub fog_opacity: f32,
    pub fog_color: [f32; 3],
    pub fog_anim_speed: f32,
    pub time: f32,
}
```

`SceneParams` is passed to the `ui()` method and controls the CPU noise background pass. The engine renders a Perlin fog layer on top of a gradient background, animated over time.

### Noise Functions

```rust
pub fn hex_to_rgb(hex: &str) -> (u8, u8, u8)
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32
pub fn draw_gradient(buffer, width, height, top_rgb, bot_rgb)
pub fn apply_fog(buffer, width, height, time, density, opacity, perlin, seed, fog_rgb, anim_speed)
pub fn render_scene_to_buffer(buffer, width, height, params, perlin_cache)
```

These are called internally by the engine render loop. Game code typically only modifies `SceneParams` through the `ui()` callback.

---

## Re-exports

The engine's `lib.rs` re-exports commonly used types for ergonomic imports:

```rust
//? Instead of engine::context::Context, just use:
use engine::Context;

//? All re-exports:
pub use audio::{AudioManager, AudioResponse, AudioTrack, UiAudioEvent, load_sound_data};
pub use camera::ScreenShake;
pub use context::Context;
pub use glam::{Vec2, Vec3, Vec4};
pub use input::{GameAction, InputMap, InputState, Key, MouseBinding};
pub use kira::sound::static_sound::StaticSoundData;
pub use math::move_towards;
pub use physics::{AABB, BoxVolume, CollisionLayer, SweepResult};
pub use sprite::{BlendMode, Rect};
pub use animation::{AnimationDef, AnimationState};
pub use texture::Texture;
pub use texture_manager::TextureHandle;
pub use time::FixedTime;

//? Re-exported dependencies for direct use by game crates:
pub use egui;
#[cfg(not(target_arch = "wasm32"))]
pub use gilrs;
```

### Entry Points

```rust
//? Native: blocks on async GPU init
pub fn run<G: GameApp>()

//? WASM: non-blocking entry
pub fn run_wasm<G: GameApp>()
```
