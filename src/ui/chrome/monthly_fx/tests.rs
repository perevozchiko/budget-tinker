use super::*;
struct Fixture {
    app: App,
    ctx: egui::Context,
    root: std::path::PathBuf,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn fixture() -> Fixture {
    let root = std::env::temp_dir().join(format!("upgrade-ui-{}", id()));
    let mut b = Budget::new("EUR");
    b.version = 2;
    let previous = month_shift(&current_month(), -1);
    b.add_snapshot(
        &previous,
        "USD",
        90_000_000,
        &Budget::month_end(&previous),
        "Synthetic",
    )
    .unwrap();
    b.add_account(
        "Synthetic USD",
        "USD",
        true,
        100000,
        &format!("{previous}-01"),
    )
    .unwrap();
    let mut store = Store::open(&root.join("demo.budgettinker")).unwrap();
    store.save(&b).unwrap();
    let ctx = egui::Context::default();
    let mut app = App::new(
        &eframe::CreationContext::_new_kittest(ctx.clone()),
        store,
        b,
        false,
    );
    for name in ["First", "Second"] {
        let mut b = app.b.clone();
        b.name = name.into();
        app.commit(b).unwrap();
    }
    app.undo(false);
    app.screen(Screen::Properties);
    Fixture { app, ctx, root }
}
fn frame(f: &mut Fixture, events: Vec<egui::Event>) -> egui::FullOutput {
    f.ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(1050., 680.),
            )),
            events,
            ..Default::default()
        },
        |ctx| {
            if f.app.chrome.monthly.is_some() {
                egui::CentralPanel::default().show(ctx, |_| {});
                f.app.monthly_fx_dialog(ctx);
            } else {
                f.app.chrome_screen(ctx);
            }
        },
    )
}
fn click(f: &mut Fixture, label: &str) {
    frame(f, vec![]);
    let output = frame(f, vec![]);
    let pos = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text() == label => Some(t.pos + Vec2::splat(3.)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing {label}"));
    for pressed in [true, false] {
        frame(
            f,
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
}
fn value(b: &Budget) -> serde_json::Value {
    serde_json::to_value(b).unwrap()
}

fn open(f: &mut Fixture) {
    click(f, "Monthly exchange rates…");
    let b = f.app.b.clone();
    let editor = f.app.chrome.monthly.as_mut().unwrap();
    editor.currency = "USD".into();
    editor.load(&b);
}
fn edit(f: &mut Fixture, rate: &str) {
    let editor = f.app.chrome.monthly.as_mut().unwrap();
    editor.rate = rate.into();
    editor.source = "Synthetic new source".into();
    editor.reason = "Correct closing quote".into();
}
fn stored(f: &mut Fixture) -> serde_json::Value {
    value(&f.app.store.load().unwrap().unwrap())
}

#[test]
fn current_rate_invalid_draft_save_cancel_and_undo_redo() {
    let mut f = fixture();
    open(&mut f);
    let original = value(&f.app.b);
    edit(&mut f, "0,95");
    click(&mut f, "Save rate");
    frame(&mut f, vec![]);
    assert!(!f.app.chrome.monthly.as_ref().unwrap().error.is_empty());
    assert_eq!(f.app.chrome.monthly.as_ref().unwrap().rate, "0,95");
    assert_eq!(stored(&mut f), original);
    edit(&mut f, "0.95000000");
    click(&mut f, "Save rate");
    assert!(f.app.chrome.monthly.as_ref().unwrap().error.is_empty());
    let posted = value(&f.app.b);
    assert_eq!(posted["transactions"], original["transactions"]);
    assert_eq!(
        f.app.b.snapshot(&current_month(), "USD").unwrap().rate,
        95_000_000
    );
    assert_eq!(stored(&mut f), posted);
    click(&mut f, "Close");
    f.app.undo(false);
    assert_eq!(stored(&mut f), original);
    f.app.undo(true);
    assert_eq!(stored(&mut f), posted);
    open(&mut f);
    edit(&mut f, "0.99");
    frame(
        &mut f,
        vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert!(f.app.chrome.monthly.is_none());
    assert_eq!(stored(&mut f), posted);
}

#[test]
fn correction_preview_back_confirm_audit_finalize_and_undo() {
    let mut f = fixture();
    open(&mut f);
    click(&mut f, "Previous month");
    let original = value(&f.app.b);
    let histories = (f.app.undo.len(), f.app.redo.len());
    edit(&mut f, "0.95");
    click(&mut f, "Preview correction");
    let preview = f
        .app
        .chrome
        .monthly
        .as_ref()
        .unwrap()
        .preview
        .as_ref()
        .unwrap();
    assert_eq!(preview.before[0].net_worth, Some(90000));
    assert_eq!(preview.after[0].net_worth, Some(95000));
    assert_eq!(preview.before.len(), 2);
    assert_eq!(stored(&mut f), original);
    click(&mut f, "Back");
    assert_eq!((f.app.undo.len(), f.app.redo.len()), histories);
    assert_eq!(f.app.chrome.monthly.as_ref().unwrap().rate, "0.95");
    click(&mut f, "Preview correction");
    click(&mut f, "Confirm correction");
    let corrected = value(&f.app.b);
    assert_eq!(corrected["transactions"], original["transactions"]);
    assert_eq!(f.app.b.monthly_fx.corrections.len(), 1);
    assert_eq!(stored(&mut f), corrected);
    click(&mut f, "Finalize month…");
    click(&mut f, "Back");
    assert_eq!(stored(&mut f), corrected);
    click(&mut f, "Finalize month…");
    click(&mut f, "Confirm finalization");
    let finalized = value(&f.app.b);
    assert_eq!(
        f.app
            .b
            .monthly_valuation_status(&month_shift(&current_month(), -1))
            .unwrap(),
        ValuationStatus::Finalized
    );
    assert_eq!(stored(&mut f), finalized);
    click(&mut f, "Close");
    f.app.undo(false);
    assert_eq!(stored(&mut f), corrected);
    f.app.undo(false);
    assert_eq!(stored(&mut f), original);
    f.app.undo(true);
    f.app.undo(true);
    assert_eq!(stored(&mut f), finalized);
}

#[test]
fn correction_stale_and_failed_save_retain_confirmation_and_history() {
    let mut f = fixture();
    open(&mut f);
    click(&mut f, "Previous month");
    edit(&mut f, "0.95");
    click(&mut f, "Preview correction");
    let mut changed = f.app.b.clone();
    changed.name = "Changed after preview".into();
    f.app.commit(changed).unwrap();
    let current = value(&f.app.b);
    let histories = (f.app.undo.len(), f.app.redo.len());
    click(&mut f, "Confirm correction");
    assert!(
        f.app
            .chrome
            .monthly
            .as_ref()
            .unwrap()
            .error
            .contains("changed")
    );
    assert_eq!(stored(&mut f), current);
    click(&mut f, "Back");
    click(&mut f, "Preview correction");
    let conn = rusqlite::Connection::open(&f.app.store.path).unwrap();
    conn.execute_batch("CREATE TRIGGER reject_monthly_gui BEFORE UPDATE ON document BEGIN SELECT RAISE(ABORT, 'Synthetic failure'); END;").unwrap();
    click(&mut f, "Confirm correction");
    assert!(f.app.chrome.monthly.as_ref().unwrap().preview.is_some());
    assert!(!f.app.chrome.monthly.as_ref().unwrap().error.is_empty());
    assert_eq!(value(&f.app.b), current);
    assert_eq!(stored(&mut f), current);
    assert_eq!((f.app.undo.len(), f.app.redo.len()), histories);
}
