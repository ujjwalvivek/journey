/**----------------------------------------------------
 *!  Start sequence for the game
 *----------------------------------------------------**/
use crate::input::JourneyAction;
use crate::{GameState, JourneyGame, MenuReturnState, OptionsTab};
use engine::egui;
use engine::{AudioResponse, AudioTrack, Context};

pub(crate) fn menu_letterbox_rect(ctx: &egui::Context) -> egui::Rect {
    let screen_rect = ctx.viewport_rect();
    let target_aspect = 16.0 / 9.0;
    let screen_aspect = screen_rect.width() / screen_rect.height();

    let size = if screen_aspect > target_aspect {
        egui::vec2(screen_rect.height() * target_aspect, screen_rect.height())
    } else {
        egui::vec2(screen_rect.width(), screen_rect.width() / target_aspect)
    };

    egui::Rect::from_center_size(screen_rect.center(), size)
}

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
            40,
            40,
            43,
            (255.0 * alpha.clamp(0.0, 1.0)) as u8,
        );

        let splash_bg = egui::Color32::from_rgb(223, 249, 251);
        let letterbox = menu_letterbox_rect(ctx);
        ctx.layer_painter(egui::LayerId::new(
            egui::Order::Background,
            egui::Id::new("splash_bg"),
        ))
        .rect_filled(letterbox, 0.0, splash_bg);

        let ui_scale = (letterbox.height() / 1080.0).clamp(0.3, 1.0);

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |_| {});

        egui::Area::new(egui::Id::new("splash_center"))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                let padded_w = (letterbox.width() - 80.0 * ui_scale).max(160.0);
                ui.set_min_width(padded_w);
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("Untitled Game")
                            .size(48.0 * ui_scale)
                            .strong()
                            .color(color),
                    );
                    ui.add_space(12.0 * ui_scale);
                    ui.label(
                        egui::RichText::new("Journey Engine")
                            .size(16.0 * ui_scale)
                            .color(egui::Color32::from_rgba_unmultiplied(
                                40,
                                40,
                                43,
                                (180.0 * alpha.clamp(0.0, 1.0)) as u8,
                            )),
                    );
                });
            });
    }

    pub(crate) fn show_start_menu(
        &mut self,
        ctx: &egui::Context,
        engine_ctx: &mut Context<JourneyAction>,
        animation_progress: f32,
    ) {
        let bg_color = egui::Color32::from_rgb(223, 249, 251);
        let letterbox = menu_letterbox_rect(ctx);
        ctx.layer_painter(egui::LayerId::new(
            egui::Order::Background,
            egui::Id::new("menu_bg"),
        ))
        .rect_filled(letterbox, 0.0, bg_color);

        let letterbox_w = letterbox.width();
        let ui_scale = (letterbox.height() / 1080.0).clamp(0.3, 1.0);
        let dark = egui::Color32::from_rgb(40, 40, 43);

        egui::Window::new("start_overlay")
            .title_bar(false)
            .resizable(false)
            .movable(false)
            .fixed_pos(letterbox.min)
            .fixed_size(letterbox.size())
            .frame(egui::Frame::NONE)
            .show(ctx, |_| {
                let t = (animation_progress * std::f32::consts::PI / 2.0).sin();

                let title_x_center = 0.0;
                let title_x_right = letterbox_w / 2.0 - (500.0 * ui_scale);
                let current_title_offset = title_x_center + (title_x_right - title_x_center) * t;

                egui::Area::new(egui::Id::new("start_title"))
                    .anchor(egui::Align2::CENTER_CENTER, [current_title_offset, 0.0])
                    .show(ctx, |ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                        ui.label(
                            egui::RichText::new("Untitled Game")
                                .size(48.0 * ui_scale)
                                .strong()
                                .color(dark),
                        );
                    });

                let btn_alpha = (t * 2.0 - 1.0).clamp(0.0, 1.0);

                if btn_alpha > 0.0 {
                    let btn_color = egui::Color32::from_rgba_unmultiplied(
                        40,
                        40,
                        43,
                        (255.0 * btn_alpha) as u8,
                    );

                    egui::Area::new(egui::Id::new("start_buttons"))
                        .anchor(
                            egui::Align2::CENTER_CENTER,
                            [-letterbox_w / 2.0 + (200.0 * ui_scale), 0.0],
                        )
                        .show(ctx, |ui| {
                            ui.vertical(|ui| {
                                let btn_w = 280.0 * ui_scale;
                                let btn_h = 44.0 * ui_scale;
                                let font = 20.0 * ui_scale;
                                let spacing = 8.0 * ui_scale;

                                let menu_btn = |label: &str| {
                                    egui::Button::new(
                                        egui::RichText::new(label).size(font).color(btn_color),
                                    )
                                    .fill(egui::Color32::TRANSPARENT)
                                    .stroke(egui::Stroke::new(
                                        1.0,
                                        egui::Color32::from_rgba_unmultiplied(
                                            40,
                                            40,
                                            43,
                                            (60.0 * btn_alpha) as u8,
                                        ),
                                    ))
                                    .corner_radius(2.0)
                                };

                                if ui
                                    .add_sized([btn_w, btn_h], menu_btn("Start Game"))
                                    .with_ui_sound(&mut engine_ctx.pending_ui_audio)
                                    .clicked()
                                {
                                    self.state = GameState::InGame;
                                }
                                ui.add_space(spacing);

                                if ui
                                    .add_sized([btn_w, btn_h], menu_btn("Level Editor"))
                                    .with_ui_sound(&mut engine_ctx.pending_ui_audio)
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
                                ui.add_space(spacing);

                                if ui
                                    .add_sized([btn_w, btn_h], menu_btn("Options"))
                                    .with_ui_sound(&mut engine_ctx.pending_ui_audio)
                                    .clicked()
                                {
                                    self.state = GameState::Options {
                                        return_state: MenuReturnState::StartMenu,
                                        tab: OptionsTab::Graphics,
                                    };
                                }
                                ui.add_space(spacing);

                                #[cfg(not(target_arch = "wasm32"))]
                                if ui
                                    .add_sized([btn_w, btn_h], menu_btn("Exit Game"))
                                    .with_ui_sound(&mut engine_ctx.pending_ui_audio)
                                    .clicked()
                                {
                                    engine_ctx.request_exit = true;
                                }
                            });
                        });
                }
            });
    }

    pub(crate) fn show_paused_menu(
        &mut self,
        ctx: &egui::Context,
        engine_ctx: &mut Context<JourneyAction>,
    ) {
        let letterbox = menu_letterbox_rect(ctx);
        let bg_color = egui::Color32::from_rgb(223, 249, 251);
        ctx.layer_painter(egui::LayerId::new(
            egui::Order::Background,
            egui::Id::new("paused_bg"),
        ))
        .rect_filled(letterbox, 0.0, bg_color);

        let ui_scale = (letterbox.height() / 1080.0).clamp(0.3, 1.0);
        let dark = egui::Color32::from_rgb(40, 40, 43);

        egui::Window::new("paused_overlay")
            .title_bar(false)
            .resizable(false)
            .movable(false)
            .fixed_pos(letterbox.min)
            .fixed_size(letterbox.size())
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                ui.set_min_size(ui.available_size());
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0 * ui_scale);
                        ui.label(
                            egui::RichText::new("Paused")
                                .size(36.0 * ui_scale)
                                .strong()
                                .color(dark),
                        );
                        ui.add_space(32.0 * ui_scale);

                        let btn_w = 280.0 * ui_scale;
                        let btn_h = 40.0 * ui_scale;
                        let font = 18.0 * ui_scale;
                        let spacing = 6.0 * ui_scale;

                        let menu_btn = |label: &str| {
                            egui::Button::new(egui::RichText::new(label).size(font).color(dark))
                                .fill(egui::Color32::TRANSPARENT)
                                .stroke(egui::Stroke::new(
                                    1.0,
                                    egui::Color32::from_rgba_unmultiplied(40, 40, 43, 50),
                                ))
                                .corner_radius(2.0)
                        };

                        if ui
                            .add_sized([btn_w, btn_h], menu_btn("Continue"))
                            .with_ui_sound(&mut engine_ctx.pending_ui_audio)
                            .clicked()
                        {
                            self.state = GameState::InGame;
                        }
                        ui.add_space(spacing);

                        if ui
                            .add_sized([btn_w, btn_h], menu_btn("Options"))
                            .with_ui_sound(&mut engine_ctx.pending_ui_audio)
                            .clicked()
                        {
                            self.state = GameState::Options {
                                return_state: MenuReturnState::Paused,
                                tab: OptionsTab::Graphics,
                            };
                        }
                        ui.add_space(spacing);

                        if ui
                            .add_sized([btn_w, btn_h], menu_btn("Level Editor"))
                            .with_ui_sound(&mut engine_ctx.pending_ui_audio)
                            .clicked()
                        {
                            let start_pos = self.player.position();
                            let level_floor_y = self.level.death_y_threshold - 100.0;
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
                        ui.add_space(spacing);

                        if ui
                            .add_sized([btn_w, btn_h], menu_btn("Main Menu"))
                            .with_ui_sound(&mut engine_ctx.pending_ui_audio)
                            .clicked()
                        {
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
        engine_ctx: &mut Context<JourneyAction>,
        params: &mut engine::SceneParams,
        return_state: MenuReturnState,
        current_tab: OptionsTab,
    ) {
        let mut new_tab = current_tab;
        let letterbox = menu_letterbox_rect(ctx);
        let menu_bg = egui::Color32::from_rgb(40, 40, 43);
        ctx.layer_painter(egui::LayerId::new(
            egui::Order::Background,
            egui::Id::new("options_bg"),
        ))
        .rect_filled(letterbox, 0.0, menu_bg);

        let ui_scale = (letterbox.height() / 1080.0).clamp(0.3, 1.0);
        let heading_size = 24.0 * ui_scale;
        let label_size = 16.0 * ui_scale;
        let _small_size = 13.0 * ui_scale;
        let section_spacing = 12.0 * ui_scale;

        egui::Window::new("options_overlay")
            .title_bar(false)
            .resizable(false)
            .movable(false)
            .fixed_pos(letterbox.min)
            .fixed_size(letterbox.size())
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                ui.set_min_size(ui.available_size());
                ui.vertical_centered(|ui| {
                    ui.add_space(16.0 * ui_scale);
                    ui.label(
                        egui::RichText::new("Options")
                            .size(28.0 * ui_scale)
                            .strong(),
                    );
                    ui.add_space(16.0 * ui_scale);

                    ui.horizontal(|ui| {
                        let tab_size = label_size;
                        let tab_spacing = 20.0 * ui_scale;
                        let total_tabs_w = 4.0 * 80.0 * ui_scale + 3.0 * tab_spacing;
                        ui.add_space((ui.available_width() - total_tabs_w).max(0.0) / 2.0);
                        ui.selectable_value(
                            &mut new_tab,
                            OptionsTab::Graphics,
                            egui::RichText::new("Graphics").size(tab_size),
                        )
                        .with_tab_sound(&mut engine_ctx.pending_ui_audio);
                        ui.add_space(tab_spacing);
                        ui.selectable_value(
                            &mut new_tab,
                            OptionsTab::Physics,
                            egui::RichText::new("Gameplay").size(tab_size),
                        )
                        .with_tab_sound(&mut engine_ctx.pending_ui_audio);
                        ui.add_space(tab_spacing);
                        ui.selectable_value(
                            &mut new_tab,
                            OptionsTab::Controls,
                            egui::RichText::new("Controls").size(tab_size),
                        )
                        .with_tab_sound(&mut engine_ctx.pending_ui_audio);
                        ui.add_space(tab_spacing);
                        ui.selectable_value(
                            &mut new_tab,
                            OptionsTab::Audio,
                            egui::RichText::new("Audio").size(tab_size),
                        )
                        .with_tab_sound(&mut engine_ctx.pending_ui_audio);
                    });
                    ui.add_space(4.0 * ui_scale);
                    ui.separator();
                });

                egui::TopBottomPanel::bottom("options_bottom")
                    .frame(egui::Frame::NONE.inner_margin(16.0 * ui_scale))
                    .show_inside(ui, |ui| {
                        ui.centered_and_justified(|ui| {
                            if ui
                                .add_sized(
                                    [160.0 * ui_scale, 36.0 * ui_scale],
                                    egui::Button::new(egui::RichText::new("Back").size(label_size))
                                        .corner_radius(2.0),
                                )
                                .with_ui_sound(&mut engine_ctx.pending_ui_audio)
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
                            .inner_margin(egui::vec2(24.0 * ui_scale, 16.0 * ui_scale)),
                    )
                    .show_inside(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(ui.available_height() - (16.0 * ui_scale))
                            .show(ui, |ui| {
                                ui.vertical_centered(|ui| {
                                    ui.set_max_width(560.0 * ui_scale);
                                    ui.add_space(12.0 * ui_scale);

                                    match current_tab {
                                        OptionsTab::Graphics => {
                                            ui.group(|ui| {
                                                ui.set_width(ui.available_width());
                                                ui.vertical_centered(|ui| {
                                                    ui.label(
                                                        egui::RichText::new("Graphics")
                                                            .size(heading_size)
                                                            .strong(),
                                                    );
                                                    ui.add_space(section_spacing);
                                                    #[cfg(not(target_arch = "wasm32"))]
                                                    {
                                                        ui.group(|ui| {
                                                            ui.label(
                                                                egui::RichText::new("Display")
                                                                    .size(label_size)
                                                                    .strong(),
                                                            );
                                                            ui.add_space(4.0 * ui_scale);

                                                            let mut fullscreen =
                                                                engine_ctx.fullscreen_enabled;
                                                            let fullscreen_resp = ui.checkbox(
                                                                &mut fullscreen,
                                                                egui::RichText::new("Fullscreen")
                                                                    .size(label_size),
                                                            );
                                                            let fullscreen_changed =
                                                                fullscreen_resp.changed();
                                                            fullscreen_resp.with_checkbox_sound(
                                                                fullscreen,
                                                                &mut engine_ctx.pending_ui_audio,
                                                            );
                                                            if fullscreen_changed {
                                                                engine_ctx.set_fullscreen_enabled(
                                                                    fullscreen,
                                                                );
                                                            }

                                                            let mut hdr = engine_ctx.hdr_enabled;
                                                            let hdr_resp = ui.add_enabled(
                                                                fullscreen,
                                                                egui::Checkbox::new(
                                                                    &mut hdr,
                                                                    egui::RichText::new(
                                                                        "HDR Output",
                                                                    )
                                                                    .size(label_size),
                                                                ),
                                                            );
                                                            let hdr_changed = hdr_resp.changed();
                                                            hdr_resp.with_checkbox_sound(
                                                                hdr,
                                                                &mut engine_ctx.pending_ui_audio,
                                                            );
                                                            if hdr_changed {
                                                                engine_ctx.set_hdr_enabled(hdr);
                                                            }

                                                            if !fullscreen {
                                                                ui.label(
                                                                    egui::RichText::new(
                                                                        "HDR requires fullscreen",
                                                                    )
                                                                    .size(_small_size)
                                                                    .weak(),
                                                                );
                                                            }
                                                        });
                                                        ui.add_space(section_spacing);
                                                    }
                                                    ui.horizontal(|ui| {
                                                        ui.label(
                                                            egui::RichText::new("Background")
                                                                .size(label_size),
                                                        );
                                                        ui.color_edit_button_rgb(
                                                            &mut params.background_color,
                                                        );
                                                    });
                                                    ui.add_space(section_spacing);
                                                    {
                                                        let r = ui.checkbox(
                                                            &mut params.fog_enabled,
                                                            egui::RichText::new("Fog")
                                                                .size(label_size),
                                                        );
                                                        r.with_checkbox_sound(
                                                            params.fog_enabled,
                                                            &mut engine_ctx.pending_ui_audio,
                                                        );
                                                    }
                                                    if params.fog_enabled {
                                                        ui.horizontal(|ui| {
                                                            ui.label(
                                                                egui::RichText::new("Fog Color")
                                                                    .size(label_size),
                                                            );
                                                            ui.color_edit_button_rgb(
                                                                &mut params.fog_color,
                                                            );
                                                        });
                                                        ui.add(
                                                            egui::Slider::new(
                                                                &mut params.seed,
                                                                0..=999,
                                                            )
                                                            .text("Seed"),
                                                        );
                                                        ui.add(
                                                            egui::Slider::new(
                                                                &mut params.fog_density,
                                                                0.5..=10.0,
                                                            )
                                                            .text("Density"),
                                                        );
                                                        ui.add(
                                                            egui::Slider::new(
                                                                &mut params.fog_opacity,
                                                                0.0..=1.0,
                                                            )
                                                            .text("Opacity"),
                                                        );
                                                        ui.add(
                                                            egui::Slider::new(
                                                                &mut params.fog_anim_speed,
                                                                0.0..=2.0,
                                                            )
                                                            .text("Speed"),
                                                        );
                                                    }
                                                });
                                            });
                                        }
                                        OptionsTab::Physics => {
                                            ui.group(|ui| {
                                                ui.set_width(ui.available_width());
                                                ui.vertical_centered(|ui| {
                                                    ui.label(
                                                        egui::RichText::new("Gameplay")
                                                            .size(heading_size)
                                                            .strong(),
                                                    );
                                                    ui.add_space(section_spacing);
                                                    {
                                                        let r = ui.checkbox(
                                                            &mut self.show_physics_tuner_in_game,
                                                            egui::RichText::new(
                                                                "Show Physics Tuner In-Game",
                                                            )
                                                            .size(label_size),
                                                        );
                                                        r.with_checkbox_sound(
                                                            self.show_physics_tuner_in_game,
                                                            &mut engine_ctx.pending_ui_audio,
                                                        );
                                                    }
                                                    ui.add_space(section_spacing);
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
                                                    ui.label(
                                                        egui::RichText::new("Controls")
                                                            .size(heading_size)
                                                            .strong(),
                                                    );
                                                    ui.add_space(section_spacing);
                                                    crate::scene::controls_ui(ui, true);
                                                    ui.add_space(24.0 * ui_scale);
                                                    crate::scene::controls_ui(ui, false);
                                                });
                                            });
                                        }
                                        OptionsTab::Audio => {
                                            ui.group(|ui| {
                                                ui.set_width(ui.available_width());
                                                ui.vertical_centered(|ui| {
                                                    ui.label(
                                                        egui::RichText::new("Audio")
                                                            .size(heading_size)
                                                            .strong(),
                                                    );
                                                    ui.add_space(section_spacing);

                                                    let mut master =
                                                        engine_ctx.audio.master_volume() as f32;
                                                    if ui
                                                        .add(
                                                            egui::Slider::new(
                                                                &mut master,
                                                                0.0..=1.0,
                                                            )
                                                            .text("Master"),
                                                        )
                                                        .changed()
                                                    {
                                                        engine_ctx
                                                            .audio
                                                            .set_master_volume(master as f64);
                                                        let mv = engine_ctx
                                                            .audio
                                                            .effective_volume(AudioTrack::Music);
                                                        let av = engine_ctx
                                                            .audio
                                                            .effective_volume(AudioTrack::Ambience);
                                                        engine_ctx
                                                            .audio
                                                            .set_music_live_volume(mv, 0.1);
                                                        engine_ctx
                                                            .audio
                                                            .set_ambience_live_volume(av, 0.1);
                                                    }

                                                    let mut music =
                                                        engine_ctx.audio.music_volume() as f32;
                                                    if ui
                                                        .add(
                                                            egui::Slider::new(
                                                                &mut music,
                                                                0.0..=1.0,
                                                            )
                                                            .text("Music"),
                                                        )
                                                        .changed()
                                                    {
                                                        engine_ctx
                                                            .audio
                                                            .set_music_volume(music as f64);
                                                        let mv = engine_ctx
                                                            .audio
                                                            .effective_volume(AudioTrack::Music);
                                                        engine_ctx
                                                            .audio
                                                            .set_music_live_volume(mv, 0.1);
                                                    }

                                                    let mut amb =
                                                        engine_ctx.audio.ambience_volume() as f32;
                                                    if ui
                                                        .add(
                                                            egui::Slider::new(&mut amb, 0.0..=1.0)
                                                                .text("Ambience"),
                                                        )
                                                        .changed()
                                                    {
                                                        engine_ctx
                                                            .audio
                                                            .set_ambience_volume(amb as f64);
                                                        let av = engine_ctx
                                                            .audio
                                                            .effective_volume(AudioTrack::Ambience);
                                                        engine_ctx
                                                            .audio
                                                            .set_ambience_live_volume(av, 0.1);
                                                    }

                                                    let mut sfx =
                                                        engine_ctx.audio.sfx_volume() as f32;
                                                    if ui
                                                        .add(
                                                            egui::Slider::new(&mut sfx, 0.0..=1.0)
                                                                .text("SFX"),
                                                        )
                                                        .changed()
                                                    {
                                                        engine_ctx.audio.set_sfx_volume(sfx as f64);
                                                    }

                                                    let mut ui_vol =
                                                        engine_ctx.audio.ui_volume() as f32;
                                                    if ui
                                                        .add(
                                                            egui::Slider::new(
                                                                &mut ui_vol,
                                                                0.0..=1.0,
                                                            )
                                                            .text("UI"),
                                                        )
                                                        .changed()
                                                    {
                                                        engine_ctx
                                                            .audio
                                                            .set_ui_volume(ui_vol as f64);
                                                    }
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
