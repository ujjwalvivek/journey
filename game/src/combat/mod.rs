/**--------------------------------------------------------------------------------
*!  Combat system types for Sekiro-Ghostrunner-style gameplay.
*?  Defines Health, Hitbox, and the frame-deterministic combat FSM.
*?  All combat timing uses integer tick counts (at 60Hz) for deterministic
*?  frame-data windows - never float accumulators.
*--------------------------------------------------------------------------------**/
pub mod fsm;
pub mod input_buffer;
pub mod moves;
use engine::AABB;
pub use fsm::CombatState;
pub use input_buffer::CombatInputBuffer;
pub use moves::{MoveData, MoveDatabase, MoveId};

#[derive(Debug, Clone)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    pub fn new(max: f32) -> Self {
        Self { current: max, max }
    }

    pub fn take_damage(&mut self, amount: f32) {
        self.current = (self.current - amount).max(0.0);
    }

    pub fn is_dead(&self) -> bool {
        self.current <= 0.0
    }

    pub fn fraction(&self) -> f32 {
        self.current / self.max
    }
}

//? The Hitbox struct represents a tick-windowed hitbox (knows both where and when it's active).
//? Spawned by attack animations and destroyed when the window closes.
//? Currently the live system uses  BoxVolume + CombatPhase::Active for timing.
//* Hitbox is the more self-contained design for when the combat system grows (multiple overlapping hit windows, multi-hit combos).
#[derive(Debug, Clone)]
pub struct Hitbox {
    pub aabb: AABB,
    pub damage: f32,
    pub posture_damage: f32,
    //* Inclusive tick range during which this hitbox can deal damage.
    pub start_tick: u64,
    pub end_tick: u64,
}

impl Hitbox {
    pub fn is_active(&self, tick: u64) -> bool {
        tick >= self.start_tick && tick <= self.end_tick
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_damage_clamps() {
        let mut h = Health::new(100.0);
        h.take_damage(60.0);
        assert!((h.current - 40.0).abs() < f32::EPSILON);
        h.take_damage(100.0);
        assert_eq!(h.current, 0.0);
        assert!(h.is_dead());
    }

    #[test]
    fn hitbox_active_window() {
        let hb = Hitbox {
            aabb: AABB::new(engine::Vec2::ZERO, engine::Vec2::new(10.0, 10.0)),
            damage: 10.0,
            posture_damage: 5.0,
            start_tick: 10,
            end_tick: 15,
        };
        assert!(!hb.is_active(9));
        assert!(hb.is_active(10));
        assert!(hb.is_active(12));
        assert!(hb.is_active(15));
        assert!(!hb.is_active(16));
    }
}
