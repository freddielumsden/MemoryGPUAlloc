//! Visual style: restrained, monochrome-first palettes (light by default) with
//! a single red signal colour, in the spirit of NOTHING's industrial design.

use std::sync::atomic::{AtomicBool, Ordering};

use egui::{Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle, Visuals};

pub const RADIUS: u8 = 4;
pub const CARD_RADIUS: u8 = 5;

struct Palette {
    bg: Color32,
    panel: Color32,
    card: Color32,
    inset: Color32,
    border: Color32,
    text: Color32,
    text_dim: Color32,
    /// The one signal colour: tutorial outlines, drop targets, key state.
    accent: Color32,
    memory: Color32,
    pool: Color32,
    gpu: Color32,
    danger: Color32,
    widget: Color32,
    widget_hover: Color32,
    widget_active: Color32,
}

const LIGHT: Palette = Palette {
    bg: Color32::from_rgb(0xee, 0xee, 0xeb),
    panel: Color32::from_rgb(0xf7, 0xf7, 0xf5),
    card: Color32::from_rgb(0xff, 0xff, 0xff),
    inset: Color32::from_rgb(0xf3, 0xf3, 0xf1),
    border: Color32::from_rgb(0xd6, 0xd6, 0xd2),
    text: Color32::from_rgb(0x14, 0x14, 0x14),
    text_dim: Color32::from_rgb(0x72, 0x72, 0x6e),
    accent: Color32::from_rgb(0xc8, 0x2a, 0x2a),
    memory: Color32::from_rgb(0x55, 0x6b, 0x67),
    pool: Color32::from_rgb(0x5d, 0x66, 0x75),
    gpu: Color32::from_rgb(0x6a, 0x72, 0x58),
    danger: Color32::from_rgb(0xb0, 0x3a, 0x34),
    widget: Color32::from_rgb(0xff, 0xff, 0xff),
    widget_hover: Color32::from_rgb(0xf0, 0xf0, 0xed),
    widget_active: Color32::from_rgb(0xe6, 0xe6, 0xe2),
};

const DARK: Palette = Palette {
    bg: Color32::from_rgb(0x0f, 0x0f, 0x0f),
    panel: Color32::from_rgb(0x15, 0x15, 0x15),
    card: Color32::from_rgb(0x1b, 0x1b, 0x1b),
    inset: Color32::from_rgb(0x12, 0x12, 0x12),
    border: Color32::from_rgb(0x2e, 0x2e, 0x2e),
    text: Color32::from_rgb(0xe6, 0xe6, 0xe3),
    text_dim: Color32::from_rgb(0x8a, 0x8a, 0x86),
    accent: Color32::from_rgb(0xd0, 0x45, 0x40),
    memory: Color32::from_rgb(0x86, 0x9c, 0x97),
    pool: Color32::from_rgb(0x8e, 0x97, 0xa6),
    gpu: Color32::from_rgb(0x98, 0xa0, 0x84),
    danger: Color32::from_rgb(0xc8, 0x5a, 0x52),
    widget: Color32::from_rgb(0x24, 0x24, 0x24),
    widget_hover: Color32::from_rgb(0x2c, 0x2c, 0x2c),
    widget_active: Color32::from_rgb(0x34, 0x34, 0x34),
};

static DARK_MODE: AtomicBool = AtomicBool::new(false);

fn palette() -> &'static Palette {
    if is_dark() { &DARK } else { &LIGHT }
}

pub fn is_dark() -> bool {
    DARK_MODE.load(Ordering::Relaxed)
}

pub fn bg() -> Color32 {
    palette().bg
}
pub fn panel() -> Color32 {
    palette().panel
}
pub fn card() -> Color32 {
    palette().card
}
pub fn inset() -> Color32 {
    palette().inset
}
pub fn border() -> Color32 {
    palette().border
}
pub fn text() -> Color32 {
    palette().text
}
pub fn text_dim() -> Color32 {
    palette().text_dim
}
pub fn accent() -> Color32 {
    palette().accent
}
pub fn memory() -> Color32 {
    palette().memory
}
pub fn pool() -> Color32 {
    palette().pool
}
pub fn gpu() -> Color32 {
    palette().gpu
}
pub fn danger() -> Color32 {
    palette().danger
}

/// Switches palette and pushes the matching egui style.
pub fn apply(ctx: &egui::Context, dark: bool) {
    DARK_MODE.store(dark, Ordering::Relaxed);
    let p = palette();

    // Pin the theme so the browser's preference cannot swap the style out.
    ctx.set_theme(if dark { egui::Theme::Dark } else { egui::Theme::Light });
    let mut visuals = if dark { Visuals::dark() } else { Visuals::light() };
    visuals.panel_fill = p.panel;
    visuals.window_fill = p.panel;
    visuals.extreme_bg_color = p.inset;
    visuals.faint_bg_color = p.card;
    visuals.override_text_color = Some(p.text);
    visuals.selection.bg_fill = p.accent.gamma_multiply(0.25);
    visuals.selection.stroke = Stroke::new(1.0_f32, p.accent);
    visuals.window_corner_radius = CornerRadius::same(CARD_RADIUS);
    visuals.menu_corner_radius = CornerRadius::same(CARD_RADIUS);
    visuals.window_stroke = Stroke::new(1.0_f32, p.border);

    let w = &mut visuals.widgets;
    for wv in [&mut w.noninteractive, &mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
        wv.corner_radius = CornerRadius::same(RADIUS);
        wv.expansion = 0.0;
        wv.fg_stroke = Stroke::new(1.0_f32, p.text);
    }
    w.noninteractive.bg_fill = p.card;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.border);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.text_dim);
    for (wv, fill, stroke) in [
        (&mut w.inactive, p.widget, p.border),
        (&mut w.hovered, p.widget_hover, p.text_dim),
        (&mut w.active, p.widget_active, p.text),
        (&mut w.open, p.widget_active, p.text_dim),
    ] {
        wv.bg_fill = fill;
        wv.weak_bg_fill = fill;
        wv.bg_stroke = Stroke::new(1.0_f32, stroke);
    }
    ctx.set_visuals(visuals);

    ctx.style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 4.0);
        style.text_styles.insert(TextStyle::Heading, FontId::new(15.0, FontFamily::Proportional));
        style.text_styles.insert(TextStyle::Body, FontId::new(13.0, FontFamily::Proportional));
        style.text_styles.insert(TextStyle::Button, FontId::new(12.5, FontFamily::Monospace));
        style.text_styles.insert(TextStyle::Small, FontId::new(10.5, FontFamily::Proportional));
        style.text_styles.insert(TextStyle::Monospace, FontId::new(12.0, FontFamily::Monospace));
    });
}
