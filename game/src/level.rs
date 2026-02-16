/**--------------------------------------------------------------------------------
*!  Infinite level generation with platforms and obstacles.
*?  Level: The Gym - A handcrafted tutorial level to test core mechanics:
*--------------------------------------------------------------------------------**/
use engine::{AABB, Vec2};

pub struct Platform {
    pub aabb: AABB,
    pub platform_type: PlatformType,
}

#[derive(Clone, Copy, PartialEq)]
pub enum PlatformType {
    Floor,
    Crate,
    //* Pit could be represented by gaps between platforms
}

impl Platform {
    pub fn new(center: Vec2, size: Vec2, platform_type: PlatformType) -> Self {
        Self {
            aabb: AABB::new(center, size),
            platform_type,
        }
    }
}

//? Level manager that holds platforms and handles procedural generation
//? and cleanup as player moves.
pub struct Level {
    pub platforms: Vec<Platform>,
    screen_height: f32,
}

//? Level 1: The Gym - A static level to test core mechanics.
impl Level {
    pub fn new(_screen_width: f32, screen_height: f32) -> Self {
        let floor_y = screen_height - 50.0;
        let platforms = Self::build_level_1(floor_y);

        Self {
            platforms,
            screen_height,
        }
    }

    fn build_level_1(floor_y: f32) -> Vec<Platform> {
        let mut platforms = Vec::new();
        let platform_height = 100.0;
        let thin_platform_height = 40.0;
        let crate_size = 60.0;
        let crate_y_base = floor_y - 100.0; //* Base Y for crates (on top of floor)
        let stack_x = 4000.0;
        let floor_top = floor_y - platform_height / 2.0;
        let tower_x = 4150.0;

        //? Spawn zone
        platforms.push(Platform::new(
            Vec2::new(300.0, floor_y),
            Vec2::new(600.0, platform_height),
            PlatformType::Floor,
        ));

        //? The Jump Gap Test (150px gap)
        //* Ends at 600. Gap 150 -> Start 750. Width 400 -> Center 950.
        platforms.push(Platform::new(
            Vec2::new(950.0, floor_y),
            Vec2::new(400.0, platform_height),
            PlatformType::Floor,
        ));

        //? The "Commitment" Gap text (250px - requires momentum)
        //* Ends at 1150. Gap 250 -> Start 1400. Width 400 -> Center 1600.
        platforms.push(Platform::new(
            Vec2::new(1600.0, floor_y),
            Vec2::new(400.0, platform_height),
            PlatformType::Floor,
        ));

        //? The Staircase (Verticality & Air Control) (120px step up)
        platforms.push(Platform::new(
            Vec2::new(2000.0, floor_y - 120.0),
            Vec2::new(200.0, thin_platform_height),
            PlatformType::Floor,
        ));

        platforms.push(Platform::new(
            Vec2::new(2300.0, floor_y - 240.0),
            Vec2::new(200.0, thin_platform_height),
            PlatformType::Floor,
        ));

        platforms.push(Platform::new(
            Vec2::new(2700.0, floor_y - 360.0),
            Vec2::new(400.0, thin_platform_height),
            PlatformType::Floor,
        ));

        //? The "Crate" Precision Section - Floating crates to hop across
        platforms.push(Platform::new(
            Vec2::new(3100.0, crate_y_base),
            Vec2::new(crate_size, crate_size),
            PlatformType::Crate,
        ));

        platforms.push(Platform::new(
            Vec2::new(3300.0, crate_y_base - 100.0),
            Vec2::new(crate_size, crate_size),
            PlatformType::Crate,
        ));

        platforms.push(Platform::new(
            Vec2::new(3500.0, crate_y_base),
            Vec2::new(crate_size, crate_size),
            PlatformType::Crate,
        ));

        //? Landing Pad
        platforms.push(Platform::new(
            Vec2::new(3900.0, floor_y),
            Vec2::new(600.0, platform_height),
            PlatformType::Floor,
        ));

        //? Obstacle Stack (Test collision/jumping over)
        platforms.push(Platform::new(
            Vec2::new(stack_x, floor_top - crate_size / 2.0),
            Vec2::new(crate_size, crate_size),
            PlatformType::Crate,
        ));

        platforms.push(Platform::new(
            Vec2::new(stack_x, floor_top - crate_size * 1.5),
            Vec2::new(crate_size, crate_size),
            PlatformType::Crate,
        ));

        //? Tower of crates to test verticality and wall-jumping
        platforms.push(Platform::new(
            Vec2::new(tower_x, floor_y),
            Vec2::new(400.0, platform_height),
            PlatformType::Floor,
        ));

        //* Left
        platforms.push(Platform::new(
            Vec2::new(tower_x - 150.0, floor_y - 140.0),
            Vec2::new(150.0, thin_platform_height),
            PlatformType::Floor,
        ));

        //* Right
        platforms.push(Platform::new(
            Vec2::new(tower_x + 150.0, floor_y - 280.0),
            Vec2::new(150.0, thin_platform_height),
            PlatformType::Floor,
        ));

        //* Center/High
        platforms.push(Platform::new(
            Vec2::new(tower_x, floor_y - 420.0),
            Vec2::new(150.0, thin_platform_height),
            PlatformType::Floor,
        ));

        //? The Sky Bridge (High altitude traversal)
        platforms.push(Platform::new(
            Vec2::new(tower_x + 400.0, floor_y - 420.0),
            Vec2::new(300.0, thin_platform_height),
            PlatformType::Floor,
        ));

        //? The Descent
        platforms.push(Platform::new(
            Vec2::new(tower_x + 800.0, floor_y),
            Vec2::new(600.0, platform_height),
            PlatformType::Floor,
        ));

        //? Dummy platforms to test infinite generation
        //? and cleanup as player moves right
        platforms.push(Platform::new(
            Vec2::new(5000.0, floor_y),
            Vec2::new(500.0, platform_height),
            PlatformType::Floor,
        ));

        platforms.push(Platform::new(
            Vec2::new(5400.0, floor_y - platform_height / 2.0 - crate_size / 2.0),
            Vec2::new(crate_size, crate_size),
            PlatformType::Crate,
        ));

        platforms.push(Platform::new(
            Vec2::new(5900.0, floor_y),
            Vec2::new(300.0, thin_platform_height),
            PlatformType::Floor,
        ));

        platforms
    }

    //? Update level (handles screen resize)
    pub fn update(&mut self, _player_x: f32, _screen_width: f32, screen_height: f32) {
        //* If screen height changed, shift all existing platforms to match.
        if (self.screen_height - screen_height).abs() > 1.0 {
            let dy = (screen_height - 50.0) - (self.screen_height - 50.0);
            for platform in &mut self.platforms {
                platform.aabb.center.y += dy;
            }
            self.screen_height = screen_height;
        }

        //* Level 1 is static - no procedural generation or cleanup
    }

    //? Get color for platform rendering
    pub fn platform_color(platform_type: PlatformType) -> [f32; 4] {
        match platform_type {
            PlatformType::Floor => [0.0, 0.0, 0.0, 1.0],
            PlatformType::Crate => [0.0, 0.0, 0.0, 1.0],
        }
    }
}
