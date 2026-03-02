/**----------------------------------------------------
*!  Game context providing access to engine systems.
*----------------------------------------------------**/
use crate::audio::{AudioManager, UiAudioEvent};
use crate::input::{GameAction, InputState};
use crate::sprite::{BlendMode, Rect, Sprite};
use glam::Vec2;

//? This struct is passed to `GameApp` methods and provides:
//? - Input state (keyboard, mouse, gamepad)
//? - Delta time for frame-rate independent updates
//? - Sprite drawing API
//? - Screen dimensions
//? Interpolation alpha for render-time smoothing between physics frames.
//? - Sprite batch (current frame, internal use, cleared each frame)
pub struct Context<A: GameAction> {
    pub input: InputState<A>,
    pub delta_time: f32,
    pub screen_width: f32,
    pub screen_height: f32,
    pub camera_offset_x: f32,
    pub camera_offset_y: f32,
    pub fps: f32,
    pub frame_time_ms: f32,
    pub fixed_tick_rate: u32,
    pub target_fps: u32,
    pub interpolation_alpha: f32,
    pub freeze_frames: u16,
    pub pending_shakes: Vec<(f32, f32)>,
    pub request_exit: bool,
    pub fullscreen_enabled: bool,
    pub request_fullscreen: Option<bool>,
    pub hdr_enabled: bool,
    pub request_hdr: Option<bool>,
    pub audio: AudioManager,
    pub pending_ui_audio: Vec<UiAudioEvent>,

    //* pub(crate) - public only within the current crate
    //* Vec<T> - growable, heap-allocated array
    pub(crate) sprite_batch: Vec<Sprite>,
    pub(crate) pending_textures: Vec<PendingTexture>,
}

//? A texture load request queued by the game during init.
pub(crate) struct PendingTexture {
    pub bytes: &'static [u8],
    pub label: String,
}

impl<A: GameAction> Context<A> {
    //? Create a new context with given screen dimensions
    //? and default values for other fields.
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            input: InputState::new(),
            delta_time: 0.0,
            screen_width,
            screen_height,
            sprite_batch: Vec::new(),
            camera_offset_x: 0.0,
            camera_offset_y: 0.0,
            fps: 0.0,
            frame_time_ms: 0.0,
            fixed_tick_rate: crate::time::DEFAULT_FIXED_HZ,
            target_fps: 60, //* Default to 60 FPS target. can be changed by the game
            interpolation_alpha: 0.0,
            freeze_frames: 0,
            pending_shakes: Vec::new(),
            pending_textures: Vec::new(),
            request_exit: false,
            fullscreen_enabled: true,
            request_fullscreen: None,
            hdr_enabled: false,
            request_hdr: None,
            audio: AudioManager::new(),
            pending_ui_audio: Vec::new(),
        }
    }

    //? Queue a texture for loading. Called during `GameApp::init()`.
    //* Returns a 1-based texture ID for use with `draw_sprite_from_sheet()`.
    //* The engine processes these after init completes.
    pub fn load_texture(&mut self, bytes: &'static [u8], label: &str) -> usize {
        let id = self.pending_textures.len() + 1; //*1-based (0 = white pixel)
        self.pending_textures.push(PendingTexture {
            bytes,
            label: label.to_string(),
        });
        id
    }

    //? Trigger frame-perfect freeze for N render frames. FSM and physics pause.
    pub fn trigger_freeze(&mut self, frames: u16) {
        self.freeze_frames = self.freeze_frames.max(frames);
    }

    //? Trigger a screen shake effect with given intensity and duration (in seconds).
    pub fn trigger_shake(&mut self, intensity: f32, duration: f32) {
        self.pending_shakes.push((intensity, duration));
    }

    //? Draw a sprite (colored rectangle or textured).
    //? # Arguments:
    //? - position: Top-left corner in screen space (0,0)
    //? - size: Width and height in pixels
    //? - color: RGBA color (each component 0.0 to 1.0)
    //? - flip_x: Horizontally flip the sprite
    pub fn draw_sprite(&mut self, position: Vec2, size: Vec2, color: [f32; 4], flip_x: bool) {
        self.sprite_batch
            .push(Sprite::new(position, size, color).with_flip(flip_x));
        //* push to the array just like in C++ std::vector or python list.
    }

    //* Same as draw_sprite, without flip
    pub fn draw_rect(&mut self, position: Vec2, size: Vec2, color: [f32; 4]) {
        self.sprite_batch.push(Sprite::new(position, size, color));
    }

    //? Draw a sprite from a sprite sheet.
    //? # Arguments:
    //? - position: Top-left corner in screen space (0,0)
    //? - size: Width and height in pixels
    //? - color: RGBA color (each component 0.0 to 1.0)
    //? - source_rect: Rectangle defining the sprite sheet region (pixel coordinates)
    //? - flip_x: Horizontally flip the sprite
    //? - texture_id: Which texture to use (1+ for game textures, 0 for white pixel)
    pub fn draw_sprite_from_sheet(
        &mut self,
        position: Vec2,
        size: Vec2,
        color: [f32; 4],
        source_rect: Rect,
        flip_x: bool,
        texture_id: usize,
    ) {
        self.sprite_batch.push(
            Sprite::new(position, size, color)
                .with_source(source_rect)
                .with_flip(flip_x)
                .with_texture_id(texture_id),
        );
    }

    //? Draw a sprite from a sprite sheet with additive blending.
    pub fn draw_sprite_from_sheet_additive(
        &mut self,
        position: Vec2,
        size: Vec2,
        color: [f32; 4],
        source_rect: Rect,
        flip_x: bool,
        texture_id: usize,
    ) {
        self.sprite_batch.push(
            Sprite::new(position, size, color)
                .with_source(source_rect)
                .with_flip(flip_x)
                .with_texture_id(texture_id)
                .with_blend_mode(BlendMode::Additive),
        );
    }

    pub fn screen_center(&self) -> Vec2 {
        Vec2::new(self.screen_width / 2.0, self.screen_height / 2.0)
    }

    //? Deduplicate UI audio events queued during this frame.
    pub(crate) fn drain_ui_audio_events(&mut self) {
        if self.pending_ui_audio.is_empty() {
            return;
        }
        self.pending_ui_audio.sort_unstable();
        self.pending_ui_audio.dedup();
    }

    //? Clear sprite batch (called internally between frames).
    //* Method is public within the crate, not outside.
    pub(crate) fn clear_sprites(&mut self) {
        self.sprite_batch.clear();
    }

    //? Update screen dimensions on window resize.
    #[allow(dead_code)] //* needed if dynamic resolution or windowed mode is added
    pub(crate) fn resize(&mut self, width: f32, height: f32) {
        self.screen_width = width;
        self.screen_height = height;
    }

    pub fn set_fullscreen_enabled(&mut self, enabled: bool) {
        self.request_fullscreen = Some(enabled);
    }

    pub fn set_hdr_enabled(&mut self, enabled: bool) {
        self.request_hdr = Some(enabled);
    }
}
