use super::*;
use budgettinker::model::operation_fx::{OperationRate, RateOrigin};

#[test]
fn confirmed_operation_fx_commit_undo_redo_and_storage_failure_preserve_entire_document() {
    let root = std::env::temp_dir().join(format!("operation-fx-ui-{}", id()));
    let path = root.join("demo.budgettinker");
    let mut b = Budget::new("EUR");
    b.version = 2;
    let account = b
        .add_account("Synthetic RSD", "RSD", true, 0, &today())
        .unwrap();
    let mut store = Store::open(&path).unwrap();
    store.save(&b).unwrap();
    let ctx = egui::Context::default();
    let mut app = App::new(&eframe::CreationContext::_new_kittest(ctx), store, b, false);
    let original = serde_json::to_value(&app.b).unwrap();
    let mut draft = RegisterDraft::new(&app.b, Some(&account), None).unwrap();
    draft.outflow = "1 180.00".into();
    draft.category = app.b.categories[0].id.clone();
    // A schedule draft validates native fields without requiring a monthly quote.
    let native = draft.build_scheduled(&app.b).unwrap();
    let mut next = app.b.clone();
    let t = next
        .prepare_operation_fx(
            native,
            OperationRate {
                rate: 855000,
                rate_date: today(),
                source: "Synthetic manual entry".into(),
                origin: RateOrigin::Manual,
            },
            Some((-1009, vec![])),
        )
        .unwrap();
    next.put_transaction(t).unwrap();
    app.commit(next).unwrap();
    let posted = serde_json::to_value(&app.b).unwrap();
    assert_eq!(
        serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
        posted
    );
    app.undo(false);
    assert_eq!(serde_json::to_value(&app.b).unwrap(), original);
    assert_eq!(
        serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
        original
    );
    app.undo(true);
    assert_eq!(serde_json::to_value(&app.b).unwrap(), posted);
    assert_eq!(
        serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
        posted
    );
    let histories = (app.undo.len(), app.redo.len());
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch("CREATE TRIGGER reject_fx_save BEFORE UPDATE ON document BEGIN SELECT RAISE(ABORT, 'Synthetic failure'); END;").unwrap();
    let mut changed = app.b.clone();
    changed.transactions.last_mut().unwrap().memo = "Rejected edit".into();
    assert!(app.commit(changed).is_err());
    assert_eq!(serde_json::to_value(&app.b).unwrap(), posted);
    assert_eq!(
        serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
        posted
    );
    assert_eq!((app.undo.len(), app.redo.len()), histories);
    drop(conn);
    drop(app);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn monthly_correction_and_finalization_commit_undo_redo_and_failed_save() {
    use budgettinker::model::MonthlyQuote;
    let root = std::env::temp_dir().join(format!("monthly-fx-ui-{}", id()));
    let path = root.join("demo.budgettinker");
    let mut b = Budget::new("EUR");
    b.version = 2;
    b.add_snapshot("2026-01", "USD", 90_000_000, "2026-01-30", "Synthetic")
        .unwrap();
    b.add_account("Synthetic USD", "USD", true, 100000, "2026-01-01")
        .unwrap();
    let mut store = Store::open(&path).unwrap();
    store.save(&b).unwrap();
    let mut app = App::new(
        &eframe::CreationContext::_new_kittest(egui::Context::default()),
        store,
        b,
        false,
    );
    let original = serde_json::to_value(&app.b).unwrap();
    let mut next = app.b.clone();
    let preview = next
        .preview_monthly_correction(
            "2026-01",
            "USD",
            MonthlyQuote {
                rate: 95_000_000,
                rate_date: "2026-01-30".into(),
                source: "Synthetic correction".into(),
            },
            "Correct closing quote",
        )
        .unwrap();
    next.apply_monthly_correction(preview).unwrap();
    app.commit(next).unwrap();
    let corrected = serde_json::to_value(&app.b).unwrap();
    assert_eq!(corrected["transactions"], original["transactions"]);
    assert_eq!(app.b.net_worth("2026-01", false).unwrap(), 95000);
    app.undo(false);
    assert_eq!(serde_json::to_value(&app.b).unwrap(), original);
    assert_eq!(
        serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
        original
    );
    app.undo(true);
    assert_eq!(serde_json::to_value(&app.b).unwrap(), corrected);
    assert_eq!(
        serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
        corrected
    );
    let mut next = app.b.clone();
    next.finalize_valuation_month("2026-01").unwrap();
    app.commit(next).unwrap();
    let finalized = serde_json::to_value(&app.b).unwrap();
    app.undo(false);
    assert_eq!(serde_json::to_value(&app.b).unwrap(), corrected);
    app.undo(true);
    assert_eq!(serde_json::to_value(&app.b).unwrap(), finalized);
    let histories = (app.undo.len(), app.redo.len());
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch("CREATE TRIGGER reject_monthly_fx BEFORE UPDATE ON document BEGIN SELECT RAISE(ABORT, 'Synthetic monthly failure'); END;").unwrap();
    let mut next = app.b.clone();
    let preview = next
        .preview_monthly_correction(
            "2026-01",
            "USD",
            MonthlyQuote {
                rate: 96_000_000,
                rate_date: "2026-01-30".into(),
                source: "Rejected".into(),
            },
            "Rejected correction",
        )
        .unwrap();
    next.apply_monthly_correction(preview).unwrap();
    assert!(app.commit(next).is_err());
    assert_eq!(serde_json::to_value(&app.b).unwrap(), finalized);
    assert_eq!(
        serde_json::to_value(app.store.load().unwrap().unwrap()).unwrap(),
        finalized
    );
    assert_eq!((app.undo.len(), app.redo.len()), histories);
    drop(conn);
    drop(app);
    assert_eq!(
        serde_json::to_value(store::read_budget(&path).unwrap()).unwrap(),
        finalized
    );
    std::fs::remove_dir_all(root).unwrap();
}
