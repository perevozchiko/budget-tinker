use super::*;

#[test]
fn opening_a_draft_reveals_its_fields_and_save_buttons_in_long_registers() {
    for count in [300, 3_000] {
        let (b, account) = budget();
        let mut draft = Draft::new(&b, Some(&account), None).unwrap();
        let ctx = egui::Context::default();
        let mut fonts = egui::FontDefinitions::default();
        fonts.families.insert(
            egui::FontFamily::Name("arial-bold".into()),
            fonts.families[&egui::FontFamily::Proportional].clone(),
        );
        ctx.set_fonts(fonts);
        let mut editor_rect = Rect::NOTHING;
        let mut viewport = Rect::NOTHING;
        for frame in 0..8 {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(1100., 600.),
                    )),
                    time: Some(frame as f64 * 0.1),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(450.)
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.y = 0.;
                                ui.allocate_space(Vec2::new(900., count as f32 * 24.));
                                let editor = ui.scope(|ui| {
                                    draft.show(
                                        ui,
                                        &[28., 0., 83., 170., 150., 140., 79., 79., 31., 28.],
                                        &b,
                                    )
                                });
                                editor_rect = editor.response.rect;
                                viewport = ui.clip_rect();
                            });
                    });
                },
            );
        }
        assert!(
            viewport.contains_rect(editor_rect),
            "{count} rows: editor {editor_rect:?} outside viewport {viewport:?}"
        );
        assert!(editor_rect.height() >= 56., "Save/Cancel must be included");
    }
}
