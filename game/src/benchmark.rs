use crate::input::JourneyAction;
use engine::{AABB, BloomSettings, Context, FixedTime, Vec2, egui, ui as journey_ui};

const SPRITE_LIMIT: usize = 65_536;
const MAX_PARTICLES: usize = SPRITE_LIMIT;
const SPAWN_BATCH: usize = 128;
const LARGE_SPAWN_BATCH: usize = 2048;
const MAX_SPAWN_PER_FIXED_STEP: usize = 300;
const PARTICLE_SIZE: f32 = 4.0;
const GRID_CELL_SIZE: f32 = PARTICLE_SIZE;
const BURST_MIN_RADIUS: f32 = 12.0;
const BURST_MAX_RADIUS: f32 = 92.0;
const STABLE_FPS_INTERVAL: f32 = 0.35;

#[derive(Debug, Clone, Copy)]
struct Particle {
    position: Vec2,
    velocity: Vec2,
    color: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PhysicsMode {
    Aabb,
    Fast,
}

pub struct BenchmarkState {
    particles: Vec<Particle>,
    grid_heads: Vec<Option<usize>>,
    grid_next: Vec<Option<usize>>,
    physics_enabled: bool,
    physics_mode: PhysicsMode,
    rng: u32,
    physics_checks: u32,
    physics_hits: u32,
    wall_hits: u32,
    pending_spawn: usize,
    total_spawned: usize,
    stable_fps: f32,
    stable_fps_timer: f32,
}

impl BenchmarkState {
    pub fn new() -> Self {
        Self {
            particles: Vec::with_capacity(4_096),
            grid_heads: Vec::new(),
            grid_next: Vec::new(),
            physics_enabled: false,
            physics_mode: PhysicsMode::Aabb,
            rng: 0x7a5f_123d,
            physics_checks: 0,
            physics_hits: 0,
            wall_hits: 0,
            pending_spawn: 0,
            total_spawned: 0,
            stable_fps: 0.0,
            stable_fps_timer: 0.0,
        }
    }

    pub fn update(&mut self, ctx: &mut Context<JourneyAction>) {
        ctx.target_fps = 60;
        self.update_stable_fps(ctx);

        if ctx.input.is_action_just_pressed(JourneyAction::Jump) {
            self.queue_spawn(SPAWN_BATCH);
        }
    }

    pub fn fixed_update(&mut self, ctx: &mut Context<JourneyAction>, fixed_time: &FixedTime) {
        self.drain_spawn_queue(ctx.screen_center());
        self.step_particles(fixed_time.fixed_dt, ctx.screen_width, ctx.screen_height);
    }

    pub fn render(&self, ctx: &mut Context<JourneyAction>) {
        ctx.camera_offset_x = 0.0;
        ctx.camera_offset_y = 0.0;

        let half = Vec2::splat(PARTICLE_SIZE * 0.5);
        let size = Vec2::splat(PARTICLE_SIZE);
        for particle in &self.particles {
            ctx.draw_rect(particle.position - half, size, particle.color);
        }
    }

    pub fn ui(&mut self, egui_ctx: &egui::Context, ctx: &mut Context<JourneyAction>) {
        ctx.override_bloom(BloomSettings {
            enabled: true,
            threshold: 0.58,
            intensity: 0.28,
            radius: 2.0,
        });
        Self::paint_menu_grid(egui_ctx);
        self.show_stats_panel(egui_ctx, ctx);
        self.show_spawn_button(egui_ctx);
    }

    fn paint_menu_grid(egui_ctx: &egui::Context) {
        let t = journey_ui::theme();
        let rect = egui_ctx.viewport_rect();
        let painter = egui_ctx.layer_painter(egui::LayerId::new(
            egui::Order::Background,
            egui::Id::new("benchmark_grid"),
        ));

        let top = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), 36.0));
        painter.rect_filled(top, 0.0, t.bg_deep);
        painter.line_segment(
            [top.left_bottom(), top.right_bottom()],
            egui::Stroke::new(1.0, t.stroke_soft),
        );

        let step = 64.0;
        let mut x = rect.left();
        while x <= rect.right() {
            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 8)),
            );
            x += step;
        }

        let mut y = rect.top();
        while y <= rect.bottom() {
            painter.line_segment(
                [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 7)),
            );
            y += step;
        }
    }

    fn show_stats_panel(&mut self, egui_ctx: &egui::Context, ctx: &mut Context<JourneyAction>) {
        let stats = ctx.perf();
        let t = journey_ui::theme();
        let status = if stats.avg_frame_time_ms <= 16.67 && !stats.hit_fixed_step_cap {
            "PASS 60"
        } else {
            "BELOW 60"
        };
        let load = self.particles.len() as f32 / MAX_PARTICLES as f32 * 100.0;

        egui::Area::new(egui::Id::new("benchmark_panel"))
            .anchor(egui::Align2::LEFT_TOP, [12.0, 12.0])
            .show(egui_ctx, |ui| {
                journey_ui::panel_frame().show(ui, |ui| {
                    ui.set_min_width(260.0);
                    ui.label(journey_ui::title("BENCHMARK", 18.0));
                    ui.label(
                        egui::RichText::new(format!("FPS: {:.0}", self.stable_fps))
                            .font(egui::FontId::new(13.0, egui::FontFamily::Monospace))
                            .color(t.accent),
                    );
                    journey_ui::divider(ui);
                    journey_ui::key_value(
                        ui,
                        "Particles",
                        format!("{}", self.particles.len()),
                        1.0,
                    );
                    journey_ui::key_value(ui, "Load", format!("{load:.1}%"), 1.0);
                    journey_ui::key_value(ui, "Walls", format!("{}", self.wall_hits), 1.0);
                    journey_ui::key_value(
                        ui,
                        "Physics",
                        if self.physics_enabled { "On" } else { "Off" },
                        1.0,
                    );
                    if self.physics_enabled {
                        journey_ui::key_value(ui, "Mode", self.physics_mode_label(), 1.0);
                        journey_ui::key_value(
                            ui,
                            "Checks",
                            format!("{}", self.physics_checks),
                            1.0,
                        );
                        journey_ui::key_value(ui, "Hits", format!("{}", self.physics_hits), 1.0);
                    }
                    journey_ui::key_value(ui, "Queued", format!("{}", self.pending_spawn), 1.0);
                    journey_ui::key_value(ui, "Spawned", format!("{}", self.total_spawned), 1.0);
                    journey_ui::key_value(ui, "Status", status, 1.0);
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("PARTICLES")
                            .font(egui::FontId::new(10.0, egui::FontFamily::Monospace))
                            .color(t.muted),
                    );
                    ui.horizontal(|ui| {
                        if ui.button(format!("-{LARGE_SPAWN_BATCH}")).clicked() {
                            self.remove_particles(LARGE_SPAWN_BATCH);
                        }
                        if ui.button(format!("-{SPAWN_BATCH}")).clicked() {
                            self.remove_particles(SPAWN_BATCH);
                        }
                        if ui
                            .add_enabled(
                                !self.is_full(),
                                egui::Button::new(format!("+{SPAWN_BATCH}")),
                            )
                            .clicked()
                        {
                            self.queue_spawn(SPAWN_BATCH);
                        }
                        if ui
                            .add_enabled(
                                !self.is_full(),
                                egui::Button::new(format!("+{LARGE_SPAWN_BATCH}")),
                            )
                            .clicked()
                        {
                            self.queue_spawn(LARGE_SPAWN_BATCH);
                        }
                        if ui.button("Clear").clicked() {
                            self.clear();
                        }
                    });
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("PHYSICS")
                            .font(egui::FontId::new(10.0, egui::FontFamily::Monospace))
                            .color(t.muted),
                    );
                    ui.horizontal(|ui| {
                        if ui.selectable_label(self.physics_enabled, "On").clicked() {
                            self.physics_enabled = true;
                        }
                        if ui.selectable_label(!self.physics_enabled, "Off").clicked() {
                            self.physics_enabled = false;
                        }
                    });
                    if self.physics_enabled {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if ui
                                .selectable_label(self.physics_mode == PhysicsMode::Aabb, "AABB")
                                .clicked()
                            {
                                self.physics_mode = PhysicsMode::Aabb;
                            }
                            if ui
                                .selectable_label(self.physics_mode == PhysicsMode::Fast, "Fast")
                                .clicked()
                            {
                                self.physics_mode = PhysicsMode::Fast;
                            }
                        });
                    }

                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(format!("SPACE +{SPAWN_BATCH}  ESC BACK"))
                            .font(egui::FontId::new(10.0, egui::FontFamily::Monospace))
                            .color(t.muted),
                    );
                });
            });
    }

    fn show_spawn_button(&mut self, egui_ctx: &egui::Context) {
        egui::Area::new(egui::Id::new("benchmark_spawn_button"))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(egui_ctx, |ui| {
                let button = egui::Button::new(
                    egui::RichText::new(format!("+{SPAWN_BATCH}"))
                        .font(egui::FontId::new(18.0, egui::FontFamily::Monospace)),
                );
                if ui
                    .add_enabled(!self.is_full(), button.min_size(egui::vec2(104.0, 44.0)))
                    .clicked()
                {
                    self.queue_spawn(SPAWN_BATCH);
                }
            });
    }

    fn queue_spawn(&mut self, count: usize) {
        let available = MAX_PARTICLES
            .saturating_sub(self.particles.len())
            .saturating_sub(self.pending_spawn);
        self.pending_spawn += count.min(available);
    }

    fn remove_particles(&mut self, count: usize) {
        let from_queue = count.min(self.pending_spawn);
        self.pending_spawn -= from_queue;

        let remaining = count - from_queue;
        if remaining == 0 {
            return;
        }

        let new_len = self.particles.len().saturating_sub(remaining);
        self.particles.truncate(new_len);
        self.grid_heads.clear();
        self.grid_next.clear();
    }

    fn drain_spawn_queue(&mut self, center: Vec2) {
        let count = self.pending_spawn.min(MAX_SPAWN_PER_FIXED_STEP);
        if count == 0 {
            return;
        }

        let spawned = self.spawn_burst(center, count);
        self.pending_spawn = self.pending_spawn.saturating_sub(spawned);
    }

    fn spawn_burst(&mut self, center: Vec2, count: usize) -> usize {
        let available = MAX_PARTICLES.saturating_sub(self.particles.len());
        let count = count.min(available);
        if count == 0 {
            return 0;
        }

        for i in 0..count {
            let angle = self.rand_unit() * std::f32::consts::TAU;
            let speed = 45.0 + self.rand_unit() * 180.0;
            let radius =
                BURST_MIN_RADIUS + self.rand_unit().sqrt() * (BURST_MAX_RADIUS - BURST_MIN_RADIUS);
            let jitter = Vec2::new(angle.cos(), angle.sin()) * radius;
            let color = self.particle_color(self.total_spawned + i);

            self.particles.push(Particle {
                position: center + jitter,
                velocity: Vec2::new(angle.cos(), angle.sin()) * speed,
                color,
            });
        }

        self.total_spawned += count;
        count
    }

    fn clear(&mut self) {
        self.particles.clear();
        self.grid_heads.clear();
        self.grid_next.clear();
        self.physics_checks = 0;
        self.physics_hits = 0;
        self.wall_hits = 0;
        self.pending_spawn = 0;
        self.total_spawned = 0;
    }

    fn step_particles(&mut self, dt: f32, width: f32, height: f32) {
        self.wall_hits = 0;
        self.physics_checks = 0;
        self.physics_hits = 0;

        let half = PARTICLE_SIZE * 0.5;
        for particle in &mut self.particles {
            particle.position += particle.velocity * dt;
            self.wall_hits += Self::bounce_walls(particle, width, height, half);
        }

        if self.physics_enabled {
            self.rebuild_grid(width, height);
            self.resolve_particle_physics(width, height);
        }

        for particle in &mut self.particles {
            self.wall_hits += Self::bounce_walls(particle, width, height, half);
        }
    }

    fn bounce_walls(particle: &mut Particle, width: f32, height: f32, half: f32) -> u32 {
        let mut hits = 0;
        if particle.position.x < half {
            particle.position.x = half;
            particle.velocity.x = particle.velocity.x.abs();
            hits += 1;
        } else if particle.position.x > width - half {
            particle.position.x = width - half;
            particle.velocity.x = -particle.velocity.x.abs();
            hits += 1;
        }

        if particle.position.y < half {
            particle.position.y = half;
            particle.velocity.y = particle.velocity.y.abs();
            hits += 1;
        } else if particle.position.y > height - half {
            particle.position.y = height - half;
            particle.velocity.y = -particle.velocity.y.abs();
            hits += 1;
        }
        hits
    }

    fn rebuild_grid(&mut self, width: f32, height: f32) {
        let (cols, rows) = self.grid_dims(width, height);
        let cell_count = cols * rows;
        self.grid_heads.clear();
        self.grid_heads.resize(cell_count, None);
        self.grid_next.clear();
        self.grid_next.resize(self.particles.len(), None);

        for (index, particle) in self.particles.iter().enumerate() {
            let cell = self.cell_index(particle.position, cols, rows);
            self.grid_next[index] = self.grid_heads[cell];
            self.grid_heads[cell] = Some(index);
        }
    }

    fn resolve_particle_physics(&mut self, width: f32, height: f32) {
        let (cols, rows) = self.grid_dims(width, height);
        let grid_heads = &self.grid_heads;
        let grid_next = &self.grid_next;
        let particles = &mut self.particles;
        let mut checks = 0u32;
        let mut hits = 0u32;

        for i in 0..particles.len() {
            let (cx, cy) = Self::cell_coords(particles[i].position, cols, rows);
            let min_x = cx.saturating_sub(1);
            let max_x = (cx + 1).min(cols - 1);
            let min_y = cy.saturating_sub(1);
            let max_y = (cy + 1).min(rows - 1);

            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let mut cursor = grid_heads[y * cols + x];
                    while let Some(j) = cursor {
                        cursor = grid_next[j];
                        if j <= i {
                            continue;
                        }

                        checks = checks.wrapping_add(1);
                        if Self::resolve_pair(particles, i, j, self.physics_mode) {
                            hits = hits.wrapping_add(1);
                        }
                    }
                }
            }
        }

        self.physics_checks = checks;
        self.physics_hits = hits;
    }

    fn resolve_pair(
        particles: &mut [Particle],
        i: usize,
        j: usize,
        physics_mode: PhysicsMode,
    ) -> bool {
        let (left, right) = particles.split_at_mut(j);
        let a = &mut left[i];
        let b = &mut right[0];

        let mtv = match physics_mode {
            PhysicsMode::Aabb => {
                let size = Vec2::splat(PARTICLE_SIZE);
                let aabb_a = AABB::new(a.position, size);
                let aabb_b = AABB::new(b.position, size);
                let Some(mtv) = AABB::resolve_collision(&aabb_a, &aabb_b) else {
                    return false;
                };
                mtv
            }
            PhysicsMode::Fast => {
                let delta = a.position - b.position;
                let overlap_x = PARTICLE_SIZE - delta.x.abs();
                let overlap_y = PARTICLE_SIZE - delta.y.abs();
                if overlap_x <= 0.0 || overlap_y <= 0.0 {
                    return false;
                }

                if overlap_x < overlap_y {
                    let sign = if delta.x >= 0.0 { 1.0 } else { -1.0 };
                    Vec2::new(overlap_x * sign, 0.0)
                } else {
                    let sign = if delta.y >= 0.0 { 1.0 } else { -1.0 };
                    Vec2::new(0.0, overlap_y * sign)
                }
            }
        };

        a.position += mtv * 0.5;
        b.position -= mtv * 0.5;

        if mtv.x.abs() > 0.0 {
            std::mem::swap(&mut a.velocity.x, &mut b.velocity.x);
        }
        if mtv.y.abs() > 0.0 {
            std::mem::swap(&mut a.velocity.y, &mut b.velocity.y);
        }

        true
    }

    fn grid_dims(&self, width: f32, height: f32) -> (usize, usize) {
        let cols = (width / GRID_CELL_SIZE).ceil().max(1.0) as usize;
        let rows = (height / GRID_CELL_SIZE).ceil().max(1.0) as usize;
        (cols, rows)
    }

    fn cell_index(&self, position: Vec2, cols: usize, rows: usize) -> usize {
        let (x, y) = Self::cell_coords(position, cols, rows);
        y * cols + x
    }

    fn cell_coords(position: Vec2, cols: usize, rows: usize) -> (usize, usize) {
        let x = (position.x / GRID_CELL_SIZE).floor() as isize;
        let y = (position.y / GRID_CELL_SIZE).floor() as isize;
        (
            x.clamp(0, cols as isize - 1) as usize,
            y.clamp(0, rows as isize - 1) as usize,
        )
    }

    fn is_full(&self) -> bool {
        self.particles.len() + self.pending_spawn >= MAX_PARTICLES
    }

    fn physics_mode_label(&self) -> &'static str {
        match self.physics_mode {
            PhysicsMode::Aabb => "AABB",
            PhysicsMode::Fast => "Fast",
        }
    }

    fn update_stable_fps(&mut self, ctx: &Context<JourneyAction>) {
        self.stable_fps_timer += ctx.delta_time;
        if self.stable_fps == 0.0 || self.stable_fps_timer >= STABLE_FPS_INTERVAL {
            self.stable_fps = ctx.average_fps();
            self.stable_fps_timer = 0.0;
        }
    }

    fn rand_unit(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng as f32 / u32::MAX as f32
    }

    fn particle_color(&self, index: usize) -> [f32; 4] {
        match index % 6 {
            0 => [0.37, 0.95, 1.0, 0.92],
            1 => [1.0, 0.96, 0.42, 0.9],
            2 => [1.0, 0.42, 0.58, 0.9],
            3 => [0.55, 0.48, 1.0, 0.9],
            4 => [0.48, 1.0, 0.58, 0.88],
            _ => [1.0, 1.0, 1.0, 0.8],
        }
    }
}

impl Default for BenchmarkState {
    fn default() -> Self {
        Self::new()
    }
}
