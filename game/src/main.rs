/**--------------------------------------------------------------------------------
*!  Journey - Native (desktop) entry point.
*?  A 2D soulslike x metroidvania built on a fixed-step Rust/WGPU engine.
*?  It initializes the game and starts the main loop by calling `engine::run()`,
*--------------------------------------------------------------------------------**/
use game::JourneyGame;

fn main() {
    log::info!("Launching Journey Engine...");
    engine::run::<JourneyGame>();
}
