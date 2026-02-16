/**-------------------------------------------------------------------------------------
*!  Scene parameter state shared between the UI and renderer.
*?  Game-specific wrapper around engine `SceneParams` for game specific settings.
*-------------------------------------------------------------------------------------**/
use engine::scene::SceneParams;

#[derive(Debug, Clone, Default)]
pub struct GameScene {
    pub params: SceneParams,
    pub show_collision_box: bool,
}

//? Keep the game wrapper in sync with engine-owned `SceneParams`.
pub fn show_ui(ctx: &egui::Context, scene: &mut GameScene, params: &mut SceneParams) {
    scene.params = params.clone();

    let content_rect = ctx.available_rect();
    let window_width = 280.0f32.min(content_rect.width() * 0.9);

    egui::Window::new("Game Exposed Controls")
        .default_open(false)
        .default_width(window_width)
        .default_pos([10.0, 10.0])
        .constrain(true)
        .show(ctx, |ui| {
            ui.label(
                "Move: WASD/L/R
Attack: LMB/X/Square
Block: RMB/RB/R1
Roll: Alt/RT/R2
Jump: Spacebar/A/X
Run: Shift/LS/L3",
            );
            ui.separator();
            ui.checkbox(&mut scene.show_collision_box, "Show collision Box");
        });
}
