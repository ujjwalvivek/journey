/**--------------------------------------------------------------------------------
*!  Player controller
*?  State-machine-driven: each PlayerState handles its own movement,
*?  transitions, coyote timings, jump buffering, and animations.
*?  Combat FSM handles frame-data timing.
*?  Uses Entity composition for shared physics/combat state.
*?  Combat input is sampled per render frame but consumed in fixed_update()
*?  via a tick-stamped CombatInputBuffer for deterministic FSM synchronization.
*--------------------------------------------------------------------------------**/
use crate::anim::AnimationState;
use crate::assets::{FRAME_HEIGHT, FRAME_WIDTH};
use crate::combat::fsm::{self, CombatPhase};
use crate::combat::input_buffer::CombatInputBuffer;
use crate::combat::moves::{MoveDatabase, MoveId};
use crate::config::*;
use crate::entity::{self, Entity};
use engine::{AABB, Context, GameAction, Vec2, math::move_towards};

//? The primary driver of movement behavior and animation.
//? Combat timing is still handled by the CombatState FSM inside entity.combat.
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum PlayerState {
    Idle,
    Run,
    Jump,
    Fall,
    Dash,
    AirDash,
    Parry,
    AttackHorizontal,
    AttackUp,
    AttackDown,
    WallGrab,
    WallSlide,
    GrapplePull,
    GrappleSlingshot,
    Death,
}

#[derive(Debug, Clone, Copy, Default)]
struct FrameInput {
    move_x: f32,
    move_y: f32,
    jump_down: bool,
    jump_held: bool,
}

//? Per-frame combat input snapshot -> sampled in update() -> consumed in fixed_update().
#[derive(Debug, Clone, Copy, Default)]
struct CombatInputSnapshot {
    attack_pressed: bool,
    block_pressed: bool,
    dash_pressed: bool,
    grapple_pressed: bool,
}

//? Player controller wraps Entity with player-specific input and animation logic.
pub struct Player {
    pub entity: Entity,
    pub state: PlayerState,
    pub anim_state: AnimationState,
    pub render_scale: f32,
    pub move_db: MoveDatabase,
    pub input_buffer: CombatInputBuffer,

    //? Stats and tunable parameters
    was_grounded: bool,
    stats: PlayerStats,
    frame_input: FrameInput,
    combat_snapshot: CombatInputSnapshot,
    pub current_tick: u64,
    jump_to_consume: bool,
    buffered_jump_usable: bool,
    ended_jump_early: bool,
    coyote_usable: bool,
    tick_jump_was_pressed: u64,
    tick_left_grounded: u64,
    pub dash_cooldown_timer: u16,
    has_air_dashed: bool,
    dash_direction: f32,
    wall_grab_timer: u16,
    wall_direction: f32,
    wall_jump_lock_timer: u16,
    wall_detach_timer: u16,
    last_wall_jump_dir: f32,
    wall_contact_lost_ticks: u16,
    grabbed_wall_bottom: f32,
    //? Ticks remaining after any wall-detach during which WallGrab re-entry is
    //? blocked. Prevents the player from latching onto air when their AABB still
    //? overlaps the top of a tile they just slid past the bottom of.
    wall_detach_cooldown: u16,

    //? Cached tuner values (synced each tick from PhysicsConfig)
    cached_wall_slide_speed: f32,
    cached_wall_jump_power_x: f32,
    cached_wall_jump_power_y: f32,
    cached_wall_grab_timeout: u16,
    cached_wall_jump_lock_ticks: u16,

    hitstop_timer: u16,
    hitstop_return_state: PlayerState,

    pub drop_through_timer: u16,
    pub grapple_target: Option<Vec2>,
    pub grapple_launch_dir: Vec2,
    grapple_slingshot_timer: u16,
    //? Set by lib.rs when the grapple target is a staggered enemy (not a static node).
    pub grapple_is_enemy_target: bool,
    //? Set by handle_grapple_movement when the player arrives at an enemy target.
    //? lib.rs reads this to decide execute (attack buffered) or bounce (no attack).
    pub grapple_arrived_at_enemy: bool,

    //? Previous physics position (for render-time interpolation at high refresh rates)
    prev_position: Vec2,

    pub is_dead: bool,
}

impl Player {
    pub fn new(start_pos: Vec2, anim_state: AnimationState) -> Self {
        Self {
            entity: Entity::new(
                start_pos,
                Vec2::new(PLAYER_WIDTH, PLAYER_HEIGHT),
                1.0,
                100.0,
            ),
            state: PlayerState::Idle,
            anim_state,
            move_db: MoveDatabase::default(),
            input_buffer: CombatInputBuffer::default(),
            was_grounded: false,
            stats: PlayerStats::default(),
            frame_input: FrameInput::default(),
            combat_snapshot: CombatInputSnapshot::default(),
            current_tick: 0,
            jump_to_consume: false,
            buffered_jump_usable: false,
            ended_jump_early: false,
            coyote_usable: false,
            tick_jump_was_pressed: 0,
            tick_left_grounded: 0,
            render_scale: 1.0,
            dash_cooldown_timer: 0,
            has_air_dashed: false,
            dash_direction: 0.0,
            wall_grab_timer: 0,
            wall_direction: 0.0,
            wall_jump_lock_timer: 0,
            wall_detach_timer: 0,
            last_wall_jump_dir: 0.0,
            wall_contact_lost_ticks: 0,
            grabbed_wall_bottom: f32::INFINITY,
            wall_detach_cooldown: 0,
            cached_wall_slide_speed: WALL_SLIDE_SPEED,
            cached_wall_jump_power_x: WALL_JUMP_POWER_X,
            cached_wall_jump_power_y: WALL_JUMP_POWER_Y,
            cached_wall_grab_timeout: WALL_GRAB_TIMEOUT_TICKS,
            cached_wall_jump_lock_ticks: WALL_JUMP_LOCK_TICKS,
            hitstop_timer: 0,
            hitstop_return_state: PlayerState::Idle,
            drop_through_timer: 0,
            grapple_target: None,
            grapple_launch_dir: Vec2::ZERO,
            grapple_slingshot_timer: 0,
            grapple_is_enemy_target: false,
            grapple_arrived_at_enemy: false,
            prev_position: start_pos,
            is_dead: false,
        }
    }

    pub fn position(&self) -> Vec2 {
        self.entity.position
    }
    pub fn set_position(&mut self, pos: Vec2) {
        self.entity.position = pos;
    }
    pub fn velocity(&self) -> Vec2 {
        self.entity.velocity
    }
    pub fn set_velocity(&mut self, vel: Vec2) {
        self.entity.velocity = vel;
    }
    pub fn facing_right(&self) -> bool {
        self.entity.facing_right
    }
    pub fn is_grounded(&self) -> bool {
        self.entity.is_grounded
    }
    pub fn has_air_dashed(&self) -> bool {
        self.has_air_dashed
    }
    pub fn wall_grab_timer(&self) -> u16 {
        self.wall_grab_timer
    }

    pub fn gather_input(&mut self, ctx: &Context) {
        if self.state == PlayerState::Death {
            return;
        }

        self.frame_input = FrameInput {
            move_x: ctx.input.get_move_x(),
            move_y: ctx.input.get_move_y(),
            jump_down: ctx.input.is_action_just_pressed(GameAction::Jump),
            jump_held: ctx.input.is_action_pressed(GameAction::Jump),
        };

        //? Accumulate between fixed steps
        self.combat_snapshot.attack_pressed |= ctx.input.is_action_just_pressed(GameAction::Attack);
        self.combat_snapshot.block_pressed |= ctx.input.is_action_just_pressed(GameAction::Block);
        self.combat_snapshot.dash_pressed |= ctx.input.is_action_just_pressed(GameAction::Dash);
        self.combat_snapshot.grapple_pressed |=
            ctx.input.is_action_just_pressed(GameAction::Grapple);

        if self.frame_input.jump_down {
            self.jump_to_consume = true;
            self.tick_jump_was_pressed = self.current_tick;
        }
    }

    fn has_buffered_jump(&self) -> bool {
        self.buffered_jump_usable
            && self.current_tick <= self.tick_jump_was_pressed + self.stats.jump_buffer_ticks as u64
    }

    fn can_use_coyote(&self) -> bool {
        self.coyote_usable
            && !self.entity.is_grounded
            && self.current_tick <= self.tick_left_grounded + self.stats.coyote_ticks as u64
    }

    fn handle_jump(&mut self) {
        if !self.ended_jump_early
            && !self.entity.is_grounded
            && !self.frame_input.jump_held
            && self.entity.velocity.y < 0.0
        {
            self.ended_jump_early = true;
        }

        if !self.jump_to_consume && !self.has_buffered_jump() {
            return;
        }

        if self.entity.is_grounded || self.can_use_coyote() {
            self.execute_jump();
        }

        self.jump_to_consume = false;
    }

    fn execute_jump(&mut self) {
        self.ended_jump_early = false;
        self.tick_jump_was_pressed = 0;
        self.buffered_jump_usable = false;
        self.coyote_usable = false;
        self.entity.velocity.y = -self.stats.jump_power;
        self.state = PlayerState::Jump;
        self.anim_state.play("Jump");
    }

    fn handle_direction(&mut self, dt: f32) {
        //?Wall jump lock overrides directional input briefly
        if self.wall_jump_lock_timer > 0 {
            return;
        }

        if self.frame_input.move_x == 0.0 {
            let decel = if self.entity.is_grounded {
                self.stats.ground_decel
            } else {
                self.stats.air_decel
            };
            self.entity.velocity.x = move_towards(self.entity.velocity.x, 0.0, decel * dt);
            return;
        }

        self.entity.velocity.x = move_towards(
            self.entity.velocity.x,
            self.frame_input.move_x * self.stats.max_speed,
            self.stats.acceleration * dt,
        );
    }

    fn handle_gravity(&mut self, dt: f32) {
        if self.entity.is_grounded && self.entity.velocity.y >= 0.0 {
            self.entity.velocity.y = self.stats.grounding_force;
        } else {
            let mut gravity = self.stats.fall_acceleration;
            if self.ended_jump_early && self.entity.velocity.y < 0.0 {
                gravity *= self.stats.jump_end_early_gravity_mod;
            }
            self.entity.velocity.y = move_towards(
                self.entity.velocity.y,
                self.stats.max_fall_speed,
                gravity * dt,
            );
        }
    }

    fn can_dash(&self) -> bool {
        if self.dash_cooldown_timer > 0 {
            return false;
        }
        if !self.entity.is_grounded && self.has_air_dashed {
            return false;
        }
        !matches!(
            self.state,
            PlayerState::Death | PlayerState::Dash | PlayerState::AirDash
        )
    }

    fn enter_dash(&mut self) {
        self.dash_direction = if self.entity.facing_right { 1.0 } else { -1.0 };
        self.dash_cooldown_timer = DASH_COOLDOWN_TICKS;

        let is_air = !self.entity.is_grounded;
        if is_air {
            self.has_air_dashed = true;
            self.state = PlayerState::AirDash;
        } else {
            self.state = PlayerState::Dash;
        }

        //?Combat FSM tracks i-frame timing
        fsm::begin_move(&mut self.entity.combat, MoveId::Dash, &self.move_db);
        self.anim_state.play("Dash");
    }

    //? Same wall the player last jumped from, prevents single-wall climbing.
    fn is_wall_jump_blocked(&self) -> bool {
        self.last_wall_jump_dir != 0.0 && self.last_wall_jump_dir == self.wall_direction
    }

    fn wall_jump(&mut self) {
        if self.is_wall_jump_blocked() {
            self.jump_to_consume = false;
            return;
        }

        let away = -self.wall_direction;
        self.entity.velocity.x = self.cached_wall_jump_power_x * away;
        self.entity.velocity.y = -self.cached_wall_jump_power_y;
        self.wall_jump_lock_timer = self.cached_wall_jump_lock_ticks;
        self.last_wall_jump_dir = self.wall_direction;
        //?Snap facing toward the destination wall immediately
        self.entity.facing_right = away > 0.0;
        self.state = PlayerState::Jump;
        self.anim_state.play("Jump");
        self.coyote_usable = false;
        self.ended_jump_early = false;
        self.jump_to_consume = false;
    }

    fn enter_wall_grab(&mut self) {
        let dir = if self.entity.touching_wall_left {
            -1.0
        } else {
            1.0
        };
        self.wall_direction = dir;
        self.wall_grab_timer = self.cached_wall_grab_timeout;
        self.wall_jump_lock_timer = 0; //* Arriving at a wall unlocks input
        self.has_air_dashed = false;
        self.state = PlayerState::WallGrab;
        self.anim_state.play("WallGrab");
    }

    pub fn enter_hitstop(&mut self, ticks: u16) {
        self.hitstop_timer = ticks;
        self.hitstop_return_state = self.state;
    }

    pub fn enter_death(&mut self) {
        self.is_dead = true;
        self.state = PlayerState::Death;
        self.entity.velocity = Vec2::ZERO;
        self.anim_state.play("Death");
    }

    pub fn respawn(&mut self, pos: Vec2) {
        self.is_dead = false;
        self.entity.position = pos;
        self.entity.velocity = Vec2::ZERO;
        self.entity.health = crate::combat::Health::new(1.0);
        self.entity.combat = crate::combat::CombatState::default();
        entity::despawn_hitbox(&mut self.entity);
        self.input_buffer.clear();
        self.combat_snapshot = CombatInputSnapshot::default();
        self.state = PlayerState::Idle;
        self.dash_cooldown_timer = 0;
        self.has_air_dashed = false;
        self.wall_jump_lock_timer = 0;
        self.wall_grab_timer = 0;
        self.wall_detach_timer = 0;
        self.last_wall_jump_dir = 0.0;
        self.grabbed_wall_bottom = f32::INFINITY;
        self.wall_detach_cooldown = 0;
        self.hitstop_timer = 0;
        self.hitstop_return_state = PlayerState::Idle;
        self.drop_through_timer = 0;
        self.grapple_target = None;
        self.grapple_launch_dir = Vec2::ZERO;
        self.grapple_slingshot_timer = 0;
        self.grapple_is_enemy_target = false;
        self.grapple_arrived_at_enemy = false;
        self.prev_position = pos;
        self.anim_state.play("Idle");
    }

    pub fn fixed_update(
        &mut self,
        dt: f32,
        tick: u64,
        solid_platforms: &[AABB],
        one_way_platforms: &[AABB],
        wall_platforms: &[AABB],
        physics_config: &PhysicsConfig,
    ) {
        self.current_tick = tick;

        //? This sync overwrites PlayerStats each tick so the
        //? egui Physics Tuner takes effect immediately without a recompile.
        self.stats.fall_acceleration = physics_config.gravity;
        self.stats.max_fall_speed = physics_config.max_fall_speed;
        self.stats.max_speed = physics_config.movement_speed;
        self.stats.acceleration = physics_config.acceleration;
        self.stats.ground_decel = physics_config.ground_decel;
        self.stats.air_decel = physics_config.air_decel;
        self.stats.jump_power = physics_config.jump_power;
        self.stats.jump_end_early_gravity_mod = physics_config.jump_end_early_gravity_mod;
        self.stats.coyote_ticks = physics_config.coyote_ticks;
        self.stats.jump_buffer_ticks = physics_config.jump_buffer_ticks;
        self.cached_wall_slide_speed = physics_config.wall_slide_speed;
        self.cached_wall_jump_power_x = physics_config.wall_jump_power_x;
        self.cached_wall_jump_power_y = physics_config.wall_jump_power_y;
        self.cached_wall_grab_timeout = physics_config.wall_grab_timeout_ticks;
        self.cached_wall_jump_lock_ticks = physics_config.wall_jump_lock_ticks;

        //? Save position for render interpolation (before any physics this step)
        self.prev_position = self.entity.position;

        self.dash_cooldown_timer = self.dash_cooldown_timer.saturating_sub(1);
        self.wall_jump_lock_timer = self.wall_jump_lock_timer.saturating_sub(1);
        self.drop_through_timer = self.drop_through_timer.saturating_sub(1);
        self.wall_detach_cooldown = self.wall_detach_cooldown.saturating_sub(1);

        //? HITSTOP: freeze all logic while timer is active
        if self.hitstop_timer > 0 {
            self.hitstop_timer -= 1;
            return;
        }

        //? DEATH: minimal physics only
        if self.state == PlayerState::Death {
            self.entity.velocity.x = 0.0;
            self.handle_gravity(dt);
            entity::integrate_and_collide_with_one_way(
                &mut self.entity,
                solid_platforms,
                one_way_platforms,
                dt,
            );
            return;
        }

        //? 1. Push combat inputs into buffer
        self.push_combat_inputs(tick);

        //? 2. Try consume buffered input (start combat, dash, or parry)
        if !matches!(
            self.state,
            PlayerState::Dash
                | PlayerState::AirDash
                | PlayerState::WallSlide
                | PlayerState::WallGrab
                | PlayerState::GrapplePull
                | PlayerState::GrappleSlingshot
        ) {
            self.try_consume_combat_input(tick);
        }

        //? 3. Advance combat FSM for timed states
        if self.is_combat_fsm_state() {
            self.advance_combat_state(tick);
        }

        //? 4. Expire old buffer entries
        self.input_buffer.expire(tick);

        //? 5. State-specific movement
        match self.state {
            PlayerState::Idle | PlayerState::Run => {
                self.handle_jump();
                self.handle_direction(dt);
                self.handle_gravity(dt);
            }
            PlayerState::Jump | PlayerState::Fall => {
                self.handle_jump();
                self.handle_direction(dt);
                self.handle_gravity(dt);
            }
            PlayerState::WallSlide => {
                self.handle_wall_slide_movement(dt);
            }
            PlayerState::WallGrab => {
                self.handle_wall_grab_movement();
            }
            PlayerState::Dash => {
                self.handle_dash_movement(physics_config.dash_speed);
            }
            PlayerState::AirDash => {
                self.handle_dash_movement(physics_config.dash_speed);
            }
            PlayerState::Parry => {
                self.handle_parry_movement(dt);
            }
            PlayerState::AttackHorizontal | PlayerState::AttackUp | PlayerState::AttackDown => {
                self.handle_combat_movement(dt);
            }
            PlayerState::GrapplePull => {
                self.handle_grapple_movement(
                    dt,
                    physics_config.grapple_pull_speed,
                    physics_config.grapple_slingshot_force,
                    physics_config.grapple_slingshot_ticks,
                );
            }
            PlayerState::GrappleSlingshot => {
                self.handle_grapple_slingshot();
            }
            PlayerState::Death => {}
        }

        //? 6. Clamp horizontal velocity for non-override states
        if !matches!(
            self.state,
            PlayerState::Dash
                | PlayerState::AirDash
                | PlayerState::GrapplePull
                | PlayerState::GrappleSlingshot
        ) {
            self.entity.velocity.x = self
                .entity
                .velocity
                .x
                .clamp(-self.stats.max_speed, self.stats.max_speed);
        }

        //? 7. Physics: integrate velocity + resolve collisions
        //* Dash/AirDash/Grapple states bypass collision entirely (ghost through geometry)
        self.was_grounded = self.entity.is_grounded;
        if matches!(
            self.state,
            PlayerState::Dash
                | PlayerState::AirDash
                | PlayerState::GrapplePull
                | PlayerState::GrappleSlingshot
        ) {
            self.entity.position += self.entity.velocity * dt;
        } else {
            let pre_collision_vx = self.entity.velocity.x;

            entity::integrate_and_collide_with_one_way(
                &mut self.entity,
                solid_platforms,
                if self.drop_through_timer > 0 {
                    &[]
                } else {
                    one_way_platforms
                },
                dt,
            );

            //? Remember which wall flags the collision resolver set
            let collision_wall_left = self.entity.touching_wall_left;
            let collision_wall_right = self.entity.touching_wall_right;

            //? 8. Filter wall flags: only count Wall-type platforms (not floor edges)
            self.filter_wall_contacts(wall_platforms);

            if (collision_wall_left && !self.entity.touching_wall_left)
                || (collision_wall_right && !self.entity.touching_wall_right)
            {
                self.entity.velocity.x = pre_collision_vx;
            }

            //? When touching a confirmed Wall while airborne, inject a small velocity
            //? toward the wall so contact persists next tick.
            //? Only when the player isn't actively pressing away from the wall.
            if !self.entity.is_grounded
                && matches!(self.state, PlayerState::Jump | PlayerState::Fall)
            {
                let pressing_away_left =
                    self.entity.touching_wall_left && self.frame_input.move_x > 0.1;
                let pressing_away_right =
                    self.entity.touching_wall_right && self.frame_input.move_x < -0.1;
                if self.entity.touching_wall_left && !pressing_away_left {
                    self.entity.velocity.x = self.entity.velocity.x.min(-15.0);
                } else if self.entity.touching_wall_right && !pressing_away_right {
                    self.entity.velocity.x = self.entity.velocity.x.max(15.0);
                }
            }
        }

        //? 9. Post-physics state transitions
        self.post_physics_transitions(wall_platforms);
    }

    fn is_combat_fsm_state(&self) -> bool {
        //? GrapplePull is intentionally excluded. It has no fixed frame budget.
        //* It runs until the position-based arrival check fires the slingshot.
        matches!(
            self.state,
            PlayerState::Dash
                | PlayerState::AirDash
                | PlayerState::Parry
                | PlayerState::AttackHorizontal
                | PlayerState::AttackUp
                | PlayerState::AttackDown
        )
    }

    //? Push combat input snapshot into buffer, then clear snapshot.
    fn push_combat_inputs(&mut self, tick: u64) {
        if self.combat_snapshot.attack_pressed {
            //?Directional attacks based on vertical aim
            let move_id = if self.frame_input.move_y < -0.5 {
                MoveId::AttackUp
            } else if self.frame_input.move_y > 0.5 && !self.entity.is_grounded {
                MoveId::AttackDown
            } else {
                MoveId::AttackHorizontal
            };
            self.input_buffer.push(move_id, tick);
        }
        if self.combat_snapshot.dash_pressed && self.can_dash() {
            self.input_buffer.push(MoveId::Dash, tick);
        }
        if self.combat_snapshot.block_pressed {
            self.input_buffer.push(MoveId::Parry, tick);
        }
        if self.combat_snapshot.grapple_pressed {
            self.input_buffer.push(MoveId::Grapple, tick);
        }

        //todo Drop-through: Down + Jump on one-way platform
        if self.frame_input.jump_down && self.frame_input.move_y > 0.5 && self.entity.is_grounded {
            self.drop_through_timer = 6;
            self.jump_to_consume = false;
        }

        self.combat_snapshot = CombatInputSnapshot::default();
    }

    //? Try to consume buffered input and start a move or dash.
    fn try_consume_combat_input(&mut self, tick: u64) {
        if let Some(move_id) = self
            .input_buffer
            .consume(&self.entity.combat, &self.move_db, tick)
        {
            self.start_combat_move(move_id);
        }
    }

    //? Single source of truth for entering any combat move from a MoveId.
    //? Called from both the normal input path and the recovery-cancel path.
    fn start_combat_move(&mut self, move_id: MoveId) {
        match move_id {
            MoveId::Dash => self.enter_dash(),
            MoveId::Parry => {
                fsm::begin_move(&mut self.entity.combat, MoveId::Parry, &self.move_db);
                self.state = PlayerState::Parry;
                self.anim_state.play("Parry");
            }
            MoveId::AttackHorizontal => {
                fsm::begin_move(
                    &mut self.entity.combat,
                    MoveId::AttackHorizontal,
                    &self.move_db,
                );
                self.state = PlayerState::AttackHorizontal;
                self.anim_state.play("AttackHorizontal");
            }
            MoveId::AttackUp => {
                fsm::begin_move(&mut self.entity.combat, MoveId::AttackUp, &self.move_db);
                self.state = PlayerState::AttackUp;
                self.anim_state.play("AttackUp");
            }
            MoveId::AttackDown => {
                fsm::begin_move(&mut self.entity.combat, MoveId::AttackDown, &self.move_db);
                self.state = PlayerState::AttackDown;
                self.anim_state.play("AttackDown");
            }
            MoveId::Grapple => {
                //?Requires a valid target (populated in lib.rs each tick)
                if let Some(target) = self.grapple_target {
                    let diff = target - self.entity.position;
                    let dist = diff.length();
                    self.grapple_launch_dir = if dist > 0.001 {
                        diff / dist
                    } else {
                        let face = if self.entity.facing_right { 1.0 } else { -1.0 };
                        Vec2::new(face, -0.3).normalize()
                    };
                    fsm::begin_move(&mut self.entity.combat, MoveId::Grapple, &self.move_db);
                    self.entity.combat.invincible = true;
                    self.state = PlayerState::GrapplePull;
                    self.anim_state.play("Grapple");
                }
            }
        }
    }

    //? Advance the combat FSM and handle phase transitions.
    fn advance_combat_state(&mut self, tick: u64) {
        let transition = fsm::advance_combat_fsm(&mut self.entity.combat, &self.move_db);

        if let Some(new_phase) = transition {
            match new_phase {
                CombatPhase::Idle => {
                    entity::despawn_hitbox(&mut self.entity);
                    self.entity.hit_landed = false;
                    self.state = if self.entity.is_grounded {
                        PlayerState::Idle
                    } else {
                        PlayerState::Fall
                    };
                }
                CombatPhase::Active => {
                    entity::spawn_hitbox(&mut self.entity, &self.move_db);
                }
                CombatPhase::Startup => {}
                CombatPhase::Recovery => {
                    entity::despawn_hitbox(&mut self.entity);
                    //?On recovery, check buffer for queued cancel
                    if let Some(move_id) =
                        self.input_buffer
                            .consume(&self.entity.combat, &self.move_db, tick)
                    {
                        self.start_combat_move(move_id);
                    }
                }
            }
        }
    }

    fn handle_wall_slide_movement(&mut self, _dt: f32) {
        self.entity.velocity.y = self.cached_wall_slide_speed;
        //? Small push INTO the wall to maintain collision contact every tick.
        self.entity.velocity.x = self.wall_direction * 100.0;

        if self.jump_to_consume {
            self.wall_jump();
            self.jump_to_consume = false;
            self.wall_detach_timer = 0;
            return;
        }

        let away = -self.wall_direction;
        if self.frame_input.move_x * away > 0.1 {
            self.wall_detach_timer += 1;
            if self.wall_detach_timer >= WALL_DETACH_GRACE_TICKS {
                self.wall_detach_timer = 0;
                self.state = PlayerState::Fall;
                self.anim_state.play("Fall");
            }
        } else {
            self.wall_detach_timer = 0;
        }
    }

    fn handle_wall_grab_movement(&mut self) {
        self.entity.velocity.y = 0.0;
        self.entity.velocity.x = self.wall_direction * 100.0;

        //? Wall jump (always takes priority)
        if self.jump_to_consume {
            self.wall_jump();
            self.jump_to_consume = false;
            self.wall_detach_timer = 0;
            return;
        }

        //? Move away → grace timer then fall
        let away = -self.wall_direction;
        if self.frame_input.move_x * away > 0.1 {
            self.wall_detach_timer += 1;
            if self.wall_detach_timer >= WALL_DETACH_GRACE_TICKS {
                self.wall_detach_timer = 0;
                self.state = PlayerState::Fall;
                self.anim_state.play("Fall");
            }
            return;
        } else {
            self.wall_detach_timer = 0;
        }

        //? Direction input never re-enters grab to avoid oscillation on held input.
        self.wall_grab_timer = self.wall_grab_timer.saturating_sub(1);
        if self.wall_grab_timer == 0 {
            self.state = PlayerState::WallSlide;
            self.anim_state.play("WallSlide");
        }
    }

    fn handle_dash_movement(&mut self, dash_speed: f32) {
        self.entity.velocity.x = dash_speed * self.dash_direction;
        self.entity.velocity.y = 0.0;
    }

    fn handle_parry_movement(&mut self, dt: f32) {
        self.entity.velocity.x = 0.0;
        self.handle_gravity(dt);
    }

    fn handle_combat_movement(&mut self, dt: f32) {
        self.entity.velocity.x =
            move_towards(self.entity.velocity.x, 0.0, self.stats.ground_decel * dt);
        self.handle_gravity(dt);
    }

    fn handle_grapple_movement(
        &mut self,
        dt: f32,
        pull_speed: f32,
        slingshot_force: f32,
        slingshot_ticks: u16,
    ) {
        if let Some(target) = self.grapple_target {
            let diff = target - self.entity.position;
            let dist = diff.length();

            let movement_step = pull_speed * dt;
            if dist <= movement_step {
                self.entity.position = target;

                if self.grapple_is_enemy_target {
                    self.entity.velocity = Vec2::ZERO;
                    self.grapple_arrived_at_enemy = true;
                    //* Don't clear grapple_target yet   lib.rs needs it for the check
                } else {
                    //? Static node
                    self.entity.velocity = self.grapple_launch_dir * slingshot_force;
                    self.grapple_target = None;
                    self.grapple_slingshot_timer = slingshot_ticks;
                    self.entity.combat = crate::combat::CombatState::default();
                    self.entity.combat.invincible = true;
                    entity::despawn_hitbox(&mut self.entity);
                    self.state = PlayerState::GrappleSlingshot;
                    self.anim_state.play("Fall");
                }
            } else {
                self.entity.velocity = self.grapple_launch_dir * pull_speed;
            }
        } else {
            self.exit_grapple();
        }
    }

    //? Slingshot coast: no gravity, velocity unchanged, decrement timer.
    fn handle_grapple_slingshot(&mut self) {
        self.grapple_slingshot_timer = self.grapple_slingshot_timer.saturating_sub(1);
        if self.grapple_slingshot_timer == 0 {
            self.entity.combat.invincible = false;
            self.state = PlayerState::Fall;
            self.anim_state.play("Fall");
        }
    }

    //? Safely exit grapple state: reset target, combat FSM, and transition to Idle/Fall.
    fn exit_grapple(&mut self) {
        self.grapple_target = None;
        self.entity.combat = crate::combat::CombatState::default();
        entity::despawn_hitbox(&mut self.entity);
        self.state = if self.entity.is_grounded {
            PlayerState::Idle
        } else {
            PlayerState::Fall
        };
    }

    fn post_physics_transitions(&mut self, wall_aabbs: &[AABB]) {
        if !self.was_grounded && self.entity.is_grounded {
            self.coyote_usable = true;
            self.buffered_jump_usable = true;
            self.ended_jump_early = false;
            self.has_air_dashed = false;
            self.last_wall_jump_dir = 0.0;
            self.wall_jump_lock_timer = 0;
            self.wall_detach_cooldown = 0;
        } else if self.was_grounded && !self.entity.is_grounded {
            self.tick_left_grounded = self.current_tick;
        }

        //? Reset air dash on wall touch
        if self.entity.touching_wall_left || self.entity.touching_wall_right {
            self.has_air_dashed = false;
            let touching_dir = if self.entity.touching_wall_left {
                -1.0
            } else {
                1.0
            };
            if self.last_wall_jump_dir != 0.0 && touching_dir != self.last_wall_jump_dir {
                self.last_wall_jump_dir = 0.0;
            }
        }

        if !self.entity.is_grounded
            && self.entity.velocity.y >= 0.0
            && (self.entity.touching_wall_left || self.entity.touching_wall_right)
            && self.wall_detach_cooldown == 0
            && matches!(self.state, PlayerState::Jump | PlayerState::Fall)
        {
            self.enter_wall_grab();
            //? Cache the grabbed wall's bottom for slide-past-bottom detach.
            self.grabbed_wall_bottom = self
                .find_grabbed_wall_bottom(wall_aabbs)
                .unwrap_or(f32::INFINITY);
            return;
        }

        if matches!(self.state, PlayerState::WallSlide | PlayerState::WallGrab) {
            //? Escape: player has slid past the bottom edge of the grabbed wall.
            let player_bottom = self.entity.position.y + self.entity.pushbox_size.y * 0.5;
            if player_bottom > self.grabbed_wall_bottom {
                self.grabbed_wall_bottom = f32::INFINITY;
                self.wall_contact_lost_ticks = 0;
                self.wall_detach_cooldown = 8;
                self.state = PlayerState::Fall;
                self.anim_state.play("Fall");
                return;
            }

            let still_touching = if self.wall_direction < 0.0 {
                self.entity.touching_wall_left
            } else {
                self.entity.touching_wall_right
            };
            if self.entity.is_grounded {
                self.wall_contact_lost_ticks = 0;
                self.state = PlayerState::Idle;
                self.anim_state.play("Idle");
                return;
            }
            if !still_touching {
                self.wall_contact_lost_ticks += 1;
                if self.wall_contact_lost_ticks >= 1 {
                    self.wall_contact_lost_ticks = 0;
                    self.wall_detach_cooldown = 8;
                    self.state = PlayerState::Fall;
                    self.anim_state.play("Fall");
                    return;
                }
            } else {
                self.wall_contact_lost_ticks = 0;
            }
        }

        if matches!(
            self.state,
            PlayerState::Idle | PlayerState::Run | PlayerState::Jump | PlayerState::Fall
        ) {
            if self.entity.is_grounded {
                let speed = self.entity.velocity.x.abs();
                self.state = if speed > VELOCITY_EPSILON {
                    PlayerState::Run
                } else {
                    PlayerState::Idle
                };
            } else if self.entity.velocity.y < 0.0 {
                self.state = PlayerState::Jump;
            } else {
                self.state = PlayerState::Fall;
            }
        }
    }

    pub fn update(&mut self, ctx: &Context) {
        let dt = ctx.delta_time;

        self.gather_input(ctx);

        if self.wall_jump_lock_timer == 0
            && matches!(
                self.state,
                PlayerState::Idle | PlayerState::Run | PlayerState::Jump | PlayerState::Fall
            )
        {
            if self.frame_input.move_x > 0.1 {
                self.entity.facing_right = true;
            } else if self.frame_input.move_x < -0.1 {
                self.entity.facing_right = false;
            }
        }

        self.update_animation();
        self.anim_state.update(dt);
    }

    fn update_animation(&mut self) {
        //? FSM-driven states have their animations set on entry
        match self.state {
            PlayerState::Dash
            | PlayerState::AirDash
            | PlayerState::Parry
            | PlayerState::AttackHorizontal
            | PlayerState::AttackUp
            | PlayerState::AttackDown
            | PlayerState::GrapplePull
            | PlayerState::GrappleSlingshot
            | PlayerState::Death => {
                return;
            }
            PlayerState::WallSlide => {
                if self.anim_state.current_animation_name() != Some("WallSlide") {
                    self.anim_state.play("WallSlide");
                }
                return;
            }
            PlayerState::WallGrab => {
                if self.anim_state.current_animation_name() != Some("WallGrab") {
                    self.anim_state.play("WallGrab");
                }
                return;
            }
            _ => {}
        }

        let anim_name = if !self.entity.is_grounded {
            if self.entity.velocity.y < 0.0 {
                "Jump"
            } else {
                "Fall"
            }
        } else {
            match self.state {
                PlayerState::Run => "Run",
                _ => "Idle",
            }
        };

        if self.anim_state.current_animation_name() != Some(anim_name) {
            self.anim_state.play(anim_name);
        }
    }

    pub fn collision_aabb(&self) -> AABB {
        AABB::new(self.entity.position, Vec2::new(PLAYER_WIDTH, PLAYER_HEIGHT))
    }

    pub fn render_size(&self) -> Vec2 {
        Vec2::new(
            FRAME_WIDTH * self.render_scale,
            FRAME_HEIGHT * self.render_scale,
        )
    }

    pub fn draw_position(&self, alpha: f32) -> Vec2 {
        let interp = self.prev_position + (self.entity.position - self.prev_position) * alpha;
        let size = self.render_size();
        let horiz_offset = if self.entity.facing_right {
            VISUAL_OFFSET_X
        } else {
            -VISUAL_OFFSET_X
        };
        //? Always return the top-left corner of the sprite rect.
        //? Flipping is handled entirely by UV mirroring in the sprite renderer,
        let pos = interp - size / 2.0 + Vec2::new(horiz_offset, VISUAL_OFFSET_Y);

        Vec2::new(pos.x.round(), pos.y.round())
    }

    //? Interpolated center position for camera tracking (between physics frames).
    pub fn interpolated_position(&self, alpha: f32) -> Vec2 {
        self.prev_position + (self.entity.position - self.prev_position) * alpha
    }

    pub fn clamp_to_bounds(&mut self, min_x: f32, max_x: f32) {
        let half_width = self.collision_aabb().size.x / 2.0;
        self.entity.position.x = self
            .entity
            .position
            .x
            .clamp(min_x + half_width, max_x - half_width);
    }

    //? Find the bottom Y of the grabbed wall column (the lowest tile on the side the
    //? player is touching). Scans ALL X-adjacent tiles, not just ones that overlap
    //? the player's current AABB. So a 24 px tall player sliding down 16 px tiles
    //? always gets the true floor of the column, never an intermediate tile boundary.
    fn find_grabbed_wall_bottom(&self, wall_aabbs: &[AABB]) -> Option<f32> {
        let pb = self.entity.pushbox();
        let pb_min = pb.min();
        let pb_max = pb.max();

        let mut furthest_bottom: Option<f32> = None;

        for wall in wall_aabbs {
            let w_min = wall.min();
            let w_max = wall.max();

            let dist_left = (pb_min.x - w_max.x).abs();
            let dist_right = (pb_max.x - w_min.x).abs();

            let on_left_side = self.entity.touching_wall_left && dist_left < 2.0;
            let on_right_side = self.entity.touching_wall_right && dist_right < 2.0;

            if on_left_side || on_right_side {
                furthest_bottom = Some(match furthest_bottom {
                    Some(cur) => cur.max(w_max.y),
                    None => w_max.y,
                });
            }
        }

        furthest_bottom
    }

    //? Clear wall flags that came from non-Wall platforms (floor edges, crate sides).
    //? Only Wall-type platforms should trigger wall-grab/wall-slide.
    fn filter_wall_contacts(&mut self, wall_aabbs: &[AABB]) {
        let pb = self.entity.pushbox();
        let pb_left = pb.center.x - pb.size.x / 2.0;
        let pb_right = pb.center.x + pb.size.x / 2.0;
        let pb_top = pb.center.y - pb.size.y / 2.0;
        let pb_bottom = pb.center.y + pb.size.y / 2.0;

        let mut confirmed_left = false;
        let mut confirmed_right = false;

        for wall in wall_aabbs {
            let w_left = wall.center.x - wall.size.x / 2.0;
            let w_right = wall.center.x + wall.size.x / 2.0;
            let w_top = wall.center.y - wall.size.y / 2.0;
            let w_bottom = wall.center.y + wall.size.y / 2.0;

            //* Must have vertical overlap
            if pb_top >= w_bottom || pb_bottom <= w_top {
                continue;
            }

            if (pb_left - w_right).abs() < 2.0 {
                confirmed_left = true;
            }
            if (pb_right - w_left).abs() < 2.0 {
                confirmed_right = true;
            }
        }

        //? Clear flags that MTV set from non-Wall surfaces
        if self.entity.touching_wall_left && !confirmed_left {
            self.entity.touching_wall_left = false;
        }
        if self.entity.touching_wall_right && !confirmed_right {
            self.entity.touching_wall_right = false;
        }
        if confirmed_left {
            self.entity.touching_wall_left = true;
        }
        if confirmed_right {
            self.entity.touching_wall_right = true;
        }
    }
}
