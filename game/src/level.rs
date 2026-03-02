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
    pub ceiling_y_threshold: f32,
    pub death_y_threshold: f32,
    cached_solid: Vec<AABB>,
    cached_one_way: Vec<AABB>,
    cached_wall: Vec<AABB>,
    cached_all: Vec<AABB>,
}

//? A static level to test core mechanics.
impl Level {
    //? Load level text from disk (native) or localStorage (WASM),
    //? falling back to the embedded default.
    pub fn load_level_text() -> String {
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::fs::read_to_string("game/assets/level/world.txt")
                .unwrap_or_else(|_| include_str!("../assets/level/world.txt").to_string())
        }
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::window()
                .and_then(|w| w.local_storage().ok().flatten())
                .and_then(|s| s.get_item("world.txt").ok().flatten())
                .unwrap_or_else(|| include_str!("../assets/level/world.txt").to_string())
        }
    }

    pub fn new(_screen_width: f32, screen_height: f32) -> Self {
        let level_data = Self::load_level_text();

        let mut level = Self {
            platforms: Vec::new(),
            grapple_nodes: Vec::new(),
            player_spawn: Vec2::new(100.0, 100.0),
            enemy_spawns: Vec::new(),
            exit_spawn: Vec2::new(0.0, 0.0),
            ceiling_y_threshold: 0.0,
            screen_height,
            death_y_threshold: 0.0,
            cached_solid: Vec::new(),
            cached_one_way: Vec::new(),
            cached_wall: Vec::new(),
            cached_all: Vec::new(),
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

        //? Vertical bounds used by camera clamping.
        //? ceiling_y_threshold = highest platform top edge.
        //? death_y_threshold = 50px below the lowest platform bottom edge.
        if self.platforms.is_empty() {
            self.ceiling_y_threshold = 0.0;
            self.death_y_threshold = screen_height + 50.0;
        } else {
            self.ceiling_y_threshold = self
                .platforms
                .iter()
                .map(|p| p.aabb.center.y - p.aabb.size.y * 0.5)
                .fold(f32::INFINITY, f32::min);
            self.death_y_threshold = self
                .platforms
                .iter()
                .map(|p| p.aabb.center.y + p.aabb.size.y * 0.5)
                .fold(f32::NEG_INFINITY, f32::max)
                + 50.0;
        }

        self.screen_height = screen_height;
        self.rebuild_caches();
    }

    fn rebuild_caches(&mut self) {
        self.cached_solid.clear();
        self.cached_one_way.clear();
        self.cached_wall.clear();
        self.cached_all.clear();
        for p in &self.platforms {
            self.cached_all.push(p.aabb);
            match p.platform_type {
                PlatformType::OneWay => self.cached_one_way.push(p.aabb),
                PlatformType::Wall => {
                    self.cached_solid.push(p.aabb);
                    self.cached_wall.push(p.aabb);
                }
                _ => self.cached_solid.push(p.aabb),
            }
        }
    }

    //? Returns the vertical delta applied to world geometry, if a resize occurred.
    pub fn update(&mut self, _player_x: f32, _screen_width: f32, screen_height: f32) -> f32 {
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
            self.ceiling_y_threshold += dy;
            self.death_y_threshold += dy;
            self.screen_height = screen_height;
            self.rebuild_caches();
            dy
        } else {
            0.0
        }
    }

    pub fn platform_color(platform_type: PlatformType) -> [f32; 4] {
        match platform_type {
            PlatformType::Floor | PlatformType::Crate => [0.055, 0.055, 0.41, 1.0], //* #0E0E68
            PlatformType::Wall => [0.055, 0.055, 0.41, 1.0],                        //* #0E0E68
            PlatformType::OneWay => [0.137, 0.282, 0.761, 1.0],                     //* #234DC2
        }
    }

    pub fn wall_aabbs(&self) -> &[AABB] {
        &self.cached_wall
    }

    pub fn solid_aabbs(&self) -> &[AABB] {
        &self.cached_solid
    }

    pub fn one_way_aabbs(&self) -> &[AABB] {
        &self.cached_one_way
    }

    pub fn all_aabbs(&self) -> &[AABB] {
        &self.cached_all
    }

    pub fn camera_y_bounds(&self, screen_height: f32) -> (f32, f32) {
        let camera_y_min = self.ceiling_y_threshold;
        let level_floor_y = self.death_y_threshold - 50.0;
        let camera_y_max = level_floor_y - screen_height;
        (camera_y_min, camera_y_max)
    }

    //? Clamp camera_y to level vertical bounds, supporting negative camera offsets
    //? for tall maps whose top extends above y=0.
    pub fn clamp_camera_y(&self, camera_y: f32, screen_height: f32) -> f32 {
        let (camera_y_min, camera_y_max) = self.camera_y_bounds(screen_height);
        if camera_y_min <= camera_y_max {
            camera_y.clamp(camera_y_min, camera_y_max)
        } else {
            camera_y_min
        }
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
