/**--------------------------------------------------------------------------------
 *!  Physics and game feel configuration constants.
 *?  Centralized tuning for player physics, combat timing, and impact feel params.
 *?  Constants used across modules via wildcard import
 *?  (`use config::*`) for easy access and maintainability.
 *--------------------------------------------------------------------------------**/
pub const PIXELS_PER_UNIT: f32 = 16.0;

//? Player stats and tunable physics parameters.
//* All spatial values are in pixels/sec. Timing values are in fixed ticks (60Hz).
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
    pub coyote_ticks: u16,
    pub jump_buffer_ticks: u16,
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self {
            max_speed: 14.0 * PIXELS_PER_UNIT,
            acceleration: 500.0 * PIXELS_PER_UNIT,
            ground_decel: 500.0 * PIXELS_PER_UNIT,
            air_decel: 40.0 * PIXELS_PER_UNIT,
            grounding_force: 1.5 * PIXELS_PER_UNIT,
            jump_power: 36.0 * PIXELS_PER_UNIT,
            max_fall_speed: 40.0 * PIXELS_PER_UNIT,
            fall_acceleration: 110.0 * PIXELS_PER_UNIT,
            jump_end_early_gravity_mod: 3.0,
            coyote_ticks: 6,
            jump_buffer_ticks: 8,
        }
    }
}

pub const DASH_DISTANCE: f32 = 100.0;
pub const DASH_SPEED: f32 = DASH_DISTANCE / (DASH_DURATION_TICKS as f32 / 60.0);
pub const DASH_DURATION_TICKS: u16 = 8;
pub const DASH_COOLDOWN_TICKS: u16 = 12;
pub const WALL_GRAB_TIMEOUT_TICKS: u16 = 10;
pub const WALL_SLIDE_SPEED: f32 = 120.0;
pub const WALL_JUMP_POWER_X: f32 = 350.0;
pub const WALL_JUMP_POWER_Y: f32 = 450.0;
pub const WALL_JUMP_LOCK_TICKS: u16 = 30;
pub const WALL_DETACH_GRACE_TICKS: u16 = 5;
pub const GRAPPLE_PULL_SPEED: f32 = 400.0;
pub const GRAPPLE_DETECT_RANGE: f32 = 120.0;
//todo: wire up in parry system
// pub const PARRY_KNOCKBACK: f32 = 40.0;
// pub const ATTACK_THROUGH_OFFSET: f32 = 100.0;
pub const HITSTOP_PARRY_TICKS: u16 = 3;
pub const HITSTOP_KILL_TICKS: u16 = 5;

pub const VELOCITY_EPSILON: f32 = 2.0;

pub const PLAYER_WIDTH: f32 = 8.0;
pub const PLAYER_HEIGHT: f32 = 24.0;
pub const VISUAL_OFFSET_Y: f32 = -3.0;
pub const VISUAL_OFFSET_X: f32 = -4.0;

//? 3-sided coverage: front, top, bottom. Back left exposed.
//? Offset toward facing direction so front extends 12px, back only 4px.
pub const PARRY_BOX_WIDTH: f32 = 16.0;
pub const PARRY_BOX_HEIGHT: f32 = 32.0;
pub const PARRY_BOX_FRONT_OFFSET: f32 = 4.0; //* Center shifted 4px toward face

pub const ENEMY_WIDTH: f32 = 9.0;
pub const ENEMY_HEIGHT: f32 = 32.0;
pub const ENEMY_PATROL_SPEED: f32 = 2.0 * PIXELS_PER_UNIT;
pub const ENEMY_AGGRO_RANGE: f32 = 120.0;
pub const ENEMY_MELEE_RANGE: f32 = 24.0; //* Proximity trigger for melee punish
pub const ENEMY_STAGGER_TICKS: u16 = 60;
pub const ENEMY_AIM_TICKS: u16 = 30; //* Delay before firing (Grunt default)
pub const ENEMY_MELEE_WINDUP_TICKS: u16 = 12;
pub const ENEMY_LEDGE_SENSOR_SIZE: f32 = 4.0; //* 4×4 pixel sensor ahead of feet

//? Runtime-tunable physics configuration.
#[derive(Debug, Clone)]
pub struct PhysicsConfig {
    pub gravity: f32,
    pub max_fall_speed: f32,
    pub movement_speed: f32,
    pub acceleration: f32,
    pub ground_decel: f32,
    pub air_decel: f32,
    pub jump_power: f32,
    pub jump_end_early_gravity_mod: f32,
    pub coyote_ticks: u16,
    pub jump_buffer_ticks: u16,
    pub dash_speed: f32,
    pub dash_duration_frames: u16,
    pub wall_slide_speed: f32,
    pub wall_jump_power_x: f32,
    pub wall_jump_power_y: f32,
    pub wall_grab_timeout_ticks: u16,
    pub wall_jump_lock_ticks: u16,
    pub grapple_pull_speed: f32,
    pub grapple_slingshot_force: f32,
    pub grapple_slingshot_ticks: u16,
    pub grapple_bounce_velocity_x: f32,
    pub grapple_bounce_velocity_y: f32,
    pub knockback: f32,
    pub enemy_patrol_speed: f32,
    pub enemy_aggro_range: f32,
    pub enemy_melee_range: f32,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            gravity: 110.0 * PIXELS_PER_UNIT,
            max_fall_speed: 40.0 * PIXELS_PER_UNIT,
            movement_speed: 14.0 * PIXELS_PER_UNIT,
            acceleration: 500.0 * PIXELS_PER_UNIT,
            ground_decel: 500.0 * PIXELS_PER_UNIT,
            air_decel: 40.0 * PIXELS_PER_UNIT,
            jump_power: 36.0 * PIXELS_PER_UNIT,
            jump_end_early_gravity_mod: 3.0,
            coyote_ticks: 6,
            jump_buffer_ticks: 8,
            dash_speed: DASH_SPEED,
            dash_duration_frames: DASH_DURATION_TICKS,
            wall_slide_speed: WALL_SLIDE_SPEED,
            wall_jump_power_x: WALL_JUMP_POWER_X,
            wall_jump_power_y: WALL_JUMP_POWER_Y,
            wall_grab_timeout_ticks: WALL_GRAB_TIMEOUT_TICKS,
            wall_jump_lock_ticks: WALL_JUMP_LOCK_TICKS,
            grapple_pull_speed: GRAPPLE_PULL_SPEED,
            grapple_slingshot_force: 800.0,
            grapple_slingshot_ticks: 5,
            grapple_bounce_velocity_x: 600.0,
            grapple_bounce_velocity_y: -250.0,
            knockback: 600.0,
            enemy_patrol_speed: ENEMY_PATROL_SPEED,
            enemy_aggro_range: ENEMY_AGGRO_RANGE,
            enemy_melee_range: ENEMY_MELEE_RANGE,
        }
    }
}
