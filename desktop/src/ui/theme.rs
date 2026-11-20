//! The Android screen's palette and widgets, ported to egui.
//!
//! The values are the `PainHub` object from `ui/Theme.kt` and the shapes from
//! `ui/Components.kt`, so the window reads as the same product as the phone
//! app: near-black paper, one hairline border per card, no gradients, no
//! elevation, and exactly one high-contrast action per screen.

use eframe::egui::{self, Button, Color32, Frame, Margin, Response, RichText, Rounding, Stroke, Vec2};

pub const BG: Color32 = Color32::from_rgb(0x08, 0x08, 0x0A);
pub const PAPER: Color32 = Color32::from_rgb(0x12, 0x12, 0x15);
pub const SURFACE: Color32 = Color32::from_rgb(0x1A, 0x1A, 0x1F);
pub const INK: Color32 = Color32::from_rgb(0xF2, 0xF2, 0xF4);
pub const MUTED: Color32 = Color32::from_rgb(0x8E, 0x8E, 0x96);
pub const LINE: Color32 = Color32::from_rgb(0x26, 0x26, 0x2C);
pub const ACCENT: Color32 = Color32::WHITE;
pub const ON_ACCENT: Color32 = BG;
pub const OK: Color32 = Color32::from_rgb(0x6F, 0xD0, 0x8C);
pub const DANGER: Color32 = Color32::from_rgb(0xFF, 0x7A, 0x7A);

const CARD_RADIUS: f32 = 14.0;
const BUTTON_RADIUS: f32 = 10.0;

/// The author's card page, the same target the phone app opens.
pub const CARD_URL: &str = "https://painhahub.github.io/painhahubcard/";

pub fn apply(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();

    let visuals = &mut style.visuals;
    visuals.panel_fill = BG;
    visuals.window_fill = BG;
    visuals.extreme_bg_color = BG;
    visuals.faint_bg_color = PAPER;
    visuals.override_text_color = Some(INK);
    visuals.window_stroke = Stroke::new(1.0_f32, LINE);
    visuals.selection.bg_fill = SURFACE;

    visuals.widgets.noninteractive.bg_fill = PAPER;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, LINE);

    visuals.widgets.inactive.bg_fill = SURFACE;
    visuals.widgets.inactive.weak_bg_fill = SURFACE;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, LINE);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x24, 0x24, 0x2B);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x24, 0x24, 0x2B);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(0x3A, 0x3A, 0x44));

    visuals.widgets.active.bg_fill = Color32::from_rgb(0x2C, 0x2C, 0x34);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(0x2C, 0x2C, 0x34);

    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.button_padding = Vec2::new(14.0, 9.0);
    ctx.set_style(style);
}

/// One solid colour plus a hairline border — the `SurfaceCard` of the phone app.
pub fn card(ui: &mut egui::Ui, body: impl FnOnce(&mut egui::Ui)) {
    Frame::none()
        .fill(PAPER)
        .stroke(Stroke::new(1.0_f32, LINE))
        .rounding(Rounding::same(CARD_RADIUS))
        .inner_margin(Margin::same(16.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            body(ui);
        });
}

/// Small muted caption above a block.
pub fn caption(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(12.0).color(MUTED));
}

/// Green while the proxy runs, grey otherwise.
pub fn status_dot(ui: &mut egui::Ui, ok: bool) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), egui::Sense::hover());
    ui.painter()
        .circle_filled(rect.center(), 5.0, if ok { OK } else { LINE });
}

/// The only high-contrast action on the screen: white fill, dark ink.
pub fn primary_button(ui: &mut egui::Ui, text: &str, width: f32, enabled: bool) -> Response {
    let (fill, ink) = if enabled {
        (ACCENT, ON_ACCENT)
    } else {
        (SURFACE, MUTED)
    };

    let button = Button::new(RichText::new(text).size(15.0).strong().color(ink))
        .fill(fill)
        .stroke(Stroke::NONE)
        .rounding(Rounding::same(BUTTON_RADIUS));

    ui.add_enabled_ui(enabled, |ui| ui.add_sized([width, 44.0], button))
        .inner
}

/// Secondary action: outlined, same shape as the primary one.
pub fn ghost_button(ui: &mut egui::Ui, text: &str, width: f32) -> Response {
    let button = Button::new(RichText::new(text).size(14.0).color(INK))
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0_f32, LINE))
        .rounding(Rounding::same(BUTTON_RADIUS));

    ui.add_sized([width, 40.0], button)
}

/// Section link in the header: ink when current, muted otherwise.
pub fn nav_item(ui: &mut egui::Ui, selected: bool, text: &str) -> Response {
    let mut label = RichText::new(text).size(13.5);
    label = if selected {
        label.strong().color(INK)
    } else {
        label.color(MUTED)
    };

    ui.add(Button::new(label).frame(false))
}

/// The author's card: role on top, name below, the whole block clickable.
pub fn author_tile(ui: &mut egui::Ui, role: &str, name: &str, width: f32) -> Response {
    let inner = Frame::none()
        .fill(PAPER)
        .stroke(Stroke::new(1.0_f32, LINE))
        .rounding(Rounding::same(CARD_RADIUS))
        .inner_margin(Margin::same(16.0))
        .show(ui, |ui| {
            ui.set_width(width - 32.0);
            ui.label(RichText::new(role).size(11.0).color(MUTED));
            ui.add_space(3.0);
            ui.label(RichText::new(name).size(15.0).strong().color(INK));
        });

    let response = ui.interact(
        inner.response.rect,
        ui.id().with("author-tile"),
        egui::Sense::click(),
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// Monospaced, selectable text — links, addresses, the log.
pub fn mono(ui: &mut egui::Ui, text: &str, size: f32) -> Response {
    ui.add(
        egui::Label::new(RichText::new(text).monospace().size(size).color(INK))
            .wrap()
            .selectable(true),
    )
}

pub fn hint(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(11.0).color(MUTED));
}

pub fn footer(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(11.0).color(MUTED));
}
