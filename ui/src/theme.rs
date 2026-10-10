//! Visual style: a dark, industrial control-room look with rounded corners.

use egui::{Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle, Visuals};

pub const BG: Color32 = Color32::from_rgb(0x12, 0x15, 0x1a);
pub const PANEL: Color32 = Color32::from_rgb(0x18, 0x1c, 0x22);
pub const CARD: Color32 = Color32::from_rgb(0x1f, 0x24, 0x2c);
pub const INSET: Color32 = Color32::from_rgb(0x15, 0x19, 0x1f);
pub const BORDER: Color32 = Color32::from_rgb(0x2e, 0x35, 0x40);
pub const TEXT: Color32 = Color32::from_rgb(0xd6, 0xdc, 0xe4);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x7d, 0x88, 0x96);

pub const ACCENT: Color32 = Color32::from_rgb(0xf2, 0xa1, 0x2c); // amber
pub const MEMORY: Color32 = Color32::from_rgb(0x3c, 0xb4, 0xa6); // teal
pub const POOL: Color32 = Color32::from_rgb(0x5a, 0x8d, 0xd6); // steel blue
pub const GPU: Color32 = Color32::from_rgb(0x8c, 0xc0, 0x4f); // signal green
pub const DANGER: Color32 = Color32::from_rgb(0xe0, 0x5a, 0x4f);

pub const RADIUS: u8 = 6;
pub const CARD_RADIUS: u8 = 8;

pub fn apply(ctx: &egui::Context) {
    // Pin the dark theme so the browser's light/dark preference cannot swap the style out.
    ctx.set_theme(egui::Theme::Dark);
    let mut visuals = Visuals::dark();
    visuals.panel_fill = PANEL;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = INSET;
    visuals.faint_bg_color = CARD;
    visuals.override_text_color = Some(TEXT);
    visuals.selection.bg_fill = ACCENT.linear_multiply(0.45);
    visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    visuals.window_corner_radius = CornerRadius::same(CARD_RADIUS);
    visuals.menu_corner_radius = CornerRadius::same(CARD_RADIUS);
    visuals.window_stroke = Stroke::new(1.0_f32, BORDER);

    let w = &mut visuals.widgets;
    for wv in [&mut w.noninteractive, &mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
        wv.corner_radius = CornerRadius::same(RADIUS);
    }
    w.noninteractive.bg_fill = CARD;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    w.inactive.bg_fill = Color32::from_rgb(0x2a, 0x31, 0x3b);
    w.inactive.weak_bg_fill = Color32::from_rgb(0x2a, 0x31, 0x3b);
    w.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    w.hovered.bg_fill = Color32::from_rgb(0x34, 0x3c, 0x48);
    w.hovered.weak_bg_fill = Color32::from_rgb(0x34, 0x3c, 0x48);
    w.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT.linear_multiply(0.7));
    w.active.bg_fill = Color32::from_rgb(0x3d, 0x46, 0x53);
    w.active.weak_bg_fill = Color32::from_rgb(0x3d, 0x46, 0x53);
    w.active.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    ctx.set_visuals(visuals);

    ctx.style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 4.0);
        style.text_styles.insert(TextStyle::Heading, FontId::new(15.0, FontFamily::Proportional));
        style.text_styles.insert(TextStyle::Body, FontId::new(13.0, FontFamily::Proportional));
        style.text_styles.insert(TextStyle::Button, FontId::new(13.0, FontFamily::Proportional));
        style.text_styles.insert(TextStyle::Small, FontId::new(10.5, FontFamily::Proportional));
        style.text_styles.insert(TextStyle::Monospace, FontId::new(12.0, FontFamily::Monospace));
    });
}
