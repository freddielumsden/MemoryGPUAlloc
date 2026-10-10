//! Small reusable egui building blocks (no backend knowledge).

use std::any::Any;

use egui::{
    Align2, Color32, CornerRadius, DragAndDrop, FontId, Frame, Id, LayerId, Margin, Order, Painter, Rect, Response,
    RichText, Sense, Stroke, Ui, vec2,
};

use crate::theme;

/// Standard rounded card.
pub fn card() -> Frame {
    Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(1.0_f32, theme::border()))
        .corner_radius(CornerRadius::same(theme::CARD_RADIUS))
        .inner_margin(Margin::same(10))
}

/// Recessed area inside a card.
pub fn inset() -> Frame {
    Frame::new()
        .fill(theme::inset())
        .stroke(Stroke::new(1.0_f32, theme::border()))
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
) -> (egui::InnerResponse<R>, Option<P>)
where
    P: Any + Send + Sync + Clone,
{
    let acceptable = DragAndDrop::payload::<P>(ui.ctx()).is_some_and(|p| accepts(&p));

    let mut prepared = frame.begin(ui);
    let inner = add_contents(&mut prepared.content_ui);
    let response = prepared.allocate_space(ui);
    let hovering = acceptable && response.contains_pointer();

    if acceptable {
        let color = if hovering { theme::text() } else { theme::text_dim() };
        prepared.frame.stroke = Stroke::new(if hovering { 1.5_f32 } else { 1.0 }, color);
        if hovering {
            prepared.frame.fill = prepared.frame.fill.lerp_to_gamma(theme::text(), 0.04);
        }
    }
    prepared.paint(ui);

    let mut dropped = None;
    if hovering && ui.input(|i| i.pointer.any_released()) {
        dropped = DragAndDrop::take_payload::<P>(ui.ctx()).map(|p| (*p).clone());
    }
    (egui::InnerResponse::new(inner, response), dropped)
}

/// Uppercase section title with a trailing hairline and an optional right-hand note.
pub fn section_header(ui: &mut Ui, title: &str, note: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(title.to_uppercase()).strong().color(theme::text_dim()).size(11.5));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(note).color(theme::text_dim()).small());
            let rect = ui.available_rect_before_wrap();
            let y = rect.center().y;
            ui.painter().hline(rect.x_range().shrink(6.0), y, Stroke::new(1.0_f32, theme::border()));
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

/// What a hardware-style tile shows.
pub struct Tile<'a> {
    pub size: egui::Vec2,
    pub color: Color32,
    pub title: &'a str,
    pub subtitle: &'a str,
    /// Status LED in the corner: `Some(true)` green, `Some(false)` red.
    pub led: Option<bool>,
}

impl Tile<'_> {
    /// A static tile.
    pub fn show(&self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(self.size, Sense::hover());
        if ui.is_rect_visible(rect) {
            self.paint(ui.painter(), rect, response.hovered());
        }
        response
    }

    /// A tile that can be picked up, carrying `payload`.
    ///
    /// Unlike `Ui::dnd_drag_source` this allocates a single rect, so it wraps
    /// correctly inside `horizontal_wrapped`. While dragged, a ghost copy
    /// follows the pointer.
    pub fn show_draggable<P: Any + Send + Sync>(&self, ui: &mut Ui, id: Id, payload: P) -> Response {
        let (rect, _) = ui.allocate_exact_size(self.size, Sense::hover());
        let response = ui.interact(rect, id, Sense::drag());
        if response.dragged() {
            DragAndDrop::set_payload(ui.ctx(), payload);
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            self.paint(ui.painter(), rect, false);
            if let Some(pointer) = ui.ctx().pointer_interact_pos() {
                let painter = ui.ctx().layer_painter(LayerId::new(Order::Tooltip, id));
                self.paint(&painter, Rect::from_center_size(pointer, self.size), true);
            }
        } else if ui.is_rect_visible(rect) {
            self.paint(ui.painter(), rect, response.hovered());
        }
        response.on_hover_cursor(egui::CursorIcon::Grab)
    }

    fn paint(&self, painter: &Painter, rect: Rect, highlight: bool) {
        let color = self.color;
        painter.rect(
            rect,
            CornerRadius::same(theme::RADIUS),
            if highlight { theme::card().lerp_to_gamma(color, 0.12) } else { theme::card() },
            Stroke::new(1.0_f32, if highlight { color } else { color.linear_multiply(0.55) }),
            egui::StrokeKind::Inside,
        );
        let strip = Rect::from_min_size(rect.min, vec2(rect.width(), 4.0));
        painter.rect_filled(strip, CornerRadius { nw: theme::RADIUS, ne: theme::RADIUS, sw: 0, se: 0 }, color);
        let text_center = rect.center() + vec2(0.0, 2.0);
        if self.subtitle.is_empty() {
            painter.text(text_center, Align2::CENTER_CENTER, self.title, FontId::monospace(11.0), theme::text());
        } else {
            let title_pos = text_center - vec2(0.0, 6.0);
            painter.text(title_pos, Align2::CENTER_CENTER, self.title, FontId::monospace(11.0), theme::text());
            let sub_pos = text_center + vec2(0.0, 7.0);
            painter.text(sub_pos, Align2::CENTER_CENTER, self.subtitle, FontId::proportional(9.5), theme::text_dim());
        }
        if let Some(on) = self.led {
            let c = rect.right_top() + vec2(-7.0, 10.0);
            let color = if on { theme::gpu() } else { theme::danger() };
            painter.circle_filled(c, 3.0, color);
            painter.circle_stroke(c, 5.0, Stroke::new(1.0_f32, color.linear_multiply(0.3)));
        }
    }
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
        .text_color(if valid.is_some() { theme::text() } else { theme::danger() });
    ui.add(edit).on_hover_text("Amount");
    valid
}

/// Draws a "comet" that runs anticlockwise around `rect`'s rounded outline.
///
/// `phase` in `[0, 1)` is where the comet's head is along the perimeter. Pass
/// the same phase to every call (e.g. derived from `InputState::time`) so all
/// highlighted widgets orbit in lockstep.
pub fn orbit_outline(painter: &Painter, rect: Rect, radius: f32, phase: f32, color: Color32) {
    let path = rounded_rect_path_anticlockwise(rect, radius);
    let mut lengths = Vec::with_capacity(path.len());
    let mut total = 0.0;
    lengths.push(0.0);
    for w in path.windows(2) {
        total += w[0].distance(w[1]);
        lengths.push(total);
    }
    if total <= 0.0 {
        return;
    }

    painter.add(egui::Shape::closed_line(path.clone(), Stroke::new(1.0_f32, color.gamma_multiply(0.18))));

    let span = total * 0.3;
    let head = phase.rem_euclid(1.0) * total;
    // Sample the trailing span behind the head, wrapping around the closed path.
    const SAMPLES: usize = 48;
    let points: Vec<egui::Pos2> = (0..=SAMPLES)
        .map(|i| point_at(&path, &lengths, total, head - span + span * i as f32 / SAMPLES as f32))
        .collect();
    // Fade the tail in segments so the head reads as the leading edge.
    for (i, seg) in points.windows(2).enumerate() {
        let t = (i + 1) as f32 / SAMPLES as f32;
        painter.line_segment([seg[0], seg[1]], Stroke::new(1.0 + 1.0 * t, color.gamma_multiply(t.powf(0.7))));
    }
}

fn point_at(path: &[egui::Pos2], lengths: &[f32], total: f32, s: f32) -> egui::Pos2 {
    let s = s.rem_euclid(total);
    let i = lengths.partition_point(|&l| l <= s).clamp(1, path.len() - 1);
    let (l0, l1) = (lengths[i - 1], lengths[i]);
    let t = if l1 > l0 { (s - l0) / (l1 - l0) } else { 0.0 };
    path[i - 1].lerp(path[i], t)
}

/// Closed polyline around a rounded rect, ordered anticlockwise on screen
/// (top edge right→left, left edge down, bottom edge left→right, right edge up).
fn rounded_rect_path_anticlockwise(rect: Rect, radius: f32) -> Vec<egui::Pos2> {
    use std::f32::consts::FRAC_PI_2;
    let r = radius.min(rect.width() / 2.0).min(rect.height() / 2.0).max(0.0);
    // Corner centres paired with the angle each arc starts at; every arc sweeps -90°.
    let corners = [
        (egui::pos2(rect.left() + r, rect.top() + r), -FRAC_PI_2),
        (egui::pos2(rect.left() + r, rect.bottom() - r), 2.0 * FRAC_PI_2),
        (egui::pos2(rect.right() - r, rect.bottom() - r), FRAC_PI_2),
        (egui::pos2(rect.right() - r, rect.top() + r), 0.0),
    ];
    const ARC_STEPS: usize = 6;
    let mut path = vec![egui::pos2(rect.right() - r, rect.top())];
    for (centre, start) in corners {
        for step in 0..=ARC_STEPS {
            let a = start - FRAC_PI_2 * step as f32 / ARC_STEPS as f32;
            path.push(centre + r * vec2(a.cos(), a.sin()));
        }
    }
    path.push(path[0]);
    path
}
