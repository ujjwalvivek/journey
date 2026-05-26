/**----------------------------------------------------
 *!  Start sequence for the game
 *----------------------------------------------------**/
use crate::input::JourneyAction;
use crate::{GameState, JourneyGame, MenuReturnState, OptionsTab};
use engine::egui;
use engine::{AudioResponse, AudioTrack, Context, UiAudioEvent, ui as journey_ui};

#[cfg(target_arch = "wasm32")]
fn open_resonance_panel() {
    if let Some(web_window) = web_sys::window() {
        if let Ok(event) = web_sys::Event::new("journey:open-resonance") {
            let _ = web_window.dispatch_event(&event);
        }
        if let Ok(event) = web_sys::Event::new("journey:open-cadence") {
            let _ = web_window.dispatch_event(&event);
        }
    }
}

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
        let theme = journey_ui::theme();
        let color = egui::Color32::from_rgba_unmultiplied(
            theme.text.r(),
            theme.text.g(),
            theme.text.b(),
            (255.0 * alpha.clamp(0.0, 1.0)) as u8,
        );

        let splash_bg = theme.bg_deep;
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
                                theme.accent.r(),
                                theme.accent.g(),
                                theme.accent.b(),
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
        let letterbox = menu_letterbox_rect(ctx);
        journey_ui::paint_screen(ctx, "menu_bg", letterbox);

        let letterbox_w = letterbox.width();
        let ui_scale = (letterbox.height() / 1080.0).clamp(0.3, 1.0);

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
                        ui.label(journey_ui::title("UNTITLED GAME", 52.0 * ui_scale));
                        ui.add_space(8.0 * ui_scale);
                        ui.label(journey_ui::command_label("Journey Engine", 14.0 * ui_scale));
                    });

                let btn_alpha = (t * 2.0 - 1.0).clamp(0.0, 1.0);

                if btn_alpha > 0.0 {
                    #[cfg(not(target_arch = "wasm32"))]
                    let menu_count: usize = 5;
                    #[cfg(target_arch = "wasm32")]
                    let menu_count: usize = 5;

                    if engine_ctx
                        .input
                        .is_action_just_pressed(JourneyAction::MoveUp)
                    {
                        self.menu_index = if self.menu_index == 0 {
                            menu_count - 1
                        } else {
                            self.menu_index - 1
                        };
                        engine_ctx.pending_ui_audio.push(UiAudioEvent::Hover);
                    }
                    if engine_ctx
                        .input
                        .is_action_just_pressed(JourneyAction::MoveDown)
                    {
                        self.menu_index = (self.menu_index + 1) % menu_count;
                        engine_ctx.pending_ui_audio.push(UiAudioEvent::Hover);
                    }
                    if self.menu_index >= menu_count {
                        self.menu_index = 0;
                    }
                    let confirmed = engine_ctx.input.is_action_just_pressed(JourneyAction::Jump);

                    egui::Area::new(egui::Id::new("start_buttons"))
                        .anchor(
                            egui::Align2::CENTER_CENTER,
                            [-letterbox_w / 2.0 + (200.0 * ui_scale), 0.0],
                        )
                        .show(ctx, |ui| {
                            ui.vertical(|ui| {
                                let btn_w = 280.0 * ui_scale;
                                let btn_h = 48.0 * ui_scale;
                                let spacing = 10.0 * ui_scale;

                                let menu_btn = |label: &str, focused: bool| {
                                    journey_ui::menu_button(label, focused, ui_scale)
                                };

                                let mut idx = 0usize;

                                let r = ui
                                    .add_sized(
                                        [btn_w, btn_h],
                                        menu_btn("Start Game", self.menu_index == idx),
                                    )
                                    .with_ui_sound(&mut engine_ctx.pending_ui_audio);
                                if r.clicked() || (self.menu_index == idx && confirmed) {
                                    self.state = GameState::InGame;
                                }
                                idx += 1;
                                ui.add_space(spacing);

                                let r = ui
                                    .add_sized(
                                        [btn_w, btn_h],
                                        menu_btn("Benchmark", self.menu_index == idx),
                                    )
                                    .with_ui_sound(&mut engine_ctx.pending_ui_audio);
                                if r.clicked() || (self.menu_index == idx && confirmed) {
                                    self.state = GameState::Benchmark;
                                }
                                idx += 1;
                                ui.add_space(spacing);

                                let r = ui
                                    .add_sized(
                                        [btn_w, btn_h],
                                        menu_btn("Level Editor", self.menu_index == idx),
                                    )
                                    .with_ui_sound(&mut engine_ctx.pending_ui_audio);
                                if r.clicked() || (self.menu_index == idx && confirmed) {
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
                                idx += 1;
                                ui.add_space(spacing);

                                let r = ui
                                    .add_sized(
                                        [btn_w, btn_h],
                                        menu_btn("Options", self.menu_index == idx),
                                    )
                                    .with_ui_sound(&mut engine_ctx.pending_ui_audio);
                                if r.clicked() || (self.menu_index == idx && confirmed) {
                                    self.state = GameState::Options {
                                        return_state: MenuReturnState::StartMenu,
                                        tab: OptionsTab::Graphics,
                                    };
                                }
                                idx += 1;
                                ui.add_space(spacing);

                                #[cfg(target_arch = "wasm32")]
                                {
                                    let r = ui
                                        .add_sized(
                                            [btn_w, btn_h],
                                            menu_btn("Synthesizer", self.menu_index == idx),
                                        )
                                        .with_ui_sound(&mut engine_ctx.pending_ui_audio);
                                    if r.clicked() || (self.menu_index == idx && confirmed) {
                                        open_resonance_panel();
                                    }
                                }

                                #[cfg(not(target_arch = "wasm32"))]
                                {
                                    let r = ui
                                        .add_sized(
                                            [btn_w, btn_h],
                                            menu_btn("Exit Journey", self.menu_index == idx),
                                        )
                                        .with_ui_sound(&mut engine_ctx.pending_ui_audio);
                                    if r.clicked() || (self.menu_index == idx && confirmed) {
                                        engine_ctx.request_exit = true;
                                    }
                                    let _ = idx;
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
        journey_ui::paint_screen(ctx, "paused_bg", letterbox);

        let ui_scale = (letterbox.height() / 1080.0).clamp(0.3, 1.0);

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
                        ui.label(journey_ui::title("PAUSED", 38.0 * ui_scale));
                        ui.add_space(32.0 * ui_scale);

                        let btn_w = 280.0 * ui_scale;
                        let btn_h = 44.0 * ui_scale;
                        let spacing = 8.0 * ui_scale;

                        let menu_count: usize = 4;
                        if engine_ctx
                            .input
                            .is_action_just_pressed(JourneyAction::MoveUp)
                        {
                            self.menu_index = if self.menu_index == 0 {
                                menu_count - 1
                            } else {
                                self.menu_index - 1
                            };
                            engine_ctx.pending_ui_audio.push(UiAudioEvent::Hover);
                        }
                        if engine_ctx
                            .input
                            .is_action_just_pressed(JourneyAction::MoveDown)
                        {
                            self.menu_index = (self.menu_index + 1) % menu_count;
                            engine_ctx.pending_ui_audio.push(UiAudioEvent::Hover);
                        }
                        if self.menu_index >= menu_count {
                            self.menu_index = 0;
                        }
                        let confirmed =
                            engine_ctx.input.is_action_just_pressed(JourneyAction::Jump);

                        let menu_btn = |label: &str, focused: bool| {
                            journey_ui::menu_button(label, focused, ui_scale)
                        };

                        let mut idx = 0usize;

                        let r = ui
                            .add_sized([btn_w, btn_h], menu_btn("Continue", self.menu_index == idx))
                            .with_ui_sound(&mut engine_ctx.pending_ui_audio);
                        if r.clicked() || (self.menu_index == idx && confirmed) {
                            self.state = GameState::InGame;
                        }
                        idx += 1;
                        ui.add_space(spacing);

                        let r = ui
                            .add_sized([btn_w, btn_h], menu_btn("Options", self.menu_index == idx))
                            .with_ui_sound(&mut engine_ctx.pending_ui_audio);
                        if r.clicked() || (self.menu_index == idx && confirmed) {
                            self.state = GameState::Options {
                                return_state: MenuReturnState::Paused,
                                tab: OptionsTab::Graphics,
                            };
                        }
                        idx += 1;
                        ui.add_space(spacing);

                        let r = ui
                            .add_sized(
                                [btn_w, btn_h],
                                menu_btn("Level Editor", self.menu_index == idx),
                            )
                            .with_ui_sound(&mut engine_ctx.pending_ui_audio);
                        if r.clicked() || (self.menu_index == idx && confirmed) {
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
                        idx += 1;
                        ui.add_space(spacing);

                        let r = ui
                            .add_sized(
                                [btn_w, btn_h],
                                menu_btn("Main Menu", self.menu_index == idx),
                            )
                            .with_ui_sound(&mut engine_ctx.pending_ui_audio);
                        if r.clicked() || (self.menu_index == idx && confirmed) {
                            self.state = GameState::StartMenu {
                                animation_progress: 1.0,
                            };
                        }
                        let _ = idx;
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
        journey_ui::paint_screen(ctx, "options_bg", letterbox);

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

                let tabs = [
                    OptionsTab::Graphics,
                    OptionsTab::Physics,
                    OptionsTab::Controls,
                    OptionsTab::Audio,
                ];
                let tab_count = tabs.len();
                let cur_tab_idx = tabs.iter().position(|t| *t == current_tab).unwrap_or(0);

                if engine_ctx
                    .input
                    .is_action_just_pressed(JourneyAction::MoveLeft)
                {
                    new_tab = tabs[if cur_tab_idx == 0 {
                        tab_count - 1
                    } else {
                        cur_tab_idx - 1
                    }];
                    engine_ctx.pending_ui_audio.push(UiAudioEvent::TabChange);
                }
                if engine_ctx
                    .input
                    .is_action_just_pressed(JourneyAction::MoveRight)
                {
                    new_tab = tabs[(cur_tab_idx + 1) % tab_count];
                    engine_ctx.pending_ui_audio.push(UiAudioEvent::TabChange);
                }

                ui.vertical_centered(|ui| {
                    ui.add_space(48.0 * ui_scale);
                    ui.label(journey_ui::title("OPTIONS", 30.0 * ui_scale));
                    ui.add_space(16.0 * ui_scale);

                    ui.horizontal(|ui| {
                        let tab_spacing = 8.0 * ui_scale;
                        let total_tabs_w = 4.0 * 112.0 * ui_scale + 3.0 * tab_spacing;
                        ui.add_space((ui.available_width() - total_tabs_w).max(0.0) / 2.0);
                        if journey_ui::tab(
                            ui,
                            "Graphics",
                            current_tab == OptionsTab::Graphics,
                            ui_scale,
                        )
                        .with_tab_sound(&mut engine_ctx.pending_ui_audio)
                        .clicked()
                        {
                            new_tab = OptionsTab::Graphics;
                        }
                        ui.add_space(tab_spacing);
                        if journey_ui::tab(
                            ui,
                            "Gameplay",
                            current_tab == OptionsTab::Physics,
                            ui_scale,
                        )
                        .with_tab_sound(&mut engine_ctx.pending_ui_audio)
                        .clicked()
                        {
                            new_tab = OptionsTab::Physics;
                        }
                        ui.add_space(tab_spacing);
                        if journey_ui::tab(
                            ui,
                            "Controls",
                            current_tab == OptionsTab::Controls,
                            ui_scale,
                        )
                        .with_tab_sound(&mut engine_ctx.pending_ui_audio)
                        .clicked()
                        {
                            new_tab = OptionsTab::Controls;
                        }
                        ui.add_space(tab_spacing);
                        if journey_ui::tab(ui, "Audio", current_tab == OptionsTab::Audio, ui_scale)
                            .with_tab_sound(&mut engine_ctx.pending_ui_audio)
                            .clicked()
                        {
                            new_tab = OptionsTab::Audio;
                        }
                    });
                    ui.add_space(4.0 * ui_scale);
                    journey_ui::divider(ui);
                });

                egui::TopBottomPanel::bottom("options_bottom")
                    .frame(egui::Frame::NONE.inner_margin(16.0 * ui_scale))
                    .show_inside(ui, |ui| {
                        let back_focused = self.menu_index > 0;
                        let confirmed =
                            engine_ctx.input.is_action_just_pressed(JourneyAction::Jump);

                        if engine_ctx
                            .input
                            .is_action_just_pressed(JourneyAction::MoveDown)
                            || engine_ctx
                                .input
                                .is_action_just_pressed(JourneyAction::MoveUp)
                        {
                            self.menu_index = if back_focused { 0 } else { 1 };
                            engine_ctx.pending_ui_audio.push(UiAudioEvent::Hover);
                        }

                        let back_btn = journey_ui::command_button("Back", back_focused, ui_scale);

                        ui.centered_and_justified(|ui| {
                            let r = ui
                                .add_sized([160.0 * ui_scale, 36.0 * ui_scale], back_btn)
                                .with_ui_sound(&mut engine_ctx.pending_ui_audio);
                            if r.clicked() || (back_focused && confirmed) {
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
                                            journey_ui::section_frame().show(ui, |ui| {
                                                ui.set_width(ui.available_width());
                                                ui.vertical_centered(|ui| {
                                                    ui.label(journey_ui::title(
                                                        "GRAPHICS",
                                                        heading_size,
                                                    ));
                                                    ui.add_space(section_spacing);
                                                    #[cfg(not(target_arch = "wasm32"))]
                                                    {
                                                        journey_ui::section_frame().show(
                                                            ui,
                                                            |ui| {
                                                                ui.label(
                                                                    journey_ui::command_label(
                                                                        "Display", label_size,
                                                                    ),
                                                                );
                                                                ui.add_space(4.0 * ui_scale);

                                                                let mut fullscreen =
                                                                    engine_ctx.fullscreen_enabled;
                                                                let fullscreen_resp =
                                                                    journey_ui::toggle(
                                                                        ui,
                                                                        &mut fullscreen,
                                                                        "Fullscreen",
                                                                        ui_scale,
                                                                    );
                                                                let fullscreen_changed =
                                                                    fullscreen_resp.changed();
                                                                fullscreen_resp
                                                                    .with_checkbox_sound(
                                                                        fullscreen,
                                                                        &mut engine_ctx
                                                                            .pending_ui_audio,
                                                                    );
                                                                if fullscreen_changed {
                                                                    engine_ctx
                                                                        .set_fullscreen_enabled(
                                                                            fullscreen,
                                                                        );
                                                                }

                                                                let mut hdr =
                                                                    engine_ctx.hdr_enabled;
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
                                                                let hdr_changed =
                                                                    hdr_resp.changed();
                                                                hdr_resp.with_checkbox_sound(
                                                                    hdr,
                                                                    &mut engine_ctx
                                                                        .pending_ui_audio,
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
                                                            },
                                                        );
                                                        ui.add_space(section_spacing);
                                                    }
                                                    params.sky.enabled = true;
                                                    ui.horizontal(|ui| {
                                                        ui.label(
                                                            egui::RichText::new("Sky Top")
                                                                .size(label_size),
                                                        );
                                                        ui.color_edit_button_rgb(
                                                            &mut params.sky.top_color,
                                                        );
                                                    });
                                                    ui.horizontal(|ui| {
                                                        ui.label(
                                                            egui::RichText::new("Sky Horizon")
                                                                .size(label_size),
                                                        );
                                                        ui.color_edit_button_rgb(
                                                            &mut params.sky.horizon_color,
                                                        );
                                                    });
                                                    ui.horizontal(|ui| {
                                                        ui.label(
                                                            egui::RichText::new("Sky Bottom")
                                                                .size(label_size),
                                                        );
                                                        ui.color_edit_button_rgb(
                                                            &mut params.sky.bottom_color,
                                                        );
                                                    });
                                                    journey_ui::slider_f32(
                                                        ui,
                                                        "Horizon Glow",
                                                        &mut params.sky.horizon_glow,
                                                        0.0..=1.0,
                                                        ui_scale,
                                                        |v| format!("{v:.2}"),
                                                    );
                                                    journey_ui::slider_f32(
                                                        ui,
                                                        "Horizon Y",
                                                        &mut params.sky.horizon_y,
                                                        0.0..=1.0,
                                                        ui_scale,
                                                        |v| format!("{v:.2}"),
                                                    );
                                                    journey_ui::slider_f32(
                                                        ui,
                                                        "Softness",
                                                        &mut params.sky.horizon_width,
                                                        0.01..=0.6,
                                                        ui_scale,
                                                        |v| format!("{v:.2}"),
                                                    );
                                                    ui.add_space(section_spacing);
                                                    {
                                                        let r = journey_ui::toggle(
                                                            ui,
                                                            &mut params.fog_enabled,
                                                            "Fog",
                                                            ui_scale,
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
                                                        journey_ui::slider_u32(
                                                            ui,
                                                            "Seed",
                                                            &mut params.seed,
                                                            0..=999,
                                                            ui_scale,
                                                        );
                                                        journey_ui::slider_f32(
                                                            ui,
                                                            "Density",
                                                            &mut params.fog_density,
                                                            0.5..=10.0,
                                                            ui_scale,
                                                            |v| format!("{v:.2}"),
                                                        );
                                                        journey_ui::slider_f32(
                                                            ui,
                                                            "Opacity",
                                                            &mut params.fog_opacity,
                                                            0.0..=1.0,
                                                            ui_scale,
                                                            |v| format!("{v:.2}"),
                                                        );
                                                        journey_ui::slider_f32(
                                                            ui,
                                                            "Speed",
                                                            &mut params.fog_anim_speed,
                                                            0.0..=2.0,
                                                            ui_scale,
                                                            |v| format!("{v:.2}"),
                                                        );
                                                    }
                                                });
                                            });
                                        }
                                        OptionsTab::Physics => {
                                            journey_ui::section_frame().show(ui, |ui| {
                                                ui.set_width(ui.available_width());
                                                ui.vertical_centered(|ui| {
                                                    ui.label(journey_ui::title(
                                                        "GAMEPLAY",
                                                        heading_size,
                                                    ));
                                                    ui.add_space(section_spacing);
                                                    {
                                                        let r = journey_ui::toggle(
                                                            ui,
                                                            &mut self.show_physics_tuner_in_game,
                                                            "Show Physics Tuner In-Game",
                                                            ui_scale,
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
                                            journey_ui::section_frame().show(ui, |ui| {
                                                ui.set_width(ui.available_width());
                                                ui.vertical_centered(|ui| {
                                                    ui.label(journey_ui::title(
                                                        "CONTROLS",
                                                        heading_size,
                                                    ));
                                                    ui.add_space(section_spacing);
                                                    crate::scene::controls_ui(ui, true);
                                                    ui.add_space(24.0 * ui_scale);
                                                    crate::scene::controls_ui(ui, false);
                                                });
                                            });
                                        }
                                        OptionsTab::Audio => {
                                            journey_ui::section_frame().show(ui, |ui| {
                                                ui.set_width(ui.available_width());
                                                ui.vertical_centered(|ui| {
                                                    ui.label(journey_ui::title(
                                                        "AUDIO",
                                                        heading_size,
                                                    ));
                                                    ui.add_space(section_spacing);

                                                    let mut master =
                                                        engine_ctx.audio.master_volume() as f32;
                                                    if journey_ui::slider_f32(
                                                        ui,
                                                        "Master",
                                                        &mut master,
                                                        0.0..=1.0,
                                                        ui_scale,
                                                        |v| format!("{:.0}%", v * 100.0),
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
                                                    if journey_ui::slider_f32(
                                                        ui,
                                                        "Music",
                                                        &mut music,
                                                        0.0..=1.0,
                                                        ui_scale,
                                                        |v| format!("{:.0}%", v * 100.0),
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
                                                    if journey_ui::slider_f32(
                                                        ui,
                                                        "Ambience",
                                                        &mut amb,
                                                        0.0..=1.0,
                                                        ui_scale,
                                                        |v| format!("{:.0}%", v * 100.0),
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
                                                    if journey_ui::slider_f32(
                                                        ui,
                                                        "SFX",
                                                        &mut sfx,
                                                        0.0..=1.0,
                                                        ui_scale,
                                                        |v| format!("{:.0}%", v * 100.0),
                                                    )
                                                    .changed()
                                                    {
                                                        engine_ctx.audio.set_sfx_volume(sfx as f64);
                                                    }

                                                    let mut ui_vol =
                                                        engine_ctx.audio.ui_volume() as f32;
                                                    if journey_ui::slider_f32(
                                                        ui,
                                                        "UI",
                                                        &mut ui_vol,
                                                        0.0..=1.0,
                                                        ui_scale,
                                                        |v| format!("{:.0}%", v * 100.0),
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
