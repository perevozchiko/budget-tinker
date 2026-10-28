use super::*;

/// The timeline uses a silver, pointed end cap, with no triangular glyph inside.
pub fn month_arrow(ui: &mut Ui, rect: Rect, right: bool) -> Response {
    let response = ui.interact(
        rect,
        ui.id().with(if right {
            "month_next"
        } else {
            "month_previous"
        }),
        egui::Sense::click(),
    );
    let pressed = response.is_pointer_button_down_on();
    let top = if pressed {
        185
    } else if response.hovered() {
        255
    } else {
        250
    };
    let bottom = if pressed { 225 } else { 203 };
    let r = rect.shrink(0.75);
    let point = |x: f32, y: f32| {
        egui::pos2(
            if right { r.right() - x } else { r.left() + x },
            r.top() + y,
        )
    };
    let vertices = [
        point(0., r.height() / 2.),
        point(7., 1.),
        point(10., 0.),
        point(r.width() - 1., 0.),
        point(r.width(), 1.),
        point(r.width(), r.height() - 1.),
        point(r.width() - 1., r.height()),
        point(10., r.height()),
        point(7., r.height() - 1.),
    ];
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(
        r.center(),
        Color32::from_gray(((top as u16 + bottom as u16) / 2) as u8),
    );
    for vertex in vertices {
        let t = (vertex.y - r.top()) / r.height();
        mesh.colored_vertex(
            vertex,
            Color32::from_gray((top as f32 * (1. - t) + bottom as f32 * t) as u8),
        );
    }
    for i in 0..vertices.len() {
        mesh.add_triangle(0, i as u32 + 1, (i as u32 + 1) % vertices.len() as u32 + 1);
    }
    ui.painter().add(egui::Shape::mesh(mesh));
    ui.painter().add(egui::Shape::closed_line(
        vertices.to_vec(),
        Stroke::new(1_f32, BORDER),
    ));
    ui.painter().line_segment(
        [point(10., 1.5), point(r.width() - 1.5, 1.5)],
        Stroke::new(1_f32, Color32::WHITE),
    );
    response
}

pub fn large_button(ui: &mut Ui, label: &str, size: Vec2, primary: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let down = response.is_pointer_button_down_on();
    let (top, bottom) = if primary {
        (
            Color32::from_rgb(0, if down { 154 } else { 201 }, 5),
            Color32::from_rgb(0, 159, 4),
        )
    } else if down {
        (Color32::from_gray(184), Color32::from_gray(225))
    } else {
        (Color32::from_gray(252), Color32::from_gray(176))
    };
    gradient(
        ui.painter(),
        rect.translate(Vec2::new(0., 1.5)),
        4.,
        Color32::from_black_alpha(40),
        Color32::from_black_alpha(60),
    );
    gradient(ui.painter(), rect, 4., top, bottom);
    ui.painter().rect_stroke(
        rect.shrink(0.5),
        4.,
        Stroke::new(
            1_f32,
            if primary {
                Color32::from_rgb(0, 121, 8)
            } else {
                BORDER
            },
        ),
        egui::StrokeKind::Inside,
    );
    ui.painter().line_segment(
        [
            rect.min + Vec2::new(3., 1.5),
            rect.right_top() + Vec2::new(-3., 1.5),
        ],
        Stroke::new(1_f32, Color32::from_white_alpha(160)),
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        crate::i18n::tr(ui.ctx(), label),
        crate::palette::bold(16.),
        if primary { Color32::WHITE } else { TEXT },
    );
    response
}
