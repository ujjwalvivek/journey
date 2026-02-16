/**-----------------------------------------------------------------
*!  Scene parameter state shared between the UI and renderer.
*?  Controls for the procedural sky/fog generation pipeline.
*-----------------------------------------------------------------**/
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
            background_color: [1.0, 0.827, 0.0], // #FFD300
            seed: 42,
            fog_enabled: true,
            fog_density: 10.0,
            fog_opacity: 1.0,
            fog_color: [1.0, 1.0, 1.0], // #ffffff
            fog_anim_speed: 0.5,
            time: 0.0,
        }
    }
}

//? UI-only helper: Render an egui window that allows editing `SceneParams`.
pub fn show_ui(ctx: &egui::Context, params: &mut SceneParams) {
    egui::Window::new("Engine Debug Controls ")
        .default_open(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Background:");
                ui.color_edit_button_rgb(&mut params.background_color);
            });
            if params.fog_enabled {
                ui.horizontal(|ui| {
                    ui.label("Fog:");
                    ui.color_edit_button_rgb(&mut params.fog_color);
                });
            }

            ui.checkbox(&mut params.fog_enabled, "Enable Fog");
            if params.fog_enabled {
                ui.add(egui::Slider::new(&mut params.seed, 0..=999).text("Seed"));
                ui.add(egui::Slider::new(&mut params.fog_density, 0.5..=10.0).text("Density"));
                ui.add(egui::Slider::new(&mut params.fog_opacity, 0.0..=1.0).text("Opacity"));
                ui.add(
                    egui::Slider::new(&mut params.fog_anim_speed, 0.0..=2.0)
                        .text("Animation Speed"),
                );
            }
        });
}
