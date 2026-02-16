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

    egui::Window::new("Game Exposed Controls")
        .default_open(false)
        .show(ctx, |ui| {
            ui.checkbox(&mut scene.show_collision_box, "Show collision Box");
        });
}
