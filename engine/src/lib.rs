//! Journey Engine — cross-platform rendering engine.
//!
//! Re-exports the legacy procedural noise pipeline ([`noise`]) alongside the
//! wGPU + egui runtime ([`runtime`]). Both native and WASM targets use the
//! same rendering pipeline (CPU noise → GPU texture → full-screen quad
//! with egui overlay).

pub mod noise;
pub mod scene;
mod runtime;

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

/// Start the Journey Engine rendering loop.
///
/// On native, blocks until the window is closed. On WASM, launches
/// the event loop into the browser's animation frame system.
pub async fn run() {
    init_logging();
    runtime::start();
}

/// WASM entry point — called automatically when the module is instantiated.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn wasm_main() {
    init_logging();
    runtime::start();
}
