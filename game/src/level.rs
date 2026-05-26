/**--------------------------------------------------------------------------------
*!  Infinite level generation with platforms and obstacles.
*?  Level: The Gym - A handcrafted tutorial level to test core mechanics:
*--------------------------------------------------------------------------------**/
use crate::enemy::EnemyType;
use engine::{AABB, SpatialGrid, Vec2};

const TILE_SIZE: f32 = 16.0;
const BROADPHASE_CELL_SIZE: f32 = 64.0;

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

fn approx_eq(a: f32, b: f32) -> bool {
    (a - b).abs() <= 0.01
}

fn aabb_from_min_max(min: Vec2, max: Vec2) -> AABB {
    AABB::from_top_left(min, max - min)
}

fn same_y_band(a: &AABB, b: &AABB) -> bool {
    approx_eq(a.min().y, b.min().y) && approx_eq(a.max().y, b.max().y)
}

fn same_x_span(a: &AABB, b: &AABB) -> bool {
    approx_eq(a.min().x, b.min().x) && approx_eq(a.max().x, b.max().x)
}

fn merge_aabbs(mut aabbs: Vec<AABB>) -> Vec<AABB> {
    if aabbs.is_empty() {
        return aabbs;
    }

    aabbs.sort_by(|a, b| {
        a.min()
            .y
            .partial_cmp(&b.min().y)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                a.min()
                    .x
                    .partial_cmp(&b.min().x)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    let mut horizontal = Vec::with_capacity(aabbs.len());
    for aabb in aabbs {
        if let Some(last) = horizontal.last_mut()
            && same_y_band(last, &aabb)
            && approx_eq(last.max().x, aabb.min().x)
        {
            let min = last.min();
            let max = Vec2::new(aabb.max().x, last.max().y);
            *last = aabb_from_min_max(min, max);
            continue;
        }
        horizontal.push(aabb);
    }

    horizontal.sort_by(|a, b| {
        a.min()
            .x
            .partial_cmp(&b.min().x)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                a.max()
                    .x
                    .partial_cmp(&b.max().x)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                a.min()
                    .y
                    .partial_cmp(&b.min().y)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    let mut merged = Vec::with_capacity(horizontal.len());
    for aabb in horizontal {
        if let Some(last) = merged.last_mut()
            && same_x_span(last, &aabb)
            && approx_eq(last.max().y, aabb.min().y)
        {
            let min = last.min();
            let max = Vec2::new(last.max().x, aabb.max().y);
            *last = aabb_from_min_max(min, max);
            continue;
        }
        merged.push(aabb);
    }

    merged
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelCollisionStats {
    pub raw_platforms: usize,
    pub raw_solid: usize,
    pub raw_one_way: usize,
    pub raw_wall: usize,
    pub merged_solid: usize,
    pub merged_one_way: usize,
    pub merged_wall: usize,
    pub merged_all: usize,
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
    solid_grid: SpatialGrid,
    wall_grid: SpatialGrid,
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
            solid_grid: SpatialGrid::new(BROADPHASE_CELL_SIZE),
            wall_grid: SpatialGrid::new(BROADPHASE_CELL_SIZE),
        };

        level.reload_from_str(&level_data, screen_height);
        level
    }

    pub fn reload_from_str(&mut self, level_data: &str, screen_height: f32) {
        self.platforms.clear();
        self.grapple_nodes.clear();
        self.enemy_spawns.clear();

        let half_tile = TILE_SIZE / 2.0;
        let total_rows = level_data.lines().count();

        //? Parse the ASCII grid
        for (row, line) in level_data.lines().enumerate() {
            for (col, character) in line.chars().enumerate() {
                let x = (col as f32 * TILE_SIZE) + half_tile;
                //? Invert Y so the bottom-most row aligns with the screen floor.
                //? Row 0 (top of text) maps to the highest pixel; last row maps to screen_height.
                let y = screen_height - ((total_rows - row) as f32 * TILE_SIZE) + half_tile;
                let center = Vec2::new(x, y);

                match character {
                    '#' => self.platforms.push(Platform::new(
                        center,
                        Vec2::new(TILE_SIZE, TILE_SIZE),
                        PlatformType::Wall,
                    )),
                    '=' => self.platforms.push(Platform::new(
                        center,
                        Vec2::new(TILE_SIZE, TILE_SIZE),
                        PlatformType::Floor,
                    )),
                    '_' => self.platforms.push(Platform::new(
                        center,
                        Vec2::new(TILE_SIZE, 4.0), //* Make one-way platforms thin
                        PlatformType::OneWay,
                    )),
                    '*' => self.grapple_nodes.push(GrappleNode::new(center, 4.0)),
                    '@' => self.player_spawn = Vec2::new(center.x, center.y - TILE_SIZE),
                    'E' | 'G' => self
                        .enemy_spawns
                        .push((Vec2::new(center.x, center.y - TILE_SIZE), EnemyType::Grunt)),
                    'S' => self
                        .enemy_spawns
                        .push((Vec2::new(center.x, center.y - TILE_SIZE), EnemyType::Sniper)),
                    'R' => self
                        .enemy_spawns
                        .push((Vec2::new(center.x, center.y - TILE_SIZE), EnemyType::Ronin)),
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
        self.log_collision_stats();
    }

    fn rebuild_caches(&mut self) {
        let solid = self
            .platforms
            .iter()
            .filter(|p| p.platform_type != PlatformType::OneWay)
            .map(|p| p.aabb)
            .collect();
        let one_way = self
            .platforms
            .iter()
            .filter(|p| p.platform_type == PlatformType::OneWay)
            .map(|p| p.aabb)
            .collect();
        let wall = self
            .platforms
            .iter()
            .filter(|p| p.platform_type == PlatformType::Wall)
            .map(|p| p.aabb)
            .collect();

        self.cached_solid = merge_aabbs(solid);
        self.cached_one_way = merge_aabbs(one_way);
        self.cached_wall = merge_aabbs(wall);
        self.cached_all.clear();
        self.cached_all.extend(self.cached_solid.iter().copied());
        self.cached_all.extend(self.cached_one_way.iter().copied());
        self.solid_grid.rebuild(&self.cached_solid);
        self.wall_grid.rebuild(&self.cached_wall);
    }

    fn log_collision_stats(&self) {
        let s = self.collision_stats();
        log::info!(
            "Level collision cache: raw platforms={} solid={} walls={} one-way={} -> merged solid={} walls={} one-way={} all={}",
            s.raw_platforms,
            s.raw_solid,
            s.raw_wall,
            s.raw_one_way,
            s.merged_solid,
            s.merged_wall,
            s.merged_one_way,
            s.merged_all,
        );
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
            PlatformType::Floor | PlatformType::Crate => [0.045, 0.070, 0.060, 1.0],
            PlatformType::Wall => [0.035, 0.052, 0.050, 1.0],
            PlatformType::OneWay => [0.17, 0.27, 0.22, 1.0],
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

    pub fn collision_stats(&self) -> LevelCollisionStats {
        let raw_one_way = self
            .platforms
            .iter()
            .filter(|p| p.platform_type == PlatformType::OneWay)
            .count();
        let raw_wall = self
            .platforms
            .iter()
            .filter(|p| p.platform_type == PlatformType::Wall)
            .count();
        let raw_solid = self.platforms.len().saturating_sub(raw_one_way);

        LevelCollisionStats {
            raw_platforms: self.platforms.len(),
            raw_solid,
            raw_one_way,
            raw_wall,
            merged_solid: self.cached_solid.len(),
            merged_one_way: self.cached_one_way.len(),
            merged_wall: self.cached_wall.len(),
            merged_all: self.cached_all.len(),
        }
    }

    pub fn enemy_collision_parts(&mut self) -> (&[AABB], &[AABB], &mut SpatialGrid) {
        (&self.cached_all, &self.cached_wall, &mut self.wall_grid)
    }

    pub fn solid_broadphase_parts(&mut self) -> (&[AABB], &mut SpatialGrid) {
        (&self.cached_solid, &mut self.solid_grid)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_merges_horizontal_floor_collision() {
        let mut level = Level::new(640.0, 360.0);
        level.reload_from_str("....\n====", 360.0);

        assert_eq!(level.platforms.len(), 4);
        assert_eq!(level.solid_aabbs().len(), 1);
        assert_eq!(level.solid_aabbs()[0].size, Vec2::new(64.0, 16.0));
    }

    #[test]
    fn level_merges_vertical_wall_collision() {
        let mut level = Level::new(640.0, 360.0);
        level.reload_from_str("#...\n#...\n#...", 360.0);

        assert_eq!(level.platforms.len(), 3);
        assert_eq!(level.wall_aabbs().len(), 1);
        assert_eq!(level.wall_aabbs()[0].size, Vec2::new(16.0, 48.0));
    }

    #[test]
    fn level_keeps_one_way_separate_from_solids() {
        let mut level = Level::new(640.0, 360.0);
        level.reload_from_str("____\n====", 360.0);

        assert_eq!(level.solid_aabbs().len(), 1);
        assert_eq!(level.one_way_aabbs().len(), 1);
        assert_eq!(level.all_aabbs().len(), 2);
    }

    #[test]
    fn level_collision_stats_report_raw_and_merged_counts() {
        let mut level = Level::new(640.0, 360.0);
        level.reload_from_str("##..\n____\n====", 360.0);

        let stats = level.collision_stats();
        assert_eq!(stats.raw_platforms, 10);
        assert_eq!(stats.raw_solid, 6);
        assert_eq!(stats.raw_one_way, 4);
        assert_eq!(stats.raw_wall, 2);
        assert_eq!(stats.merged_solid, 2);
        assert_eq!(stats.merged_one_way, 1);
        assert_eq!(stats.merged_wall, 1);
    }
}
