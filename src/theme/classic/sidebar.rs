use super::*;

pub fn sidebar_toggle(ui: &mut Ui, rect: Rect, compact: bool) -> Response {
    let response = ui
        .interact(
            rect,
            ui.id().with("sidebar_width_toggle"),
            egui::Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(if compact {
            "Click to expand the sidebar"
        } else {
            "Click to collapse the sidebar"
        });
    let center = rect.center();
    for right in [false, true] {
        let color = if response.hovered() && right == compact {
            Color32::WHITE
        } else {
            Color32::from_rgb(0, 119, 158)
        };
        let direction = if right { 1. } else { -1. };
        let base = center + Vec2::new(direction * 1., 0.);
        let points = vec![
            base + Vec2::new(0., -2.),
            base + Vec2::new(direction * 4., -2.),
            base + Vec2::new(direction * 4., -5.),
            base + Vec2::new(direction * 10., 0.),
            base + Vec2::new(direction * 4., 5.),
            base + Vec2::new(direction * 4., 2.),
            base + Vec2::new(0., 2.),
        ];
        ui.painter()
            .add(egui::Shape::convex_polygon(points, color, Stroke::NONE));
    }
    response
}

#[cfg(test)]
mod register_menu_tests {
    use super::*;

    fn frame(ctx: &egui::Context, pointer: Option<egui::Pos2>) -> (egui::FullOutput, [Rect; 3]) {
        let mut rects = [Rect::NOTHING; 3];
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(600., 700.))),
                events: pointer.map(egui::Event::PointerMoved).into_iter().collect(),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let trigger = ui.button("Edit Transactions");
                    egui::Popup::menu(&trigger)
                        .open(true)
                        .frame(register_menu_frame())
                        .show(|ui| {
                            register_menu_style(ui);
                            rects[0] = ui
                                .add(
                                    egui::Button::new("Payee settings...")
                                        .frame_when_inactive(false),
                                )
                                .rect;
                            rects[1] = ui
                                .menu_button("Categorize As", |ui| {
                                    let _ = ui.button("Groceries");
                                })
                                .response
                                .rect;
                            rects[2] = ui
                                .add_enabled(
                                    false,
                                    egui::Button::new("Approve").frame_when_inactive(false),
                                )
                                .rect;
                        });
                });
            },
        );
        (output, rects)
    }

    #[test]
    fn ordinary_items_and_submenus_both_highlight_blue_with_white_text() {
        for (target, label) in [(0, "Payee settings..."), (1, "Categorize As")] {
            let ctx = egui::Context::default();
            ctx.style_mut(|style| style.animation_time = 0.);
            frame(&ctx, None);
            let (_, rects) = frame(&ctx, None);
            let point = rects[target].center();
            frame(&ctx, Some(point));
            let (output, _) = frame(&ctx, Some(point));
            assert!(
                output
                    .shapes
                    .iter()
                    .any(|s| matches!(&s.shape, egui::Shape::Rect(r)
                if r.rect.contains(point) && r.fill == Color32::from_rgb(82, 151, 249)))
            );
            assert!(
                output
                    .shapes
                    .iter()
                    .any(|s| matches!(&s.shape, egui::Shape::Text(t)
                if t.galley.text().contains(label) && t.fallback_color == Color32::WHITE))
            );
        }
    }

    #[test]
    fn disabled_menu_items_do_not_highlight_when_hovered() {
        let ctx = egui::Context::default();
        frame(&ctx, None);
        let (_, rects) = frame(&ctx, None);
        let point = rects[2].center();
        let (output, _) = frame(&ctx, Some(point));
        assert!(
            !output
                .shapes
                .iter()
                .any(|s| matches!(&s.shape, egui::Shape::Rect(r)
            if r.rect.contains(point) && r.fill == Color32::from_rgb(82, 151, 249)))
        );
    }
}
