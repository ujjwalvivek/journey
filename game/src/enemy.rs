/**--------------------------------------------------------------------------------
*!  Enemy system for 1-hit-kill momentum platformer.
*?  Enemies are traversal nodes that patrol their spawn platform, never falling off.
*?  Three types (Grunt, Sniper, Ronin) share the same FSM but differ in behavior
*?  via a data-driven EnemyConfig table.
*?  Core mechanic: Shoot → Player Parries → Enemy Staggers → Player Grapples → Execute.
*--------------------------------------------------------------------------------**/
use crate::combat::fsm::{self, CombatPhase};
use crate::combat::moves::{MoveDatabase, MoveId};
use crate::config::*;
use crate::entity::{self, Entity};
use crate::projectile;
use engine::{AABB, Vec2};

//? Returned by `fixed_update` when an enemy fires a projectile.
//? `lib.rs` processes this to call `projectiles.spawn()`.
pub struct ShootEvent {
    pub origin: Vec2,
    pub target: Vec2,
    pub speed: f32,
    pub color: [f32; 4],
}

//? Enemy types and their Visual and behavioral variant. Data-driven via [`EnemyConfig`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyType {
    Grunt,
    Sniper,
    Ronin,
}

//? Per-type tuning parameters. Mirrors the `MoveDatabase` pattern.
#[derive(Debug, Clone)]
pub struct EnemyConfig {
    pub patrol_speed: f32,
    pub aggro_range: f32,
    pub melee_range: f32,
    pub stagger_ticks: u16,
    pub aim_ticks: u16,
    pub accent_color: [f32; 4],
}

impl EnemyConfig {
    pub fn for_type(enemy_type: EnemyType) -> Self {
        match enemy_type {
            EnemyType::Grunt => Self {
                patrol_speed: ENEMY_PATROL_SPEED,
                aggro_range: ENEMY_AGGRO_RANGE,
                melee_range: ENEMY_MELEE_RANGE,
                stagger_ticks: ENEMY_STAGGER_TICKS,
                aim_ticks: ENEMY_AIM_TICKS,
                //* Yellow neon
                accent_color: [1.0, 0.85, 0.0, 0.9],
            },
            EnemyType::Sniper => Self {
                patrol_speed: 0.0, //* Snipers don't patrol
                aggro_range: 200.0,
                melee_range: ENEMY_MELEE_RANGE,
                stagger_ticks: ENEMY_STAGGER_TICKS,
                aim_ticks: 45, //* Longer lock-on
                //* Red neon
                accent_color: [1.0, 0.15, 0.15, 0.9],
            },
            EnemyType::Ronin => Self {
                patrol_speed: ENEMY_PATROL_SPEED,
                aggro_range: 80.0, //* Shorter range melee only
                melee_range: 32.0, //* Wider melee trigger
                stagger_ticks: ENEMY_STAGGER_TICKS,
                aim_ticks: 0, //* Ronin doesn't shoot
                //* Blue neon
                accent_color: [0.2, 0.4, 1.0, 0.9],
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyState {
    Idle,
    Patrol { direction: i8 },
    Aim { timer: u16 },
    MeleeWindup { timer: u16 },
    Attacking,
    Cooldown { timer: u16 },
    Staggered { timer: u16 },
    Dead,
}

//? Ticks between shots after firing. Prevents spam.
const SHOOT_COOLDOWN: u16 = 30;

pub struct Enemy {
    pub entity: Entity,
    pub state: EnemyState,
    pub enemy_type: EnemyType,
    pub config: EnemyConfig,
    pub move_db: MoveDatabase,
    pub spawn_platform: Option<AABB>,
    pub spawn_position: Vec2,
    pub death_flash_timer: u16,
}

//? Enemy struct encapsulates both the Entity (physics + combat data) and the Enemy-specific AI state.
impl Enemy {
    pub fn new(position: Vec2, enemy_type: EnemyType) -> Self {
        let config = EnemyConfig::for_type(enemy_type);
        Self {
            entity: Entity::new(
                position,
                Vec2::new(ENEMY_WIDTH, ENEMY_HEIGHT),
                1.0,   //* 1-HP: any hit = dead
                100.0, //* Posture kept for future Sekiro-style stagger depth
            ),
            state: EnemyState::Idle,
            enemy_type,
            config,
            move_db: MoveDatabase::default(),
            spawn_platform: None,
            spawn_position: position,
            death_flash_timer: 0,
        }
    }

    pub fn bind_to_platform(&mut self, solid_platforms: &[AABB]) {
        //? Cast a thin probe downward from the enemy's feet
        let feet_y = self.entity.position.y + ENEMY_HEIGHT / 2.0;
        let probe = AABB::new(
            Vec2::new(self.entity.position.x, feet_y + 2.0),
            Vec2::new(ENEMY_WIDTH, 4.0),
        );
        for platform in solid_platforms {
            if probe.check_collision(platform) {
                self.spawn_platform = Some(*platform);
                return;
            }
        }
    }

    //? Fixed-rate update: AI decision → combat FSM → physics.
    //? Returns a ShootEvent if the enemy fired this tick.
    pub fn fixed_update(
        &mut self,
        dt: f32,
        _tick: u64,
        player_pos: Vec2,
        platforms: &[AABB],
        walls: &[AABB],
    ) -> Option<ShootEvent> {
        if self.state == EnemyState::Dead {
            self.death_flash_timer = self.death_flash_timer.saturating_sub(1);
            return None;
        }

        //? Update AI state (needs platforms for ledge detection, walls for LOS)
        let shoot_event = self.update_ai(player_pos, platforms, walls);

        //? Advance combat FSM if in a move
        if !self.entity.combat.is_idle() {
            let transition = fsm::advance_combat_fsm(&mut self.entity.combat, &self.move_db);
            if let Some(new_phase) = transition {
                match new_phase {
                    CombatPhase::Active => {
                        entity::spawn_hitbox(&mut self.entity, &self.move_db);
                    }
                    CombatPhase::Recovery | CombatPhase::Idle => {
                        entity::despawn_hitbox(&mut self.entity);
                        self.entity.hit_landed = false;
                        if new_phase == CombatPhase::Idle {
                            self.state = EnemyState::Cooldown {
                                timer: SHOOT_COOLDOWN,
                            };
                        }
                    }
                    CombatPhase::Startup => {}
                }
            }
        }

        //? Physics: gravity + platform collision
        entity::fixed_update_physics(
            &mut self.entity,
            platforms,
            dt,
            110.0 * PIXELS_PER_UNIT,
            40.0 * PIXELS_PER_UNIT,
        );

        shoot_event
    }

    fn update_ai(
        &mut self,
        player_pos: Vec2,
        platforms: &[AABB],
        walls: &[AABB],
    ) -> Option<ShootEvent> {
        let distance = (player_pos.x - self.entity.position.x).abs();
        let vertical_dist = (player_pos.y - self.entity.position.y).abs();

        //* Face the player when in aggro range
        if distance < self.config.aggro_range {
            self.entity.facing_right = player_pos.x > self.entity.position.x;
        }

        match self.state {
            EnemyState::Idle | EnemyState::Patrol { .. } => {
                //? Check for aggro: in range + LOS clear
                let in_range = distance < self.config.aggro_range;
                let has_los =
                    in_range && check_line_of_sight(self.entity.position, player_pos, walls);

                if in_range && has_los {
                    //? Melee punish: player too close
                    if distance < self.config.melee_range && vertical_dist < ENEMY_HEIGHT {
                        self.entity.velocity.x = 0.0;
                        self.state = EnemyState::MeleeWindup {
                            timer: ENEMY_MELEE_WINDUP_TICKS,
                        };
                        return None;
                    }

                    //? Ranged: enter Aim
                    if self.config.aim_ticks > 0 {
                        self.entity.velocity.x = 0.0;
                        self.state = EnemyState::Aim {
                            timer: self.config.aim_ticks,
                        };
                        return None;
                    }
                }

                //? No threat: patrol (movement + ledge tethering)
                if let EnemyState::Patrol { direction } = self.state {
                    self.entity.velocity.x = self.config.patrol_speed * direction as f32;
                    if self.should_reverse(direction, platforms) {
                        self.state = EnemyState::Patrol {
                            direction: -direction,
                        };
                        self.entity.velocity.x = 0.0;
                    }
                } else {
                    //? Idle → start patrolling
                    self.entity.velocity.x = 0.0;
                    if self.config.patrol_speed > 0.0 {
                        self.state = EnemyState::Patrol {
                            direction: if self.entity.facing_right { 1 } else { -1 },
                        };
                    }
                }
            }

            EnemyState::Attacking => {
                self.entity.velocity.x = 0.0;
            }

            EnemyState::Aim { timer } => {
                self.entity.velocity.x = 0.0;
                if timer <= 1 {
                    //? Build a ShootEvent and transition to Cooldown.
                    let flip = if self.entity.facing_right { 1.0 } else { -1.0 };
                    let origin =
                        self.entity.position + Vec2::new((ENEMY_WIDTH / 2.0 + 2.0) * flip, 0.0);

                    self.state = EnemyState::Cooldown {
                        timer: SHOOT_COOLDOWN,
                    };

                    return Some(ShootEvent {
                        origin,
                        target: player_pos,
                        speed: projectile::PROJECTILE_SPEED,
                        color: self.config.accent_color,
                    });
                } else {
                    self.state = EnemyState::Aim { timer: timer - 1 };
                }
            }

            EnemyState::MeleeWindup { timer } => {
                self.entity.velocity.x = 0.0;
                if timer <= 1 {
                    //? Execute melee attack via combat FSM
                    fsm::begin_move(
                        &mut self.entity.combat,
                        MoveId::AttackHorizontal,
                        &self.move_db,
                    );
                    self.state = EnemyState::Attacking;
                } else {
                    self.state = EnemyState::MeleeWindup { timer: timer - 1 };
                }
            }

            EnemyState::Cooldown { timer } => {
                self.entity.velocity.x = 0.0;
                if timer <= 1 {
                    self.state = EnemyState::Idle;
                } else {
                    self.state = EnemyState::Cooldown { timer: timer - 1 };
                }
            }

            EnemyState::Staggered { timer } => {
                self.entity.velocity.x *= 0.9;
                if timer <= 1 {
                    self.state = EnemyState::Idle;
                } else {
                    self.state = EnemyState::Staggered { timer: timer - 1 };
                }
            }

            EnemyState::Dead => {}
        }
        None
    }

    fn should_reverse(&self, direction: i8, platforms: &[AABB]) -> bool {
        let half_w = ENEMY_WIDTH / 2.0;
        let half_h = ENEMY_HEIGHT / 2.0;
        let probe_x =
            self.entity.position.x + (half_w + ENEMY_LEDGE_SENSOR_SIZE) * direction as f32;
        let probe_y = self.entity.position.y + half_h + 1.0;
        let sensor = AABB::new(
            Vec2::new(probe_x, probe_y),
            Vec2::new(ENEMY_LEDGE_SENSOR_SIZE, ENEMY_LEDGE_SENSOR_SIZE),
        );

        let has_floor = platforms.iter().any(|p| sensor.check_collision(p));
        if !has_floor {
            return true;
        }

        if (direction > 0 && self.entity.touching_wall_right)
            || (direction < 0 && self.entity.touching_wall_left)
        {
            return true;
        }

        false
    }

    //? Stagger this enemy (called when player parries their projectile).
    pub fn enter_stagger(&mut self) {
        //? Interrupt any current action
        self.entity.combat.phase = CombatPhase::Idle;
        self.entity.combat.current_move = None;
        self.entity.combat.frame_timer = 0;
        entity::despawn_hitbox(&mut self.entity);
        self.entity.velocity.x = 0.0;

        self.state = EnemyState::Staggered {
            timer: self.config.stagger_ticks,
        };
    }

    //? Kill this enemy (called on player execute attack).
    pub fn kill(&mut self) {
        self.entity.combat.phase = CombatPhase::Idle;
        self.entity.combat.current_move = None;
        entity::despawn_hitbox(&mut self.entity);
        self.entity.velocity = Vec2::ZERO;
        self.state = EnemyState::Dead;
        self.death_flash_timer = 6;
    }

    pub fn is_alive(&self) -> bool {
        self.state != EnemyState::Dead
    }

    pub fn is_staggered(&self) -> bool {
        matches!(self.state, EnemyState::Staggered { .. })
    }

    //? Freeze stagger timer at max. helps keeps enemy staggered while player is grappling to them.
    pub fn freeze_stagger(&mut self) {
        if let EnemyState::Staggered { ref mut timer } = self.state {
            *timer = self.config.stagger_ticks;
        }
    }
}

//? Line of sight check: returns false if any wall blocks the view from `from` to `to`.
pub fn check_line_of_sight(from: Vec2, to: Vec2, walls: &[AABB]) -> bool {
    let diff = to - from;
    let dist = diff.length();
    if dist < 1.0 {
        return true;
    }
    let steps = (dist / 8.0) as usize + 1;
    let probe_size = Vec2::new(2.0, 2.0);

    for i in 1..steps {
        let t = i as f32 / steps as f32;
        let point = from + diff * t;
        let probe = AABB::new(point, probe_size);
        for wall in walls {
            if probe.check_collision(wall) {
                return false; //* Wall blocks LOS
            }
        }
    }
    true
}

//? Render the enemy as a colored rectangle with type-based accent.
pub fn render_enemy(ctx: &mut engine::Context, enemy: &Enemy) {
    if !enemy.is_alive() {
        if enemy.death_flash_timer > 0 {
            let half = Vec2::new(ENEMY_WIDTH / 2.0, ENEMY_HEIGHT / 2.0);
            let top_left = enemy.entity.position - half;
            let t = enemy.death_flash_timer as f32 / 6.0;
            let flash_a = t.clamp(0.0, 1.0);
            ctx.draw_rect(
                top_left,
                Vec2::new(ENEMY_WIDTH, ENEMY_HEIGHT),
                [1.0, 0.15, 0.1, flash_a],
            );
        }
        return;
    }
    let half = Vec2::new(ENEMY_WIDTH / 2.0, ENEMY_HEIGHT / 2.0);
    let top_left = enemy.entity.position - half;

    //* State-based color modulation on top of type accent
    let color = match enemy.state {
        EnemyState::Staggered { .. } => [0.3, 0.3, 0.8, 0.9],
        EnemyState::MeleeWindup { .. } => [1.0, 1.0, 1.0, 0.9],
        EnemyState::Attacking => enemy.config.accent_color,
        EnemyState::Aim { .. } => {
            //* Pulsing aim indicator
            let pulse = 0.7 + 0.3 * (enemy.config.aim_ticks as f32 * 0.3).sin().abs();
            [
                enemy.config.accent_color[0] * pulse,
                enemy.config.accent_color[1] * pulse,
                enemy.config.accent_color[2] * pulse,
                0.9,
            ]
        }
        _ => enemy.config.accent_color,
    };

    ctx.draw_rect(top_left, Vec2::new(ENEMY_WIDTH, ENEMY_HEIGHT), color);

    //? Aim indicator: colored dot at bullet spawn point
    if let EnemyState::Aim { .. } = enemy.state {
        let flip = if enemy.entity.facing_right { 1.0 } else { -1.0 };
        let dot_pos = enemy.entity.position + Vec2::new((ENEMY_WIDTH / 2.0 + 4.0) * flip, 0.0);
        let dot_size = 4.0;
        ctx.draw_rect(
            dot_pos - Vec2::new(dot_size / 2.0, dot_size / 2.0),
            Vec2::new(dot_size, dot_size),
            [1.0, 1.0, 1.0, 0.9], //* Bright white dot
        );
    }
}

//? Render color-coded debug boxes for an entity.
pub fn render_debug_boxes(ctx: &mut engine::Context, entity: &Entity) {
    let t = 1.0; //* outline thickness

    //* Green: pushbox
    let pb = entity.pushbox();
    let pb_pos = pb.top_left();
    draw_outline(ctx, pb_pos, pb.size, [0.0, 1.0, 0.0, 0.8], t);

    //* Blue: hurtbox (semi-transparent fill + outline)
    let hb = entity.hurtbox();
    let hb_pos = hb.top_left();
    ctx.draw_rect(hb_pos, hb.size, [0.0, 0.3, 1.0, 0.15]);
    draw_outline(ctx, hb_pos, hb.size, [0.0, 0.3, 1.0, 0.6], t);

    //* Red: active hitbox
    if let Some(ref volume) = entity.hitbox_volume {
        let hitbox_aabb = volume.world_aabb(entity.position, entity.facing_right);
        let hit_pos = hitbox_aabb.top_left();
        ctx.draw_rect(hit_pos, hitbox_aabb.size, [1.0, 0.0, 0.0, 0.3]);
        draw_outline(ctx, hit_pos, hitbox_aabb.size, [1.0, 0.0, 0.0, 1.0], t);
    }

    //* Yellow: parry shield (3-sided, front-biased)
    if entity.combat.current_move == Some(MoveId::Parry)
        && entity.combat.phase == CombatPhase::Active
    {
        let parry_size = Vec2::new(PARRY_BOX_WIDTH, PARRY_BOX_HEIGHT);
        let flip = if entity.facing_right { 1.0 } else { -1.0 };
        let parry_center = entity.position + Vec2::new(PARRY_BOX_FRONT_OFFSET * flip, 0.0);
        let parry_pos = parry_center - parry_size / 2.0;
        ctx.draw_rect(parry_pos, parry_size, [1.0, 1.0, 0.0, 0.25]);
        draw_outline(ctx, parry_pos, parry_size, [1.0, 1.0, 0.0, 1.0], t);
    }
}

fn draw_outline(ctx: &mut engine::Context, pos: Vec2, size: Vec2, color: [f32; 4], t: f32) {
    ctx.draw_rect(pos, Vec2::new(size.x, t), color); //* top
    ctx.draw_rect(
        pos + Vec2::new(0.0, size.y - t),
        Vec2::new(size.x, t),
        color,
    ); //* bottom
    ctx.draw_rect(pos, Vec2::new(t, size.y), color); //* left
    ctx.draw_rect(
        pos + Vec2::new(size.x - t, 0.0),
        Vec2::new(t, size.y),
        color,
    ); //* right
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_platform() -> AABB {
        //? A 200px wide floor at y=200
        AABB::new(Vec2::new(100.0, 200.0), Vec2::new(200.0, 16.0))
    }

    #[test]
    fn enemy_starts_idle_with_1hp() {
        let e = Enemy::new(Vec2::new(100.0, 180.0), EnemyType::Grunt);
        assert_eq!(e.state, EnemyState::Idle);
        assert!((e.entity.health.max - 1.0).abs() < f32::EPSILON);
        assert!(e.is_alive());
    }

    #[test]
    fn enemy_type_configs_differ() {
        let grunt = EnemyConfig::for_type(EnemyType::Grunt);
        let sniper = EnemyConfig::for_type(EnemyType::Sniper);
        let ronin = EnemyConfig::for_type(EnemyType::Ronin);

        assert!(grunt.patrol_speed > 0.0);
        assert_eq!(sniper.patrol_speed, 0.0); //* Snipers are static
        assert!(ronin.aggro_range < grunt.aggro_range); //* Ronin is close-range
    }

    #[test]
    fn enemy_binds_to_platform() {
        let platform = make_platform();
        let mut e = Enemy::new(Vec2::new(100.0, 180.0), EnemyType::Grunt);
        e.bind_to_platform(&[platform]);
        assert!(e.spawn_platform.is_some());
    }

    #[test]
    fn ledge_detection_reverses_patrol() {
        let platform = make_platform();
        let platforms = [platform];
        //? Platform spans x=0..200 (center=100, half_width=100)
        let mut e = Enemy::new(Vec2::new(199.0, 180.0), EnemyType::Grunt);
        e.bind_to_platform(&platforms);
        e.entity.is_grounded = true;
        e.state = EnemyState::Patrol { direction: 1 };

        assert!(e.should_reverse(1, &platforms)); //* At x=199, near right edge, ledge sensor should detect drop-off
        assert!(!e.should_reverse(-1, &platforms)); //* But looking left should be fine (floor exists that way)
    }

    #[test]
    fn wall_contact_reverses_patrol() {
        let platform = make_platform();
        let platforms = [platform];
        let mut e = Enemy::new(Vec2::new(100.0, 180.0), EnemyType::Grunt);
        e.entity.touching_wall_right = true;
        assert!(e.should_reverse(1, &platforms));
        assert!(!e.should_reverse(-1, &platforms));
    }

    #[test]
    fn stagger_halts_enemy() {
        let mut e = Enemy::new(Vec2::new(100.0, 180.0), EnemyType::Grunt);
        e.entity.velocity.x = 100.0;
        e.enter_stagger();
        assert!(e.is_staggered());
        assert_eq!(e.entity.velocity.x, 0.0);
    }

    #[test]
    fn kill_enters_dead_state() {
        let mut e = Enemy::new(Vec2::new(100.0, 180.0), EnemyType::Grunt);
        e.kill();
        assert_eq!(e.state, EnemyState::Dead);
        assert!(!e.is_alive());
    }

    #[test]
    fn dead_enemy_skips_update() {
        let mut e = Enemy::new(Vec2::new(100.0, 180.0), EnemyType::Grunt);
        e.kill();
        let pos_before = e.entity.position; //* Position should not change, update was skipped
        e.fixed_update(1.0 / 60.0, 1, Vec2::new(50.0, 180.0), &[], &[]);
        assert_eq!(e.entity.position, pos_before);
    }

    #[test]
    fn stagger_timer_decrements_to_idle() {
        let mut e = Enemy::new(Vec2::new(100.0, 180.0), EnemyType::Grunt);
        e.enter_stagger();
        let platform = make_platform();
        let player_far = Vec2::new(1000.0, 180.0);

        //? Tick through stagger duration
        for _ in 0..ENEMY_STAGGER_TICKS {
            e.fixed_update(1.0 / 60.0, 0, player_far, &[platform], &[]);
        }
        assert_eq!(e.state, EnemyState::Idle);
    }

    #[test]
    fn los_blocked_by_wall() {
        let wall = AABB::new(Vec2::new(150.0, 180.0), Vec2::new(16.0, 32.0));
        //? Enemy at x=100, player at x=200, wall at x=150 blocks LOS
        assert!(!check_line_of_sight(
            Vec2::new(100.0, 180.0),
            Vec2::new(200.0, 180.0),
            &[wall],
        ));
    }

    #[test]
    fn los_clear_without_wall() {
        assert!(check_line_of_sight(
            Vec2::new(100.0, 180.0),
            Vec2::new(200.0, 180.0),
            &[], //* No walls
        ));
    }

    #[test]
    fn aim_timer_fires_shoot_event() {
        let platform = make_platform();
        let mut e = Enemy::new(Vec2::new(100.0, 180.0), EnemyType::Grunt);
        e.state = EnemyState::Aim { timer: 1 }; //* Last frame of aim
        let player_pos = Vec2::new(200.0, 180.0);

        let result = e.fixed_update(1.0 / 60.0, 1, player_pos, &[platform], &[]);
        assert!(result.is_some(), "Should return a ShootEvent");
        //? After firing, should be in Cooldown
        assert!(matches!(e.state, EnemyState::Cooldown { .. }));
    }
}
