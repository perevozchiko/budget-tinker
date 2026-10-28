use super::*;
#[test]
fn long_modal_scrolls_while_footer_remains_visible_in_a_small_window() {
    let ctx = egui::Context::default();
    for _ in 0..3 {
        let mut footer_rect = egui::Rect::NOTHING;
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(1050., 680.),
                )),
                ..Default::default()
            },
            |ctx| {
                modal(ctx, "long_modal").show(ctx, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.;
                    header(ui, "Title", 490.);
                    body(ui, 490., |ui| {
                        for _ in 0..120 {
                            ui.label("Content");
                        }
                    });
                    footer(ui, 490., |ui| {
                        footer_rect = classic::button(ui, "Save").rect;
                    });
                });
            },
        );
        assert!(
            footer_rect.top() > 0. && footer_rect.bottom() < 680.,
            "footer clipped: {footer_rect:?}"
        );
    }
}
#[test]
fn changing_modal_contents_keeps_the_header_and_footer_inside_the_window() {
    let ctx = egui::Context::default();
    for lines in [2, 12, 3, 1] {
        for _ in 0..3 {
            let mut rect = egui::Rect::NOTHING;
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(1200., 800.),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    rect = modal(ctx, "layout_regression")
                        .show(ctx, |ui| {
                            ui.spacing_mut().item_spacing.y = 0.;
                            header(ui, "Title", 575.);
                            body(ui, 575., |ui| {
                                for _ in 0..lines {
                                    ui.label("Content");
                                }
                            });
                            footer(ui, 575., |ui| {
                                classic::button(ui, "Close");
                            });
                        })
                        .response
                        .rect;
                },
            );
            assert!(rect.height() < 500., "modal grew to {rect:?}");
            assert!(
                rect.top() >= 0. && rect.bottom() <= 800.,
                "modal was clipped: {rect:?}"
            );
        }
    }
}
