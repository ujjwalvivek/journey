/**-------------------------------------------------------------------------------------
*!  Scene parameter state shared between the UI and renderer.
*?  Game-specific wrapper around engine `SceneParams` for game specific settings.
*-------------------------------------------------------------------------------------**/
use crate::combat::fsm::CombatState;
use crate::combat::input_buffer::CombatInputBuffer;
use crate::config::PhysicsConfig;
use crate::enemy::Enemy;
use crate::player::PlayerState;
use engine::SceneParams;
use engine::egui;
use engine::{AudioResponse, UiAudioEvent};

#[derive(Debug, Clone, Default)]
pub struct GameScene {
    pub params: SceneParams,
    pub show_collision_box: bool,
    pub show_fps: bool,
    pub show_combat: bool,
}

//? Bundled debug UI parameters to avoid too many function arguments.
pub struct DebugUiParams<'a> {
    pub ctx: &'a egui::Context,
    pub scene: &'a mut GameScene,
    pub params: &'a mut SceneParams,
    pub fps: f32,
    pub frame_time_ms: f32,
    pub fixed_tick_rate: &'a mut u32,
    pub target_fps: &'a mut u32,
    pub combat: &'a CombatState,
    pub input_buffer: &'a CombatInputBuffer,
    pub enemies: &'a [Enemy],
    pub player_state: PlayerState,
    pub wall_left: bool,
    pub wall_right: bool,
    pub dash_cooldown: u16,
    pub has_air_dashed: bool,
    pub wall_grab_timer: u16,
    pub grapple_target: Option<engine::Vec2>,
    pub anim_name: Option<String>,
    pub physics_config: &'a mut PhysicsConfig,
    pub using_gamepad: bool, //* Preserve this bool for later UI updates INGAME
    pub show_physics_tuner_in_game: bool,
    pub pending_audio: &'a mut Vec<UiAudioEvent>,
}

//? Keeps the game wrapper in sync with engine-owned `SceneParams`.
pub fn show_ui(p: DebugUiParams<'_>) {
    let DebugUiParams {
        ctx,
        scene,
        params,
        fps,
        frame_time_ms,
        fixed_tick_rate,
        target_fps,
        combat,
        input_buffer,
        enemies,
        player_state,
        wall_left,
        wall_right,
        dash_cooldown,
        has_air_dashed,
        wall_grab_timer,
        grapple_target,
        anim_name,
        physics_config,
        using_gamepad: _using_gamepad,
        show_physics_tuner_in_game,
        pending_audio,
    } = p;
    scene.params = params.clone();

    let content_rect = ctx.available_rect();
    let window_width = 280.0f32.min(content_rect.width() * 0.9);

    egui::Window::new("Game Controls")
        .default_open(false)
        .default_width(window_width)
        .default_pos([10.0, 10.0])
        .constrain(true)
        .show(ctx, |ui| {
            ui.checkbox(&mut scene.show_collision_box, "Show collision Box")
                .with_checkbox_sound(scene.show_collision_box, pending_audio);
            ui.checkbox(&mut scene.show_fps, "Show FPS")
                .with_checkbox_sound(scene.show_fps, pending_audio);
            ui.checkbox(&mut scene.show_combat, "Show combat FSM")
                .with_checkbox_sound(scene.show_combat, pending_audio);

            if scene.show_fps {
                ui.separator();
                ui.label(format!("FPS: {:.1}", fps));
                ui.label(format!("Frame: {:.2}ms", frame_time_ms));
                ui.separator();
                ui.label("Fixed Tick Rate:");
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(*fixed_tick_rate == 30, "30 Hz")
                        .clicked()
                    {
                        *fixed_tick_rate = 30;
                    }
                    if ui
                        .selectable_label(*fixed_tick_rate == 60, "60 Hz")
                        .clicked()
                    {
                        *fixed_tick_rate = 60;
                    }
                });
                ui.separator();
                ui.label("Visual FPS Lock:");
                ui.horizontal(|ui| {
                    if ui.selectable_label(*target_fps == 0, "Uncapped").clicked() {
                        *target_fps = 0;
                    }
                    if ui.selectable_label(*target_fps == 60, "60 FPS").clicked() {
                        *target_fps = 60;
                    }
                    if ui.selectable_label(*target_fps == 30, "30 FPS").clicked() {
                        *target_fps = 30;
                    }
                });
            }

            if scene.show_combat {
                ui.separator();
                ui.heading("Player");
                ui.label(format!("State: {:?}", player_state));
                if let Some(ref name) = anim_name {
                    ui.label(format!("Anim: {}", name));
                }
                ui.label(format!("Phase: {:?}", combat.phase));
                ui.label(format!("Frame: {}", combat.frame_timer));
                ui.label(format!("Move: {:?}", combat.current_move));
                if combat.invincible {
                    ui.colored_label(egui::Color32::YELLOW, "I-FRAMES");
                }
                if dash_cooldown > 0 {
                    ui.label(format!("Dash CD: {}", dash_cooldown));
                }
                if has_air_dashed {
                    ui.colored_label(egui::Color32::from_rgb(255, 180, 100), "Air-dash used");
                }
                if wall_left || wall_right {
                    let side = if wall_left { "LEFT" } else { "RIGHT" };
                    ui.colored_label(
                        egui::Color32::from_rgb(100, 255, 100),
                        format!("Wall: {}", side),
                    );
                }
                if wall_grab_timer > 0 {
                    ui.label(format!("Wall grab: {} ticks", wall_grab_timer));
                }
                if let Some(target) = grapple_target {
                    ui.colored_label(
                        egui::Color32::from_rgb(50, 220, 255),
                        format!("Grapple: ({:.0}, {:.0})", target.x, target.y),
                    );
                }
                if input_buffer.has_pending() {
                    ui.colored_label(
                        egui::Color32::from_rgb(100, 200, 255),
                        format!("Input Queue: {} pending", input_buffer.len()),
                    );
                }

                ui.separator();
                let alive = enemies.iter().filter(|e| e.is_alive()).count();
                ui.heading(format!("Enemies ({}/{})", alive, enemies.len()));
                if let Some(e) = enemies.iter().find(|e| e.is_alive()) {
                    ui.label(format!("Type: {:?}", e.enemy_type));
                    ui.label(format!("State: {:?}", e.state));
                    ui.label(format!("Phase: {:?}", e.entity.combat.phase));
                }
            }
        });

    if show_physics_tuner_in_game {
        show_physics_tuner_window(ctx, physics_config, content_rect);
    }
}

//? Standalone egui window for hot-reloading physics parameters.
//* All values are edited in-place on the shared `PhysicsConfig` so changes
//* take effect on the very next fixed-update tick, no recompile needed.
pub fn show_physics_tuner_window(
    ctx: &egui::Context,
    cfg: &mut PhysicsConfig,
    content_rect: egui::Rect,
) {
    let window_width = 300.0f32.min(content_rect.width() * 0.9);

    egui::Window::new("Physics Tuning")
        .default_open(false)
        .default_width(window_width)
        .default_pos([10.0, 90.0])
        .constrain(true)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .max_height(content_rect.height() - 50.0)
                .show(ui, |ui| {
                    physics_tuner_ui(ui, cfg);
                });
        });
}

pub fn physics_tuner_ui(ui: &mut egui::Ui, cfg: &mut PhysicsConfig) {
    ui.heading("Gravity");
    ui.add(
        egui::Slider::new(&mut cfg.gravity, 10.0..=5000.0)
            .text("gravity (px/s²)")
            .logarithmic(true),
    );
    ui.add(egui::Slider::new(&mut cfg.max_fall_speed, 50.0..=2000.0).text("max fall speed"));

    ui.separator();
    ui.heading("Movement");
    ui.add(egui::Slider::new(&mut cfg.movement_speed, 50.0..=1200.0).text("move speed"));
    ui.add(
        egui::Slider::new(&mut cfg.acceleration, 100.0..=20000.0)
            .text("acceleration")
            .logarithmic(true),
    );
    ui.add(
        egui::Slider::new(&mut cfg.ground_decel, 100.0..=20000.0)
            .text("ground decel")
            .logarithmic(true),
    );
    ui.add(
        egui::Slider::new(&mut cfg.air_decel, 10.0..=5000.0)
            .text("air decel")
            .logarithmic(true),
    );

    ui.separator();
    ui.heading("Jump");
    ui.add(egui::Slider::new(&mut cfg.jump_power, 100.0..=1500.0).text("jump power"));
    ui.add(
        egui::Slider::new(&mut cfg.jump_end_early_gravity_mod, 1.0..=10.0)
            .text("early release gravity"),
    );
    ui.add(egui::Slider::new(&mut cfg.coyote_ticks, 0..=20).text("coyote (ticks)"));
    ui.add(egui::Slider::new(&mut cfg.jump_buffer_ticks, 0..=20).text("jump buffer (ticks)"));

    ui.separator();
    ui.heading("Dash");
    ui.add(
        egui::Slider::new(&mut cfg.dash_speed, 100.0..=3000.0)
            .text("dash speed")
            .logarithmic(true),
    );
    ui.add(egui::Slider::new(&mut cfg.dash_duration_frames, 1..=30).text("dash duration (ticks)"));

    ui.separator();
    ui.heading("Wall");
    ui.add(egui::Slider::new(&mut cfg.wall_slide_speed, 5.0..=200.0).text("slide speed"));
    ui.add(egui::Slider::new(&mut cfg.wall_jump_power_x, 50.0..=800.0).text("jump power X"));
    ui.add(egui::Slider::new(&mut cfg.wall_jump_power_y, 100.0..=1000.0).text("jump power Y"));
    ui.add(
        egui::Slider::new(&mut cfg.wall_grab_timeout_ticks, 5..=120).text("grab timeout (ticks)"),
    );
    ui.add(egui::Slider::new(&mut cfg.wall_jump_lock_ticks, 5..=60).text("jump lock (ticks)"));

    ui.separator();
    ui.heading("Grapple");
    ui.add(egui::Slider::new(&mut cfg.grapple_pull_speed, 50.0..=2000.0).text("pull speed"));
    ui.add(
        egui::Slider::new(&mut cfg.grapple_slingshot_force, 50.0..=2000.0).text("slingshot force"),
    );
    ui.add(
        egui::Slider::new(&mut cfg.grapple_slingshot_ticks, 1..=30).text("slingshot coast (ticks)"),
    );
    ui.add(
        egui::Slider::new(&mut cfg.grapple_bounce_velocity_x, 100.0..=1500.0).text("bounce vel X"),
    );
    ui.add(
        egui::Slider::new(&mut cfg.grapple_bounce_velocity_y, -1500.0..=0.0).text("bounce vel Y"),
    );

    ui.separator();
    ui.heading("Knockback");
    ui.add(egui::Slider::new(&mut cfg.knockback, 100.0..=1500.0).text("knockback force"));

    ui.separator();
    ui.heading("Enemy");
    ui.add(egui::Slider::new(&mut cfg.enemy_patrol_speed, 5.0..=200.0).text("patrol speed"));
    ui.add(egui::Slider::new(&mut cfg.enemy_aggro_range, 20.0..=400.0).text("aggro range"));
    ui.add(egui::Slider::new(&mut cfg.enemy_melee_range, 5.0..=100.0).text("melee range"));

    ui.separator();
    if ui.button("Reset to Defaults").clicked() {
        *cfg = PhysicsConfig::default();
    }
}

pub fn controls_ui(ui: &mut egui::Ui, using_gamepad: bool) {
    let dim = egui::Color32::from_rgba_unmultiplied(243, 204, 172, 150); //* #F3CCAC
    let val = egui::Color32::from_rgba_unmultiplied(243, 204, 172, 255);

    let controls: &[(&str, &str)] = if using_gamepad {
        &[
            ("Move", "Left stick"),
            ("Jump", "A / Cross"),
            ("Dash", "B / Circle"),
            ("Attack", "X / Square"),
            ("Parry", "Y / Triangle"),
            ("Grapple", "RT / R2"),
            ("Drop", "Left Stick Down + A"),
        ]
    } else {
        &[
            ("Move", "WASD / Arrow keys"),
            ("Jump", "Space"),
            ("Dash", "Shift"),
            ("Attack", "LMB"),
            ("Parry", "RMB"),
            ("Grapple", "Alt"),
            ("Drop", "S/Arrow Down + Space"),
        ]
    };

    ui.set_min_width(148.0);
    ui.push_id(
        if using_gamepad {
            "controller"
        } else {
            "keyboard"
        },
        |ui| {
            let header = if using_gamepad {
                "Controller"
            } else {
                "Keyboard & Mouse"
            };
            ui.colored_label(
                egui::Color32::from_rgba_unmultiplied(243, 204, 172, 255),
                egui::RichText::new(header).size(16.0).strong(),
            );
            ui.add_space(4.0);

            egui::Grid::new("controls_grid")
                .num_columns(2)
                .spacing([12.0, 3.0])
                .show(ui, |ui| {
                    for &(action, key) in controls {
                        ui.colored_label(dim, egui::RichText::new(action).size(16.0));
                        ui.colored_label(val, egui::RichText::new(key).size(16.0).strong());
                        ui.end_row();
                    }
                });
        },
    );
}
