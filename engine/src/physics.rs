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
}
