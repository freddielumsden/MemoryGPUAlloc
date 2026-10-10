//! Small reusable egui building blocks (no backend knowledge).

use std::any::Any;

use egui::{
    Align2, Color32, CornerRadius, DragAndDrop, FontId, Frame, Margin, Response, RichText, Sense, Stroke, Ui, vec2,
};

use crate::theme;

/// Standard rounded card.
pub fn card() -> Frame {
    Frame::new()
        .fill(theme::CARD)
        .stroke(Stroke::new(1.0_f32, theme::BORDER))
        .corner_radius(CornerRadius::same(theme::CARD_RADIUS))
        .inner_margin(Margin::same(10))
}

/// Recessed area inside a card.
pub fn inset() -> Frame {
    Frame::new()
        .fill(theme::INSET)
        .stroke(Stroke::new(1.0_f32, theme::BORDER))
        .corner_radius(CornerRadius::same(theme::RADIUS))
        .inner_margin(Margin::same(8))
}

/// Like `Ui::dnd_drop_zone`, but only reacts to payloads `accepts` approves,
/// so nested zones for different payloads do not steal each other's drops.
/// Returns the accepted payload on the frame it is released.
pub fn drop_zone<P, R>(
    ui: &mut Ui,
    frame: Frame,
    accepts: impl Fn(&P) -> bool,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> (R, Option<P>)
where
    P: Any + Send + Sync + Clone,
{
    let acceptable = DragAndDrop::payload::<P>(ui.ctx()).is_some_and(|p| accepts(&p));

    let mut prepared = frame.begin(ui);
    let inner = add_contents(&mut prepared.content_ui);
    let response = prepared.allocate_space(ui);
    let hovering = acceptable && response.contains_pointer();

    if acceptable {
        let color = if hovering { theme::ACCENT } else { theme::ACCENT.linear_multiply(0.45) };
        prepared.frame.stroke = Stroke::new(if hovering { 2.0_f32 } else { 1.0 }, color);
        if hovering {
            prepared.frame.fill = prepared.frame.fill.lerp_to_gamma(theme::ACCENT, 0.08);
        }
    }
    prepared.paint(ui);

    let mut dropped = None;
    if hovering && ui.input(|i| i.pointer.any_released()) {
        dropped = DragAndDrop::take_payload::<P>(ui.ctx()).map(|p| (*p).clone());
    }
    (inner, dropped)
}

/// Uppercase section title with a trailing hairline and an optional right-hand note.
pub fn section_header(ui: &mut Ui, title: &str, note: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(title.to_uppercase()).strong().color(theme::TEXT_DIM).size(11.5));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(note).color(theme::TEXT_DIM).small());
            let rect = ui.available_rect_before_wrap();
            let y = rect.center().y;
            ui.painter().hline(rect.x_range().shrink(6.0), y, Stroke::new(1.0_f32, theme::BORDER));
        });
    });
}

/// Small coloured status badge.
pub fn badge(ui: &mut Ui, text: &str, color: Color32) -> Response {
    Frame::new()
        .fill(color.linear_multiply(0.18))
        .stroke(Stroke::new(1.0_f32, color.linear_multiply(0.8)))
        .corner_radius(CornerRadius::same(theme::RADIUS))
        .inner_margin(Margin::symmetric(6, 1))
        .show(ui, |ui| ui.label(RichText::new(text).color(color).small().strong()))
        .response
}

/// A rounded hardware-style tile: coloured top strip, title, subtitle and an
/// optional status LED.
pub fn tile(ui: &mut Ui, size: egui::Vec2, color: Color32, title: &str, subtitle: &str, led: Option<bool>) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = CornerRadius::same(theme::RADIUS);
        let hovered = response.hovered();
        painter.rect(
            rect,
            radius,
            if hovered { theme::CARD.lerp_to_gamma(color, 0.12) } else { theme::CARD },
            Stroke::new(1.0_f32, if hovered { color } else { color.linear_multiply(0.55) }),
            egui::StrokeKind::Inside,
        );
        let strip = egui::Rect::from_min_size(rect.min, vec2(rect.width(), 4.0));
        painter.rect_filled(strip, CornerRadius { nw: theme::RADIUS, ne: theme::RADIUS, sw: 0, se: 0 }, color);
        let text_center = rect.center() + vec2(0.0, 2.0);
        if subtitle.is_empty() {
            painter.text(text_center, Align2::CENTER_CENTER, title, FontId::monospace(11.0), theme::TEXT);
        } else {
            painter.text(
                text_center - vec2(0.0, 6.0),
                Align2::CENTER_CENTER,
                title,
                FontId::monospace(11.0),
                theme::TEXT,
            );
            painter.text(
                text_center + vec2(0.0, 7.0),
                Align2::CENTER_CENTER,
                subtitle,
                FontId::proportional(9.5),
                theme::TEXT_DIM,
            );
        }
        if let Some(on) = led {
            let c = rect.right_top() + vec2(-7.0, 10.0);
            let color = if on { theme::GPU } else { theme::DANGER };
            painter.circle_filled(c, 3.0, color);
            painter.circle_stroke(c, 5.0, Stroke::new(1.0_f32, color.linear_multiply(0.3)));
        }
    }
    response
}

/// Lays out `count` fixed-width items in rows that fit the available width.
pub fn wrap_grid(ui: &mut Ui, count: usize, item_width: f32, mut add_item: impl FnMut(&mut Ui, usize)) {
    let spacing = ui.spacing().item_spacing.x;
    let per_row = (((ui.available_width() + spacing) / (item_width + spacing)).floor() as usize).max(1);
    for row_start in (0..count).step_by(per_row) {
        ui.horizontal_top(|ui| {
            for i in row_start..(row_start + per_row).min(count) {
                ui.allocate_ui_with_layout(vec2(item_width, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(item_width);
                    add_item(ui, i);
                });
            }
        });
    }
}

/// Compact numeric text field. Returns the parsed value (if valid).
pub fn count_input(ui: &mut Ui, text: &mut String) -> Option<usize> {
    let valid = text.trim().parse::<usize>().ok();
    let edit = egui::TextEdit::singleline(text)
        .desired_width(46.0)
        .horizontal_align(egui::Align::Center)
        .font(egui::TextStyle::Monospace)
        .text_color(if valid.is_some() { theme::TEXT } else { theme::DANGER });
    ui.add(edit).on_hover_text("Amount");
    valid
}
