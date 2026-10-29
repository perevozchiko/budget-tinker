use super::*;

#[test]
fn editing_and_quick_menu_preserve_every_budget_row_position() {
    for width in [1050., 1440.] {
        let root = std::env::temp_dir().join(format!("budget-cell-layout-{}", id()));
        let mut b = Budget::new("EUR");
        b.add_account("Synthetic cash", "EUR", true, 100_000, "2026-10-01")
            .unwrap();
        let category = b.categories[6].id.clone();
        b.allocate("2026-10", &category, 12345).unwrap();
        let ctx = egui::Context::default();
        let mut store = Store::open(&root.join("demo.budgettinker")).unwrap();
        store.save(&b).unwrap();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            store,
            b,
            false,
        );
        app.month = "2026-10".into();
        let mut time = 0.;
        let mut frame = |app: &mut App, events| {
            time += 0.02;
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(width, 900.),
                    )),
                    events,
                    time: Some(time),
                    focused: true,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| app.budget(ui));
                },
            );
        };
        frame(&mut app, vec![]);
        frame(&mut app, vec![]);
        let rows = |app: &App| {
            app.b
                .categories
                .iter()
                .map(|c| {
                    ctx.data(|d| {
                        d.get_temp::<egui::Rect>(egui::Id::new(("budget_category_row", &c.id)))
                            .unwrap()
                    })
                })
                .collect::<Vec<_>>()
        };
        let original_rows = rows(&app);
        let original = serde_json::to_value(&app.b).unwrap();
        let cell: egui::Rect = ctx.data(|d| {
            d.get_temp(egui::Id::new(("budget_cell", "2026-10", &category)))
                .unwrap()
        });
        for pos in [
            cell.center(),
            egui::pos2(cell.right() + 9., cell.center().y),
        ] {
            for pressed in [true, false] {
                frame(
                    &mut app,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
            frame(&mut app, vec![]);
            assert_eq!(
                rows(&app),
                original_rows,
                "editing moved rows at width {width}"
            );
        }
        assert!(egui::Popup::is_any_open(&ctx));
        assert!(app.budget_editor.is_editing(&BudgetCell {
            month: "2026-10".into(),
            category: category.clone()
        }));
        // Cancel the popup and the edit, then confirm persistence has not changed.
        for _ in 0..2 {
            frame(
                &mut app,
                vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: Some(egui::Key::Escape),
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            frame(&mut app, vec![]);
        }
        assert_eq!(rows(&app), original_rows);
        assert!(app.undo.is_empty());
        assert_eq!(
            serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
            original
        );
        drop(app);
        std::fs::remove_dir_all(root).unwrap();
    }
}
