use super::*;

#[test]
fn create_old_debt_budget_and_transfer_undo_redo_persist_atomically() {
    let root = std::env::temp_dir().join(format!("credit-ui-{}", id()));
    let mut b = Budget::new("USD");
    let cash = b
        .add_account("Checking", "USD", true, 200_000, "2026-01-01")
        .unwrap();
    let ctx = egui::Context::default();
    let mut store = Store::open(&root.join("budget.sqlite3")).unwrap();
    store.save(&b).unwrap();
    let mut app = App::new(
        &eframe::CreationContext::_new_kittest(ctx.clone()),
        store,
        b,
        false,
    );
    let original = serde_json::to_value(&app.b).unwrap();
    app.open(DialogKind::Account);
    let mut d = app.dialog.take().unwrap();
    d.fields[0] = "Visa".into();
    d.fields[2] = "-1000".into();
    d.fields[3] = "2026-01-01".into();
    d.fields[4] = "credit".into();
    d.check = true;
    app.save_dialog(&d).unwrap();
    let card = app.b.accounts.last().unwrap().id.clone();
    let debt = app.b.initial_debt_category(&card).unwrap().id.clone();
    assert_eq!(app.b.available("2026-01").unwrap(), 200_000);
    let created = serde_json::to_value(&app.b).unwrap();
    assert_eq!(
        serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
        created
    );
    app.undo(false);
    assert_eq!(serde_json::to_value(&app.b).unwrap(), original);
    assert_eq!(
        serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
        original
    );
    app.undo(true);
    assert_eq!(serde_json::to_value(&app.b).unwrap(), created);
    let mut next = app.b.clone();
    next.allocate("2026-01", &debt, 20_000).unwrap();
    app.commit(next).unwrap();
    let allocated = serde_json::to_value(&app.b).unwrap();
    let mut next = app.b.clone();
    next.transfer(&cash, &card, 20_000, 20_000, "2026-01-10", "Card payment")
        .unwrap();
    app.commit(next).unwrap();
    assert_eq!(app.b.category_balance("2026-02", &debt), -80_000);
    assert_eq!(app.b.available("2026-01").unwrap(), 180_000);
    let paid = serde_json::to_value(&app.b).unwrap();
    app.undo(false);
    assert_eq!(
        serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
        allocated
    );
    app.undo(true);
    assert_eq!(
        serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
        paid
    );
    app.open(DialogKind::AccountSettings(card.clone()));
    let mut d = app.dialog.take().unwrap();
    d.fields[0] = "Family Visa".into();
    app.save_dialog(&d).unwrap();
    assert_eq!(
        app.b.initial_debt_category(&card).unwrap().name,
        "Family Visa"
    );
    app.undo(false);
    assert_eq!(serde_json::to_value(&app.b).unwrap(), paid);
    app.open(DialogKind::EditCategory(debt.clone()));
    let mut d = app.dialog.take().unwrap();
    d.fields[1] = "Accidental rename".into();
    assert!(app.save_dialog(&d).is_err());
    assert_eq!(serde_json::to_value(&app.b).unwrap(), paid);
    // Render the actual budget with its negative carry and category details
    // at both supported window sizes. Done must accept unchanged debt details.
    d.fields[1] = d.fields[8].clone();
    app.save_dialog(&d).unwrap();
    app.month = "2026-01".into();
    for size in [Vec2::new(1440., 900.), Vec2::new(1050., 712.)] {
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.budget(ui));
            },
        );
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(t) if t.galley.text() == "Starting Debt")));
    }
    drop(app);
    std::fs::remove_dir_all(root).unwrap();
}
