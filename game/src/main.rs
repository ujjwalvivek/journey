/**--------------------------------------------------------------------------------
*!  Journey - Native (desktop) entry point.
*?  A 2D souls-like game built with a custom ECS Rust game engine
*?  It initializes the game and starts the main loop by calling `engine::run()`,
*--------------------------------------------------------------------------------**/
use game::JourneyGame;

fn main() {
    log::info!("Launching Journey Engine...");
    engine::run::<JourneyGame>();
}
