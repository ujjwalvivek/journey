/**----------------------------------------------------------------------
*!  Game assets spritesheets, grid-based animation definitions.
*?  Combat animation durations are derived from FSM frame data so
*?  visual playback stays locked to combat timing.
*?  Grid layout: 5 columns × 13 rows, 100×100 px per cell.
*?  Player Spritesheet row mapping:
*?    Row 0 (0-3):   Idle         4 frames
*?    Row 1 (5-8):   Run          4 frames
*?    Row 2 (10-14): Jump         5 frames (10-12 ascend, 13-14 fall)
*?    Row 3 (15-18): Death        4 frames
*?    Row 4 (20-23): Parry        4 frames
*?    Row 5 (25-28): AttackHoriz  4 frames
*?    Row 6 (30-33): AttackUp     4 frames
*?    Row 7 (35-38): AttackDown   4 frames
*?    Row 8 (40-43): Dash         4 frames
*?    Row 9 (45-47): WallGrab     3 frames
*?    Row 10 (50-51): GrappleSame 2 frames
*?    Row 11 (55-56): GrappleUp   2 frames
*?    Row 12 (60-61): GrappleDown 2 frames
*----------------------------------------------------------------------**/
use crate::anim::Animation;
use crate::combat::moves::MoveDatabase;

pub const FRAME_WIDTH: f32 = 100.0;
pub const FRAME_HEIGHT: f32 = 100.0;
pub const SHEET_COLS: usize = 5;

pub struct PlayerAnimations;

impl PlayerAnimations {
    pub fn create_all() -> Vec<Animation> {
        let db = MoveDatabase::default();
        vec![
            Self::idle(),
            Self::run(),
            Self::jump(),
            Self::fall(),
            Self::death(),
            Self::parry(&db),
            Self::attack_horizontal(&db),
            Self::attack_up(&db),
            Self::attack_down(&db),
            Self::dash(&db),
            Self::wall_slide(),
            Self::wall_grab(),
            Self::grapple(&db),
        ]
    }

    fn idle() -> Animation {
        Animation::new("Idle", 0, 4, 0.12, true)
    }
    fn run() -> Animation {
        Animation::new("Run", 5, 4, 0.08, true)
    }
    fn jump() -> Animation {
        Animation::new("Jump", 10, 3, 0.1, false)
    }
    fn fall() -> Animation {
        //?Fall uses the last 2 frames of the Jump row (descent portion)
        Animation::new("Fall", 13, 2, 0.1, true)
    }
    fn death() -> Animation {
        Animation::new("Death", 15, 4, 0.1, false)
    }
    fn parry(db: &MoveDatabase) -> Animation {
        let frames = 4;
        let fd = db
            .get_base(crate::combat::MoveId::Parry)
            .anim_frame_duration(frames);
        Animation::new("Parry", 20, frames, fd, false)
    }
    fn attack_horizontal(db: &MoveDatabase) -> Animation {
        let frames = 4;
        let fd = db
            .get_base(crate::combat::MoveId::AttackHorizontal)
            .anim_frame_duration(frames);
        Animation::new("AttackHorizontal", 25, frames, fd, false)
    }
    fn attack_up(db: &MoveDatabase) -> Animation {
        let frames = 4;
        let fd = db
            .get_base(crate::combat::MoveId::AttackUp)
            .anim_frame_duration(frames);
        Animation::new("AttackUp", 30, frames, fd, false)
    }
    fn attack_down(db: &MoveDatabase) -> Animation {
        let frames = 4;
        let fd = db
            .get_base(crate::combat::MoveId::AttackDown)
            .anim_frame_duration(frames);
        Animation::new("AttackDown", 35, frames, fd, false)
    }
    fn dash(db: &MoveDatabase) -> Animation {
        let frames = 4;
        let fd = db
            .get_base(crate::combat::MoveId::Dash)
            .anim_frame_duration(frames);
        Animation::new("Dash", 40, frames, fd, false)
    }
    fn wall_slide() -> Animation {
        Animation::new("WallSlide", 45, 3, 0.1, true)
    }
    fn wall_grab() -> Animation {
        Animation::new("WallGrab", 45, 3, 0.15, true)
    }
    fn grapple(db: &MoveDatabase) -> Animation {
        let frames = 2;
        let fd = db
            .get_base(crate::combat::MoveId::Grapple)
            .anim_frame_duration(frames);
        Animation::new("Grapple", 50, frames, fd, false)
    }
}
