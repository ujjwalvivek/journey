/**--------------------------------------------------------------------------------
*!  Generic game entity: Shared foundation for Player, Enemy, and NPC.
*?  Holds physics state, combat components, and collision boxes.
*?  System functions operate on entities generically so the same physics
*?  and combat logic applies to all actors.
*--------------------------------------------------------------------------------**/
use crate::combat::fsm::CombatPhase;
use crate::combat::moves::MoveDatabase;
use crate::combat::{CombatState, Health};
use engine::{AABB, BoxVolume, CollisionLayer, Vec2};

//? A game entity with physics, collision, and combat state.
#[derive(Debug, Clone)]
pub struct Entity {
    pub position: Vec2,
    pub velocity: Vec2,
    pub facing_right: bool,
    pub combat: CombatState,
    pub health: Health,
    pub pushbox_size: Vec2,
    pub hurtbox_size: Vec2,
    pub hitbox_volume: Option<BoxVolume>,
    pub hit_landed: bool,
    pub is_grounded: bool,
    pub touching_wall_left: bool,
    pub touching_wall_right: bool,
}

impl Entity {
    pub fn new(position: Vec2, pushbox_size: Vec2, health: f32) -> Self {
        Self {
            position,
            velocity: Vec2::ZERO,
            facing_right: true,
            combat: CombatState::default(),
            health: Health::new(health),
            pushbox_size,
            hurtbox_size: pushbox_size,
            hitbox_volume: None,
            hit_landed: false,
            is_grounded: false,
            touching_wall_left: false,
            touching_wall_right: false,
        }
    }

    //? Build the physics pushbox AABB from current position + size.
    pub fn pushbox(&self) -> AABB {
        AABB::new(self.position, self.pushbox_size)
    }

    //? Build the vulnerable hurtbox AABB from current position + size.
    pub fn hurtbox(&self) -> AABB {
        AABB::new(self.position, self.hurtbox_size)
    }
}

//? Resolves collisions on all axes using MTV (minimum translation vector).
pub fn fixed_update_physics(
    entity: &mut Entity,
    platforms: &[AABB],
    fixed_dt: f32,
    gravity: f32,
    max_fall_speed: f32,
) {
    //? Gravity (Y-down: positive = downward)
    if !entity.is_grounded || entity.velocity.y < 0.0 {
        entity.velocity.y = (entity.velocity.y + gravity * fixed_dt).min(max_fall_speed);
    }

    integrate_and_collide(entity, platforms, fixed_dt);
}

//? Use this when the caller handles gravity separately (e.g., player with variable jump height).
pub fn integrate_and_collide(entity: &mut Entity, platforms: &[AABB], fixed_dt: f32) {
    integrate_and_collide_with_one_way(entity, platforms, &[], fixed_dt);
}

//? Integrate and collide with one-way platform support.
//? One-way platforms only resolve downward collisions (landing on top).
//* CCD is decoupled into separate X and Y passes to prevent the "floor catch" bug:
//* a high-speed horizontal sweep against a floor tile would otherwise report a
//* horizontal wall hit (zeroing vx) because the combined diagonal displacement
//* enters the Minkowski-expanded floor from its side.
pub fn integrate_and_collide_with_one_way(
    entity: &mut Entity,
    solid_platforms: &[AABB],
    one_way_platforms: &[AABB],
    fixed_dt: f32,
) {
    //? Skin width: the X-sweep AABB is shrunk vertically by this amount on each
    //? side so the floor the player stands on is never detected as a side wall.
    const SKIN: f32 = 0.5;

    let vel_x = entity.velocity.x;
    let vel_y = entity.velocity.y;
    let disp_x = Vec2::new(vel_x * fixed_dt, 0.0);
    let disp_y = Vec2::new(0.0, vel_y * fixed_dt);

    let min_dim = entity.pushbox_size.x.min(entity.pushbox_size.y);
    let ccd_threshold_sq = min_dim * min_dim * 0.25;

    //? X sweep skin-shrunk AABB prevents floor surfaces from being
    //? registered as horizontal walls during fast horizontal movement.
    {
        let pb = entity.pushbox();
        let pb_x = AABB::new(
            pb.center,
            Vec2::new(pb.size.x, (pb.size.y - SKIN * 2.0).max(1.0)),
        );

        if disp_x.length_squared() > ccd_threshold_sq {
            let mut earliest_t = 1.0f32;
            let mut hit_normal = Vec2::ZERO;

            for platform in solid_platforms {
                if let Some(hit) = pb_x.swept_collision(disp_x, platform)
                    && hit.time < earliest_t
                {
                    earliest_t = hit.time;
                    hit_normal = hit.normal;
                }
            }

            if earliest_t < 1.0 {
                entity.position.x += disp_x.x * (earliest_t - 0.001).max(0.0);
                let v_dot = entity.velocity.dot(hit_normal);
                if v_dot < 0.0 {
                    entity.velocity -= hit_normal * v_dot;
                }
            } else {
                entity.position.x += disp_x.x;
            }
        } else {
            entity.position.x += disp_x.x;
        }
    }

    //? Y sweep full AABB, uses position already advanced by X sweep.
    {
        let pb = entity.pushbox();

        if disp_y.length_squared() > ccd_threshold_sq {
            let mut earliest_t = 1.0f32;
            let mut hit_normal = Vec2::ZERO;

            for platform in solid_platforms {
                if let Some(hit) = pb.swept_collision(disp_y, platform)
                    && hit.time < earliest_t
                {
                    earliest_t = hit.time;
                    hit_normal = hit.normal;
                }
            }

            if vel_y > 0.0 {
                for platform in one_way_platforms {
                    if let Some(hit) = pb.swept_collision(disp_y, platform)
                        && hit.normal.y < 0.0
                        && hit.time < earliest_t
                    {
                        earliest_t = hit.time;
                        hit_normal = hit.normal;
                    }
                }
            }

            if earliest_t < 1.0 {
                entity.position.y += disp_y.y * (earliest_t - 0.001).max(0.0);
                let v_dot = entity.velocity.dot(hit_normal);
                if v_dot < 0.0 {
                    entity.velocity -= hit_normal * v_dot;
                }
            } else {
                entity.position.y += disp_y.y;
            }
        } else {
            entity.position.y += disp_y.y;
        }
    }

    entity.is_grounded = false;
    entity.touching_wall_left = false;
    entity.touching_wall_right = false;

    //? Solid platforms: full MTV collision.
    //* Guard against false grounding: only land (zero vy, set is_grounded) when the
    //* entity center was ABOVE the platform center before the push. Without this,
    //* a player sliding down the side of a Floor tile can trigger an upward MTV at
    //* the bottom corner, clamping Y velocity and getting "glued" to the floor side.
    for platform in solid_platforms {
        let pushbox = entity.pushbox();
        if let Some(mtv) = AABB::resolve_collision(&pushbox, platform) {
            let pre_push_y = entity.position.y;
            entity.position += mtv;
            if mtv.y < 0.0 {
                if pre_push_y <= platform.center.y {
                    entity.velocity.y = 0.0;
                    entity.is_grounded = true;
                }
            } else if mtv.y > 0.0 {
                entity.velocity.y = 0.0; //* Ceiling hit
            }
            if mtv.x > 0.0 {
                entity.velocity.x = 0.0;
                entity.touching_wall_left = true;
            } else if mtv.x < 0.0 {
                entity.velocity.x = 0.0;
                entity.touching_wall_right = true;
            }
        }
    }

    for platform in one_way_platforms {
        let pushbox = entity.pushbox();
        if entity.velocity.y <= 0.0 {
            continue;
        }
        if let Some(mtv) = AABB::resolve_collision(&pushbox, platform)
            && mtv.y < 0.0
        {
            entity.position += mtv;
            entity.velocity.y = 0.0;
            entity.is_grounded = true;
        }
    }
}

pub fn spawn_hitbox(entity: &mut Entity, move_db: &MoveDatabase) {
    if let Some(move_id) = entity.combat.current_move {
        let data = move_db.get(move_id);
        if data.hitbox_size.x > 0.0 && data.hitbox_size.y > 0.0 {
            entity.hitbox_volume = Some(BoxVolume::new(
                CollisionLayer::Hitbox,
                data.hitbox_offset,
                data.hitbox_size,
            ));
        }
    }
    entity.hit_landed = false;
}

pub fn despawn_hitbox(entity: &mut Entity) {
    entity.hitbox_volume = None;
}

//? `direction` is +1.0 (attacker facing right) or -1.0 (facing left).
pub fn apply_knockback(entity: &mut Entity, knockback: Vec2, direction: f32) {
    entity.velocity.x += knockback.x * direction;
    entity.velocity.y += knockback.y;
}

//? Apply heavy friction during hit-stun. Decelerates much faster than normal.
pub fn apply_hitstun_friction(entity: &mut Entity, fixed_dt: f32, friction: f32) {
    if entity.is_grounded {
        let decel = friction * fixed_dt;
        if entity.velocity.x.abs() <= decel {
            entity.velocity.x = 0.0;
        } else {
            entity.velocity.x -= decel * entity.velocity.x.signum();
        }
    }
}

//? Result of a hitbox <-> hurtbox collision check.
#[derive(Debug, Clone, Copy)]
pub struct HitEvent {
    pub damage: u16,
    pub knockback: Vec2,
    pub recoil: Vec2,
    pub freeze_frames: u16,
    pub shake_intensity: f32,
}

//? Returns a HitEvent if a hit occurred.
pub fn check_hit(attacker: &Entity, defender: &Entity, move_db: &MoveDatabase) -> Option<HitEvent> {
    if attacker.hit_landed {
        return None;
    }
    let volume = attacker.hitbox_volume.as_ref()?;
    if !volume.active {
        return None;
    }
    if attacker.combat.phase != CombatPhase::Active {
        return None;
    }

    let hitbox_aabb = volume.world_aabb(attacker.position, attacker.facing_right);
    let hurtbox_aabb = defender.hurtbox();

    if hitbox_aabb.check_collision(&hurtbox_aabb) {
        let move_id = attacker.combat.current_move?;
        let data = move_db.get(move_id);
        //? Scale freeze/shake by damage (heavier moves = more juice)
        let freeze = if data.damage >= 30 {
            8
        } else if data.damage >= 20 {
            5
        } else {
            3
        };
        let shake = if data.damage >= 30 {
            6.0
        } else if data.damage >= 20 {
            4.0
        } else {
            2.0
        };
        Some(HitEvent {
            damage: data.damage,
            knockback: data.knockback,
            recoil: data.recoil,
            freeze_frames: freeze,
            shake_intensity: shake,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_falls_with_gravity() {
        let mut e = Entity::new(Vec2::new(100.0, 0.0), Vec2::new(30.0, 128.0), 100.0);
        let platforms = [];
        let gravity = 110.0 * 32.0; //* matches config
        let max_fall = 40.0 * 32.0;
        let dt = 1.0 / 60.0;

        let start_y = e.position.y;
        for _ in 0..10 {
            fixed_update_physics(&mut e, &platforms, dt, gravity, max_fall);
        }
        assert!(e.position.y > start_y, "entity should fall");
        assert!(!e.is_grounded);
    }

    #[test]
    fn entity_lands_on_platform() {
        let mut e = Entity::new(Vec2::new(100.0, 0.0), Vec2::new(30.0, 50.0), 100.0);
        let platform = AABB::new(Vec2::new(100.0, 100.0), Vec2::new(200.0, 20.0));
        let gravity = 3520.0;
        let max_fall = 1280.0;
        let dt = 1.0 / 60.0;

        for _ in 0..120 {
            fixed_update_physics(&mut e, &[platform], dt, gravity, max_fall);
        }
        assert!(e.is_grounded, "entity should land on platform");
    }

    //? Determinism test: same physics simulation at different "visual" frame rates
    //? must produce identical entity positions at tick boundaries.
    #[test]
    fn determinism_across_frame_rates() {
        let gravity = 110.0 * 32.0;
        let max_fall = 40.0 * 32.0;
        let fixed_dt = 1.0 / 60.0;
        let platform = AABB::new(Vec2::new(200.0, 300.0), Vec2::new(400.0, 20.0));
        let total_ticks = 180; //* 3 seconds at 60Hz

        //? Simulate at "60fps" - 1 fixed step per visual frame
        let mut e60 = Entity::new(Vec2::new(200.0, 0.0), Vec2::new(30.0, 50.0), 100.0);
        let mut positions_60: Vec<Vec2> = Vec::new();
        for _ in 0..total_ticks {
            fixed_update_physics(&mut e60, &[platform], fixed_dt, gravity, max_fall);
            positions_60.push(e60.position);
        }

        //? Simulate at "30fps" - 2 fixed steps per visual frame
        let mut e30 = Entity::new(Vec2::new(200.0, 0.0), Vec2::new(30.0, 50.0), 100.0);
        let mut positions_30: Vec<Vec2> = Vec::new();
        for _ in 0..(total_ticks / 2) {
            //? Two fixed steps per "visual frame"
            fixed_update_physics(&mut e30, &[platform], fixed_dt, gravity, max_fall);
            positions_30.push(e30.position);
            fixed_update_physics(&mut e30, &[platform], fixed_dt, gravity, max_fall);
            positions_30.push(e30.position);
        }

        //? Both should produce identical positions at every tick
        for (tick, (p60, p30)) in positions_60.iter().zip(positions_30.iter()).enumerate() {
            assert_eq!(
                *p60, *p30,
                "Position diverged at tick {tick}: 60fps={p60:?} vs 30fps={p30:?}"
            );
        }
    }

    #[test]
    fn hitbox_spawns_and_despawns_with_phase() {
        use crate::combat::fsm;
        let db = MoveDatabase::default();
        let mut e = Entity::new(Vec2::new(100.0, 100.0), Vec2::new(30.0, 50.0), 100.0);

        fsm::begin_move(&mut e.combat, crate::combat::MoveId::AttackHorizontal, &db);

        //? No hitbox during Startup
        assert!(e.hitbox_volume.is_none());

        //? Advance to Active phase (frame 3)
        for _ in 0..3 {
            fsm::advance_combat_fsm(&mut e.combat, &db);
        }
        assert_eq!(e.combat.phase, CombatPhase::Active);

        spawn_hitbox(&mut e, &db);
        assert!(e.hitbox_volume.is_some());
        let vol = e.hitbox_volume.as_ref().unwrap();
        assert_eq!(vol.layer, CollisionLayer::Hitbox);

        //? Verify world_aabb flips correctly
        let aabb_right = vol.world_aabb(e.position, true);
        let aabb_left = vol.world_aabb(e.position, false);
        assert!(aabb_right.center.x > e.position.x);
        assert!(aabb_left.center.x < e.position.x);

        //? Advance to Recovery (frame 6)
        for _ in 0..3 {
            fsm::advance_combat_fsm(&mut e.combat, &db);
        }
        assert_eq!(e.combat.phase, CombatPhase::Recovery);

        despawn_hitbox(&mut e);
        assert!(e.hitbox_volume.is_none());
    }

    #[test]
    fn check_hit_detects_overlap() {
        use crate::combat::fsm;
        let db = MoveDatabase::default();

        //? Attacker at x=100, facing right
        let mut attacker = Entity::new(Vec2::new(100.0, 100.0), Vec2::new(8.0, 32.0), 100.0);
        //? Defender at x=115 (within hitbox reach at internal resolution)
        let defender = Entity::new(Vec2::new(115.0, 100.0), Vec2::new(9.0, 32.0), 100.0);

        //? No hit when idle
        assert!(check_hit(&attacker, &defender, &db).is_none());

        fsm::begin_move(
            &mut attacker.combat,
            crate::combat::MoveId::AttackHorizontal,
            &db,
        );
        for _ in 0..3 {
            fsm::advance_combat_fsm(&mut attacker.combat, &db);
        }
        spawn_hitbox(&mut attacker, &db);

        //? Should detect hit
        let event = check_hit(&attacker, &defender, &db);
        assert!(event.is_some());
        let event = event.unwrap();
        assert_eq!(event.damage, 20);
    }

    #[test]
    fn check_hit_misses_when_far() {
        use crate::combat::fsm;
        let db = MoveDatabase::default();

        let mut attacker = Entity::new(Vec2::new(100.0, 100.0), Vec2::new(8.0, 32.0), 100.0);
        //? Defender far away
        let defender = Entity::new(Vec2::new(200.0, 100.0), Vec2::new(8.0, 32.0), 100.0);

        fsm::begin_move(
            &mut attacker.combat,
            crate::combat::MoveId::AttackHorizontal,
            &db,
        );
        for _ in 0..3 {
            fsm::advance_combat_fsm(&mut attacker.combat, &db);
        }
        spawn_hitbox(&mut attacker, &db);

        assert!(check_hit(&attacker, &defender, &db).is_none());
    }

    #[test]
    fn knockback_applies_directional_impulse() {
        let mut e = Entity::new(Vec2::ZERO, Vec2::new(30.0, 50.0), 100.0);
        apply_knockback(&mut e, Vec2::new(200.0, -50.0), 1.0);
        assert_eq!(e.velocity.x, 200.0);
        assert_eq!(e.velocity.y, -50.0);

        //? Opposite direction
        let mut e2 = Entity::new(Vec2::ZERO, Vec2::new(30.0, 50.0), 100.0);
        apply_knockback(&mut e2, Vec2::new(200.0, 0.0), -1.0);
        assert_eq!(e2.velocity.x, -200.0);
    }

    #[test]
    fn hitstun_friction_decelerates() {
        let mut e = Entity::new(Vec2::ZERO, Vec2::new(30.0, 50.0), 100.0);
        e.is_grounded = true;
        e.velocity.x = 300.0;
        let dt = 1.0 / 60.0;
        let friction = 3000.0;
        //? Apply several frames of friction
        for _ in 0..10 {
            apply_hitstun_friction(&mut e, dt, friction);
        }
        assert!(e.velocity.x < 300.0, "velocity should decrease");
        //? With enough friction, should reach 0
        for _ in 0..100 {
            apply_hitstun_friction(&mut e, dt, friction);
        }
        assert_eq!(e.velocity.x, 0.0);
    }
}
