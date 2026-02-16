/**----------------------------------------------------
*!  Game context providing access to engine systems.
*----------------------------------------------------**/
use crate::input::InputState;
use crate::sprite::{Rect, Sprite};
use glam::Vec2;

//? This struct is passed to `GameApp` methods and provides:
//? - Input state (keyboard, mouse, gamepad)
//? - Delta time for frame-rate independent updates
//? - Sprite drawing API
//? - Screen dimensions
//? - Hitstop timer for freeze-frame effects on impact
//? - Sprite batch (current frame, internal use, cleared each frame)
pub struct Context {
    pub input: InputState,
    pub delta_time: f32,
    pub screen_width: f32,
    pub screen_height: f32,
    pub camera_offset_x: f32,
    pub hitstop_timer: f32,

    //* pub(crate) - public only within the current crate
    //* Vec<T> - growable, heap-allocated array
    pub(crate) sprite_batch: Vec<Sprite>,
}

impl Context {
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
            hitstop_timer: 0.0,
        }
    }

    //? Trigger hitstop (freeze-frame) for the specified duration.
    //? Returns max between current hitstop timer and new duration.
    //* Used for heavy impact"crunch", allows multiple hits to extend the effect.
    pub fn trigger_hitstop(&mut self, duration: f32) {
        self.hitstop_timer = self.hitstop_timer.max(duration);
    }

    //? Draw a sprite (colored rectangle or textured).
    //? # Arguments:
    //? - position: Top-left corner in screen space (0,0)
    //? - size: Width and height in pixels
    //? - color: RGBA color (each component 0.0 to 1.0)
    //? - flip_x: Horizontally flip the sprite
    //* Builder Pattern: create a Sprite with required fields, then chain optional modifiers (like flip).
    pub fn draw_sprite(&mut self, position: Vec2, size: Vec2, color: [f32; 4], flip_x: bool) {
        self.sprite_batch
            .push(Sprite::new(position, size, color).with_flip(flip_x));
        /* //! push to the array just like in C++ std::vector or python list. */
    }

    //? Draw a rectangle (solid colored, uses white pixel texture).
    //? # Arguments:
    //? - position: Top-left corner in screen space (0,0)
    //? - size: Width and height in pixels
    //? - color: RGBA color (each component 0.0 to 1.0)
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

    pub fn screen_center(&self) -> Vec2 {
        Vec2::new(self.screen_width / 2.0, self.screen_height / 2.0)
    }

    //? Clear sprite batch (called internally between frames).
    //* Method is public within the crate, not outside.
    pub(crate) fn clear_sprites(&mut self) {
        self.sprite_batch.clear();
    }

    //? Update screen dimensions on window resize.
    pub(crate) fn resize(&mut self, width: f32, height: f32) {
        self.screen_width = width;
        self.screen_height = height;
    }
}
