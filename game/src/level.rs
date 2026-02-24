/**--------------------------------------------------------------------------------
*!  Infinite level generation with platforms and obstacles.
*?  Level: The Gym - A handcrafted tutorial level to test core mechanics:
*--------------------------------------------------------------------------------**/
use crate::enemy::EnemyType;
use engine::{AABB, Vec2};

pub struct Platform {
    pub aabb: AABB,
    pub platform_type: PlatformType,
}

#[derive(Clone, Copy, PartialEq)]
pub enum PlatformType {
    Floor,
    Crate,
    OneWay,
    Wall,
}

impl Platform {
    pub fn new(center: Vec2, size: Vec2, platform_type: PlatformType) -> Self {
        Self {
            aabb: AABB::new(center, size),
            platform_type,
        }
    }
}

pub struct GrappleNode {
    pub position: Vec2,
    pub radius: f32,
}

impl GrappleNode {
    pub fn new(position: Vec2, radius: f32) -> Self {
        Self { position, radius }
    }
}

//? Level manager that holds platforms and handles procedural generation
//? and cleanup as player moves.
pub struct Level {
    pub platforms: Vec<Platform>,
    pub grapple_nodes: Vec<GrappleNode>,
    screen_height: f32,
    pub player_spawn: Vec2,
    pub enemy_spawns: Vec<(Vec2, EnemyType)>,
    pub exit_spawn: Vec2,
    pub death_y_threshold: f32,
}

//? A static level to test core mechanics.
impl Level {
    pub fn new(_screen_width: f32, screen_height: f32) -> Self {
        let level_data = {
            #[cfg(not(target_arch = "wasm32"))]
            {
                std::fs::read_to_string("game/assets/level/world.txt")
                    .unwrap_or_else(|_| include_str!("../assets/level/world.txt").to_string())
            }
            #[cfg(target_arch = "wasm32")]
            {
                let window = web_sys::window().unwrap();
                let storage = window.local_storage().unwrap().unwrap();
                if let Ok(Some(saved)) = storage.get_item("world.txt") {
                    saved
                } else {
                    include_str!("../assets/level/world.txt").to_string()
                }
            }
        };

        let mut level = Self {
            platforms: Vec::new(),
            grapple_nodes: Vec::new(),
            player_spawn: Vec2::new(100.0, 100.0),
            enemy_spawns: Vec::new(),
            exit_spawn: Vec2::new(0.0, 0.0),
            screen_height,
            death_y_threshold: 0.0,
        };

        level.reload_from_str(&level_data, screen_height);
        level
    }

    pub fn reload_from_str(&mut self, level_data: &str, screen_height: f32) {
        self.platforms.clear();
        self.grapple_nodes.clear();
        self.enemy_spawns.clear();

        let tile_size = 16.0;
        let half_tile = tile_size / 2.0;
        let total_rows = level_data.lines().count();

        //? Parse the ASCII grid
        for (row, line) in level_data.lines().enumerate() {
            for (col, character) in line.chars().enumerate() {
                let x = (col as f32 * tile_size) + half_tile;
                //? Invert Y so the bottom-most row aligns with the screen floor.
                //? Row 0 (top of text) maps to the highest pixel; last row maps to screen_height.
                let y = screen_height - ((total_rows - row) as f32 * tile_size) + half_tile;
                let center = Vec2::new(x, y);

                match character {
                    '#' => self.platforms.push(Platform::new(
                        center,
                        Vec2::new(tile_size, tile_size),
                        PlatformType::Wall,
                    )),
                    '=' => self.platforms.push(Platform::new(
                        center,
                        Vec2::new(tile_size, tile_size),
                        PlatformType::Floor,
                    )),
                    '_' => self.platforms.push(Platform::new(
                        center,
                        Vec2::new(tile_size, 4.0), //* Make one-way platforms thin
                        PlatformType::OneWay,
                    )),
                    '*' => self.grapple_nodes.push(GrappleNode::new(center, 4.0)),
                    '@' => self.player_spawn = Vec2::new(center.x, center.y - tile_size),
                    'E' | 'G' => self
                        .enemy_spawns
                        .push((Vec2::new(center.x, center.y - tile_size), EnemyType::Grunt)),
                    'S' => self
                        .enemy_spawns
                        .push((Vec2::new(center.x, center.y - tile_size), EnemyType::Sniper)),
                    'R' => self
                        .enemy_spawns
                        .push((Vec2::new(center.x, center.y - tile_size), EnemyType::Ronin)),
                    'O' => self.exit_spawn = center,
                    '.' => {} //* Empty air, do nothing
                    _ => {}   //* Ignore any unknown characters
                }
            }
        }

        //? Death threshold: 50px below the lowest platform bottom edge.
        self.death_y_threshold = self
            .platforms
            .iter()
            .map(|p| p.aabb.center.y + p.aabb.size.y * 0.5)
            .fold(f32::NEG_INFINITY, f32::max)
            + 50.0;

        self.screen_height = screen_height;
    }

    //? Update level (handles screen resize)
    pub fn update(&mut self, _player_x: f32, _screen_width: f32, screen_height: f32) {
        if (self.screen_height - screen_height).abs() > 1.0 {
            let dy = (screen_height - 13.0) - (self.screen_height - 13.0);
            for platform in &mut self.platforms {
                platform.aabb.center.y += dy;
            }
            for node in &mut self.grapple_nodes {
                node.position.y += dy;
            }
            self.player_spawn.y += dy;
            self.exit_spawn.y += dy;
            for spawn in &mut self.enemy_spawns {
                spawn.0.y += dy;
            }
            self.death_y_threshold += dy;
            self.screen_height = screen_height;

            //* Level 1 is static - no procedural generation or cleanup needed
            //* but this is where it would go for an infinite level.
        }
    }

    pub fn platform_color(platform_type: PlatformType) -> [f32; 4] {
        match platform_type {
            PlatformType::Floor | PlatformType::Crate => [0.0, 0.0, 0.0, 1.0],
            PlatformType::Wall => [0.2, 0.2, 0.2, 1.0],
            PlatformType::OneWay => [0.4, 0.4, 0.6, 0.8],
        }
    }

    //? Collect wall-only platform AABBs (for wall-grab filtering).
    pub fn wall_aabbs(&self) -> Vec<AABB> {
        self.platforms
            .iter()
            .filter(|p| p.platform_type == PlatformType::Wall)
            .map(|p| p.aabb)
            .collect()
    }

    //? Collect solid platform AABBs (everything except OneWay).
    pub fn solid_aabbs(&self) -> Vec<AABB> {
        self.platforms
            .iter()
            .filter(|p| p.platform_type != PlatformType::OneWay)
            .map(|p| p.aabb)
            .collect()
    }

    //? Collect one-way platform AABBs.
    pub fn one_way_aabbs(&self) -> Vec<AABB> {
        self.platforms
            .iter()
            .filter(|p| p.platform_type == PlatformType::OneWay)
            .map(|p| p.aabb)
            .collect()
    }

    //? Find the nearest grapple node within range of a position.
    pub fn find_nearest_grapple_node(&self, pos: Vec2, max_range: f32) -> Option<Vec2> {
        self.grapple_nodes
            .iter()
            .map(|node| (node.position, (node.position - pos).length()))
            .filter(|&(_, dist)| dist <= max_range)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(position, _)| position)
    }
}
