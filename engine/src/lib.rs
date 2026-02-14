//! Journey Engine — cross-platform rendering engine.
//!
//! Re-exports the legacy procedural noise pipeline ([`noise`]) alongside the
//! new wGPU + egui runtime ([`runtime`], native-only for now).

pub mod noise;
pub mod scene;

#[cfg(not(target_arch = "wasm32"))]
mod runtime;

/// Start the Journey Engine rendering loop.
///
/// On native, this blocks until the window is closed (bridged via
/// `pollster::block_on` in the game binary). On WASM, initialization
/// hooks are set up but the full engine loop is not yet supported —
/// use [`noise::render_scene`] for the web rendering path.
pub async fn run() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::try_init().ok();
        runtime::start();
    }

    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        log::info!("WASM native engine loop not yet implemented — use noise::render_scene()");
    }
}
