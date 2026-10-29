use super::*;

/// Join a rounded pointer to the popup's top border on the popup's own layer.
pub fn popup_pointer(
    painter: &egui::Painter,
    popup: Rect,
    center_x: f32,
    tip_y: f32,
    fill: Color32,
    border: Stroke,
) {
    let baseline = popup.top() + border.width * 0.5;
    if tip_y >= baseline - 2. || popup.width() < 48. {
        return;
    }
    let center = center_x.clamp(popup.left() + 24., popup.right() - 24.);
    let tip = baseline - (baseline - tip_y).min(18.);
    let point = |x, y| egui::pos2(center + x, y);
    let mut contour = Vec::new();
    curve(
        &mut contour,
        [
            point(-20., baseline),
            point(-16., baseline),
            point(-16., baseline),
            point(-13., baseline - 3.),
        ],
    );
    contour.push(point(-2., tip + 1.5));
    curve(
        &mut contour,
        [
            point(-2., tip + 1.5),
            point(-0.7, tip),
            point(0.7, tip),
            point(2., tip + 1.5),
        ],
    );
    contour.push(point(13., baseline - 3.));
    curve(
        &mut contour,
        [
            point(13., baseline - 3.),
            point(16., baseline),
            point(16., baseline),
            point(20., baseline),
        ],
    );

    // This strip covers the frame's top stroke through the attachment. Only the
    // open outer contour is stroked, so there is no horizontal line at its base.
    let mut mesh = egui::Mesh::default();
    for p in &contour {
        mesh.colored_vertex(*p, fill);
        mesh.colored_vertex(egui::pos2(p.x, baseline + 3.), fill);
    }
    for index in 1..contour.len() as u32 {
        let n = index * 2;
        mesh.add_triangle(n - 2, n - 1, n);
        mesh.add_triangle(n - 1, n + 1, n);
    }
    let painter = painter.with_clip_rect(popup.expand(24.).intersect(painter.ctx().screen_rect()));
    painter.add(egui::Shape::mesh(mesh));
    painter.add(egui::Shape::line(contour, border));
}

fn curve(points: &mut Vec<egui::Pos2>, control: [egui::Pos2; 4]) {
    let curve = egui::epaint::CubicBezierShape::from_points_stroke(
        control,
        false,
        Color32::TRANSPARENT,
        Stroke::NONE,
    );
    for step in 0..=8 {
        points.push(curve.sample(step as f32 / 8.));
    }
}
