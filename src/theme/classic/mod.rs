//! Original legacy-style controls: silver bevels and small green action tiles.
use eframe::egui::{self, Color32, FontId, Rect, Response, Stroke, Ui, Vec2, Widget, WidgetText};

pub const CHROME: Color32 = Color32::from_rgb(219, 219, 219);
pub const TEXT: Color32 = Color32::from_rgb(80, 80, 80);
pub const BORDER: Color32 = Color32::from_rgb(133, 136, 135);

pub fn gradient(p: &egui::Painter, r: Rect, radius: f32, top: Color32, bottom: Color32) {
    let mut mesh = egui::Mesh::default();
    let rows = r.height().ceil().clamp(1., 40.) as usize;
    let inset = |y: f32| {
        let d = (radius - y.min(r.height() - y)).max(0.);
        radius - (radius * radius - d * d).max(0.).sqrt()
    };
    for i in 0..=rows {
        let t = i as f32 / rows as f32;
        let y = t * r.height();
        let channel = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        let color = Color32::from_rgba_premultiplied(
            channel(top.r(), bottom.r()),
            channel(top.g(), bottom.g()),
            channel(top.b(), bottom.b()),
            channel(top.a(), bottom.a()),
        );
        for x in [r.left() + inset(y), r.right() - inset(y)] {
            mesh.colored_vertex(egui::pos2(x, r.top() + y), color);
        }
        if i > 0 {
            let n = i as u32 * 2;
            mesh.add_triangle(n - 2, n - 1, n);
            mesh.add_triangle(n - 1, n + 1, n);
        }
    }
    p.add(egui::Shape::mesh(mesh));
}

mod buttons;
pub use buttons::*;

mod navigation;
pub use navigation::*;

mod dates;
pub use dates::*;

mod badges;
pub use badges::*;

mod menus;
pub use menus::*;

mod sidebar;
pub use sidebar::*;
