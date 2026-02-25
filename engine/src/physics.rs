/**------------------------------------------------------------
*!  Basic 2D physics primitives for collision detection.
*?  Usage: Use AABB for player, enemies, platforms,
*?  and any rectangular object needing collision checks.
*------------------------------------------------------------**/
use glam::Vec2;

//? Axis-Aligned Bounding Box for 2D collision detection.
//* AABBs are simple and efficient for collision detection
//* in 2D games, since their sides are always parallel to the axes.
#[derive(Debug, Clone, Copy)]
pub struct AABB {
    pub center: Vec2,
    pub size: Vec2,
}

impl AABB {
    //? Create a new AABB from center position and size.
    pub fn new(center: Vec2, size: Vec2) -> Self {
        Self { center, size }
    }

    //? Create AABB from top-left position and size (common for sprite rendering).
    pub fn from_top_left(top_left: Vec2, size: Vec2) -> Self {
        let center = top_left + size * 0.5;
        Self { center, size }
    }

    //* Different coordinate conventions (center vs. top-left) are used
    //* in rendering and physics, so both constructors make integration easy.

    //? Get the minimum corner (top-left in screen space).
    pub fn min(&self) -> Vec2 {
        self.center - self.size * 0.5
    }

    //? Get the maximum corner (bottom-right in screen space).
    pub fn max(&self) -> Vec2 {
        self.center + self.size * 0.5
    }

    //? Alias - Get the top-left position (useful for sprite rendering).
    pub fn top_left(&self) -> Vec2 {
        self.min()
    }

    //? Check if this AABB overlaps with another.
    //* If the min of one is less than the max of the other,
    //* and vice versa, on both axes, they overlap.
    pub fn check_collision(&self, other: &AABB) -> bool {
        let self_min = self.min();
        let self_max = self.max();
        let other_min = other.min();
        let other_max = other.max();

        self_min.x < other_max.x
            && self_max.x > other_min.x
            && self_min.y < other_max.y
            && self_max.y > other_min.y
    }

    //? Get the overlap amount on each axis (positive means collision).
    //* Returns (x_overlap, y_overlap).
    pub fn get_overlap(&self, other: &AABB) -> Vec2 {
        let self_min = self.min();
        let self_max = self.max();
        let other_min = other.min();
        let other_max = other.max();

        let x_overlap = (self_max.x - other_min.x).min(other_max.x - self_min.x);
        let y_overlap = (self_max.y - other_min.y).min(other_max.y - self_min.y);

        Vec2::new(x_overlap, y_overlap)
    }

    //? Compute the minimum translation vector (MTV) to push `mover` out of `obstacle`.
    //? Returns `None` if they don't overlap. Pushes along the smallest overlap axis.
    //* When both axes have equal overlap, the Y axis wins (the `else` branch).
    //* The default push direction for Y is upward (`sign = -1.0`) because in a
    //* Y-down coordinate system this places entities on top of platforms rather
    //* than pushing them through. This intentional directional bias is correct
    //* for a platformer where landing on surfaces is the dominant collision case.
    pub fn resolve_collision(mover: &AABB, obstacle: &AABB) -> Option<Vec2> {
        let overlap = mover.get_overlap(obstacle);
        if overlap.x <= 0.0 || overlap.y <= 0.0 {
            return None;
        }
        if overlap.x < overlap.y {
            let sign = (mover.center.x - obstacle.center.x).signum();
            let sign = if sign == 0.0 { 1.0 } else { sign };
            Some(Vec2::new(overlap.x * sign, 0.0))
        } else {
            let sign = (mover.center.y - obstacle.center.y).signum();
            let sign = if sign == 0.0 { -1.0 } else { sign };
            Some(Vec2::new(0.0, overlap.y * sign))
        }
    }
}

//? Collision layer tag for multi-layer AABB system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CollisionLayer {
    Pushbox,
    Hurtbox,
    Hitbox,
    Parrybox,
}

//? A positioned collision volume relative to an entity center.
#[derive(Debug, Clone, Copy)]
pub struct BoxVolume {
    pub layer: CollisionLayer,
    pub local_offset: Vec2,
    pub size: Vec2,
    pub active: bool,
}

impl BoxVolume {
    pub fn new(layer: CollisionLayer, offset: Vec2, size: Vec2) -> Self {
        Self {
            layer,
            local_offset: offset,
            size,
            active: true,
        }
    }

    //? Generate the world-space AABB, flipping X offset based on facing direction.
    pub fn world_aabb(&self, entity_pos: Vec2, facing_right: bool) -> AABB {
        let flip = if facing_right { 1.0 } else { -1.0 };
        let center = entity_pos + Vec2::new(self.local_offset.x * flip, self.local_offset.y);
        AABB::new(center, self.size)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SweepResult {
    pub time: f32,
    pub normal: Vec2,
}

impl AABB {
    //? Swept AABB: move `self` along `displacement` and find the earliest
    //? collision with `obstacle`. Uses Minkowski-expanded ray cast.
    pub fn swept_collision(&self, displacement: Vec2, obstacle: &AABB) -> Option<SweepResult> {
        if displacement.x == 0.0 && displacement.y == 0.0 {
            return None;
        }

        //* Minkowski expansion: grow obstacle by mover's half-extents
        let expanded_half = (obstacle.size + self.size) * 0.5;
        let exp_min = obstacle.center - expanded_half;
        let exp_max = obstacle.center + expanded_half;
        let origin = self.center;

        let (t_near_x, t_far_x) = if displacement.x.abs() > f32::EPSILON {
            let t1 = (exp_min.x - origin.x) / displacement.x;
            let t2 = (exp_max.x - origin.x) / displacement.x;
            (t1.min(t2), t1.max(t2))
        } else if origin.x >= exp_min.x && origin.x <= exp_max.x {
            (f32::NEG_INFINITY, f32::INFINITY)
        } else {
            return None;
        };

        let (t_near_y, t_far_y) = if displacement.y.abs() > f32::EPSILON {
            let t1 = (exp_min.y - origin.y) / displacement.y;
            let t2 = (exp_max.y - origin.y) / displacement.y;
            (t1.min(t2), t1.max(t2))
        } else if origin.y >= exp_min.y && origin.y <= exp_max.y {
            (f32::NEG_INFINITY, f32::INFINITY)
        } else {
            return None;
        };

        let t_entry = t_near_x.max(t_near_y);
        let t_exit = t_far_x.min(t_far_y);

        if t_entry > t_exit || t_entry >= 1.0 || t_exit <= 0.0 {
            return None;
        }

        let time = t_entry.clamp(0.0, 1.0);

        let normal = if t_near_x > t_near_y {
            Vec2::new(-displacement.x.signum(), 0.0)
        } else {
            Vec2::new(0.0, -displacement.y.signum())
        };

        Some(SweepResult { time, normal })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swept_detects_head_on_collision() {
        let mover = AABB::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        let wall = AABB::new(Vec2::new(30.0, 0.0), Vec2::new(10.0, 20.0));
        let displacement = Vec2::new(50.0, 0.0);

        let result = mover.swept_collision(displacement, &wall);
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.time > 0.0 && r.time < 1.0);
        assert_eq!(r.normal, Vec2::new(-1.0, 0.0));
    }

    #[test]
    fn swept_misses_when_parallel() {
        let mover = AABB::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        let wall = AABB::new(Vec2::new(0.0, 50.0), Vec2::new(10.0, 10.0));
        let displacement = Vec2::new(100.0, 0.0);

        assert!(mover.swept_collision(displacement, &wall).is_none());
    }

    #[test]
    fn swept_returns_none_for_zero_displacement() {
        let mover = AABB::new(Vec2::ZERO, Vec2::new(10.0, 10.0));
        let wall = AABB::new(Vec2::new(20.0, 0.0), Vec2::new(10.0, 10.0));
        assert!(mover.swept_collision(Vec2::ZERO, &wall).is_none());
    }

    #[test]
    fn swept_detects_downward_landing() {
        let mover = AABB::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        let floor = AABB::new(Vec2::new(0.0, 40.0), Vec2::new(100.0, 10.0));
        let displacement = Vec2::new(0.0, 60.0);

        let result = mover.swept_collision(displacement, &floor);
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.time < 1.0);
        assert_eq!(r.normal, Vec2::new(0.0, -1.0));
    }

    #[test]
    fn resolve_collision_pushes_out() {
        let mover = AABB::new(Vec2::new(10.0, 0.0), Vec2::new(10.0, 10.0));
        let wall = AABB::new(Vec2::new(18.0, 0.0), Vec2::new(10.0, 10.0));
        let mtv = AABB::resolve_collision(&mover, &wall);
        assert!(mtv.is_some());
        let mtv = mtv.unwrap();
        assert!(mtv.x < 0.0, "should push mover left, away from wall");
    }
}
