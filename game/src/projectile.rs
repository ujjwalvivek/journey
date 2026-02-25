/**--------------------------------------------------------------------------------
*!  Projectile system for enemy ranged attacks.
*?  Projectiles are simple AABB entities that move in a straight line.
*?  They despawn after first bounce, kill the player on hurtbox contact,
*?  and are deflected (destroyed + source enemy staggered) on parry contact.
*?  Core chain: Enemy shoots → Projectile flies → Player parries → Enemy staggers.
*--------------------------------------------------------------------------------**/
use crate::combat::fsm::CombatPhase;
use crate::combat::moves::MoveId;
use crate::config::*;
use crate::entity::Entity;
use engine::{AABB, Vec2};


const PROJECTILE_SIZE: f32 = 4.0; //* 4×4 pixel bullet
pub const PROJECTILE_SPEED: f32 = 200.0; //* px/s   Grunt default
const PROJECTILE_MAX_RANGE: f32 = 400.0; //* Despawn after this distance from spawn
const MAX_BOUNCES: u8 = 1; //* Ricochet once, then despawn on next wall

#[derive(Debug, Clone)]
pub struct Projectile {
    pub position: Vec2,
    pub velocity: Vec2,
    pub spawn_origin: Vec2,
    //? Index into the `enemies` Vec. Used to stagger the source on parry.
    pub source_enemy_idx: usize,
    pub alive: bool,
    pub color: [f32; 4],
    pub bounces: u8,
}

impl Projectile {
    //? Spawn a projectile from an enemy toward a target position.
    pub fn new(
        origin: Vec2,
        target: Vec2,
        source_enemy_idx: usize,
        speed: f32,
        color: [f32; 4],
    ) -> Self {
        let diff = target - origin;
        let dist = diff.length();
        let dir = if dist > 0.001 {
            diff / dist
        } else {
            Vec2::new(1.0, 0.0)
        };

        Self {
            position: origin,
            velocity: dir * speed,
            spawn_origin: origin,
            source_enemy_idx,
            alive: true,
            color,
            bounces: 0,
        }
    }

    //? AABB for collision checks at the current position.
    pub fn aabb(&self) -> AABB {
        AABB::new(self.position, Vec2::new(PROJECTILE_SIZE, PROJECTILE_SIZE))
    }

    //? Advance position by one tick. Despawn if past max range.
    pub fn update(&mut self, dt: f32) {
        if !self.alive {
            return;
        }
        self.position += self.velocity * dt;

        //* Range limit: despawn if too far from spawn point
        let dist_sq = (self.position - self.spawn_origin).length_squared();
        if dist_sq > PROJECTILE_MAX_RANGE * PROJECTILE_MAX_RANGE {
            self.alive = false;
        }
    }
}

//? Simple growable pool of projectiles. Dead projectiles are recycled.
pub struct ProjectilePool {
    pub projectiles: Vec<Projectile>,
}

impl Default for ProjectilePool {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectilePool {
    pub fn new() -> Self {
        Self {
            projectiles: Vec::with_capacity(32),
        }
    }

    //? Spawn a new projectile, reusing a dead slot if available.
    pub fn spawn(
        &mut self,
        origin: Vec2,
        target: Vec2,
        source_enemy_idx: usize,
        speed: f32,
        color: [f32; 4],
    ) {
        let proj = Projectile::new(origin, target, source_enemy_idx, speed, color);

        //? Reuse a dead slot if available
        if let Some(slot) = self.projectiles.iter_mut().find(|p| !p.alive) {
            *slot = proj;
        } else {
            self.projectiles.push(proj);
        }
    }

    pub fn update_all(&mut self, dt: f32) {
        for proj in &mut self.projectiles {
            proj.update(dt);
        }
    }

    //? Collide projectiles with solid walls/floors (NOT one-way platforms).
    //? On first contact the bullet ricochets (reflects velocity); on second it despawns.
    //? Returns the number of ricochets that occurred (for audio).
    pub fn collide_walls(&mut self, walls: &[AABB]) -> u32 {
        let mut bounce_count = 0u32;
        for proj in &mut self.projectiles {
            if !proj.alive {
                continue;
            }
            let proj_aabb = proj.aabb();
            for wall in walls {
                if proj_aabb.check_collision(wall) {
                    if proj.bounces >= MAX_BOUNCES {
                        proj.alive = false;
                    } else {
                        //? Reflect off the wall: determine which axis to flip
                        //? by comparing overlap depths on each axis.
                        let overlap_x = (proj_aabb.center.x - wall.center.x).abs()
                            - (proj_aabb.size.x + wall.size.x) / 2.0;
                        let overlap_y = (proj_aabb.center.y - wall.center.y).abs()
                            - (proj_aabb.size.y + wall.size.y) / 2.0;

                        if overlap_x > overlap_y {
                            //* Shallower X overlap = hitting a vertical surface
                            proj.velocity.x = -proj.velocity.x;
                        } else {
                            //* Shallower Y overlap = hitting a horizontal surface
                            proj.velocity.y = -proj.velocity.y;
                        }
                        //? Nudge out of the wall to prevent double-bounce
                        proj.position += proj.velocity * (1.0 / 60.0);
                        proj.bounces += 1;
                        bounce_count += 1;
                    }
                    break;
                }
            }
        }
        bounce_count
    }

    pub fn check_player_hit(&mut self, player: &Entity) -> bool {
        let player_hurtbox = player.hurtbox();
        for proj in &mut self.projectiles {
            if !proj.alive {
                continue;
            }
            if proj.aabb().check_collision(&player_hurtbox) {
                proj.alive = false;
                return true;
            }
        }
        false
    }

    //? Check if any alive projectile is deflected by the player's active parry.
    //? Returns the `source_enemy_idx` of the deflected projectile's source, if any.
    pub fn check_parry_deflect(&mut self, player: &Entity) -> Option<usize> {
        //? Only check if player is in active parry phase
        if player.combat.current_move != Some(MoveId::Parry)
            || player.combat.phase != CombatPhase::Active
        {
            return None;
        }

        let parry_box = parry_aabb(player);
        for proj in &mut self.projectiles {
            if !proj.alive {
                continue;
            }
            if proj.aabb().check_collision(&parry_box) {
                let source = proj.source_enemy_idx;
                proj.alive = false;
                return Some(source);
            }
        }
        None
    }

    //? Count of alive projectiles (for debug UI).
    pub fn alive_count(&self) -> usize {
        self.projectiles.iter().filter(|p| p.alive).count()
    }
}

pub fn parry_aabb(entity: &Entity) -> AABB {
    let flip = if entity.facing_right { 1.0 } else { -1.0 };
    let parry_center = entity.position + Vec2::new(PARRY_BOX_FRONT_OFFSET * flip, 0.0);
    AABB::new(parry_center, Vec2::new(PARRY_BOX_WIDTH, PARRY_BOX_HEIGHT))
}

//? Render all alive projectiles as neon-colored squares.
pub fn render_projectiles(ctx: &mut engine::Context, pool: &ProjectilePool) {
    let half = PROJECTILE_SIZE / 2.0;
    for proj in &pool.projectiles {
        if !proj.alive {
            continue;
        }
        let top_left = proj.position - Vec2::new(half, half);
        ctx.draw_rect(
            top_left,
            Vec2::new(PROJECTILE_SIZE, PROJECTILE_SIZE),
            proj.color,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_player_entity() -> Entity {
        Entity::new(
            Vec2::new(100.0, 100.0),
            Vec2::new(PLAYER_WIDTH, PLAYER_HEIGHT),
            1.0,
            100.0,
        )
    }

    #[test]
    fn projectile_moves_toward_target() {
        let mut proj = Projectile::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(100.0, 0.0),
            0,
            200.0,
            [1.0, 1.0, 0.0, 1.0],
        );
        let dt = 1.0 / 60.0;
        proj.update(dt);

        assert!(proj.position.x > 0.0); //* Should move rightward
        assert!((proj.position.y).abs() < 0.01);
        assert!(proj.alive);
    }

    #[test]
    fn projectile_despawns_past_max_range() {
        let mut proj = Projectile::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            0,
            10000.0, //* Very fast   will exceed range in one tick
            [1.0, 1.0, 0.0, 1.0],
        );
        proj.update(1.0); //* 1 second at 10000 px/s = 10000px >> 300px max range
        assert!(!proj.alive);
    }

    #[test]
    fn wall_collision_despawns_projectile() {
        let mut pool = ProjectilePool::new();
        pool.spawn(
            Vec2::new(10.0, 10.0),
            Vec2::new(20.0, 10.0),
            0,
            200.0,
            [1.0, 0.0, 0.0, 1.0],
        );

        //? Wall at x=15   directly in the path
        let wall = AABB::new(Vec2::new(15.0, 10.0), Vec2::new(10.0, 10.0));
        pool.collide_walls(&[wall]);

        //? Initial position (10,10) already overlaps wall at center=15 ± 5
        assert_eq!(pool.alive_count(), 1);
        assert_eq!(pool.projectiles[0].bounces, 1); //* First collision = ricochet (bounces 0 → 1)
        pool.collide_walls(&[wall]); //* Second collision = despawn
        //? Might still be alive if nudged out of wall; force position back in
        pool.projectiles[0].position = Vec2::new(10.0, 10.0);
        pool.collide_walls(&[wall]);
        assert_eq!(pool.alive_count(), 0);
    }

    #[test]
    fn ricochet_reflects_velocity() {
        let mut pool = ProjectilePool::new();
        pool.spawn(
            Vec2::new(10.0, 10.0),
            Vec2::new(20.0, 10.0), //* Moving right
            0,
            200.0,
            [1.0, 0.0, 0.0, 1.0],
        );
        let vel_before = pool.projectiles[0].velocity.x;

        //? Wall to the right
        let wall = AABB::new(Vec2::new(15.0, 10.0), Vec2::new(10.0, 10.0));
        pool.collide_walls(&[wall]);

        //? X velocity should be reversed after ricochet
        assert!(pool.projectiles[0].alive);
        let vel_after = pool.projectiles[0].velocity.x;
        assert!(
            vel_after < 0.0 && vel_before > 0.0,
            "Velocity should reverse: was {} now {}",
            vel_before,
            vel_after
        );
    }
    #[test]
    fn projectile_hits_player() {
        let mut pool = ProjectilePool::new();
        let player = make_player_entity();

        //? Spawn a projectile right on top of the player
        pool.spawn(
            player.position,
            Vec2::new(player.position.x + 10.0, player.position.y),
            0,
            200.0,
            [1.0, 0.0, 0.0, 1.0],
        );

        assert!(pool.check_player_hit(&player));
        assert_eq!(pool.alive_count(), 0);
    }

    #[test]
    fn parry_deflects_projectile() {
        let mut pool = ProjectilePool::new();
        let mut player = make_player_entity();

        //? Put player in active parry state
        player.combat.current_move = Some(MoveId::Parry);
        player.combat.phase = CombatPhase::Active;
        player.facing_right = true;

        //? Spawn projectile at the parry box position (slightly ahead of player)
        let parry_pos = player.position + Vec2::new(PARRY_BOX_FRONT_OFFSET, 0.0);
        pool.spawn(
            parry_pos,
            Vec2::new(parry_pos.x + 10.0, parry_pos.y),
            42, //* source enemy index
            200.0,
            [1.0, 0.0, 0.0, 1.0],
        );

        let result = pool.check_parry_deflect(&player);
        assert_eq!(result, Some(42));
        assert_eq!(pool.alive_count(), 0);
    }

    #[test]
    fn no_parry_when_not_in_active_phase() {
        let mut pool = ProjectilePool::new();
        let mut player = make_player_entity();

        //? Player is parrying but in startup (not active yet)
        player.combat.current_move = Some(MoveId::Parry);
        player.combat.phase = CombatPhase::Startup;

        pool.spawn(
            player.position,
            Vec2::new(player.position.x + 10.0, player.position.y),
            0,
            200.0,
            [1.0, 0.0, 0.0, 1.0],
        );

        //? Should NOT deflect, parry isn't active yet
        assert!(pool.check_parry_deflect(&player).is_none());
        assert_eq!(pool.alive_count(), 1);
    }

    #[test]
    fn pool_reuses_dead_slots() {
        let mut pool = ProjectilePool::new();
        pool.spawn(
            Vec2::ZERO,
            Vec2::new(1.0, 0.0),
            0,
            100.0,
            [1.0, 0.0, 0.0, 1.0],
        ); //* Kill the projectile
        pool.projectiles[0].alive = false; 

        //? Spawn another. Should reuse the slot
        pool.spawn(
            Vec2::new(50.0, 50.0),
            Vec2::new(60.0, 50.0),
            1,
            100.0,
            [0.0, 1.0, 0.0, 1.0],
        );

        assert_eq!(pool.projectiles.len(), 1); //* Same slot reused
        assert!(pool.projectiles[0].alive);
        assert_eq!(pool.projectiles[0].source_enemy_idx, 1);
    }
}
