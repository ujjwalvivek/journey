#![allow(dead_code)]
/**--------------------------------------------------------------------------------
*!  Physics and game feel configuration constants.
*?  Centralized tuning for player physics, combat timing, and impact feel params.
*?  Constants used across modules via wildcard import
*?  (`use config::*`) for easy access and maintainability.
*--------------------------------------------------------------------------------**/
pub const PIXELS_PER_UNIT: f32 = 32.0;

//? Player stats and tunable physics parameters.
//* All spatial values are in pixels/sec. Timing values are in seconds.
#[derive(Debug, Clone)]
pub struct PlayerStats {
    pub max_speed: f32,
    pub acceleration: f32,
    pub ground_decel: f32,
    pub air_decel: f32,
    pub grounding_force: f32,
    pub jump_power: f32,
    pub max_fall_speed: f32,
    pub fall_acceleration: f32,
    pub jump_end_early_gravity_mod: f32,
    pub coyote_time: f32,
    pub jump_buffer: f32,
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self {
            max_speed: 14.0 * PIXELS_PER_UNIT,
            acceleration: 120.0 * PIXELS_PER_UNIT,
            ground_decel: 60.0 * PIXELS_PER_UNIT,
            air_decel: 30.0 * PIXELS_PER_UNIT,
            grounding_force: 1.5 * PIXELS_PER_UNIT,
            jump_power: 36.0 * PIXELS_PER_UNIT,
            max_fall_speed: 40.0 * PIXELS_PER_UNIT,
            fall_acceleration: 110.0 * PIXELS_PER_UNIT,
            jump_end_early_gravity_mod: 3.0,
            coyote_time: 0.15,
            jump_buffer: 0.20,
        }
    }
}

//? If attack pressed during this window before recovery ends, it queues
pub const ATTACK_BUFFER_WINDOW: f32 = 0.5;

//? Combo Window Start: Time after which next attack can be chained (seconds)
pub const COMBO_WINDOW_START: f32 = 1.0;

pub const BLOCK_MAX_DURATION: f32 = 0.5;
pub const DODGE_DURATION: f32 = 0.3;

//? Dodge impulse force (pixels per second)
pub const DODGE_IMPULSE: f32 = 500.0;

//? Impact Parameters
pub const HITSTOP_LIGHT: f32 = 0.05;
pub const HITSTOP_HEAVY: f32 = 0.12;

//? Movement threshold for stopping (prevents micro-sliding)
//* If velocity magnitude < this, snap to zero (Zeno's Paradox fix)
pub const VELOCITY_EPSILON: f32 = 5.0;

//? Walk/run tuning
pub const WALK_SPEED_RATIO: f32 = 0.5;

//? Physics Parameters
pub const PLAYER_WIDTH: f32 = 30.0;
pub const PLAYER_HEIGHT: f32 = 128.0;
pub const VISUAL_OFFSET_Y: f32 = -36.0;
pub const VISUAL_OFFSET_X: f32 = -24.0;
