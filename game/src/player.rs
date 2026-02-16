/**--------------------------------------------------------------------------------
*!  Souls-like 2d player controller
*?  Implements: Acceleration/Deceleration curves, Variable jump height,
*?  Coyote Time, Jump Buffering, and Animation Commitment for combat.
*--------------------------------------------------------------------------------**/
use crate::anim::AnimationState;
use crate::assets::{FRAME_HEIGHT, FRAME_WIDTH};
use crate::config::*;
use engine::{AABB, Context, GameAction, Vec2, math::move_towards};

//? Player state machine
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum PlayerState {
    Idle,
    Walk,
    Run,
    Jump,
    Fall,
    Attack(u8, f32), //* Combo step (1-3) and timer.
    Block(f32),      //* Parrying with max block window.
    Roll,
}

//? Frame-level input state
//? Separate from `Context` to allow for input buffering
//? and more complex input handling logic.
#[derive(Debug, Clone, Copy, Default)]
struct FrameInput {
    move_x: f32,
    jump_down: bool,
    jump_held: bool,
    run_pressed: bool,
}

//? Player controller with physics and animation
pub struct Player {
    pub position: Vec2,
    pub velocity: Vec2,
    pub facing_right: bool,
    pub state: PlayerState,
    pub anim_state: AnimationState,
    pub render_scale: f32,

    //? Stats and tunable parameters
    is_grounded: bool,
    was_grounded: bool,
    stats: PlayerStats,
    frame_input: FrameInput,
    time: f32,

    //? Jump state
    jump_to_consume: bool,
    buffered_jump_usable: bool,
    ended_jump_early: bool,
    coyote_usable: bool,
    time_jump_was_pressed: f32,
    time_left_grounded: f32,

    //? Combat timers
    roll_timer: f32,
    attack_buffer: f32,
}

impl Player {
    pub fn new(start_pos: Vec2, anim_state: AnimationState) -> Self {
        Self {
            position: start_pos,
            velocity: Vec2::ZERO,
            facing_right: true,
            state: PlayerState::Idle,
            anim_state,
            is_grounded: false,
            was_grounded: false,
            stats: PlayerStats::default(),
            frame_input: FrameInput::default(),
            time: 0.0,
            jump_to_consume: false,
            buffered_jump_usable: false,
            ended_jump_early: false,
            coyote_usable: false,
            time_jump_was_pressed: f32::MIN,
            time_left_grounded: f32::MIN,
            roll_timer: 0.0,
            attack_buffer: 0.0,
            render_scale: 4.0,
        }
    }

    //? Input handling separated from `Context`
    //? to allow for buffering and complex logic
    fn gather_input(&mut self, ctx: &Context) {
        self.frame_input = FrameInput {
            move_x: ctx.input.get_move_x(),
            jump_down: ctx.input.is_action_just_pressed(GameAction::Jump),
            jump_held: ctx.input.is_action_pressed(GameAction::Jump),
            run_pressed: ctx.input.is_action_pressed(GameAction::Run),
        };

        if self.frame_input.jump_down {
            self.jump_to_consume = true;
            self.time_jump_was_pressed = self.time;
        }
    }

    //? Collision detection and response using AABBs.
    fn check_collisions(&mut self, platforms: &[AABB]) {
        let player_aabb = AABB::new(self.position, Vec2::new(PLAYER_WIDTH, PLAYER_HEIGHT));

        self.is_grounded = false;
        for platform in platforms {
            if player_aabb.check_collision(platform) && self.velocity.y > 0.0 {
                let platform_top = platform.min().y;
                self.position.y = platform_top - PLAYER_HEIGHT / 2.0;
                self.velocity.y = 0.0;
                self.is_grounded = true;
            }
        }

        if !self.was_grounded && self.is_grounded {
            self.coyote_usable = true;
            self.buffered_jump_usable = true;
            self.ended_jump_early = false;
        } else if self.was_grounded && !self.is_grounded {
            self.time_left_grounded = self.time;
        }
    }

    //? Jump handling with buffering, coyote time, and variable height
    fn has_buffered_jump(&self) -> bool {
        self.buffered_jump_usable && self.time < self.time_jump_was_pressed + self.stats.jump_buffer
    }

    fn can_use_coyote(&self) -> bool {
        self.coyote_usable
            && !self.is_grounded
            && self.time < self.time_left_grounded + self.stats.coyote_time
    }

    //? Detect early jump release for variable height
    fn handle_jump(&mut self) {
        if !self.ended_jump_early
            && !self.is_grounded
            && !self.frame_input.jump_held
            && self.velocity.y < 0.0
        {
            self.ended_jump_early = true;
        }

        if !self.jump_to_consume && !self.has_buffered_jump() {
            return;
        }

        if self.is_grounded || self.can_use_coyote() {
            self.execute_jump();
        }

        self.jump_to_consume = false;
    }

    fn execute_jump(&mut self) {
        self.ended_jump_early = false;
        self.time_jump_was_pressed = f32::MIN;
        self.buffered_jump_usable = false;
        self.coyote_usable = false;
        self.velocity.y = -self.stats.jump_power; //* Y-down: negative = upward
        self.state = PlayerState::Jump;
        self.anim_state.play("Jump");
    }

    //? Horizontal movement with acceleration and deceleration curves
    fn handle_direction(&mut self, dt: f32) {
        if self.frame_input.move_x == 0.0 {
            let decel = if self.is_grounded {
                self.stats.ground_decel
            } else {
                self.stats.air_decel
            };
            self.velocity.x = move_towards(self.velocity.x, 0.0, decel * dt);
            return;
        }

        let target_speed = if self.frame_input.run_pressed {
            self.stats.max_speed
        } else {
            self.stats.max_speed * WALK_SPEED_RATIO
        };

        self.velocity.x = move_towards(
            self.velocity.x,
            self.frame_input.move_x * target_speed,
            self.stats.acceleration * dt,
        );
    }

    //? Gravity and variable jump height
    fn handle_gravity(&mut self, dt: f32) {
        //? Y-down: grounded with positive (downward) velocity → apply grounding force
        if self.is_grounded && self.velocity.y >= 0.0 {
            self.velocity.y = self.stats.grounding_force;
        } else {
            let mut gravity = self.stats.fall_acceleration;
            //? Variable jump: if releasing early while still ascending (negative vel)
            if self.ended_jump_early && self.velocity.y < 0.0 {
                gravity *= self.stats.jump_end_early_gravity_mod;
            }
            //? Y-down: positive max_fall_speed means downward
            self.velocity.y =
                move_towards(self.velocity.y, self.stats.max_fall_speed, gravity * dt);
        }
    }

    //? Main update function called every frame
    pub fn update(&mut self, ctx: &Context, platforms: &[AABB]) {
        let dt = ctx.delta_time;
        self.time += dt;
        self.was_grounded = self.is_grounded;

        //* 1. Gather input
        self.gather_input(ctx);

        //* 2. Handle committed action states (roll, attack, block)
        if self.state == PlayerState::Roll {
            self.roll_timer -= dt;
            if self.roll_timer > 0.0 {
                let roll_direction = if self.facing_right { 1.0 } else { -1.0 };
                self.velocity.x = DODGE_IMPULSE * roll_direction;
            } else {
                self.state = PlayerState::Idle;
            }
        } else {
            self.handle_combat_input(ctx);
        }

        //* 3. Physics step
        self.check_collisions(platforms);
        self.handle_jump();
        self.handle_direction(dt);
        self.handle_gravity(dt);

        //* 4. Apply velocity
        self.position += self.velocity * dt;

        //* 5. Update facing direction from movement input
        if self.frame_input.move_x > 0.1 {
            self.facing_right = true;
        } else if self.frame_input.move_x < -0.1 {
            self.facing_right = false;
        }

        //* 6. Determine visual state (only for non-committed states)
        if !matches!(
            self.state,
            PlayerState::Attack(_, _) | PlayerState::Block(_) | PlayerState::Roll
        ) {
            if self.is_grounded {
                let speed = self.velocity.x.abs();
                self.state = if self.frame_input.run_pressed && speed > VELOCITY_EPSILON {
                    PlayerState::Run
                } else if speed > VELOCITY_EPSILON {
                    PlayerState::Walk
                } else {
                    PlayerState::Idle
                };
            } else if self.velocity.y < 0.0 {
                self.state = PlayerState::Jump;
            } else {
                self.state = PlayerState::Fall;
            }
        }

        //* 7. Animation
        self.update_animation();
        self.anim_state.update(dt);
    }

    //? Combat input handling with strict priority: Roll > Attack > Block
    fn handle_combat_input(&mut self, ctx: &Context) {
        let dt = ctx.delta_time;
        self.attack_buffer = (self.attack_buffer - dt).max(0.0);

        //? Block (instant parry from idle/walk/run/block)
        if ctx.input.is_action_just_pressed(GameAction::Block)
            && matches!(
                self.state,
                PlayerState::Idle | PlayerState::Walk | PlayerState::Run | PlayerState::Block(0.2)
            )
        {
            self.state = PlayerState::Block(0.0);
            self.anim_state.play("Block");
            self.velocity.x = 0.0;
            return;
        }

        //? Handle active attack state
        if let PlayerState::Attack(combo_step, timer) = self.state {
            let new_timer = timer + dt;
            self.state = PlayerState::Attack(combo_step, new_timer);

            if ctx.input.is_action_just_pressed(GameAction::Attack) {
                self.attack_buffer = ATTACK_BUFFER_WINDOW;
            }

            if new_timer >= COMBO_WINDOW_START
                && combo_step < 3
                && (self.attack_buffer > 0.0
                    || ctx.input.is_action_just_pressed(GameAction::Attack))
            {
                self.state = PlayerState::Attack(combo_step + 1, 0.0);
                let anim_name = match combo_step + 1 {
                    1 => "Attack1",
                    2 => "Attack2",
                    3 => "Attack3",
                    _ => "Attack1",
                };
                self.anim_state.play(anim_name);
                self.attack_buffer = 0.0;
                self.velocity.x = move_towards(self.velocity.x, 0.0, self.stats.ground_decel * dt);
                return;
            }

            //? Exit attack when animation finished
            if self.anim_state.is_finished() {
                if self.attack_buffer > 0.0 && combo_step < 3 {
                    self.state = PlayerState::Attack(combo_step + 1, 0.0);
                    let anim_name = match combo_step + 1 {
                        1 => "Attack1",
                        2 => "Attack2",
                        3 => "Attack3",
                        _ => "Attack1",
                    };
                    self.anim_state.play(anim_name);
                    self.attack_buffer = 0.0;
                } else {
                    self.state = if self.is_grounded {
                        PlayerState::Idle
                    } else {
                        PlayerState::Fall
                    };
                    self.attack_buffer = 0.0;
                }
            }

            self.velocity.x = move_towards(self.velocity.x, 0.0, self.stats.ground_decel * dt);
            return;
        }

        //? Handle block state
        if let PlayerState::Block(timer) = self.state {
            let new_timer = timer + dt;

            if new_timer >= BLOCK_MAX_DURATION || !ctx.input.is_action_pressed(GameAction::Block) {
                self.state = if self.is_grounded {
                    PlayerState::Idle
                } else {
                    PlayerState::Fall
                };
            } else {
                self.state = PlayerState::Block(new_timer);
            }

            self.velocity.x = 0.0;
            return;
        }

        //? Roll/Dodge (grounded only)
        if ctx.input.is_action_just_pressed(GameAction::Roll) && self.is_grounded {
            self.state = PlayerState::Roll;
            self.anim_state.play("Roll");
            self.roll_timer = DODGE_DURATION;
            return;
        }

        //? Attack combo states
        if ctx.input.is_action_just_pressed(GameAction::Attack)
            && matches!(
                self.state,
                PlayerState::Idle | PlayerState::Walk | PlayerState::Run
            )
        {
            self.state = PlayerState::Attack(1, 0.0);
            self.anim_state.play("Attack1");
        } else if matches!(self.state, PlayerState::Attack(1, 0.1)) {
            self.state = PlayerState::Attack(2, 0.0);
            self.anim_state.play("Attack2");
        } else if matches!(self.state, PlayerState::Attack(2, 0.1)) {
            self.state = PlayerState::Attack(3, 0.0);
            self.anim_state.play("Attack3");
        }
    }

    //? Update animation based on state with strict priority:
    //* Jump/Fall > Attack > Block > Roll > Run > Idle
    fn update_animation(&mut self) {
        let anim_name = if !self.is_grounded {
            if self.velocity.y < 0.0 {
                "Jump"
            } else {
                "Fall"
            }
        } else {
            match self.state {
                PlayerState::Attack(step, _) => match step {
                    1 => "Attack1",
                    2 => "Attack2",
                    3 => "Attack3",
                    _ => "Attack1",
                },
                PlayerState::Block(_) => "Block",
                PlayerState::Roll => "Roll",
                PlayerState::Run => "Run",
                PlayerState::Walk => "Walk",
                PlayerState::Idle => "Idle",
                _ => "Idle",
            }
        };

        if self.anim_state.current_animation_name() != Some(anim_name) {
            self.anim_state.play(anim_name);
        }
    }

    //? Collision and geometry helpers
    pub fn collision_aabb(&self) -> AABB {
        AABB::new(self.position, Vec2::new(PLAYER_WIDTH, PLAYER_HEIGHT))
    }

    //? Get the size of the rendered sprite based on current render scale.
    pub fn render_size(&self) -> Vec2 {
        Vec2::new(
            FRAME_WIDTH * self.render_scale,
            FRAME_HEIGHT * self.render_scale,
        )
    }

    //? Get the position to draw the sprite, applying visual offsets and facing direction.
    pub fn draw_position(&self) -> Vec2 {
        let size = self.render_size();
        let horiz_offset = if self.facing_right {
            VISUAL_OFFSET_X
        } else {
            -VISUAL_OFFSET_X
        };
        let mut pos = self.position - size / 2.0 + Vec2::new(horiz_offset, VISUAL_OFFSET_Y);

        if !self.facing_right {
            pos.x += size.x;
        }

        pos
    }

    //? Keep player within horizontal bounds of the level
    pub fn clamp_to_bounds(&mut self, min_x: f32, max_x: f32) {
        let half_width = self.collision_aabb().size.x / 2.0;
        self.position.x = self
            .position
            .x
            .clamp(min_x + half_width, max_x - half_width);
    }
}
