/**--------------------------------------------------------------------------------
*!  Data-driven combat move definitions.
*?  Each combat action is defined by a MoveData struct with integer frame
*?  counts for Startup, Active, and Recovery phases. A MoveDatabase holds
*?  all available moves for lookup by MoveId.
*--------------------------------------------------------------------------------**/
use engine::Vec2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MoveId {
    AttackHorizontal,
    AttackUp,
    AttackDown,
    Parry,
    Dash,
    Grapple,
}

pub const BASE_TICK_RATE: u32 = 60;

//? All durations are in ticks at the base rate (60Hz). The MoveDatabase scales
//? them at runtime when the tick rate changes.
#[derive(Debug, Clone, Copy)]
pub struct MoveData {
    pub id: MoveId,
    pub startup_frames: u16,
    pub active_frames: u16,
    pub recovery_frames: u16,
    pub damage: u16,
    pub knockback: Vec2,
    pub recoil: Vec2,
    pub hitbox_offset: Vec2,
    pub hitbox_size: Vec2,
    //? Last N% of recovery allows cancelling into another move (0.0–1.0)
    pub cancel_window_pct: f32,
}

//? Scaled view of MoveData for the current tick rate.
//? All frame counts are adjusted so wall-clock timing stays consistent.
#[derive(Debug, Clone, Copy)]
pub struct ScaledMoveData {
    pub startup_frames: u16,
    pub active_frames: u16,
    pub recovery_frames: u16,
    pub damage: u16,
    pub knockback: Vec2,
    pub recoil: Vec2,
    pub hitbox_offset: Vec2,
    pub hitbox_size: Vec2,
    pub cancel_window_pct: f32,
}

impl ScaledMoveData {
    pub fn total_frames(&self) -> u16 {
        self.startup_frames + self.active_frames + self.recovery_frames
    }

    pub fn active_start(&self) -> u16 {
        self.startup_frames
    }

    pub fn recovery_start(&self) -> u16 {
        self.startup_frames + self.active_frames
    }

    pub fn cancel_window_start(&self) -> u16 {
        if self.cancel_window_pct <= 0.0 || self.recovery_frames == 0 {
            return self.total_frames();
        }
        let cancel_frames = (self.recovery_frames as f32 * self.cancel_window_pct).ceil() as u16;
        self.recovery_start() + self.recovery_frames.saturating_sub(cancel_frames)
    }
}

impl MoveData {
    pub fn total_frames(&self) -> u16 {
        self.startup_frames + self.active_frames + self.recovery_frames
    }

    //? Compute per-sprite-frame duration so animation playback matches FSM timing.
    pub fn anim_frame_duration(&self, anim_frame_count: usize) -> f32 {
        let move_duration_secs = self.total_frames() as f32 / BASE_TICK_RATE as f32;
        move_duration_secs / anim_frame_count as f32
    }

    pub fn active_start(&self) -> u16 {
        self.startup_frames
    }

    pub fn recovery_start(&self) -> u16 {
        self.startup_frames + self.active_frames
    }

    //? First frame of the cancel window within recovery (base rate).
    pub fn cancel_window_start(&self) -> u16 {
        if self.cancel_window_pct <= 0.0 || self.recovery_frames == 0 {
            return self.total_frames(); //* no cancel window
        }
        let cancel_frames = (self.recovery_frames as f32 * self.cancel_window_pct).ceil() as u16;
        self.recovery_start() + self.recovery_frames.saturating_sub(cancel_frames)
    }

    //? Scale this move's frame data to a different tick rate.
    pub fn scaled(&self, tick_rate: u32) -> ScaledMoveData {
        let ratio = tick_rate as f32 / BASE_TICK_RATE as f32;
        ScaledMoveData {
            startup_frames: (self.startup_frames as f32 * ratio).round() as u16,
            active_frames: (self.active_frames as f32 * ratio).round().max(1.0) as u16,
            recovery_frames: (self.recovery_frames as f32 * ratio).round() as u16,
            damage: self.damage,
            knockback: self.knockback,
            recoil: self.recoil,
            hitbox_offset: self.hitbox_offset,
            hitbox_size: self.hitbox_size,
            cancel_window_pct: self.cancel_window_pct,
        }
    }
}

const MOVE_COUNT: usize = 6;

impl MoveId {
    const fn index(self) -> usize {
        match self {
            MoveId::AttackHorizontal => 0,
            MoveId::AttackUp => 1,
            MoveId::AttackDown => 2,
            MoveId::Parry => 3,
            MoveId::Dash => 4,
            MoveId::Grapple => 5,
        }
    }
}

//? Stores base frame data at 60Hz and scales it on-the-fly for the current tick rate.
pub struct MoveDatabase {
    moves: [MoveData; MOVE_COUNT],
    tick_rate: u32,
}

impl MoveDatabase {
    //? Get the raw (unscaled) base data for a move. O(1) array index.
    pub fn get_base(&self, id: MoveId) -> &MoveData {
        &self.moves[id.index()]
    }

    //? Get the scaled frame data for a move at the current tick rate.
    pub fn get(&self, id: MoveId) -> ScaledMoveData {
        self.get_base(id).scaled(self.tick_rate)
    }

    //? Update the tick rate used for scaling.
    pub fn set_tick_rate(&mut self, tick_rate: u32) {
        self.tick_rate = tick_rate;
    }
}

impl Default for MoveDatabase {
    fn default() -> Self {
        Self {
            tick_rate: BASE_TICK_RATE,
            moves: [
                MoveData {
                    id: MoveId::AttackHorizontal,
                    startup_frames: 3,
                    active_frames: 3,
                    recovery_frames: 8,
                    damage: 20,
                    knockback: Vec2::new(50.0, 0.0),
                    recoil: Vec2::new(-10.0, 0.0),
                    hitbox_offset: Vec2::new(10.0, 0.0),
                    hitbox_size: Vec2::new(14.0, 16.0),
                    cancel_window_pct: 0.40,
                },
                MoveData {
                    id: MoveId::AttackUp,
                    startup_frames: 3,
                    active_frames: 4,
                    recovery_frames: 8,
                    damage: 20,
                    knockback: Vec2::new(0.0, -80.0),
                    recoil: Vec2::ZERO,
                    hitbox_offset: Vec2::new(0.0, -12.0),
                    hitbox_size: Vec2::new(16.0, 14.0),
                    cancel_window_pct: 0.40,
                },
                MoveData {
                    id: MoveId::AttackDown,
                    startup_frames: 2,
                    active_frames: 6,
                    recovery_frames: 10,
                    damage: 25,
                    knockback: Vec2::new(0.0, 60.0),
                    recoil: Vec2::new(0.0, -120.0),
                    hitbox_offset: Vec2::new(0.0, 12.0),
                    hitbox_size: Vec2::new(14.0, 16.0),
                    cancel_window_pct: 0.0,
                },
                MoveData {
                    id: MoveId::Parry,
                    startup_frames: 0,
                    active_frames: 6,
                    recovery_frames: 14,
                    damage: 0,
                    knockback: Vec2::ZERO,
                    recoil: Vec2::ZERO,
                    hitbox_offset: Vec2::new(5.0, 0.0),
                    hitbox_size: Vec2::new(10.0, 20.0),
                    cancel_window_pct: 0.0,
                },
                MoveData {
                    id: MoveId::Dash,
                    startup_frames: 0,
                    active_frames: 8,
                    recovery_frames: 0,
                    damage: 0,
                    knockback: Vec2::ZERO,
                    recoil: Vec2::ZERO,
                    hitbox_offset: Vec2::ZERO,
                    hitbox_size: Vec2::ZERO,
                    cancel_window_pct: 0.0,
                },
                MoveData {
                    id: MoveId::Grapple,
                    startup_frames: 2,
                    active_frames: 5,
                    recovery_frames: 10,
                    damage: 0,
                    knockback: Vec2::ZERO,
                    recoil: Vec2::ZERO,
                    hitbox_offset: Vec2::new(15.0, -5.0),
                    hitbox_size: Vec2::new(8.0, 8.0),
                    cancel_window_pct: 0.0,
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_frame_calculations() {
        let db = MoveDatabase::default();
        let atk = db.get(MoveId::AttackHorizontal);
        assert_eq!(atk.total_frames(), 14); //* 3 + 3 + 8
        assert_eq!(atk.active_start(), 3);
        assert_eq!(atk.recovery_start(), 6);
    }

    #[test]
    fn parry_no_cancel_window() {
        let db = MoveDatabase::default();
        let parry = db.get(MoveId::Parry);
        assert_eq!(parry.total_frames(), 20); //* 0 + 6 + 14
        assert_eq!(parry.cancel_window_start(), 20);
    }

    #[test]
    fn dash_zero_recovery() {
        let db = MoveDatabase::default();
        let dash = db.get(MoveId::Dash);
        assert_eq!(dash.total_frames(), 8);
        assert_eq!(dash.recovery_start(), 8);
    }

    #[test]
    fn scaling_preserves_at_60hz() {
        let mut db = MoveDatabase::default();
        db.set_tick_rate(60);
        let atk = db.get(MoveId::AttackHorizontal);
        assert_eq!(atk.startup_frames, 3);
        assert_eq!(atk.active_frames, 3);
        assert_eq!(atk.recovery_frames, 8);
    }

    #[test]
    fn scaling_halves_frames_at_30hz() {
        let mut db = MoveDatabase::default();
        db.set_tick_rate(30);
        let atk = db.get(MoveId::AttackHorizontal);
        assert_eq!(atk.startup_frames, 2);
        assert_eq!(atk.active_frames, 2);
        assert_eq!(atk.recovery_frames, 4);
    }

    #[test]
    fn grapple_frame_data() {
        let db = MoveDatabase::default();
        let grapple = db.get(MoveId::Grapple);
        assert_eq!(grapple.total_frames(), 17); //* 2 + 5 + 10
        assert_eq!(grapple.damage, 0);
    }

    #[test]
    fn attack_up_has_vertical_knockback() {
        let db = MoveDatabase::default();
        let atk_up = db.get(MoveId::AttackUp);
        assert!(atk_up.knockback.y < 0.0); //* launches upward
        assert_eq!(atk_up.total_frames(), 15); //* 3 + 4 + 8
    }
}
