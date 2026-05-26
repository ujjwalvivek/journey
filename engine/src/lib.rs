/**----------------------------------------------------------------------------
*!  A cross-platform 2D game engine built with Rust and wGPU.
*?  Provides a trait-based game loop architecture where games implement
*?  `GameApp` to define their logic, while the engine handles rendering,
*?  input, and window management.

**  Modules for different engine subsystems. These are kept private to
**  encapsulate implementation details, but their public types are re-exported
**  for convenience.
*----------------------------------------------------------------------------**/
pub mod animation;
pub mod atmosphere;
pub mod audio;
pub mod camera;
pub mod context;
pub mod input;
pub mod math;
pub mod physics;
mod runtime;
pub mod sprite;
pub mod texture;
pub mod texture_manager;
pub mod time;
pub mod ui;

//* Re-export commonly used types
pub use animation::{AnimationDef, AnimationState};
pub use audio::{
    AudioManager, AudioResponse, AudioTrack, UiAudioEvent, load_sound_data,
    sound_data_from_mono_samples,
};
pub use camera::ScreenShake;
pub use context::{Context, FrameStats};
pub use glam::{Vec2, Vec3, Vec4};
pub use input::{GameAction, InputMap, InputState, Key, MouseBinding};
pub use kira::sound::static_sound::StaticSoundData;
pub use math::move_towards;
pub use physics::{AABB, BoxVolume, CollisionLayer, SpatialGrid, SweepResult};
pub use sprite::{BlendMode, Rect, RenderLayer};

pub use egui;
#[cfg(not(target_arch = "wasm32"))]
pub use gilrs;
pub use texture::Texture;
pub use texture_manager::TextureHandle;
pub use time::FixedTime;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BloomSettings {
    pub enabled: bool,
    pub threshold: f32,
    pub intensity: f32,
    pub radius: f32,
}

impl Default for BloomSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold: 0.7,
            intensity: 0.35,
            radius: 2.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkyParams {
    pub enabled: bool,
    pub horizon_glow: f32,
    pub top_color: [f32; 3],
    pub horizon_color: [f32; 3],
    pub bottom_color: [f32; 3],
    pub horizon_y: f32,
    pub horizon_width: f32,
}

impl Default for SkyParams {
    fn default() -> Self {
        Self {
            enabled: false,
            horizon_glow: 0.9,
            top_color: [0.035, 0.09, 0.105],
            horizon_color: [0.30, 0.24, 0.13],
            bottom_color: [0.035, 0.055, 0.05],
            horizon_y: 0.66,
            horizon_width: 0.36,
        }
    }
}

impl SkyParams {
    pub fn lerp(&self, other: &SkyParams, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            enabled: self.enabled || other.enabled,
            horizon_glow: lerp_f32(self.horizon_glow, other.horizon_glow, t),
            top_color: lerp_color3(self.top_color, other.top_color, t),
            horizon_color: lerp_color3(self.horizon_color, other.horizon_color, t),
            bottom_color: lerp_color3(self.bottom_color, other.bottom_color, t),
            horizon_y: lerp_f32(self.horizon_y, other.horizon_y, t),
            horizon_width: lerp_f32(self.horizon_width, other.horizon_width, t),
        }
    }
}

fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

fn lerp_color3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        lerp_f32(a[0], b[0], t),
        lerp_f32(a[1], b[1], t),
        lerp_f32(a[2], b[2], t),
    ]
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkyTransition {
    pub current: SkyParams,
    pub target: SkyParams,
    pub duration: f32,
    pub elapsed: f32,
}

impl SkyTransition {
    pub fn new(current: SkyParams, target: SkyParams, duration: f32) -> Self {
        Self {
            current,
            target,
            duration: duration.max(0.001),
            elapsed: 0.0,
        }
    }

    pub fn progress(&self) -> f32 {
        (self.elapsed / self.duration).clamp(0.0, 1.0)
    }

    pub fn advance(&mut self, dt: f32) {
        self.elapsed = (self.elapsed + dt).min(self.duration);
    }

    pub fn lerp(&self) -> SkyParams {
        self.current.lerp(&self.target, self.progress())
    }

    pub fn done(&self) -> bool {
        self.elapsed >= self.duration
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneParams {
    pub background_color: [f32; 3],
    pub sky: SkyParams,
    pub seed: u32,
    pub fog_enabled: bool,
    pub fog_density: f32,
    pub fog_opacity: f32,
    pub fog_color: [f32; 3],
    pub fog_anim_speed: f32,
    pub time: f32,
}

//? Default parameters for the scene, which can be overridden by the debug UI.
impl Default for SceneParams {
    fn default() -> Self {
        Self {
            background_color: [0.035, 0.055, 0.05],
            sky: SkyParams::default(),
            seed: 42,
            fog_enabled: true,
            fog_density: 14.0,
            fog_opacity: 0.55,
            fog_color: [0.08, 0.18, 0.14],
            fog_anim_speed: 0.35,
            time: 0.0,
        }
    }
}

pub trait GameApp: 'static {
    //? The game's action enum
    type Action: GameAction;

    fn window_title() -> &'static str {
        "Journey Engine"
    }

    fn window_icon() -> Option<&'static [u8]> {
        None
    }

    fn wasm_ready_event() -> Option<&'static str> {
        None
    }

    fn internal_resolution() -> (u32, u32) {
        (640, 360)
    }

    //* Initialize the game state. Called once when the engine starts.
    //? Use `ctx` to access screen dimensions and other initial state.
    fn init(ctx: &mut Context<Self::Action>) -> Self;

    //* Fixed-rate update for deterministic game logic (physics, combat).
    //? Called at exactly `fixed_time.fixed_dt` intervals (default 60 Hz).
    //? `ctx.delta_time` equals `fixed_time.fixed_dt`. Use `fixed_time.tick`
    //? for frame-data combat windows instead of float accumulators.
    fn fixed_update(&mut self, _ctx: &mut Context<Self::Action>, _fixed_time: &time::FixedTime) {}

    //? Use for camera smoothing, interpolation, and non-gameplay-critical updates.
    fn update(&mut self, ctx: &mut Context<Self::Action>);

    //* Render the game. Called every frame after `update`.
    //? Use `ctx.draw_sprite()` to submit draw calls. Sprites are rendered after the background but before the UI overlay.
    fn render(&mut self, ctx: &mut Context<Self::Action>);
    fn ui(
        &mut self,
        _egui_ctx: &egui::Context,
        _ctx: &mut Context<Self::Action>,
        _scene_params: &mut SceneParams,
    ) {
    }
}

fn init_logging() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::try_init().ok();
    }

    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        console_log::init_with_level(log::Level::Info).ok();
    }
}

pub fn run<G: GameApp>() {
    init_logging();
    runtime::start::<G>();
}

/**----------------------------------------------------------------------------
*!  WASM entry point {requires a default game type at compile time.}
*?  Note: WASM builds must specify a concrete game type since we can't
*?  use generics in the #[wasm_bindgen(start)] entry point. Games should
*?  export their own wasm_main that calls a type-specific run function.
*----------------------------------------------------------------------------**/
#[cfg(target_arch = "wasm32")]
pub fn run_wasm<G: GameApp>() {
    init_logging();
    runtime::start::<G>();
}
