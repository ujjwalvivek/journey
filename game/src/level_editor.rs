/**--------------------------------------------------------------------------------
*!  Level Editor
*?  Toggle with F12
*?  Visual mode: Pan with WASD/Arrow keys or middle-click drag. Saves on exit.
*?  Text mode: Edit the raw ASCII level string with a live minimap preview.
*--------------------------------------------------------------------------------**/
use crate::level::Level;
use engine::egui;
use engine::{AudioResponse, UiAudioEvent, ui as journey_ui};

pub struct LevelEditor {
    pub active: bool,
    pub visual_mode: bool,
    pub request_close: bool,
    reset_text_scroll: bool,
    pub text_buffer: String,
    pub camera_x: f32,
    pub camera_y: f32,
    pub selected_tile: char,
}

impl Default for LevelEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl LevelEditor {
    pub fn new() -> Self {
        //? Load the current level text so the buffer is populated initially
        let initial_text = Level::load_level_text();

        Self {
            active: false,
            visual_mode: false,
            request_close: false,
            reset_text_scroll: true,
            text_buffer: initial_text,
            camera_x: 0.0,
            camera_y: 0.0,
            selected_tile: '#',
        }
    }

    pub fn toggle(
        &mut self,
        player_x: f32,
        level_floor_y: f32,
        _screen_width: f32,
        _screen_height: f32,
    ) {
        self.active = !self.active;
        if self.active {
            //? Center the editor camera horizontally on the player, and pin the bottom to the level floor
            let internal_w = 640.0;
            let internal_h = 360.0;
            self.camera_x = (player_x - internal_w / 2.0).max(0.0);
            self.camera_y = (level_floor_y - internal_h).max(0.0);
            self.reset_text_scroll = true;
        }
    }

    pub fn take_close_request(&mut self) -> bool {
        let requested = self.request_close;
        self.request_close = false;
        requested
    }

    pub fn show_ui(
        &mut self,
        ctx: &egui::Context,
        _scene_params: &mut engine::SceneParams,
        level: &mut Level,
        screen_width: f32,
        screen_height: f32,
        pending_audio: &mut Vec<UiAudioEvent>,
    ) {
        if !self.active {
            return;
        }

        if self.visual_mode {
            self.show_visual_editor(ctx, level, screen_width, screen_height, pending_audio);
        } else {
            self.show_text_editor(ctx, level, screen_height, pending_audio);
        }
    }

    fn show_text_editor(
        &mut self,
        ctx: &egui::Context,
        level: &mut Level,
        screen_height: f32,
        pending_audio: &mut Vec<UiAudioEvent>,
    ) {
        let letterbox = crate::start_sequence::menu_letterbox_rect(ctx);
        journey_ui::paint_screen(ctx, "level_editor_text_bg", letterbox);

        let ui_scale = (letterbox.height() / 1080.0).clamp(0.3, 1.0);
        let heading_size = 24.0 * ui_scale;
        let label_size = 13.0 * ui_scale;
        let section_spacing = 12.0 * ui_scale;
        let theme = journey_ui::theme();
        let panel_width = (letterbox.width() * 0.88).clamp(880.0 * ui_scale, 1880.0 * ui_scale);
        let map_rows = self.text_buffer.lines().count().max(1);
        let map_cols = self
            .text_buffer
            .lines()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0);
        let editor_font_size = (11.0 * ui_scale).clamp(9.0, 12.0);
        let reset_text_scroll = self.reset_text_scroll;

        egui::Window::new("text_editor_window")
            .title_bar(false)
            .resizable(false)
            .movable(false)
            .fixed_pos(letterbox.min)
            .fixed_size(letterbox.size())
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                ui.set_min_size(ui.available_size());
                ui.vertical_centered(|ui| {
                    ui.add_space(48.0 * ui_scale);
                    ui.label(journey_ui::title("LEVEL EDITOR", 30.0 * ui_scale));
                    ui.add_space(16.0 * ui_scale);

                    ui.horizontal(|ui| {
                        let tab_spacing = 8.0 * ui_scale;
                        let save_w = 160.0 * ui_scale;
                        let total_w = 2.0 * 112.0 * ui_scale + save_w + 2.0 * tab_spacing;
                        ui.add_space((ui.available_width() - total_w).max(0.0) / 2.0);

                        let _ = journey_ui::tab(ui, "Text", true, ui_scale);
                        ui.add_space(tab_spacing);

                        if journey_ui::tab(ui, "Visual", false, ui_scale)
                            .with_tab_sound(pending_audio)
                            .clicked()
                        {
                            self.visual_mode = true;
                            self.reset_text_scroll = true;
                            self.save_and_reload(level, screen_height);
                        }
                        ui.add_space(tab_spacing);

                        if ui
                            .add_sized(
                                [save_w, 34.0 * ui_scale],
                                journey_ui::command_button("Save", false, ui_scale),
                            )
                            .with_ui_sound(pending_audio)
                            .clicked()
                        {
                            self.save_and_reload(level, screen_height);
                        }
                    });
                    ui.add_space(4.0 * ui_scale);
                    journey_ui::divider(ui);
                });

                egui::TopBottomPanel::bottom("level_editor_bottom")
                    .frame(egui::Frame::NONE.inner_margin(16.0 * ui_scale))
                    .show_inside(ui, |ui| {
                        ui.centered_and_justified(|ui| {
                            if ui
                                .add_sized(
                                    [160.0 * ui_scale, 36.0 * ui_scale],
                                    journey_ui::command_button("Back", false, ui_scale),
                                )
                                .with_ui_sound(pending_audio)
                                .clicked()
                            {
                                self.request_close = true;
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
                                    ui.set_max_width(panel_width);
                                    ui.add_space(12.0 * ui_scale);

                                    journey_ui::section_frame().show(ui, |ui| {
                                        ui.set_width(panel_width.min(ui.available_width()));
                                        ui.vertical_centered(|ui| {
                                            ui.label(journey_ui::title("TEXT MAP", heading_size));
                                            ui.add_space(section_spacing);
                                        });

                                        ui.horizontal_wrapped(|ui| {
                                            ui.label(
                                                egui::RichText::new(format!("{map_rows} ROWS"))
                                                    .size(label_size)
                                                    .strong()
                                                    .color(theme.text),
                                            );
                                            ui.add_space(12.0 * ui_scale);
                                            ui.label(
                                                egui::RichText::new(format!("{map_cols} COLS"))
                                                    .size(label_size)
                                                    .strong()
                                                    .color(theme.text),
                                            );
                                            ui.add_space(12.0 * ui_scale);
                                            ui.label(journey_ui::muted(
                                                "# WALL  = FLOOR  _ ONE-WAY  * GRAPPLE  @ SPAWN  E/S/R ENEMY  O EXIT",
                                                10.0 * ui_scale,
                                            ));
                                        });

                                        let warnings = self.validate_text_buffer();
                                        if !warnings.is_empty() {
                                            ui.add_space(8.0 * ui_scale);
                                            ui.horizontal_wrapped(|ui| {
                                                for warning in warnings {
                                                    ui.label(
                                                        egui::RichText::new(warning)
                                                            .size(10.0 * ui_scale)
                                                            .color(theme.accent),
                                                    );
                                                }
                                            });
                                        }

                                        ui.add_space(section_spacing);
                                        egui::Frame::NONE
                                            .fill(theme.bg_deep)
                                            .stroke(egui::Stroke::new(1.0, theme.stroke_soft))
                                            .inner_margin(egui::Margin::same(
                                                (8.0 * ui_scale).round() as i8,
                                            ))
                                            .show(ui, |ui| {
                                                let editor_font = egui::FontId::new(
                                                    editor_font_size,
                                                    egui::FontFamily::Monospace,
                                                );
                                                let content_gutter = 28.0 * ui_scale;
                                                let text_margin_x = 10.0 * ui_scale;
                                                let text_width = self
                                                    .text_buffer
                                                    .lines()
                                                    .map(|line| {
                                                        ui.painter()
                                                            .layout_no_wrap(
                                                                line.to_owned(),
                                                                editor_font.clone(),
                                                                theme.text,
                                                            )
                                                            .size()
                                                            .x
                                                    })
                                                    .fold(0.0, f32::max)
                                                    + text_margin_x * 2.0
                                                    + content_gutter * 2.0;
                                                let text_width =
                                                    text_width.max(panel_width - 48.0 * ui_scale);
                                                let mut no_wrap_layouter =
                                                    |ui: &egui::Ui,
                                                     text: &dyn egui::TextBuffer,
                                                     _wrap_width: f32| {
                                                        let mut job =
                                                            egui::text::LayoutJob::simple(
                                                                text.as_str().to_owned(),
                                                                editor_font.clone(),
                                                                theme.text,
                                                                f32::INFINITY,
                                                            );
                                                        job.wrap.max_width = f32::INFINITY;
                                                        ui.fonts_mut(|f| f.layout_job(job))
                                                    };

                                                let mut text_scroll =
                                                    egui::ScrollArea::horizontal()
                                                        .id_salt("text_editor_horizontal_v7_left");
                                                if reset_text_scroll {
                                                    text_scroll = text_scroll
                                                        .horizontal_scroll_offset(0.0)
                                                        .animated(false);
                                                }
                                                let text_edit_id =
                                                    ui.make_persistent_id("level_editor_text_edit_v5_left");
                                                if reset_text_scroll {
                                                    egui::text_edit::TextEditState::default()
                                                        .store(ui.ctx(), text_edit_id);
                                                }
                                                text_scroll.show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.add_space(content_gutter);
                                                        ui.add(
                                                            egui::TextEdit::multiline(
                                                                &mut self.text_buffer,
                                                            )
                                                            .id(text_edit_id)
                                                            .code_editor()
                                                            .font(editor_font.clone())
                                                            .desired_width(text_width)
                                                            .desired_rows(map_rows)
                                                            .margin(egui::Margin::symmetric(
                                                                text_margin_x.round() as i8,
                                                                (3.0 * ui_scale).round() as i8,
                                                            ))
                                                            .layouter(&mut no_wrap_layouter)
                                                            .lock_focus(false)
                                                            .cursor_at_end(false),
                                                        );
                                                        ui.add_space(content_gutter);
                                                    });
                                                });
                                            });

                                        ui.add_space(section_spacing);
                                        journey_ui::divider(ui);
                                        ui.add_space(section_spacing);

                                        ui.label(journey_ui::command_label(
                                            "Minimap",
                                            12.0 * ui_scale,
                                        ));
                                        ui.add_space(4.0 * ui_scale);
                                        self.show_text_editor_minimap(
                                            ui,
                                            ui_scale,
                                            theme,
                                            reset_text_scroll,
                                        );
                                    });
                                });
                            });
                    });
            });
        self.reset_text_scroll = false;
    }

    fn show_text_editor_minimap(
        &self,
        ui: &mut egui::Ui,
        ui_scale: f32,
        theme: journey_ui::Theme,
        reset_scroll: bool,
    ) {
        egui::Frame::NONE
            .fill(theme.bg_deep)
            .stroke(egui::Stroke::new(1.0, theme.stroke_soft))
            .inner_margin(egui::Margin::same((8.0 * ui_scale).round() as i8))
            .show(ui, |ui| {
                let lines: Vec<&str> = self.text_buffer.lines().collect();
                let rows = lines.len();
                let cols = lines
                    .iter()
                    .map(|line| line.chars().count())
                    .max()
                    .unwrap_or(0);
                let available =
                    egui::vec2(ui.available_width(), f32::INFINITY).max(egui::Vec2::splat(1.0));
                let block_size = if rows > 0 && cols > 0 {
                    ((available.x / cols as f32) * 1.7).clamp(2.5, 8.0)
                } else {
                    5.0
                };
                let minimap_size = egui::vec2(cols as f32 * block_size, rows as f32 * block_size);
                let minimap_gutter = 28.0 * ui_scale;

                let mut minimap_scroll =
                    egui::ScrollArea::horizontal().id_salt("level_editor_minimap_scroll_v3");
                if reset_scroll {
                    minimap_scroll = minimap_scroll.horizontal_scroll_offset(0.0).animated(false);
                }

                minimap_scroll.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(minimap_gutter);
                        let (map_rect, _) =
                            ui.allocate_exact_size(minimap_size, egui::Sense::hover());

                        if ui.is_rect_visible(map_rect) {
                            let painter = ui.painter();
                            for (row, line) in lines.iter().enumerate() {
                                for (col, ch) in line.chars().enumerate() {
                                    let color = match ch {
                                        '#' | '=' => theme.text,
                                        '_' | '*' | '@' | 'O' => theme.accent,
                                        'E' | 'S' | 'R' => theme.muted,
                                        _ => continue,
                                    };
                                    let b_min = map_rect.min
                                        + egui::vec2(
                                            col as f32 * block_size,
                                            row as f32 * block_size,
                                        );
                                    let b_max = b_min + egui::vec2(block_size, block_size);
                                    painter.rect_filled(
                                        egui::Rect::from_min_max(b_min, b_max),
                                        0.0,
                                        color,
                                    );
                                }
                            }
                        }
                        ui.add_space(minimap_gutter);
                    });
                });
            });
    }

    //? Returns a list of human-readable warnings for the current text buffer.
    //? Runs before save to surface structural errors in the editor UI.
    pub fn validate_text_buffer(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        let spawn_count = self.text_buffer.chars().filter(|&c| c == '@').count();
        match spawn_count {
            0 => warnings
                .push("No player spawn (@) found. Player will spawn at (100, 100).".to_string()),
            n if n > 1 => warnings.push(format!(
                "Multiple player spawns (@) found: {n}. Only the first will be used."
            )),
            _ => {}
        }
        let exit_count = self.text_buffer.chars().filter(|&c| c == 'O').count();
        if exit_count == 0 {
            warnings.push("No exit (O) found. Level has no goal.".to_string());
        }
        let has_floor = self.text_buffer.chars().any(|c| c == '=' || c == '#');
        if !has_floor {
            warnings.push("No solid tiles (= or #) found. Level has no floor.".to_string());
        }
        warnings
    }

    fn save_and_reload(&self, level: &mut Level, screen_height: f32) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Err(e) = std::fs::write("game/assets/level/world.txt", &self.text_buffer) {
                log::error!("Failed to save level file: {}", e);
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten())
            {
                if let Err(e) = storage.set_item("world.txt", &self.text_buffer) {
                    log::error!("Failed to save WASM level to local storage: {:?}", e);
                }
            } else {
                log::warn!("localStorage unavailable, level not saved");
            }
        }

        //? Reload the level purely from the string buffer in memory
        level.reload_from_str(&self.text_buffer, screen_height);
    }

    //? Visual editor mode with a game-like map and tile palette
    fn show_visual_editor(
        &mut self,
        egui_ctx: &egui::Context,
        level: &mut Level,
        _game_w: f32,
        _game_h: f32,
        pending_audio: &mut Vec<UiAudioEvent>,
    ) {
        let internal_w = 640.0;
        let internal_h = 360.0;

        let logical_rect = egui_ctx.viewport_rect();
        let logical_w = logical_rect.width();
        let logical_h = logical_rect.height();
        let ui_scale = (logical_h / 1080.0).clamp(0.45, 1.0);
        let theme = journey_ui::theme();

        let scale = (logical_w / internal_w).min(logical_h / internal_h);
        let scaled_w = internal_w * scale;
        let scaled_h = internal_h * scale;
        let vp_x = (logical_w - scaled_w) / 2.0;
        let vp_y = (logical_h - scaled_h) / 2.0;

        let pan_speed = 10.0;
        let mut drag_delta = egui::Vec2::ZERO;

        egui_ctx.input(|i| {
            if i.key_down(egui::Key::W) || i.key_down(egui::Key::ArrowUp) {
                self.camera_y -= pan_speed;
            }
            if i.key_down(egui::Key::S) || i.key_down(egui::Key::ArrowDown) {
                self.camera_y += pan_speed;
            }
            if i.key_down(egui::Key::A) || i.key_down(egui::Key::ArrowLeft) {
                self.camera_x -= pan_speed;
            }
            if i.key_down(egui::Key::D) || i.key_down(egui::Key::ArrowRight) {
                self.camera_x += pan_speed;
            }
            //? Middle click drag
            if i.pointer.middle_down() {
                drag_delta = i.pointer.delta();
            }
        });

        if drag_delta != egui::Vec2::ZERO {
            //? Negative because dragging the mouse right moves the camera left
            self.camera_x -= drag_delta.x / scale;
            self.camera_y -= drag_delta.y / scale;
        }

        self.camera_x = self.camera_x.max(0.0);

        let tile_size = 16.0;
        let total_rows = self.text_buffer.lines().count() as f32;
        let offset_y = internal_h - (total_rows * tile_size);
        self.camera_y = self.camera_y.max(offset_y);
        let start_col = (self.camera_x / tile_size).floor() as i32;
        let end_col = ((self.camera_x + internal_w) / tile_size).ceil() as i32;
        let start_row = ((self.camera_y - offset_y) / tile_size).floor() as i32;
        let end_row = ((self.camera_y + internal_h - offset_y) / tile_size).ceil() as i32;

        let painter = egui_ctx.layer_painter(egui::LayerId::background());

        let overlay_color = egui::Color32::from_black_alpha(205);
        painter.rect_filled(logical_rect, 0.0, overlay_color);

        let grid_color = egui::Color32::from_white_alpha(46);

        //? Draw grid lines (horizontal)
        for r in start_row..=end_row {
            //* matching level.rs bottom-anchor math: y = screen_height - (total_rows - row)*tile_size
            let game_y = offset_y + r as f32 * tile_size - self.camera_y;
            let egui_y = vp_y + game_y * scale;
            painter.line_segment(
                [
                    egui::pos2(vp_x, egui_y),
                    egui::pos2(vp_x + scaled_w, egui_y),
                ],
                egui::Stroke::new(2.0, grid_color),
            );
        }
        //? Draw grid lines (vertical)
        for c in start_col..=end_col {
            let game_x = c as f32 * tile_size - self.camera_x;
            let egui_x = vp_x + game_x * scale;
            painter.line_segment(
                [
                    egui::pos2(egui_x, vp_y),
                    egui::pos2(egui_x, vp_y + scaled_h),
                ],
                egui::Stroke::new(2.0, grid_color),
            );
        }

        //? Egui UI Overlay for Palette
        egui::Window::new("Visual Editor Tools")
            .title_bar(false)
            .resizable(false)
            .frame(journey_ui::panel_frame())
            .fixed_pos(egui::pos2(vp_x + 10.0, vp_y + 20.0))
            .show(egui_ctx, |ui| {
                ui.label(journey_ui::title("LEVEL EDITOR", 18.0 * ui_scale));
                ui.add_space(8.0 * ui_scale);
                ui.horizontal(|ui| {
                    if ui
                        .add_sized(
                            [170.0 * ui_scale, 32.0 * ui_scale],
                            journey_ui::command_button("Text Editor", false, ui_scale),
                        )
                        .with_ui_sound(pending_audio)
                        .clicked()
                    {
                        self.visual_mode = false;
                        self.reset_text_scroll = true;
                    }
                    if ui
                        .add_sized(
                            [170.0 * ui_scale, 32.0 * ui_scale],
                            journey_ui::command_button("Save & Reload", false, ui_scale),
                        )
                        .with_ui_sound(pending_audio)
                        .clicked()
                    {
                        self.save_and_reload(level, internal_h);
                    }
                    if ui
                        .add_sized(
                            [120.0 * ui_scale, 32.0 * ui_scale],
                            journey_ui::command_button("Back", false, ui_scale),
                        )
                        .with_ui_sound(pending_audio)
                        .clicked()
                    {
                        self.request_close = true;
                    }
                    ui.label(journey_ui::muted("VISUAL MODE", 11.0 * ui_scale));
                });
                ui.add_space(10.0 * ui_scale);
                journey_ui::divider(ui);
                ui.label(journey_ui::command_label("Palette", 12.0 * ui_scale));
                ui.horizontal_wrapped(|ui| {
                    let tiles = [
                        ('#', "Wall"),
                        ('=', "Floor"),
                        ('_', "One-Way Platform"),
                        ('*', "Grapple Node"),
                        ('@', "Player Spawn"),
                        ('E', "Grunt Enemy"),
                        ('S', "Sniper Enemy"),
                        ('R', "Ronin Enemy"),
                        ('O', "Exit"),
                        ('.', "Erase"),
                    ];
                    for (ch, name) in tiles.iter() {
                        let selected = self.selected_tile == *ch;
                        if ui
                            .add_sized(
                                [150.0 * ui_scale, 28.0 * ui_scale],
                                journey_ui::command_button(
                                    &format!("{} {}", ch, name),
                                    selected,
                                    ui_scale,
                                ),
                            )
                            .clicked()
                        {
                            self.selected_tile = *ch;
                        }
                    }
                });
                ui.add_space(4.0 * ui_scale);
                ui.label(
                    egui::RichText::new(format!("SELECTED {}", self.selected_tile))
                        .size(11.0 * ui_scale)
                        .strong()
                        .color(theme.accent),
                );
            });

        //? Handle tile painting when clicking on the gameplay area (outside the UI windows)
        if !egui_ctx.wants_pointer_input() {
            egui_ctx.input(|i| {
                #[allow(clippy::collapsible_if)]
                if i.pointer.primary_down() || i.pointer.secondary_down() {
                    if let Some(mouse_pos) = i.pointer.interact_pos() {
                        //? Ensure to paint within the game viewport letterbox
                        if mouse_pos.x >= vp_x
                            && mouse_pos.x <= vp_x + scaled_w
                            && mouse_pos.y >= vp_y
                            && mouse_pos.y <= vp_y + scaled_h
                        {
                            let game_x = (mouse_pos.x - vp_x) / scale + self.camera_x;
                            let game_y = (mouse_pos.y - vp_y) / scale + self.camera_y;

                            let col = (game_x / tile_size).floor() as i32;
                            let row_i32 = ((game_y - offset_y) / tile_size).floor() as i32;

                            //? Edit the text_buffer safely
                            let mut lines: Vec<String> =
                                self.text_buffer.lines().map(String::from).collect();
                            let mut changed = false;

                            if row_i32 >= 0 && (row_i32 as usize) < lines.len() {
                                let row = row_i32 as usize;
                                let mut chars: Vec<char> = lines[row].chars().collect();
                                if col >= 0 && (col as usize) < chars.len() {
                                    let col_u = col as usize;
                                    //? Right click = Erase ('.'). Left click = Paint selected tile
                                    let paint_char = if i.pointer.secondary_down() {
                                        '.'
                                    } else {
                                        self.selected_tile
                                    };

                                    if chars[col_u] != paint_char {
                                        chars[col_u] = paint_char;
                                        lines[row] = chars.into_iter().collect();
                                        changed = true;
                                    }
                                }
                            }

                            if changed {
                                self.text_buffer = lines.join("\n");
                                //? Hot-reload in memory.
                                level.reload_from_str(&self.text_buffer, internal_h);
                            }
                        }
                    }
                }

                //? Mouse click release saves to disk
                if i.pointer.primary_released() || i.pointer.secondary_released() {
                    self.save_and_reload(level, internal_h);
                }
            });
        }
    }
}
