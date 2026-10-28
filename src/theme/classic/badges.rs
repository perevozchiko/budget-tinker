use super::*;

pub fn icon_button(ui: &mut Ui, rect: Rect, kind: usize, color: Color32) -> Response {
    let response = ui.interact(
        rect,
        ui.id()
            .with(("icon", kind, rect.min.x.to_bits(), rect.min.y.to_bits())),
        egui::Sense::click(),
    );
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, 2., Color32::from_white_alpha(40));
    }
    crate::icons::draw(ui.painter(), rect.shrink(2.), kind, color);
    response
}

pub fn category_disclosure(ui: &mut Ui, rect: Rect, expand: bool) -> Response {
    let response = ui.interact(
        rect,
        ui.id().with(("categories_disclosure", expand)),
        egui::Sense::click(),
    );
    let p = ui.painter();
    for i in 0..3 {
        let y = rect.top() + 5. + i as f32 * 4.;
        p.rect_filled(
            Rect::from_center_size(egui::pos2(rect.left() + 1., y), Vec2::splat(1.8)),
            0.,
            TEXT,
        );
        p.line_segment(
            [
                egui::pos2(rect.left() + 4., y),
                egui::pos2(rect.left() + 11., y),
            ],
            Stroke::new(1.4_f32, TEXT),
        );
    }
    let x = rect.right() - 4.;
    let y = rect.center().y;
    let d = if expand { 1. } else { -1. };
    p.line_segment(
        [egui::pos2(x, y - 5. * d), egui::pos2(x, y + 5. * d)],
        Stroke::new(1.5_f32, TEXT),
    );
    p.add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(x, y + 6. * d),
            egui::pos2(x - 3., y + 2. * d),
            egui::pos2(x + 3., y + 2. * d),
        ],
        TEXT,
        Stroke::NONE,
    ));
    response
}

/// Padlock shared by reconciled rows and their visibility controls.
pub fn reconciled_lock(painter: &egui::Painter, center: egui::Pos2, green: bool) {
    let (top, bottom, edge) = if green {
        (
            Color32::from_rgb(111, 181, 24),
            Color32::from_rgb(53, 137, 0),
            Color32::from_rgb(57, 115, 0),
        )
    } else {
        (
            Color32::from_gray(86),
            Color32::from_gray(46),
            Color32::from_gray(48),
        )
    };
    let shackle = Rect::from_center_size(center + Vec2::new(0., -3.5), Vec2::new(9., 11.));
    painter.rect_stroke(
        shackle,
        4.,
        Stroke::new(1.6_f32, edge),
        egui::StrokeKind::Inside,
    );
    painter.rect_stroke(
        shackle.shrink(0.8),
        3.2,
        Stroke::new(0.6_f32, top),
        egui::StrokeKind::Inside,
    );
    let body = Rect::from_center_size(center + Vec2::new(0., 3.), Vec2::new(13., 11.));
    gradient(painter, body, 0.5, top, bottom);
    painter.rect_stroke(
        body,
        0.5,
        Stroke::new(0.6_f32, edge),
        egui::StrokeKind::Inside,
    );
    painter.circle_filled(center + Vec2::new(0., 1.5), 1.2, Color32::WHITE);
    painter.line_segment(
        [center + Vec2::new(0., 2.), center + Vec2::new(0., 5.)],
        Stroke::new(1.2_f32, Color32::WHITE),
    );
}

/// Cleared and uncleared use C tiles; reconciled transactions show a green lock.
pub fn cleared_badge(painter: &egui::Painter, center: egui::Pos2, cleared: bool, reconciled: bool) {
    if reconciled {
        reconciled_lock(painter, center, true);
        return;
    }
    let rect = Rect::from_center_size(center, Vec2::splat(16.));
    let (top, bottom, edge, text) = if cleared {
        (
            Color32::from_rgb(104, 174, 20),
            Color32::from_rgb(52, 133, 0),
            Color32::from_rgb(54, 111, 0),
            Color32::WHITE,
        )
    } else {
        (
            Color32::from_gray(244),
            Color32::from_gray(224),
            Color32::from_gray(185),
            Color32::from_gray(157),
        )
    };
    gradient(painter, rect, 1.5, top, bottom);
    painter.rect_stroke(
        rect,
        1.5,
        Stroke::new(0.6_f32, edge),
        egui::StrokeKind::Inside,
    );
    painter.line_segment(
        [
            rect.left_top() + Vec2::new(1., 1.),
            rect.right_top() + Vec2::new(-1., 1.),
        ],
        Stroke::new(0.6_f32, Color32::from_white_alpha(150)),
    );
    painter.text(
        rect.center() + Vec2::new(0., -0.5),
        egui::Align2::CENTER_CENTER,
        "C",
        crate::palette::bold(13.),
        text,
    );
}

pub fn selection_checkbox(painter: &egui::Painter, center: egui::Pos2, checked: bool, mixed: bool) {
    let rect = Rect::from_center_size(center, Vec2::splat(12.));
    let active = checked || mixed;
    gradient(
        painter,
        rect,
        1.,
        if active {
            Color32::from_rgb(229, 244, 252)
        } else {
            Color32::from_gray(250)
        },
        if active {
            Color32::from_rgb(160, 207, 237)
        } else {
            Color32::from_gray(195)
        },
    );
    painter.rect_stroke(
        rect,
        1.,
        Stroke::new(0.8_f32, Color32::from_gray(132)),
        egui::StrokeKind::Inside,
    );
    let ink = Stroke::new(1.5_f32, Color32::from_rgb(28, 46, 57));
    if mixed {
        painter.line_segment(
            [
                rect.left_center() + Vec2::new(3., 0.),
                rect.right_center() - Vec2::new(3., 0.),
            ],
            ink,
        );
    } else if checked {
        let a = rect.left_center() + Vec2::new(2., 0.);
        let b = rect.left_bottom() + Vec2::new(5., -3.);
        let c = rect.right_top() + Vec2::new(-2., 3.);
        painter.line_segment([a, b], ink);
        painter.line_segment([b, c], ink);
    }
}
