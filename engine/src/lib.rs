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
pub mod audio;
pub mod camera;
pub mod context;
pub mod input;
pub mod math;
pub mod noise;
pub mod physics;
mod runtime;
pub mod sprite;
pub mod texture;
pub mod texture_manager;
pub mod time;
pub mod ui;

//* Re-export commonly used types
pub use animation::{AnimationDef, AnimationState};
pub use audio::{AudioManager, AudioResponse, AudioTrack, UiAudioEvent, load_sound_data};
pub use camera::ScreenShake;
pub use context::Context;
pub use glam::{Vec2, Vec3, Vec4};
pub use input::{GameAction, InputMap, InputState, Key, MouseBinding};
pub use kira::sound::static_sound::StaticSoundData;
pub use math::move_towards;
pub use physics::{AABB, BoxVolume, CollisionLayer, SweepResult};
pub use sprite::BlendMode;
pub use sprite::Rect;

pub use egui;
#[cfg(not(target_arch = "wasm32"))]
pub use gilrs;
pub use texture::Texture;
pub use texture_manager::TextureHandle;
pub use time::FixedTime;

#[derive(Debug, Clone, PartialEq)]
pub struct SceneParams {
    pub background_color: [f32; 3],
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
            background_color: [0.67, 0.42, 0.85], //* #AC6CDA
            seed: 42,
            fog_enabled: true,
            fog_density: 10.0,
            fog_opacity: 1.0,
            fog_color: [0.41, 0.36, 0.81], //* #685DCE
            fog_anim_speed: 0.5,
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
