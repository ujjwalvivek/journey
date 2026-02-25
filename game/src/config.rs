/**--------------------------------------------------------------------------------
 *!  Physics and game feel configuration constants.
 *?  Centralized tuning for player physics, combat timing, and impact feel params.
 *?  Constants used across modules via wildcard import
 *?  (`use config::*`) for easy access and maintainability.
 *--------------------------------------------------------------------------------**/
//* All spatial values are in pixels/sec. Timing values are in fixed ticks (60Hz).
pub const PIXELS_PER_UNIT: f32 = 16.0;

//* Dash parameters.
pub const DASH_SPEED: f32 = 50.0 * PIXELS_PER_UNIT;
pub const DASH_DURATION_TICKS: u16 = 8;
pub const DASH_COOLDOWN_TICKS: u16 = 10;

//* Jump parameters.
pub const GROUNDING_FORCE: f32 = 1.5 * PIXELS_PER_UNIT;
pub const JUMP_POWER: f32 = 37.5 * PIXELS_PER_UNIT;
pub const MAX_FALL_SPEED: f32 = 40.0 * PIXELS_PER_UNIT;
pub const FALL_ACCELERATION: f32 = 110.0 * PIXELS_PER_UNIT;
pub const JUMP_END_EARLY_GRAVITY_MOD: f32 = 6.0;
pub const COYOTE_TICKS: u16 = 6;
pub const JUMP_BUFFER_TICKS: u16 = 8;

//* Wall interaction parameters.
pub const WALL_GRAB_TIMEOUT_TICKS: u16 = 10;
pub const WALL_SLIDE_SPEED: f32 = 7.5 * PIXELS_PER_UNIT;
pub const WALL_JUMP_POWER_X: f32 = 22.0 * PIXELS_PER_UNIT;
pub const WALL_JUMP_POWER_Y: f32 = 28.0 * PIXELS_PER_UNIT;
pub const WALL_JUMP_LOCK_TICKS: u16 = 30;
pub const WALL_DETACH_GRACE_TICKS: u16 = 5;

//* Grapple parameters.
pub const GRAPPLE_PULL_SPEED: f32 = 25.0 * PIXELS_PER_UNIT;
pub const GRAPPLE_DETECT_RANGE: f32 = 9.0 * PIXELS_PER_UNIT;
pub const GRAPPLE_SLINGSHOT_FORCE: f32 = 37.5 * PIXELS_PER_UNIT;
pub const GRAPPLE_SLINGSHOT_TICKS: u16 = 6;
pub const GRAPPLE_BOUNCE_VELOCITY_X: f32 = 37.5 * PIXELS_PER_UNIT;
pub const GRAPPLE_BOUNCE_VELOCITY_Y: f32 = -15.625 * PIXELS_PER_UNIT;

//* Hitstop parameters.
pub const HITSTOP_PARRY_TICKS: u16 = 3;
pub const HITSTOP_KILL_TICKS: u16 = 5;
pub const VELOCITY_EPSILON: f32 = 2.0;

//* Player dimensions and offsets.
pub const PLAYER_WIDTH: f32 = 0.5 * PIXELS_PER_UNIT;
pub const PLAYER_HEIGHT: f32 = 1.5 * PIXELS_PER_UNIT;
pub const VISUAL_OFFSET_Y: f32 = -3.0;
pub const VISUAL_OFFSET_X: f32 = -4.0;

//* Parrybox dimensions and offsets.
pub const PARRY_BOX_WIDTH: f32 = 0.625 * PIXELS_PER_UNIT;
pub const PARRY_BOX_HEIGHT: f32 = 2.0 * PIXELS_PER_UNIT;
pub const PARRY_BOX_FRONT_OFFSET: f32 = 4.0; //* Center shifted 4px toward face

//* Enemy dimensions and behavior parameters.
pub const ENEMY_WIDTH: f32 = 0.6 * PIXELS_PER_UNIT;
pub const ENEMY_HEIGHT: f32 = 2.0 * PIXELS_PER_UNIT;
pub const ENEMY_PATROL_SPEED: f32 = 2.5 * PIXELS_PER_UNIT;
pub const ENEMY_AGGRO_RANGE: f32 = 9.0 * PIXELS_PER_UNIT;
pub const ENEMY_MELEE_RANGE: f32 = 1.5 * PIXELS_PER_UNIT; //* Proximity trigger for melee punish
pub const KNOCKBACK_FORCE: f32 = 56.25 * PIXELS_PER_UNIT;
pub const ENEMY_STAGGER_TICKS: u16 = 60;
pub const ENEMY_AIM_TICKS: u16 = 20; //* Delay before firing (Grunt default)
pub const ENEMY_MELEE_WINDUP_TICKS: u16 = 10;
pub const ENEMY_LEDGE_SENSOR_SIZE: f32 = 4.0; //* 4×4 pixel sensor ahead of feet

//* Movement parameters.
pub const MAX_SPEED: f32 = 18.75 * PIXELS_PER_UNIT;
pub const MOVEMENT_SPEED: f32 = 18.75 * PIXELS_PER_UNIT;

//* Gravity and acceleration parameters.
pub const GRAVITY: f32 = 110.0 * PIXELS_PER_UNIT;
pub const ACCELERATION: f32 = 500.0 * PIXELS_PER_UNIT;
pub const GROUND_DECEL: f32 = 500.0 * PIXELS_PER_UNIT;
pub const AIR_DECEL: f32 = 40.0 * PIXELS_PER_UNIT;

//* Default player stats
pub type PlayerStats = PhysicsConfig;

//? Runtime-tunable physics configuration.
#[derive(Debug, Clone)]
pub struct PhysicsConfig {
    pub gravity: f32,
    pub max_fall_speed: f32,
    pub movement_speed: f32,
    pub max_speed: f32,
    pub acceleration: f32,
    pub ground_decel: f32,
    pub air_decel: f32,
    pub grounding_force: f32,
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
    pub fall_acceleration: f32,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            gravity: GRAVITY,
            max_fall_speed: MAX_FALL_SPEED,
            movement_speed: MOVEMENT_SPEED,
            max_speed: MAX_SPEED,
            acceleration: ACCELERATION,
            ground_decel: GROUND_DECEL,
            air_decel: AIR_DECEL,
            grounding_force: GROUNDING_FORCE,
            fall_acceleration: FALL_ACCELERATION,
            jump_power: JUMP_POWER,
            jump_end_early_gravity_mod: JUMP_END_EARLY_GRAVITY_MOD,
            coyote_ticks: COYOTE_TICKS,
            jump_buffer_ticks: JUMP_BUFFER_TICKS,
            dash_speed: DASH_SPEED,
            dash_duration_frames: DASH_DURATION_TICKS,
            wall_slide_speed: WALL_SLIDE_SPEED,
            wall_jump_power_x: WALL_JUMP_POWER_X,
            wall_jump_power_y: WALL_JUMP_POWER_Y,
            wall_grab_timeout_ticks: WALL_GRAB_TIMEOUT_TICKS,
            wall_jump_lock_ticks: WALL_JUMP_LOCK_TICKS,
            grapple_pull_speed: GRAPPLE_PULL_SPEED,
            grapple_slingshot_force: GRAPPLE_SLINGSHOT_FORCE,
            grapple_slingshot_ticks: GRAPPLE_SLINGSHOT_TICKS,
            grapple_bounce_velocity_x: GRAPPLE_BOUNCE_VELOCITY_X,
            grapple_bounce_velocity_y: GRAPPLE_BOUNCE_VELOCITY_Y,
            knockback: KNOCKBACK_FORCE,
            enemy_patrol_speed: ENEMY_PATROL_SPEED,
            enemy_aggro_range: ENEMY_AGGRO_RANGE,
            enemy_melee_range: ENEMY_MELEE_RANGE,
        }
    }
}
