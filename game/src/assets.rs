/**----------------------------------------------------------------------
*!  Game assets - spritesheet for animation.
*?  Each sprite sheet is a horizontal strip (all frames in one row).
*----------------------------------------------------------------------**/
use crate::anim::{Animation, AssetKey};

//? Player sprite dimensions (all sprites are 120x90)
pub const FRAME_WIDTH: f32 = 120.0;
pub const FRAME_HEIGHT: f32 = 90.0;

//? Player animation definitions
pub struct KnightAnimations;

//? Create all animations for the character
impl KnightAnimations {
    pub fn create_all() -> Vec<Animation> {
        vec![
            Self::idle(),
            Self::walk(),
            Self::run(),
            Self::jump(),
            Self::fall(),
            Self::attack_1(),
            Self::attack_2(),
            Self::attack_3(),
            Self::block(),
            Self::roll(),
        ]
    }
    pub fn idle() -> Animation {
        Animation::new("Idle", AssetKey::Idle, 4, 0.1, true)
    }
    pub fn walk() -> Animation {
        Animation::new("Walk", AssetKey::Run, 8, 0.15, true)
    }
    pub fn run() -> Animation {
        Animation::new("Run", AssetKey::Run, 8, 0.1, true)
    }
    pub fn jump() -> Animation {
        Animation::new("Jump", AssetKey::Jump, 2, 0.1, false)
    }
    pub fn fall() -> Animation {
        Animation::new("Fall", AssetKey::Fall, 1, 0.1, true)
    }
    pub fn attack_1() -> Animation {
        Animation::new_with_range("Attack1", AssetKey::Attack, 0, 5, 0.08, false)
    }
    pub fn attack_2() -> Animation {
        Animation::new_with_range("Attack2", AssetKey::Attack, 6, 10, 0.08, false)
    }
    pub fn attack_3() -> Animation {
        Animation::new_with_range("Attack3", AssetKey::Attack, 11, 22, 0.08, false)
    }
    pub fn block() -> Animation {
        Animation::new("Block", AssetKey::Block, 2, 0.15, false)
    }
    pub fn roll() -> Animation {
        Animation::new("Roll", AssetKey::Roll, 11, 0.05, false)
    }
}
