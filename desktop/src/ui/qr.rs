//! QR rendering.
//!
//! The code is drawn as plain rectangles instead of a texture: a link is ~90
//! characters, so the matrix is around 33×33 modules, and the whole thing is
//! redrawn only while its tab is open. Avoiding a texture keeps the UI free of
//! image-atlas bookkeeping and of a second decoder dependency.

use eframe::egui::{self, Color32, Rect, Sense, Vec2};

/// Key used to cache the generated matrix per link.
pub struct QrMatrix {
    pub modules: usize,
    pub dark: Vec<bool>,
}

impl QrMatrix {
    pub fn build(data: &str) -> Option<Self> {
        let code = qrcode::QrCode::new(data.as_bytes()).ok()?;
        let modules = code.width();
        let dark = code
            .to_colors()
            .into_iter()
            .map(|color| color == qrcode::types::Color::Dark)
            .collect();
        Some(Self { modules, dark })
    }
}

/// Draw `matrix` into `side` logical pixels, with a white quiet zone.
pub fn draw(ui: &mut egui::Ui, matrix: &QrMatrix, side: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(side), Sense::hover());
    let painter = ui.painter();

    // Scanners need the light background and the quiet zone; on a dark theme a
    // transparent QR is unreadable for most camera apps.
    painter.rect_filled(rect, 6.0, Color32::WHITE);

    const QUIET: usize = 2;
    let total = matrix.modules + QUIET * 2;
    let cell = side / total as f32;
    let origin = rect.min + egui::vec2(cell * QUIET as f32, cell * QUIET as f32);

    for y in 0..matrix.modules {
        for x in 0..matrix.modules {
            if !matrix.dark[y * matrix.modules + x] {
                continue;
            }
            let min = origin + egui::vec2(x as f32 * cell, y as f32 * cell);
            // A hair of overlap hides seams from fractional pixel positions.
            let size = Vec2::splat(cell + 0.4);
            painter.rect_filled(Rect::from_min_size(min, size), 0.0, Color32::BLACK);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_is_built_for_a_real_link() {
        let link = format!(
            "tg://proxy?server=192.168.1.5&port=1443&secret=dd{}",
            "ab".repeat(16)
        );
        let matrix = QrMatrix::build(&link).expect("qr");
        assert!(matrix.modules >= 21);
        assert_eq!(matrix.dark.len(), matrix.modules * matrix.modules);
        assert!(matrix.dark.iter().any(|d| *d));
    }
}
