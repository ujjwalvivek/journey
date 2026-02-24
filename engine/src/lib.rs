/**----------------------------------------------------------------------------
*!  Journey Engine: A cross-platform rendering engine.
*?  Provides a trait-based game loop architecture where games implement
*?  `GameApp` to define their logic, while the engine handles rendering,
*?  input, and window management.

**  Modules for different engine subsystems. These are kept private to
**  encapsulate implementation details, but their public types are re-exported
**  for convenience.
*----------------------------------------------------------------------------**/
pub mod animation;
pub mod camera;
pub mod context;
pub mod input;
pub mod math;
pub mod noise;
pub mod physics;
mod runtime;
pub mod scene;
pub mod sprite;
pub mod texture;
pub mod texture_manager;
pub mod time;

//* Re-export commonly used types
pub use camera::ScreenShake;
pub use context::Context;
pub use glam::{Vec2, Vec3, Vec4};
pub use input::{GameAction, InputState, Key};
pub use math::move_towards;
pub use physics::{AABB, BoxVolume, CollisionLayer, SweepResult};
pub use sprite::BlendMode;
pub use sprite::Rect;
pub use texture::Texture;
pub use texture_manager::TextureHandle;
pub use time::FixedTime;

pub trait GameApp: 'static {
    //* Initialize the game state. Called once when the engine starts.
    //? Use `ctx` to access screen dimensions and other initial state.
    fn init(ctx: &mut Context) -> Self;

    //* Fixed-rate update for deterministic game logic (physics, combat).
    //? Called at exactly `fixed_time.fixed_dt` intervals (default 60 Hz).
    //? `ctx.delta_time` equals `fixed_time.fixed_dt`. Use `fixed_time.tick`
    //? for frame-data combat windows instead of float accumulators.
    fn fixed_update(&mut self, _ctx: &mut Context, _fixed_time: &time::FixedTime) {}

    //? Use for camera smoothing, interpolation, and non-gameplay-critical updates.
    fn update(&mut self, ctx: &mut Context);

    //* Render the game. Called every frame after `update`.
    //? Use `ctx.draw_sprite()` to submit draw calls. Sprites are rendered after the background but before the UI overlay.
    fn render(&mut self, ctx: &mut Context);
    fn ui(&mut self, _ctx: &egui::Context, _scene_params: &mut crate::scene::SceneParams) {}
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
