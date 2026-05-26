use egui::{
    Align2, Button, Color32, Context, FontFamily, FontId, Frame, Id, Margin, Rect, Response,
    RichText, Sense, Stroke, StrokeKind, TextStyle, Ui, Vec2, Visuals, pos2, vec2,
};
use std::ops::RangeInclusive;

use crate::context::FrameStats;

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub bg: Color32,
    pub bg_deep: Color32,
    pub panel: Color32,
    pub panel_alt: Color32,
    pub panel_hot: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub dim: Color32,
    pub accent: Color32,
    pub accent_alt: Color32,
    pub warn: Color32,
    pub stroke: Color32,
    pub stroke_soft: Color32,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Color32::from_rgb(5, 5, 6),
            bg_deep: Color32::from_rgb(0, 0, 0),
            panel: Color32::from_rgb(10, 10, 11),
            panel_alt: Color32::from_rgb(20, 20, 22),
            panel_hot: Color32::from_rgba_unmultiplied(94, 242, 255, 28),
            text: Color32::from_rgb(246, 246, 244),
            muted: Color32::from_rgb(150, 150, 148),
            dim: Color32::from_rgb(64, 64, 66),
            accent: Color32::from_rgb(94, 242, 255),
            accent_alt: Color32::from_rgb(255, 255, 255),
            warn: Color32::from_rgb(246, 246, 244),
            stroke: Color32::from_rgb(246, 246, 244),
            stroke_soft: Color32::from_rgba_unmultiplied(246, 246, 244, 72),
        }
    }
}

pub fn theme() -> Theme {
    Theme::default()
}

pub fn apply_theme(ctx: &Context) {
    let t = theme();
    let mut style = (*ctx.style()).clone();
    let mut visuals = Visuals::dark();
    visuals.override_text_color = Some(t.text);
    visuals.panel_fill = Color32::TRANSPARENT;
    visuals.window_fill = t.panel;
    visuals.extreme_bg_color = t.bg_deep;
    visuals.faint_bg_color = t.panel;
    visuals.widgets.noninteractive.bg_fill = t.panel;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, t.text);
    visuals.widgets.inactive.bg_fill = t.panel_alt;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, t.text);
    visuals.widgets.hovered.bg_fill = t.panel_hot;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, t.text);
    visuals.widgets.active.bg_fill = t.accent;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, t.bg_deep);
    visuals.widgets.open.bg_fill = t.panel_hot;
    visuals.selection.bg_fill = t.accent;
    visuals.selection.stroke = Stroke::new(1.0, t.bg_deep);
    visuals.window_corner_radius = 0.0.into();
    visuals.widgets.noninteractive.corner_radius = 0.0.into();
    visuals.widgets.inactive.corner_radius = 0.0.into();
    visuals.widgets.hovered.corner_radius = 0.0.into();
    visuals.widgets.active.corner_radius = 0.0.into();
    visuals.widgets.open.corner_radius = 0.0.into();
    style.visuals = visuals;

    style.spacing.item_spacing = vec2(10.0, 8.0);
    style.spacing.button_padding = vec2(12.0, 6.0);
    style.spacing.slider_width = 180.0;
    style
        .text_styles
        .insert(TextStyle::Heading, FontId::new(30.0, FontFamily::Monospace));
    style
        .text_styles
        .insert(TextStyle::Body, FontId::new(15.0, FontFamily::Monospace));
    style
        .text_styles
        .insert(TextStyle::Button, FontId::new(14.0, FontFamily::Monospace));
    style
        .text_styles
        .insert(TextStyle::Small, FontId::new(12.0, FontFamily::Monospace));
    ctx.set_style(style);
}

pub fn show_perf_hud(ctx: &Context, stats: FrameStats) {
    let t = theme();
    egui::Area::new(Id::new("engine_perf_hud"))
        .anchor(Align2::RIGHT_TOP, [-8.0, 8.0])
        .interactable(false)
        .show(ctx, |ui| {
            Frame::NONE
                .fill(Color32::from_rgba_unmultiplied(
                    t.bg_deep.r(),
                    t.bg_deep.g(),
                    t.bg_deep.b(),
                    210,
                ))
                .stroke(Stroke::new(1.0, t.stroke_soft))
                .inner_margin(Margin::same(8))
                .corner_radius(0.0)
                .show(ui, |ui| {
                    ui.set_min_width(138.0);
                    ui.label(command_label("ENGINE", 11.0));
                    key_value(ui, "FPS", format!("{:.1}", stats.fps), 1.0);
                    key_value(ui, "AVG", format!("{:.1}", stats.avg_fps), 1.0);
                    key_value(ui, "FRAME", format!("{:.2} ms", stats.frame_time_ms), 1.0);
                    key_value(
                        ui,
                        "FIXED",
                        format!("{}/{}", stats.fixed_steps, stats.max_fixed_steps),
                        1.0,
                    );
                    let debt = format!("{:.2} ms", stats.fixed_debt_ms);
                    let label = if stats.hit_fixed_step_cap {
                        RichText::new("DEBT")
                            .font(FontId::new(12.0, FontFamily::Monospace))
                            .color(t.accent)
                    } else {
                        RichText::new("DEBT")
                            .font(FontId::new(12.0, FontFamily::Monospace))
                            .color(t.muted)
                    };
                    ui.horizontal(|ui| {
                        ui.label(label);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(debt)
                                    .font(FontId::new(12.0, FontFamily::Monospace))
                                    .strong()
                                    .color(if stats.hit_fixed_step_cap {
                                        t.accent
                                    } else {
                                        t.text
                                    }),
                            );
                        });
                    });
                });
        });
}

pub fn paint_screen(ctx: &Context, id: impl std::hash::Hash, rect: Rect) {
    let t = theme();
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Background, Id::new(id)));
    painter.rect_filled(rect, 0.0, t.bg);

    let top = Rect::from_min_size(rect.min, vec2(rect.width(), 36.0));
    painter.rect_filled(top, 0.0, t.bg_deep);
    painter.line_segment(
        [top.left_bottom(), top.right_bottom()],
        Stroke::new(1.0, t.stroke_soft),
    );

    let step = 64.0;
    let mut x = rect.left();
    while x <= rect.right() {
        painter.line_segment(
            [pos2(x, rect.top()), pos2(x, rect.bottom())],
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 8)),
        );
        x += step;
    }
    let mut y = rect.top();
    while y <= rect.bottom() {
        painter.line_segment(
            [pos2(rect.left(), y), pos2(rect.right(), y)],
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 7)),
        );
        y += step;
    }
}

pub fn panel_frame() -> Frame {
    let t = theme();
    Frame::NONE
        .fill(Color32::from_rgba_unmultiplied(
            t.panel.r(),
            t.panel.g(),
            t.panel.b(),
            235,
        ))
        .stroke(Stroke::new(1.0, t.stroke_soft))
        .inner_margin(Margin::same(16))
        .corner_radius(0.0)
}

pub fn section_frame() -> Frame {
    let t = theme();
    Frame::NONE
        .fill(Color32::from_rgba_unmultiplied(
            t.panel_alt.r(),
            t.panel_alt.g(),
            t.panel_alt.b(),
            210,
        ))
        .stroke(Stroke::new(1.0, t.stroke_soft))
        .inner_margin(Margin::same(12))
        .corner_radius(0.0)
}

pub fn title(text: impl Into<String>, size: f32) -> RichText {
    RichText::new(text.into())
        .font(FontId::new(size, FontFamily::Monospace))
        .strong()
        .color(theme().text)
}

pub fn command_label(text: impl Into<String>, size: f32) -> RichText {
    RichText::new(text.into().to_ascii_uppercase())
        .font(FontId::new(size, FontFamily::Monospace))
        .strong()
        .color(theme().accent)
}

pub fn muted(text: impl Into<String>, size: f32) -> RichText {
    RichText::new(text.into())
        .font(FontId::new(size, FontFamily::Monospace))
        .color(theme().muted)
}

pub fn menu_button(label: &str, focused: bool, scale: f32) -> Button<'static> {
    let t = theme();
    let caption = format!("<{} />", label);
    Button::new(
        RichText::new(caption)
            .font(FontId::new(18.0 * scale, FontFamily::Monospace))
            .strong()
            .color(if focused { t.bg_deep } else { t.text }),
    )
    .fill(if focused { t.text } else { t.panel })
    .stroke(Stroke::new(
        if focused { 2.0 } else { 1.0 },
        if focused { t.text } else { t.stroke_soft },
    ))
    .corner_radius(0.0)
}

pub fn command_button(label: &str, focused: bool, scale: f32) -> Button<'static> {
    let t = theme();
    Button::new(
        RichText::new(label.to_ascii_uppercase())
            .font(FontId::new(14.0 * scale, FontFamily::Monospace))
            .strong()
            .color(if focused { t.bg_deep } else { t.text }),
    )
    .fill(if focused { t.text } else { t.panel_alt })
    .stroke(Stroke::new(
        if focused { 2.0 } else { 1.0 },
        if focused { t.text } else { t.stroke_soft },
    ))
    .corner_radius(0.0)
}

pub fn tab(ui: &mut Ui, label: &str, selected: bool, scale: f32) -> Response {
    let t = theme();
    let response = ui.add_sized(
        [112.0 * scale, 34.0 * scale],
        Button::new(
            RichText::new(label.to_ascii_uppercase())
                .font(FontId::new(13.0 * scale, FontFamily::Monospace))
                .strong()
                .color(if selected { t.bg_deep } else { t.text }),
        )
        .fill(if selected { t.text } else { t.panel_alt })
        .stroke(Stroke::new(
            1.0,
            if selected { t.text } else { t.stroke_soft },
        ))
        .corner_radius(0.0),
    );
    if response.hovered() && !selected {
        let rect = response.rect.shrink(1.0);
        ui.painter().rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, t.accent_alt),
            StrokeKind::Inside,
        );
    }
    response
}

pub fn toggle(ui: &mut Ui, checked: &mut bool, label: &str, scale: f32) -> Response {
    let t = theme();
    let desired = vec2(ui.available_width().min(360.0), 32.0 * scale);
    let (rect, mut response) = ui.allocate_exact_size(desired, Sense::click());
    if response.clicked() {
        *checked = !*checked;
        response.mark_changed();
    }

    let painter = ui.painter();
    let switch_rect = Rect::from_min_size(
        pos2(rect.left(), rect.center().y - 8.0 * scale),
        vec2(34.0 * scale, 16.0 * scale),
    );
    painter.rect_stroke(
        switch_rect,
        0.0,
        Stroke::new(1.0, if *checked { t.accent } else { t.dim }),
        StrokeKind::Inside,
    );
    if *checked {
        painter.rect_filled(switch_rect.shrink(3.0 * scale), 0.0, t.accent);
    }
    painter.text(
        pos2(switch_rect.right() + 10.0 * scale, rect.center().y),
        Align2::LEFT_CENTER,
        label.to_ascii_uppercase(),
        FontId::new(13.0 * scale, FontFamily::Monospace),
        if *checked { t.text } else { t.muted },
    );
    response
}

pub fn slider_f32(
    ui: &mut Ui,
    label: &str,
    value: &mut f32,
    range: RangeInclusive<f32>,
    scale: f32,
    format: impl Fn(f32) -> String,
) -> Response {
    let start = *range.start();
    let end = *range.end();
    slider_impl(ui, label, value, start, end, scale, format)
}

pub fn slider_f32_log(
    ui: &mut Ui,
    label: &str,
    value: &mut f32,
    range: RangeInclusive<f32>,
    scale: f32,
    format: impl Fn(f32) -> String,
) -> Response {
    let start = (*range.start()).max(f32::MIN_POSITIVE);
    let end = (*range.end()).max(start);
    let mut v = value.clamp(start, end).ln();
    let response = slider_impl(ui, label, &mut v, start.ln(), end.ln(), scale, |x| {
        format(x.exp())
    });
    if response.changed() {
        *value = v.exp();
    }
    response
}

pub fn slider_u32(
    ui: &mut Ui,
    label: &str,
    value: &mut u32,
    range: RangeInclusive<u32>,
    scale: f32,
) -> Response {
    let mut v = *value as f32;
    let response = slider_impl(
        ui,
        label,
        &mut v,
        *range.start() as f32,
        *range.end() as f32,
        scale,
        |x| format!("{}", x.round() as u32),
    );
    if response.changed() {
        *value = v.round() as u32;
    }
    response
}

pub fn slider_u16(
    ui: &mut Ui,
    label: &str,
    value: &mut u16,
    range: RangeInclusive<u16>,
    scale: f32,
) -> Response {
    let mut v = *value as f32;
    let response = slider_impl(
        ui,
        label,
        &mut v,
        *range.start() as f32,
        *range.end() as f32,
        scale,
        |x| format!("{}", x.round() as u16),
    );
    if response.changed() {
        *value = v.round() as u16;
    }
    response
}

fn slider_impl(
    ui: &mut Ui,
    label: &str,
    value: &mut f32,
    min: f32,
    max: f32,
    scale: f32,
    format: impl Fn(f32) -> String,
) -> Response {
    let t = theme();
    let desired = vec2(ui.available_width().min(520.0), 42.0 * scale);
    let (rect, mut response) = ui.allocate_exact_size(desired, Sense::click_and_drag());

    if (response.dragged() || response.clicked())
        && let Some(pointer) = response.interact_pointer_pos()
    {
        let track = slider_track_rect(rect, scale);
        let ratio = ((pointer.x - track.left()) / track.width()).clamp(0.0, 1.0);
        *value = min + ratio * (max - min);
        response.mark_changed();
    }

    let ratio = ((*value - min) / (max - min)).clamp(0.0, 1.0);
    let track = slider_track_rect(rect, scale);
    let fill_rect = Rect::from_min_max(
        track.left_top(),
        pos2(track.left() + track.width() * ratio, track.bottom()),
    );
    let thumb_x = track.left() + track.width() * ratio;
    let thumb = Rect::from_center_size(
        pos2(thumb_x, track.center().y),
        vec2(10.0 * scale, 18.0 * scale),
    );

    let painter = ui.painter();
    painter.text(
        rect.left_top(),
        Align2::LEFT_TOP,
        label.to_ascii_uppercase(),
        FontId::new(12.0 * scale, FontFamily::Monospace),
        t.muted,
    );
    painter.text(
        rect.right_top(),
        Align2::RIGHT_TOP,
        format(*value),
        FontId::new(12.0 * scale, FontFamily::Monospace),
        t.accent,
    );
    painter.rect_filled(track, 0.0, t.bg_deep);
    painter.rect_stroke(track, 0.0, Stroke::new(1.0, t.dim), StrokeKind::Inside);
    painter.rect_filled(fill_rect, 0.0, t.accent);
    painter.rect_filled(
        thumb,
        0.0,
        if response.hovered() {
            t.accent_alt
        } else {
            t.text
        },
    );
    response
}

fn slider_track_rect(rect: Rect, scale: f32) -> Rect {
    Rect::from_min_size(
        pos2(rect.left(), rect.bottom() - 15.0 * scale),
        vec2(rect.width(), 8.0 * scale),
    )
}

pub fn key_value(ui: &mut Ui, key: &str, value: impl Into<String>, scale: f32) {
    let t = theme();
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(key.to_ascii_uppercase())
                .font(FontId::new(12.0 * scale, FontFamily::Monospace))
                .color(t.muted),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(value.into())
                    .font(FontId::new(12.0 * scale, FontFamily::Monospace))
                    .strong()
                    .color(t.text),
            );
        });
    });
}

pub fn divider(ui: &mut Ui) {
    let t = theme();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(
        rect,
        0.0,
        Color32::from_rgba_unmultiplied(t.stroke.r(), t.stroke.g(), t.stroke.b(), 48),
    );
}
