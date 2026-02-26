/**--------------------------------------------------------------------------------
*!  Level Editor
*?  Toggle with F12
*?  Visual mode: Pan with WASD/Arrow keys or middle-click drag. Saves on exit.
*?  Text mode: Edit the raw ASCII level string with a live minimap preview.
*--------------------------------------------------------------------------------**/
use crate::level::Level;
use engine::{AudioEvent, AudioResponse};

pub struct LevelEditor {
    pub active: bool,
    pub visual_mode: bool,
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
        let initial_text = {
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

        Self {
            active: false,
            visual_mode: false,
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
        }
    }

    pub fn show_ui(
        &mut self,
        ctx: &egui::Context,
        _scene_params: &mut engine::SceneParams,
        level: &mut Level,
        screen_width: f32,
        screen_height: f32,
        pending_audio: &mut Vec<AudioEvent>,
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
        pending_audio: &mut Vec<AudioEvent>,
    ) {
        let letterbox = crate::start_sequence::menu_letterbox_rect(ctx);
        let total_h = letterbox.height();
        let ui_scale = (letterbox.height() / 1080.0).clamp(0.3, 1.0);
        let label_size = 14.0 * ui_scale;

        let panel_frame = egui::Frame::NONE
            .fill(egui::Color32::from_rgb(40, 40, 43))
            .inner_margin(egui::vec2(16.0 * ui_scale, 12.0 * ui_scale));

        let bg_frame = egui::Frame::NONE.fill(egui::Color32::from_rgb(40, 40, 43));

        egui::Window::new("text_editor_window")
            .title_bar(false)
            .resizable(false)
            .movable(false)
            .fixed_pos(letterbox.min)
            .fixed_size(letterbox.size())
            .frame(bg_frame)
            .show(ctx, |ui| {
                egui::TopBottomPanel::top("text_editor_header")
                    .frame(panel_frame)
                    .show_separator_line(false)
                    .exact_height(total_h * 0.08)
                    .show_inside(ui, |ui| {
                        ui.centered_and_justified(|ui| {
                            ui.label(
                                egui::RichText::new("Level Editor")
                                    .size(20.0 * ui_scale)
                                    .strong()
                                    .color(egui::Color32::from_rgb(223, 249, 251)),
                            );
                        });
                    });

                egui::TopBottomPanel::top("text_editor_toolbar")
                    .frame(panel_frame)
                    .show_separator_line(false)
                    .exact_height(total_h * 0.08)
                    .show_inside(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.add_space(12.0 * ui_scale);
                            let btn_size = egui::vec2(160.0 * ui_scale, 28.0 * ui_scale);
                            if ui
                                .add_sized(
                                    btn_size,
                                    egui::Button::new(
                                        egui::RichText::new("Save & Reload")
                                            .strong()
                                            .size(label_size),
                                    )
                                    .corner_radius(2.0),
                                )
                                .with_ui_sound(pending_audio)
                                .clicked()
                            {
                                self.save_and_reload(level, screen_height);
                            }
                            ui.add_space(8.0 * ui_scale);

                            let warnings = self.validate_text_buffer();
                            if !warnings.is_empty() {
                                ui.add_space(4.0);
                                for w in &warnings {
                                    ui.label(
                                        egui::RichText::new(format!("! {w}"))
                                            .size(11.0 * ui_scale)
                                            .color(egui::Color32::from_rgb(255, 180, 60)),
                                    );
                                }
                            }
                            ui.add_space(8.0 * ui_scale);
                            if ui
                                .add_sized(
                                    btn_size,
                                    egui::Button::new(
                                        egui::RichText::new("Visual Editor")
                                            .strong()
                                            .size(label_size),
                                    )
                                    .corner_radius(2.0),
                                )
                                .with_ui_sound(pending_audio)
                                .clicked()
                            {
                                self.visual_mode = true;
                                self.save_and_reload(level, screen_height);
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        egui::RichText::new("F12 to close")
                                            .size(11.0 * ui_scale)
                                            .weak(),
                                    );
                                },
                            );
                        });
                    });

                egui::TopBottomPanel::bottom("text_editor_footer")
                    .frame(panel_frame)
                    .show_separator_line(false)
                    .exact_height(total_h * 0.04)
                    .show_inside(ui, |ui| {
                        ui.centered_and_justified(|ui| {
                            ui.label(
                                egui::RichText::new("Journey Engine")
                                    .size(10.0 * ui_scale)
                                    .color(egui::Color32::from_rgba_unmultiplied(
                                        223, 249, 251, 120,
                                    )),
                            );
                        });
                    });

                egui::TopBottomPanel::bottom("text_editor_minimap")
                    .frame(panel_frame)
                    .show_separator_line(false)
                    .exact_height(total_h * 0.40)
                    .show_inside(ui, |ui| {
                        ui.label(egui::RichText::new("Minimap").size(label_size).strong());
                        ui.add_space(4.0 * ui_scale);
                        egui::ScrollArea::both()
                            .id_salt("minimap_scroll")
                            .show(ui, |ui| {
                                let lines: Vec<&str> = self.text_buffer.lines().collect();
                                let rows = lines.len();
                                let cols =
                                    lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);

                                let available_width = ui.available_size_before_wrap().x.max(1.0);
                                let block_size = if cols > 0 {
                                    (available_width / cols as f32).clamp(4.0, 24.0)
                                } else {
                                    6.0
                                };

                                let minimap_size =
                                    egui::vec2(cols as f32 * block_size, rows as f32 * block_size);

                                let (map_rect, _response) =
                                    ui.allocate_exact_size(minimap_size, egui::Sense::hover());

                                if ui.is_rect_visible(map_rect) {
                                    let painter = ui.painter();
                                    for (r, line) in lines.iter().enumerate() {
                                        for (c, ch) in line.chars().enumerate() {
                                            let color = match ch {
                                                '#' => egui::Color32::from_rgb(20, 20, 23),
                                                '=' => egui::Color32::from_rgb(20, 20, 23),
                                                '_' => egui::Color32::from_rgb(113, 88, 226),
                                                '@' => egui::Color32::RED,
                                                'O' => egui::Color32::GREEN,
                                                '*' => egui::Color32::from_rgb(50, 200, 255),
                                                'E' | 'S' | 'R' => egui::Color32::YELLOW,
                                                _ => continue,
                                            };
                                            let b_min = map_rect.min
                                                + egui::vec2(
                                                    c as f32 * block_size,
                                                    r as f32 * block_size,
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
                            });
                    });

                egui::CentralPanel::default()
                    .frame(panel_frame)
                    .show_inside(ui, |ui| {
                        egui::ScrollArea::both()
                            .id_salt("text_editor_scroll")
                            .show(ui, |ui| {
                                ui.add_space(12.0 * ui_scale);
                                ui.label(
                                    egui::RichText::new("ASCII Preview")
                                        .size(label_size)
                                        .strong(),
                                );
                                ui.add_space(4.0 * ui_scale);
                                ui.horizontal_wrapped(|ui| {
                                    let legend = [
                                        ('#', "Wall"),
                                        ('=', "Floor"),
                                        ('_', "One-Way"),
                                        ('*', "Grapple"),
                                        ('@', "Spawn"),
                                        ('E', "Grunt"),
                                        ('S', "Sniper"),
                                        ('R', "Ronin"),
                                        ('O', "Exit"),
                                        ('.', "Air"),
                                    ];
                                    for (ch, name) in legend {
                                        ui.label(
                                            egui::RichText::new(format!("{ch} = {name}"))
                                                .monospace()
                                                .size(10.0 * ui_scale)
                                                .color(egui::Color32::from_rgb(170, 210, 220)),
                                        );
                                        ui.separator();
                                    }
                                });
                                ui.add_space(8.0);

                                egui::TextEdit::multiline(&mut self.text_buffer)
                                    .font(egui::TextStyle::Monospace)
                                    .code_editor()
                                    .desired_width(f32::INFINITY)
                                    .lock_focus(true)
                                    .show(ui);
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
            let window = web_sys::window().unwrap();
            let storage = window.local_storage().unwrap().unwrap();
            if let Err(e) = storage.set_item("world.txt", &self.text_buffer) {
                log::error!("Failed to save WASM level to local storage: {:?}", e);
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
        pending_audio: &mut Vec<AudioEvent>,
    ) {
        let internal_w = 640.0;
        let internal_h = 360.0;

        let logical_rect = egui_ctx.viewport_rect();
        let logical_w = logical_rect.width();
        let logical_h = logical_rect.height();

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

        let overlay_color = egui::Color32::from_black_alpha(150);
        painter.rect_filled(logical_rect, 0.0, overlay_color);

        let grid_color = egui::Color32::from_white_alpha(80);

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
            .fixed_pos(egui::pos2(vp_x + 10.0, vp_y + 20.0))
            .show(egui_ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button("Switch to Text Editor")
                        .with_ui_sound(pending_audio)
                        .clicked()
                    {
                        self.visual_mode = false;
                    }
                    if ui
                        .button("Save & Reload")
                        .with_ui_sound(pending_audio)
                        .clicked()
                    {
                        self.save_and_reload(level, internal_h);
                    }
                    ui.label("Pan with WASD/Arrows or Middle-Click drag");
                });
                ui.separator();
                ui.label("Palette:");
                ui.horizontal(|ui| {
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
                            .selectable_label(selected, format!("{} {}", ch, name))
                            .clicked()
                        {
                            self.selected_tile = *ch;
                        }
                    }
                });
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
