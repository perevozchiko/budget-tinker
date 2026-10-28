use super::*;

fn draw(
    app: &mut App,
    ctx: &egui::Context,
    size: Vec2,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
            events,
            ..Default::default()
        },
        |ctx| app.dialog(ctx),
    )
}
fn label_rect(out: &egui::FullOutput, label: &str) -> egui::Rect {
    out.shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(t) if t.galley.text() == label => {
                Some(egui::Rect::from_min_size(t.pos, t.galley.size()))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("Missing label: {label}"))
}
fn click(app: &mut App, ctx: &egui::Context, size: Vec2, point: egui::Pos2) {
    for pressed in [true, false] {
        draw(
            app,
            ctx,
            size,
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
}
#[test]
fn bank_preview_cancel_apply_stale_source_and_undo_redo_through_sqlite() {
    for extension in ["csv", "qif", "ofx"] {
        let root = std::env::temp_dir().join(format!("bank-ui-{}", id()));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join(format!("statement.{extension}"));
        std::fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("tests/fixtures/bank/statement.{extension}")),
            &source,
        )
        .unwrap();
        let mut b = Budget::new("USD");
        let account = b
            .add_account("Checking", "USD", true, 120000, "2026-01-01")
            .unwrap();
        let mut manual = b.transactions[0].clone();
        manual.id = "manual".into();
        manual.date = "2025-12-31".into();
        manual.amount = -350;
        manual.payee = "My coffee".into();
        manual.memo = "Keep manual month".into();
        manual.category = Some(b.categories[0].id.clone());
        manual.cleared = false;
        b.put_transaction(manual).unwrap();
        let mut store = Store::open(&root.join("budget.sqlite3")).unwrap();
        store.save(&b).unwrap();
        let ctx = egui::Context::default();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            store,
            b,
            false,
        );
        let original = serde_json::to_value(&app.b).unwrap();
        let mut d = Dialog::new(DialogKind::Import);
        d.fields[0] = account;
        d.fields[1] = source.display().to_string();
        assert!(app.save_dialog(&d).is_err());
        app.dialog = Some(d);
        let size = Vec2::new(1050., 712.);
        draw(&mut app, &ctx, size, vec![]);
        let out = draw(&mut app, &ctx, size, vec![]);
        click(
            &mut app,
            &ctx,
            size,
            label_rect(&out, "Check and Preview").center(),
        );
        assert_eq!(
            app.dialog
                .as_ref()
                .unwrap()
                .bank_preview
                .as_ref()
                .unwrap()
                .count(),
            4
        );
        assert_eq!(
            serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
            original
        );
        for dimensions in [size, Vec2::new(1440., 900.)] {
            draw(&mut app, &ctx, dimensions, vec![]);
            let out = draw(&mut app, &ctx, dimensions, vec![]);
            for name in ["Import Transactions", "Cancel"] {
                let rect = label_rect(&out, name);
                assert!(
                    rect.top() >= 0. && rect.bottom() <= dimensions.y,
                    "{name}: {rect:?}"
                );
            }
        }
        let out = draw(&mut app, &ctx, size, vec![]);
        click(&mut app, &ctx, size, label_rect(&out, "Cancel").center());
        assert!(app.dialog.is_none());
        assert!(app.undo.is_empty());
        assert_eq!(serde_json::to_value(&app.b).unwrap(), original);
        let mut d = Dialog::new(DialogKind::Import);
        d.fields[0] = app.b.accounts[0].id.clone();
        d.fields[1] = source.display().to_string();
        d.bank_preview = Some(Box::new(
            budgettinker::bank::preview(&app.b, &source, &d.fields[0]).unwrap(),
        ));
        let bytes = std::fs::read(&source).unwrap();
        std::fs::write(&source, [bytes.as_slice(), b"\n"].concat()).unwrap();
        assert!(app.save_dialog(&d).is_err());
        assert!(app.undo.is_empty());
        assert_eq!(
            serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
            original
        );
        std::fs::write(&source, bytes).unwrap();
        app.dialog = Some(d);
        draw(&mut app, &ctx, size, vec![]);
        let out = draw(&mut app, &ctx, size, vec![]);
        click(
            &mut app,
            &ctx,
            size,
            label_rect(&out, "Import Transactions").center(),
        );
        assert!(app.dialog.is_none());
        assert_eq!(app.undo.len(), 1);
        let matched = app
            .b
            .transactions
            .iter()
            .find(|t| t.id == "manual")
            .unwrap();
        assert_eq!(matched.date, "2025-12-31");
        assert_eq!(matched.memo, "Keep manual month");
        assert!(app.b.register_meta["manual"].matched.is_some());
        let imported = serde_json::to_value(&app.b).unwrap();
        assert_eq!(app.b.transactions.len(), 5);
        assert_eq!(
            serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
            imported
        );
        app.undo(false);
        assert_eq!(serde_json::to_value(&app.b).unwrap(), original);
        assert_eq!(
            serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
            original
        );
        app.undo(true);
        assert_eq!(serde_json::to_value(&app.b).unwrap(), imported);
        assert_eq!(
            serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
            imported
        );
        let mut repeated = Dialog::new(DialogKind::Import);
        repeated.fields[0] = app.b.accounts[0].id.clone();
        repeated.fields[1] = source.display().to_string();
        repeated.bank_preview = Some(Box::new(
            budgettinker::bank::preview(&app.b, &source, &repeated.fields[0]).unwrap(),
        ));
        assert_eq!(repeated.bank_preview.as_ref().unwrap().count(), 0);
        app.save_dialog(&repeated).unwrap();
        assert_eq!(app.undo.len(), 1);
        assert_eq!(serde_json::to_value(&app.b).unwrap(), imported);
        drop(app);
        std::fs::remove_dir_all(root).unwrap();
    }
}
#[test]
fn legacy_preview_cancel_apply_and_undo_are_atomic_in_app_and_sqlite() {
    preview_cancel_apply_and_undo(0);
}

#[test]
fn legacy_zero_and_mixed_splits_replace_and_undo_as_one_document() {
    preview_cancel_apply_and_undo(1);
}

#[test]
fn legacy_categorized_boundary_transfers_keep_categories_through_sqlite_undo_redo() {
    preview_cancel_apply_and_undo(2);
}

#[test]
fn legacy_matched_originals_survive_import_unmatch_reject_and_sqlite_undo_redo() {
    preview_cancel_apply_and_undo(3);
}

#[test]
fn legacy_expanded_parts_preserve_editor_details_peers_and_sqlite_undo_redo() {
    preview_cancel_apply_and_undo(4);
}

fn preview_cancel_apply_and_undo(variant: u8) {
    let root = std::env::temp_dir().join(format!("legacy-ui-{}", id()));
    std::fs::create_dir_all(&root).unwrap();
    let full = root.join("Budget.yfull");
    std::fs::write(
        &full,
        if variant == 3 {
            include_bytes!("../../../tests/fixtures/legacy/Matched.yfull").as_slice()
        } else if variant == 4 {
            include_bytes!("../../../tests/fixtures/legacy/ExpandedSplits.yfull").as_slice()
        } else {
            include_bytes!("../../../tests/fixtures/legacy/Budget.yfull").as_slice()
        },
    )
    .unwrap();
    if variant == 1 {
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&full).unwrap()).unwrap();
        value["transactions"][3]["amount"] = serde_json::json!(0);
        value["transactions"][3]["subTransactions"][1]["amount"] = serde_json::json!(25);
        let mut zero = value["transactions"][5].clone();
        zero["entityId"] = serde_json::json!("zero");
        zero["amount"] = serde_json::json!(0);
        value["transactions"].as_array_mut().unwrap().push(zero);
        std::fs::write(&full, serde_json::to_vec(&value).unwrap()).unwrap();
    }
    if variant == 2 {
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&full).unwrap()).unwrap();
        value["transactions"][6]["targetAccountId"] = serde_json::json!("asset");
        value["transactions"][6]["payeeId"] = serde_json::Value::Null;
        value["transactions"][6]["categoryId"] = serde_json::json!("food");
        value["transactions"][7]["accountId"] = serde_json::json!("asset");
        std::fs::write(&full, serde_json::to_vec(&value).unwrap()).unwrap();
    }
    let mut b = Budget::new("EUR");
    b.add_account("Existing", "EUR", true, 12345, "2026-01-01")
        .unwrap();
    let mut store = Store::open(&root.join("budget.sqlite3")).unwrap();
    store.save(&b).unwrap();
    let ctx = egui::Context::default();
    let mut app = App::new(
        &eframe::CreationContext::_new_kittest(ctx.clone()),
        store,
        b,
        false,
    );
    let original = serde_json::to_value(&app.b).unwrap();
    app.open(DialogKind::ImportLegacyBudget);
    app.dialog.as_mut().unwrap().fields[0] = full.display().to_string();
    let size = Vec2::new(1050., 712.);
    draw(&mut app, &ctx, size, vec![]);
    let out = draw(&mut app, &ctx, size, vec![]);
    click(
        &mut app,
        &ctx,
        size,
        label_rect(&out, "Check and Preview").center(),
    );
    assert!(app.dialog.as_ref().unwrap().legacy_preview.is_some());
    assert_eq!(
        serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
        original
    );
    for size in [size, Vec2::new(1440., 900.)] {
        draw(&mut app, &ctx, size, vec![]);
        let out = draw(&mut app, &ctx, size, vec![]);
        for name in ["Import Budget", "Cancel"] {
            let rect = label_rect(&out, name);
            assert!(
                rect.top() >= 0. && rect.bottom() <= size.y,
                "{name}: {rect:?}"
            );
        }
    }
    let out = draw(&mut app, &ctx, size, vec![]);
    click(&mut app, &ctx, size, label_rect(&out, "Cancel").center());
    assert!(app.dialog.is_none());
    assert!(app.undo.is_empty());
    assert_eq!(serde_json::to_value(&app.b).unwrap(), original);
    let mut d = Dialog::new(DialogKind::ImportLegacyBudget);
    d.fields[0] = full.display().to_string();
    assert!(app.save_dialog(&d).is_err());
    d.legacy_preview = Some(Box::new(
        budgettinker::legacy_budget::preview(&full).unwrap(),
    ));
    assert!(app.save_dialog(&d).is_err());
    d.check = true;
    // A source edit after confirmation must not replace any part of the budget.
    let bytes = std::fs::read(&full).unwrap();
    let mut changed = bytes.clone();
    changed.push(b' ');
    std::fs::write(&full, changed).unwrap();
    assert!(app.save_dialog(&d).is_err());
    assert!(app.undo.is_empty());
    assert_eq!(
        serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
        original
    );
    std::fs::write(&full, bytes).unwrap();
    app.save_dialog(&d).unwrap();
    let imported = serde_json::to_value(&app.b).unwrap();
    assert_eq!(app.undo.len(), 1);
    assert_eq!(app.b.accounts.len(), 3);
    assert_eq!(
        app.b.transactions.len(),
        match variant {
            1 => 9,
            4 => 10,
            _ => 8,
        }
    );
    if variant == 1 {
        assert_eq!(app.b.activity("2026-01", "food"), -1500);
        assert_eq!(app.b.activity("2026-01", "fees"), 2000);
        assert_eq!(app.b.balance("card", None, false), -9500);
        assert!(
            app.b
                .transactions
                .iter()
                .any(|t| t.id == "zero" && t.amount == 0)
        );
    }
    if variant == 2 {
        assert_eq!(app.b.activity("2026-01", "food"), -11500);
        assert_eq!(app.b.balance("card", None, false), -22500);
        assert_eq!(app.b.balance("asset", None, false), 10000);
        assert_eq!(app.b.available("2026-01").unwrap(), 90000);
    }
    assert_eq!(app.b.base, "USD");
    assert_eq!(
        serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
        imported
    );
    if variant == 3 {
        let ids = ["split".into()].into_iter().collect();
        for command in [
            RegisterCommand::Unmatch,
            RegisterCommand::Reject,
            RegisterCommand::Approve,
        ] {
            app.run_register_command(&ids, command);
            let changed = serde_json::to_value(&app.b).unwrap();
            assert_ne!(changed, imported);
            assert_eq!(
                serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
                changed
            );
            app.undo(false);
            assert_eq!(serde_json::to_value(&app.b).unwrap(), imported);
            assert_eq!(
                serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
                imported
            );
            app.undo(true);
            assert_eq!(serde_json::to_value(&app.b).unwrap(), changed);
            assert_eq!(
                serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
                changed
            );
            app.undo(false);
        }
    }
    if variant == 4 {
        let draft = RegisterDraft::new(&app.b, Some("card"), Some("split")).unwrap();
        let saved = draft.build(&app.b).unwrap();
        let parent = app.b.transactions.iter().find(|t| t.id == "split").unwrap();
        assert_eq!(
            serde_json::to_value(&saved.splits).unwrap(),
            serde_json::to_value(&parent.splits).unwrap()
        );
        let scheduled = app
            .b
            .scheduled
            .iter()
            .find(|s| s.id == "expanded-plan")
            .unwrap();
        let draft = RegisterDraft::from_scheduled(&app.b, scheduled).unwrap();
        assert_eq!(
            serde_json::to_value(draft.build_scheduled(&app.b).unwrap().splits).unwrap(),
            serde_json::to_value(&scheduled.transaction.splits).unwrap()
        );
        for step in 0..4 {
            let mut next = app.b.clone();
            match step {
                0 => {
                    let mut draft = RegisterDraft::new(&next, Some("card"), Some("split")).unwrap();
                    draft.splits[6].1 = "-30".into();
                    draft.inflow = "70".into();
                    next.put_transaction(draft.build(&next).unwrap()).unwrap();
                    assert_eq!(next.balance("cash", None, false), 123000);
                    assert_eq!(
                        next.transactions
                            .iter()
                            .find(|t| t.id == "peer-s7")
                            .unwrap()
                            .memo,
                        "Peer of s7"
                    );
                }
                1 => {
                    let schedule = next
                        .scheduled
                        .iter()
                        .find(|s| s.id == "expanded-plan")
                        .unwrap();
                    let mut draft = RegisterDraft::from_scheduled(&next, schedule).unwrap();
                    draft.date = "2026-03-31".into();
                    next.edit_scheduled(
                        "expanded-plan",
                        draft.build_scheduled(&next).unwrap(),
                        Repeat::Monthly,
                        None,
                    )
                    .unwrap();
                    assert_eq!(
                        next.scheduled
                            .iter()
                            .find(|s| s.id == "expanded-plan")
                            .unwrap()
                            .transaction
                            .splits[3]
                            .income_month
                            .as_deref(),
                        Some("2026-04")
                    );
                }
                2 => next
                    .enter_scheduled_now("legacy-counterpart/plan-s7")
                    .unwrap(),
                _ => next.delete_transaction("peer-s7").unwrap(),
            }
            app.commit(next).unwrap();
            let changed = serde_json::to_value(&app.b).unwrap();
            assert_ne!(changed, imported);
            assert_eq!(
                serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
                changed
            );
            app.undo(false);
            assert_eq!(serde_json::to_value(&app.b).unwrap(), imported);
            assert_eq!(
                serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
                imported
            );
            app.undo(true);
            assert_eq!(serde_json::to_value(&app.b).unwrap(), changed);
            assert_eq!(
                serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
                changed
            );
            app.undo(false);
        }
    }
    app.undo(false);
    assert_eq!(serde_json::to_value(&app.b).unwrap(), original);
    assert_eq!(
        serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
        original
    );
    app.undo(true);
    assert_eq!(serde_json::to_value(&app.b).unwrap(), imported);
    assert_eq!(
        serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
        imported
    );
    drop(app);
    std::fs::remove_dir_all(root).unwrap();
}
