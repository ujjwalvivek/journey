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
use engine::{AudioResponse, UiAudioEvent, ui as journey_ui};

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
    params.sky.enabled = true;
    scene.params = params.clone();
    let theme = journey_ui::theme();

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

            ui.separator();
            let scale = (ui.ctx().viewport_rect().height() / 1080.0).clamp(0.45, 1.0);
            ui.label(journey_ui::command_label("Sky", 13.0 * scale));

            {
                ui.horizontal(|ui| {
                    ui.label("Top");
                    ui.color_edit_button_rgb(&mut params.sky.top_color);
                });
                ui.horizontal(|ui| {
                    ui.label("Horizon");
                    ui.color_edit_button_rgb(&mut params.sky.horizon_color);
                });
                ui.horizontal(|ui| {
                    ui.label("Bottom");
                    ui.color_edit_button_rgb(&mut params.sky.bottom_color);
                });
                journey_ui::slider_f32(
                    ui,
                    "Horizon Glow",
                    &mut params.sky.horizon_glow,
                    0.0..=1.0,
                    scale,
                    |v| format!("{v:.2}"),
                );
                journey_ui::slider_f32(
                    ui,
                    "Horizon Y",
                    &mut params.sky.horizon_y,
                    0.0..=1.0,
                    scale,
                    |v| format!("{v:.2}"),
                );
                journey_ui::slider_f32(
                    ui,
                    "Softness",
                    &mut params.sky.horizon_width,
                    0.01..=0.6,
                    scale,
                    |v| format!("{v:.2}"),
                );
                {
                    let r = journey_ui::toggle(ui, &mut params.fog_enabled, "Fog", scale);
                    r.with_checkbox_sound(params.fog_enabled, pending_audio);
                }
                if params.fog_enabled {
                    ui.horizontal(|ui| {
                        ui.label("Fog");
                        ui.color_edit_button_rgb(&mut params.fog_color);
                    });
                    journey_ui::slider_u32(ui, "Fog Seed", &mut params.seed, 0..=9999, scale);
                    journey_ui::slider_f32(
                        ui,
                        "Fog Density",
                        &mut params.fog_density,
                        0.5..=20.0,
                        scale,
                        |v| format!("{v:.2}"),
                    );
                    journey_ui::slider_f32(
                        ui,
                        "Fog Opacity",
                        &mut params.fog_opacity,
                        0.0..=1.0,
                        scale,
                        |v| format!("{v:.2}"),
                    );
                    journey_ui::slider_f32(
                        ui,
                        "Fog Speed",
                        &mut params.fog_anim_speed,
                        0.0..=2.0,
                        scale,
                        |v| format!("{v:.2}"),
                    );
                }
                if ui.button("Reset Sky").clicked() {
                    params.sky = Default::default();
                    params.fog_enabled = true;
                    params.fog_color = [0.41, 0.36, 0.81];
                    params.fog_density = 10.0;
                    params.fog_opacity = 1.0;
                    params.fog_anim_speed = 0.5;
                }
            }

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
                #[cfg(not(target_arch = "wasm32"))]
                {
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
                #[cfg(target_arch = "wasm32")]
                {
                    let _ = target_fps;
                }
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
                    ui.colored_label(theme.accent, "I-FRAMES");
                }
                if dash_cooldown > 0 {
                    ui.label(format!("Dash CD: {}", dash_cooldown));
                }
                if has_air_dashed {
                    ui.colored_label(theme.muted, "Air-dash used");
                }
                if wall_left || wall_right {
                    let side = if wall_left { "LEFT" } else { "RIGHT" };
                    ui.colored_label(theme.accent, format!("Wall: {}", side));
                }
                if wall_grab_timer > 0 {
                    ui.label(format!("Wall grab: {} ticks", wall_grab_timer));
                }
                if let Some(target) = grapple_target {
                    ui.colored_label(
                        theme.accent,
                        format!("Grapple: ({:.0}, {:.0})", target.x, target.y),
                    );
                }
                if input_buffer.has_pending() {
                    ui.colored_label(
                        theme.accent,
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
    let scale = (ui.ctx().viewport_rect().height() / 1080.0).clamp(0.45, 1.0);
    let section_gap = 14.0 * scale;

    ui.label(journey_ui::command_label("Gravity", 13.0 * scale));
    journey_ui::slider_f32_log(
        ui,
        "Gravity px/s2",
        &mut cfg.gravity,
        10.0..=5000.0,
        scale,
        |v| format!("{v:.0}"),
    );
    journey_ui::slider_f32(
        ui,
        "Max Fall Speed",
        &mut cfg.max_fall_speed,
        50.0..=2000.0,
        scale,
        |v| format!("{v:.0}"),
    );

    ui.add_space(section_gap);
    journey_ui::divider(ui);
    ui.label(journey_ui::command_label("Movement", 13.0 * scale));
    journey_ui::slider_f32(
        ui,
        "Move Speed",
        &mut cfg.movement_speed,
        50.0..=1200.0,
        scale,
        |v| format!("{v:.0}"),
    );
    journey_ui::slider_f32_log(
        ui,
        "Acceleration",
        &mut cfg.acceleration,
        100.0..=20000.0,
        scale,
        |v| format!("{v:.0}"),
    );
    journey_ui::slider_f32_log(
        ui,
        "Ground Decel",
        &mut cfg.ground_decel,
        100.0..=20000.0,
        scale,
        |v| format!("{v:.0}"),
    );
    journey_ui::slider_f32_log(
        ui,
        "Air Decel",
        &mut cfg.air_decel,
        10.0..=5000.0,
        scale,
        |v| format!("{v:.0}"),
    );

    ui.add_space(section_gap);
    journey_ui::divider(ui);
    ui.label(journey_ui::command_label("Jump", 13.0 * scale));
    journey_ui::slider_f32(
        ui,
        "Jump Power",
        &mut cfg.jump_power,
        100.0..=1500.0,
        scale,
        |v| format!("{v:.0}"),
    );
    journey_ui::slider_f32(
        ui,
        "Early Release Gravity",
        &mut cfg.jump_end_early_gravity_mod,
        1.0..=10.0,
        scale,
        |v| format!("{v:.2}"),
    );
    journey_ui::slider_u16(ui, "Coyote Ticks", &mut cfg.coyote_ticks, 0..=20, scale);
    journey_ui::slider_u16(
        ui,
        "Jump Buffer Ticks",
        &mut cfg.jump_buffer_ticks,
        0..=20,
        scale,
    );

    ui.add_space(section_gap);
    journey_ui::divider(ui);
    ui.label(journey_ui::command_label("Dash", 13.0 * scale));
    journey_ui::slider_f32_log(
        ui,
        "Dash Speed",
        &mut cfg.dash_speed,
        100.0..=3000.0,
        scale,
        |v| format!("{v:.0}"),
    );
    journey_ui::slider_u16(
        ui,
        "Dash Duration Ticks",
        &mut cfg.dash_duration_frames,
        1..=30,
        scale,
    );

    ui.add_space(section_gap);
    journey_ui::divider(ui);
    ui.label(journey_ui::command_label("Wall", 13.0 * scale));
    journey_ui::slider_f32(
        ui,
        "Slide Speed",
        &mut cfg.wall_slide_speed,
        5.0..=200.0,
        scale,
        |v| format!("{v:.0}"),
    );
    journey_ui::slider_f32(
        ui,
        "Jump Power X",
        &mut cfg.wall_jump_power_x,
        50.0..=800.0,
        scale,
        |v| format!("{v:.0}"),
    );
    journey_ui::slider_f32(
        ui,
        "Jump Power Y",
        &mut cfg.wall_jump_power_y,
        100.0..=1000.0,
        scale,
        |v| format!("{v:.0}"),
    );
    journey_ui::slider_u16(
        ui,
        "Grab Timeout Ticks",
        &mut cfg.wall_grab_timeout_ticks,
        5..=120,
        scale,
    );
    journey_ui::slider_u16(
        ui,
        "Jump Lock Ticks",
        &mut cfg.wall_jump_lock_ticks,
        5..=60,
        scale,
    );

    ui.add_space(section_gap);
    journey_ui::divider(ui);
    ui.label(journey_ui::command_label("Grapple", 13.0 * scale));
    journey_ui::slider_f32(
        ui,
        "Pull Speed",
        &mut cfg.grapple_pull_speed,
        50.0..=2000.0,
        scale,
        |v| format!("{v:.0}"),
    );
    journey_ui::slider_f32(
        ui,
        "Slingshot Force",
        &mut cfg.grapple_slingshot_force,
        50.0..=2000.0,
        scale,
        |v| format!("{v:.0}"),
    );
    journey_ui::slider_u16(
        ui,
        "Slingshot Coast Ticks",
        &mut cfg.grapple_slingshot_ticks,
        1..=30,
        scale,
    );
    journey_ui::slider_f32(
        ui,
        "Bounce Vel X",
        &mut cfg.grapple_bounce_velocity_x,
        100.0..=1500.0,
        scale,
        |v| format!("{v:.0}"),
    );
    journey_ui::slider_f32(
        ui,
        "Bounce Vel Y",
        &mut cfg.grapple_bounce_velocity_y,
        -1500.0..=0.0,
        scale,
        |v| format!("{v:.0}"),
    );

    ui.add_space(section_gap);
    journey_ui::divider(ui);
    ui.label(journey_ui::command_label("Knockback", 13.0 * scale));
    journey_ui::slider_f32(
        ui,
        "Knockback Force",
        &mut cfg.knockback,
        100.0..=1500.0,
        scale,
        |v| format!("{v:.0}"),
    );

    ui.add_space(section_gap);
    journey_ui::divider(ui);
    ui.label(journey_ui::command_label("Enemy", 13.0 * scale));
    journey_ui::slider_f32(
        ui,
        "Patrol Speed",
        &mut cfg.enemy_patrol_speed,
        5.0..=200.0,
        scale,
        |v| format!("{v:.0}"),
    );
    journey_ui::slider_f32(
        ui,
        "Aggro Range",
        &mut cfg.enemy_aggro_range,
        20.0..=400.0,
        scale,
        |v| format!("{v:.0}"),
    );
    journey_ui::slider_f32(
        ui,
        "Melee Range",
        &mut cfg.enemy_melee_range,
        5.0..=100.0,
        scale,
        |v| format!("{v:.0}"),
    );

    ui.add_space(section_gap);
    if ui
        .add_sized(
            [ui.available_width().min(220.0), 34.0 * scale],
            journey_ui::command_button("Reset to Defaults", false, scale),
        )
        .clicked()
    {
        *cfg = PhysicsConfig::default();
    }
}

pub fn controls_ui(ui: &mut egui::Ui, using_gamepad: bool) {
    let theme = journey_ui::theme();
    let dim = theme.muted;
    let val = theme.text;

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
                theme.accent,
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
