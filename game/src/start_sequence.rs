/**----------------------------------------------------
*!  Start sequence for the game
*----------------------------------------------------**/
use crate::{GameState, JourneyGame, MenuReturnState, OptionsTab};
use engine::Context;

impl JourneyGame {
    pub(crate) fn show_splash_screen(&mut self, ctx: &egui::Context, timer: f32) {
        let alpha = if timer > 2.5 {
            (3.0 - timer) / 0.5
        } else if timer < 0.5 {
            timer / 0.5
        } else {
            1.0
        };
        let color = egui::Color32::from_rgba_unmultiplied(
            255,
            255,
            255,
            (255.0 * alpha.clamp(0.95, 1.0)) as u8,
        );

        let ui_scale = (ctx.available_rect().height() / 1080.0).clamp(0.3, 1.0);

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::BLACK))
            .show(ctx, |_| {});

        egui::Area::new(egui::Id::new("splash_center"))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading(
                        egui::RichText::new("Untitled Game")
                            .size(64.0 * ui_scale)
                            .strong()
                            .color(color),
                    );
                    ui.add_space(20.0 * ui_scale);
                    ui.label(
                        egui::RichText::new("Created in Journey Engine")
                            .size(24.0 * ui_scale)
                            .color(color),
                    );
                });
            });
    }

    pub(crate) fn show_start_menu(
        &mut self,
        ctx: &egui::Context,
        engine_ctx: &mut Context,
        animation_progress: f32,
    ) {
        let screen_rect = ctx.available_rect();
        let target_aspect = 16.0 / 9.0;
        let screen_aspect = screen_rect.width() / screen_rect.height();
        let letterbox_w = if screen_aspect > target_aspect {
            screen_rect.height() * target_aspect
        } else {
            screen_rect.width()
        };

        let ui_scale = (screen_rect.height() / 1080.0).clamp(0.3, 1.0);

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::BLACK))
            .show(ctx, |_| {
                let t = (animation_progress * std::f32::consts::PI / 2.0).sin();

                let title_x_center = 0.0;
                let title_x_right = letterbox_w / 2.0 - (500.0 * ui_scale); //? Use letterbox width to constrain ultra-wide movement
                let current_title_offset = title_x_center + (title_x_right - title_x_center) * t;

                egui::Area::new(egui::Id::new("start_title"))
                    .anchor(egui::Align2::CENTER_CENTER, [current_title_offset, 0.0])
                    .show(ctx, |ui| {
                        ui.heading(
                            egui::RichText::new("Journey Engine")
                                .size(64.0 * ui_scale)
                                .strong()
                                .color(egui::Color32::WHITE),
                        );
                    });

                let btn_alpha = (t * 2.0 - 1.0).clamp(0.95, 1.0);
                let btn_color =
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, (255.0 * btn_alpha) as u8);

                if btn_alpha > 0.0 {
                    egui::Area::new(egui::Id::new("start_buttons"))
                        //? Anchor buttons relative to the left side of the letterbox area, not the physical screen left
                        .anchor(
                            egui::Align2::CENTER_CENTER,
                            [-letterbox_w / 2.0 + (200.0 * ui_scale), 0.0],
                        )
                        .show(ctx, |ui| {
                            ui.style_mut()
                                .visuals
                                .widgets
                                .noninteractive
                                .fg_stroke
                                .color = btn_color;
                            ui.style_mut().visuals.widgets.inactive.fg_stroke.color = btn_color;

                            ui.vertical(|ui| {
                                let btn_size = egui::vec2(220.0 * ui_scale, 50.0 * ui_scale);
                                if ui
                                    .add_sized(
                                        btn_size,
                                        egui::Button::new(
                                            egui::RichText::new("Start Game")
                                                .size(28.0 * ui_scale)
                                                .color(btn_color),
                                        ),
                                    )
                                    .clicked()
                                {
                                    self.state = GameState::InGame;
                                }
                                ui.add_space(15.0 * ui_scale);

                                if ui
                                    .add_sized(
                                        btn_size,
                                        egui::Button::new(
                                            egui::RichText::new("Level Editor")
                                                .size(28.0 * ui_scale)
                                                .color(btn_color),
                                        ),
                                    )
                                    .clicked()
                                {
                                    self.state = GameState::LevelEditor {
                                        return_state: MenuReturnState::StartMenu,
                                    };
                                    let start_pos = self.player.position();
                                    let level_floor_y = self.level.death_y_threshold - 100.0;
                                    self.level_editor.toggle(
                                        start_pos.x,
                                        level_floor_y,
                                        engine_ctx.screen_width,
                                        engine_ctx.screen_height,
                                    );
                                }
                                ui.add_space(15.0 * ui_scale);

                                if ui
                                    .add_sized(
                                        btn_size,
                                        egui::Button::new(
                                            egui::RichText::new("Options")
                                                .size(28.0 * ui_scale)
                                                .color(btn_color),
                                        ),
                                    )
                                    .clicked()
                                {
                                    self.state = GameState::Options {
                                        return_state: MenuReturnState::StartMenu,
                                        tab: OptionsTab::Graphics,
                                    };
                                }
                                ui.add_space(15.0 * ui_scale);

                                #[cfg(not(target_arch = "wasm32"))]
                                if ui
                                    .add_sized(
                                        btn_size,
                                        egui::Button::new(
                                            egui::RichText::new("Exit Game")
                                                .size(28.0 * ui_scale)
                                                .color(btn_color),
                                        ),
                                    )
                                    .clicked()
                                {
                                    engine_ctx.request_exit = true;
                                }
                            });
                        });
                }
            });
    }

    pub(crate) fn show_paused_menu(&mut self, ctx: &egui::Context, engine_ctx: &mut Context) {
        let ui_scale = (ctx.available_rect().height() / 1080.0).clamp(0.3, 1.0);

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::from_black_alpha(255)))
            .show(ctx, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0 * ui_scale);
                        ui.heading(
                            egui::RichText::new("Game Paused")
                                .size(48.0 * ui_scale)
                                .strong()
                                .color(egui::Color32::WHITE),
                        );
                        ui.add_space(40.0 * ui_scale);

                        let btn_size = egui::vec2(200.0 * ui_scale, 40.0 * ui_scale);
                        if ui
                            .add_sized(
                                btn_size,
                                egui::Button::new(
                                    egui::RichText::new("Continue Game").size(24.0 * ui_scale),
                                ),
                            )
                            .clicked()
                        {
                            self.state = GameState::InGame;
                        }
                        ui.add_space(10.0 * ui_scale);

                        if ui
                            .add_sized(
                                btn_size,
                                egui::Button::new(
                                    egui::RichText::new("Options").size(24.0 * ui_scale),
                                ),
                            )
                            .clicked()
                        {
                            self.state = GameState::Options {
                                return_state: MenuReturnState::Paused,
                                tab: OptionsTab::Graphics,
                            };
                        }
                        ui.add_space(10.0 * ui_scale);

                        if ui
                            .add_sized(
                                btn_size,
                                egui::Button::new(
                                    egui::RichText::new("Level Editor").size(24.0 * ui_scale),
                                ),
                            )
                            .clicked()
                        {
                            let start_pos = self.player.position();
                            let level_floor_y = self.level.death_y_threshold - 100.0;
                            //? Ensure Level Editor state matches current screen layout
                            self.level_editor.toggle(
                                start_pos.x,
                                level_floor_y,
                                engine_ctx.screen_width,
                                engine_ctx.screen_height,
                            );
                            self.state = GameState::LevelEditor {
                                return_state: MenuReturnState::Paused,
                            };
                        }
                        ui.add_space(10.0 * ui_scale);

                        if ui
                            .add_sized(
                                btn_size,
                                egui::Button::new(
                                    egui::RichText::new("Main Menu").size(24.0 * ui_scale),
                                ),
                            )
                            .clicked()
                        {
                            //? Reset player state and anything needed for a clean reload
                            self.state = GameState::StartMenu {
                                animation_progress: 1.0,
                            };
                        }
                    });
                });
            });
    }

    pub(crate) fn show_options_menu(
        &mut self,
        ctx: &egui::Context,
        params: &mut engine::scene::SceneParams,
        return_state: MenuReturnState,
        current_tab: OptionsTab,
    ) {
        let mut new_tab = current_tab.clone();
        let ui_scale = (ctx.available_rect().height() / 1080.0).clamp(0.3, 1.0);

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::from_black_alpha(255)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0 * ui_scale);
                    ui.heading(
                        egui::RichText::new("Options")
                            .size(40.0 * ui_scale)
                            .strong(),
                    );
                    ui.add_space(20.0 * ui_scale);

                    ui.horizontal(|ui| {
                        //? Dummy space to center tabs
                        ui.add_space(ui.available_width() / 2.0 - 150.0);
                        ui.selectable_value(&mut new_tab, OptionsTab::Graphics, "Graphics");
                        ui.selectable_value(
                            &mut new_tab,
                            OptionsTab::Physics,
                            "Gameplay & Physics",
                        );
                        ui.selectable_value(&mut new_tab, OptionsTab::Controls, "Controls");
                    });
                    ui.separator();
                });

                egui::TopBottomPanel::bottom("options_bottom")
                    .frame(egui::Frame::NONE.inner_margin(20.0 * ui_scale))
                    .show_inside(ui, |ui| {
                        ui.centered_and_justified(|ui| {
                            if ui
                                .add_sized(
                                    [150.0 * ui_scale, 40.0 * ui_scale],
                                    egui::Button::new(
                                        egui::RichText::new("Back").size(20.0 * ui_scale),
                                    ),
                                )
                                .clicked()
                            {
                                self.state = match return_state {
                                    MenuReturnState::StartMenu => GameState::StartMenu {
                                        animation_progress: 1.0,
                                    },
                                    MenuReturnState::Paused => GameState::Paused,
                                    MenuReturnState::InGame => GameState::InGame,
                                };
                            }
                        });
                    });

                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::NONE
                            .inner_margin(egui::vec2(20.0 * ui_scale, 20.0 * ui_scale)),
                    )
                    .show_inside(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(ui.available_height() - (20.0 * ui_scale))
                            .show(ui, |ui| {
                                ui.vertical_centered(|ui| {
                                    //? Limit width constraint to make content look cohesive
                                    ui.set_max_width(600.0 * ui_scale);
                                    ui.add_space(20.0 * ui_scale);

                                    match current_tab {
                                        OptionsTab::Graphics => {
                                            ui.group(|ui| {
                                                ui.set_width(ui.available_width());
                                                ui.vertical_centered(|ui| {
                                                    ui.heading("Graphics Settings");
                                                    ui.add_space(10.0);
                                                    ui.horizontal(|ui| {
                                                        ui.label("Background Color:");
                                                        ui.color_edit_button_rgb(
                                                            &mut params.background_color,
                                                        );
                                                    });
                                                    ui.add_space(10.0);
                                                    ui.checkbox(
                                                        &mut params.fog_enabled,
                                                        "Enable Fog Details",
                                                    );
                                                    if params.fog_enabled {
                                                        ui.horizontal(|ui| {
                                                            ui.label("Fog Color:");
                                                            ui.color_edit_button_rgb(
                                                                &mut params.fog_color,
                                                            );
                                                        });
                                                        ui.add(
                                                            egui::Slider::new(
                                                                &mut params.seed,
                                                                0..=999,
                                                            )
                                                            .text("Random Seed"),
                                                        );
                                                        ui.add(
                                                            egui::Slider::new(
                                                                &mut params.fog_density,
                                                                0.5..=10.0,
                                                            )
                                                            .text("Fog Density"),
                                                        );
                                                        ui.add(
                                                            egui::Slider::new(
                                                                &mut params.fog_opacity,
                                                                0.0..=1.0,
                                                            )
                                                            .text("Fog Opacity"),
                                                        );
                                                        ui.add(
                                                            egui::Slider::new(
                                                                &mut params.fog_anim_speed,
                                                                0.0..=2.0,
                                                            )
                                                            .text("Animation Speed"),
                                                        );
                                                    }
                                                });
                                            });
                                        }
                                        OptionsTab::Physics => {
                                            ui.group(|ui| {
                                                ui.set_width(ui.available_width());
                                                ui.vertical_centered(|ui| {
                                                    ui.heading("Gameplay Tweaks");
                                                    ui.add_space(10.0);
                                                    ui.checkbox(
                                                        &mut self.show_physics_tuner_in_game,
                                                        "Show Physics Tuner In-Game",
                                                    );
                                                    ui.add_space(20.0);
                                                });

                                                crate::scene::physics_tuner_ui(
                                                    ui,
                                                    &mut self.physics_config,
                                                );
                                            });
                                        }
                                        OptionsTab::Controls => {
                                            ui.group(|ui| {
                                                ui.set_width(ui.available_width());
                                                ui.vertical_centered(|ui| {
                                                    ui.heading("Gamepad Bindings");
                                                    ui.add_space(10.0);
                                                    crate::scene::controls_ui(ui, true);
                                                    ui.add_space(30.0);

                                                    ui.heading("Keyboard Bindings");
                                                    ui.add_space(10.0);
                                                    crate::scene::controls_ui(ui, false);
                                                });
                                            });
                                        }
                                    }
                                });
                            });
                    });
            });

        //? Apply Tab change at end to avoid borrow checker conflicts
        if new_tab != current_tab {
            self.state = GameState::Options {
                return_state,
                tab: new_tab,
            };
        }
    }
}
