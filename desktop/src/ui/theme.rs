//! Colours and the two or three shared widgets of the administration window.
//!
//! The palette matches the Android build and the business card (near-black
//! background, off-white text), but nothing else is copied: no gradients, no
//! glass panels — the desktop window is a tool, and it should read like one.

use eframe::egui::{self, Color32, Frame, Margin, RichText, Rounding, Stroke};

pub const BG: Color32 = Color32::from_rgb(0x0B, 0x0B, 0x0D);
pub const PANEL: Color32 = Color32::from_rgb(0x12, 0x12, 0x15);
pub const LINE: Color32 = Color32::from_rgb(0x26, 0x26, 0x2C);
pub const TEXT: Color32 = Color32::from_rgb(0xF4, 0xF4, 0xF6);
pub const MUTED: Color32 = Color32::from_rgb(0x9A, 0x9A, 0xA2);
pub const OK: Color32 = Color32::from_rgb(0x53, 0xD1, 0x69);
pub const WARN: Color32 = Color32::from_rgb(0xE5, 0xA5, 0x4B);
pub const BAD: Color32 = Color32::from_rgb(0xE5, 0x5B, 0x5B);

pub fn apply(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();

    let visuals = &mut style.visuals;
    visuals.panel_fill = BG;
    visuals.window_fill = BG;
    visuals.extreme_bg_color = PANEL;
    visuals.faint_bg_color = PANEL;
    visuals.override_text_color = Some(TEXT);
    visuals.window_stroke = Stroke::new(1.0_f32, LINE);
    visuals.selection.bg_fill = Color32::from_rgb(0x2E, 0x2E, 0x36);

    visuals.widgets.noninteractive.bg_fill = PANEL;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, LINE);

    visuals.widgets.inactive.bg_fill = PANEL;
    visuals.widgets.inactive.weak_bg_fill = PANEL;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, LINE);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x1D, 0x1D, 0x22);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x1D, 0x1D, 0x22);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(0x3A, 0x3A, 0x44));

    visuals.widgets.active.bg_fill = Color32::from_rgb(0x24, 0x24, 0x2B);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(0x24, 0x24, 0x2B);

    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    ctx.set_style(style);
}

/// A titled block: small muted caption, then a bordered rectangle with `body`.
pub fn section(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(2.0);
    ui.label(RichText::new(title).size(12.0).color(MUTED));
    Frame::none()
        .fill(PANEL)
        .stroke(Stroke::new(1.0_f32, LINE))
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin::same(12.0))
        .show(ui, body);
    ui.add_space(10.0);
}

/// Coloured status line: dot plus text.
pub fn status_line(ui: &mut egui::Ui, color: Color32, text: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 4.0, color);
        ui.label(RichText::new(text).size(15.0).color(TEXT));
    });
}

/// Monospaced, selectable, muted hint — used for links and addresses.
pub fn mono(ui: &mut egui::Ui, text: &str, size: f32) -> egui::Response {
    ui.add(
        egui::Label::new(
            RichText::new(text)
                .monospace()
                .size(size)
                .color(Color32::from_rgb(0xC9, 0xC9, 0xD2)),
        )
        .wrap(),
    )
}

pub fn hint(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(11.5).color(MUTED));
}
