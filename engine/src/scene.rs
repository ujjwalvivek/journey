//! Scene parameter state shared between the UI and renderer.

/// Controls for the procedural sky/fog generation pipeline.
///
/// Colors use `[f32; 3]` in `0.0..=1.0` range for direct use with
/// [`egui::Ui::color_edit_button_rgb`].
#[derive(Debug, Clone, PartialEq)]
pub struct SceneParams {
    pub top_color: [f32; 3],
    pub bottom_color: [f32; 3],
    pub seed: u32,
    pub fog_enabled: bool,
    pub fog_density: f32,
    pub fog_opacity: f32,
    pub fog_color: [f32; 3],
    pub fog_anim_speed: f32,
    pub time: f32,
}

impl Default for SceneParams {
    fn default() -> Self {
        Self {
            top_color: [0.529, 0.808, 0.922],    // #87ceeb sky blue
            bottom_color: [0.282, 0.463, 0.282], // #487648 forest green
            seed: 42,
            fog_enabled: true,
            fog_density: 3.0,
            fog_opacity: 0.35,
            fog_color: [0.85, 0.85, 0.9],
            fog_anim_speed: 0.3,
            time: 0.0,
        }
    }
}
