use eframe::egui::{self, Color32, Pos2, Rect, Stroke};

pub fn draw(p: &egui::Painter, rect: Rect, kind: usize, color: Color32) {
    let at = |x: f32, y: f32| {
        Pos2::new(
            rect.left() + x * rect.width() / 24.,
            rect.top() + y * rect.height() / 24.,
        )
    };
    let line = |points: &[(f32, f32)], width: f32| {
        p.add(egui::Shape::line(
            points.iter().map(|(x, y)| at(*x, *y)).collect(),
            Stroke::new(width, color),
        ));
    };
    let block = |x: f32, y: f32, w: f32, h: f32| {
        p.rect_filled(Rect::from_min_max(at(x, y), at(x + w, y + h)), 1., color);
    };
    match kind {
        0 => {
            let points = [(3., 2.), (12., 2.), (22., 12.), (12., 22.), (2., 12.)];
            p.add(egui::Shape::convex_polygon(
                points.iter().map(|(x, y)| at(*x, *y)).collect(),
                color,
                Stroke::NONE,
            ));
            p.circle_filled(at(7., 7.), 2., Color32::from_rgb(0, 111, 143));
            line(&[(5., 17.), (12., 24.), (23., 13.)], 1.7);
        }
        1 => {
            block(2., 15., 4., 8.);
            block(9., 10., 4., 13.);
            block(16., 4., 4., 19.);
            line(&[(2., 10.), (8., 4.), (12., 7.), (21., 0.)], 2.);
        }
        2 => {
            p.add(egui::Shape::convex_polygon(
                vec![at(1., 8.), at(12., 1.), at(23., 8.)],
                color,
                Stroke::NONE,
            ));
            block(1., 9., 22., 2.);
            for x in [4., 10., 16.] {
                block(x, 12., 3., 8.);
            }
            block(1., 21., 22., 2.);
        }
        3 => {
            line(&[(3., 8.), (20., 8.), (16., 4.)], 2.2);
            line(&[(21., 16.), (4., 16.), (8., 20.)], 2.2);
        }
        4 => {
            p.rect_stroke(
                Rect::from_min_max(at(2., 4.), at(22., 23.)),
                2.,
                Stroke::new(1.8_f32, color),
                egui::StrokeKind::Inside,
            );
            line(&[(2., 10.), (22., 10.)], 1.5);
            line(&[(7., 1.), (7., 7.)], 2.);
            line(&[(17., 1.), (17., 7.)], 2.);
            for x in [7., 12., 17.] {
                for y in [14., 19.] {
                    p.circle_filled(at(x, y), 1., color);
                }
            }
        }
        6 => {
            line(&[(5., 2.), (19., 2.), (19., 22.), (5., 22.), (5., 2.)], 1.4);
            for y in [8., 12., 16.] {
                line(&[(8., y), (16., y)], 1.2);
            }
        }
        7 => {
            p.add(egui::Shape::convex_polygon(
                vec![
                    at(11., 1.),
                    at(21., 1.),
                    at(14., 10.),
                    at(19., 10.),
                    at(5., 24.),
                    at(10., 13.),
                    at(6., 13.),
                ],
                color,
                Stroke::NONE,
            ));
        }
        _ => {
            line(&[(12., 3.), (12., 21.)], 2.);
            line(&[(3., 12.), (21., 12.)], 2.);
        }
    }
}
