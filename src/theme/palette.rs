//! sRGB values sampled from the reference after converting its Display ICC profile.
use eframe::egui::{self, Color32, FontId, Rect, Vec2};
pub const SIDEBAR: Color32 = Color32::from_rgb(0, 150, 198);
pub const SELECTED: Color32 = Color32::from_rgb(0, 72, 109);
pub const CARD_TOP: Color32 = Color32::from_rgb(66, 81, 89);
pub const CARD_BOTTOM: Color32 = Color32::from_rgb(59, 69, 78);
pub const ROW: Color32 = Color32::from_rgb(234, 245, 251);
pub const CATEGORY_HOVER: Color32 = Color32::from_rgb(203, 220, 226);
pub const GROUP: Color32 = Color32::from_rgb(231, 244, 251);
pub const GRID: Color32 = Color32::from_rgb(134, 217, 242);
pub const CELL: Color32 = Color32::from_rgb(0, 182, 231);
pub const LIME: Color32 = Color32::from_rgb(164, 204, 65);
pub const TEXT: Color32 = Color32::from_rgb(79, 79, 79);
pub fn bold(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Name("arial-bold".into()))
}
/// Keep named style families usable when the optional macOS Arial fonts are absent.
pub fn ensure_font_families(fonts: &mut egui::FontDefinitions) {
    let fallback = fonts.families[&egui::FontFamily::Proportional].clone();
    for name in ["arial-bold", "arial-bold-italic"] {
        fonts
            .families
            .entry(egui::FontFamily::Name(name.into()))
            .or_insert_with(|| fallback.clone());
    }
}

pub fn fabric(ctx: &egui::Context) -> egui::TextureHandle {
    let size = 128;
    let mut pixels = Vec::with_capacity(size * size);
    for y in 0..size {
        for x in 0..size {
            let wave = (x as f32 * std::f32::consts::TAU / 16.).sin() * 1.4
                + (y as f32 * std::f32::consts::TAU / 16.).sin() * 1.1;
            let thread = match (x % 8, y % 8) {
                (0, _) | (_, 0) => -4.2,
                (1, _) | (_, 1) => 2.4,
                _ => 0.,
            };
            let grain = ((x * 37 + y * 17 + x * y * 3) % 7) as f32 * 0.6 - 1.8;
            let shift = wave + thread + grain;
            pixels.push(Color32::from_rgb(
                0,
                (151. + shift).clamp(0., 255.) as u8,
                (199. + shift).clamp(0., 255.) as u8,
            ));
        }
    }
    ctx.load_texture(
        "woven-sidebar",
        egui::ColorImage::new([size, size], pixels),
        egui::TextureOptions::LINEAR_REPEAT,
    )
}
pub fn paint_fabric(p: &egui::Painter, texture: &egui::TextureHandle, rect: Rect) {
    p.image(
        texture.id(),
        rect,
        Rect::from_min_size(
            egui::pos2(rect.left() / 64., rect.top() / 64.),
            Vec2::new(rect.width() / 64., rect.height() / 64.),
        ),
        Color32::WHITE,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_fonts_render_named_styles_without_replacing_installed_families() {
        let mut fonts = egui::FontDefinitions::default();
        let named = egui::FontFamily::Name("arial-bold".into());
        let installed = fonts.families[&egui::FontFamily::Monospace].clone();
        fonts.families.insert(named.clone(), installed.clone());
        ensure_font_families(&mut fonts);
        assert_eq!(fonts.families[&named], installed);
        let ctx = egui::Context::default();
        ctx.set_fonts(fonts);
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label(egui::RichText::new("BudgetTinker").font(bold(14.)));
                ui.label(egui::RichText::new("Scheduled").font(FontId::new(
                    14.,
                    egui::FontFamily::Name("arial-bold-italic".into()),
                )));
            });
        });
        assert!(!output.shapes.is_empty());
    }
}
