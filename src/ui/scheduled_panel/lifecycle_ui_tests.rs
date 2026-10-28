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
fn fixture(count: usize) -> Fixture {
    let root = std::env::temp_dir().join(format!("schedule-ui-{}", id()));
    let mut b = Budget::new("EUR");
    let a = b
        .add_account("Checking", "EUR", true, 10000, &today())
        .unwrap();
    let other = b.add_account("Savings", "EUR", true, 0, &today()).unwrap();
    let mut d = RegisterDraft::new(&b, Some(&a), None).unwrap();
    d.payee = "Scheduled coffee".into();
    d.outflow = "5".into();
    // Use a month-independent valid date for tests.
    d.date = "2027-01-31".into();
    for _ in 0..count {
        save_scheduled_draft(&mut b, &d, Repeat::Monthly).unwrap();
    }
    let ctx = egui::Context::default();
    let mut store = Store::open(&root.join("budget.sqlite3")).unwrap();
    store.save(&b).unwrap();
    let mut app = App::new(
        &eframe::CreationContext::_new_kittest(ctx.clone()),
        store,
        b,
        false,
    );
    app.account = Some(a);
    app.page = Page::Accounts;
    app.register_scheduled_open = true;
    assert!(app.b.account(&other).is_ok());
    Fixture { app, ctx, root }
}
fn frame(f: &mut Fixture, size: Vec2, events: Vec<egui::Event>) -> egui::FullOutput {
    f.ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
            time: Some(f.ctx.input(|i| i.time) + 0.3),
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| f.app.accounts(ui));
        },
    )
}
fn text_rect(output: &egui::FullOutput, label: &str) -> egui::Rect {
    output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text() == label => {
                let rect = egui::Rect::from_min_size(t.pos, t.galley.size());
                s.clip_rect.contains_rect(rect).then_some(rect)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("text is not visible: {label}"))
}
#[test]
fn schedule_actions_and_headers_stay_visible_at_both_scroll_extremes() {
    for count in [7, 30] {
        let mut f = fixture(count);
        for size in [
            Vec2::new(1440., 900.),
            Vec2::new(1050., 712.),
            Vec2::new(900., 500.),
        ] {
            frame(&mut f, size, vec![]);
            let output = frame(&mut f, size, vec![]);
            let action = text_rect(&output, "Schedule a new transaction");
            text_rect(&output, "Schedule a transfer");
            let title = text_rect(&output, &format!("Scheduled Transactions ({count})"));
            assert!(action.bottom() < size.y && title.top() > 100.);
            let hover = egui::pos2(600., (title.bottom() + action.top()) / 2.);
            for delta in [-10000., 10000.] {
                let output = frame(
                    &mut f,
                    size,
                    vec![
                        egui::Event::PointerMoved(hover),
                        egui::Event::MouseWheel {
                            unit: egui::MouseWheelUnit::Point,
                            delta: Vec2::new(0., delta),
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
                assert_eq!(text_rect(&output, "Schedule a new transaction"), action);
                text_rect(&output, "Schedule a transfer");
                text_rect(&output, &format!("Scheduled Transactions ({count})"));
            }
        }
    }
}
#[test]
fn schedule_changes_undo_and_redo_through_app_and_sqlite() {
    for action in ["skip", "post", "move", "edit"] {
        let mut f = fixture(0);
        let a = f.app.b.accounts[0].id.clone();
        let other = f.app.b.accounts[1].id.clone();
        let third = f
            .app
            .b
            .add_account("Third", "EUR", true, 0, &today())
            .unwrap();
        schedule_transfer(
            &mut f.app.b,
            TransferDraft {
                from: &a,
                to: &other,
                amount: 500,
                date: "2027-01-31",
                memo: "Before",
                repeat: Repeat::Monthly,
            },
        )
        .unwrap();
        f.app.store.save(&f.app.b).unwrap();
        let original = serde_json::to_string(&f.app.b).unwrap();
        let sid = f.app.b.scheduled[0].id.clone();
        f.app.change(|b| match action {
            "skip" => b.skip_scheduled(&sid),
            "post" => b.enter_scheduled_now(&sid),
            "move" => b.move_scheduled_account(&sid, &third),
            _ => {
                let mut t = b.scheduled[0].transaction.clone();
                t.memo = "Edited".into();
                b.edit_scheduled(&sid, t, Repeat::Weekly, None)
            }
        });
        assert!(!f.app.is_error, "{action}: {}", f.app.notice);
        let after = serde_json::to_string(&f.app.b).unwrap();
        assert_ne!(original, after);
        f.app.undo(false);
        assert_eq!(serde_json::to_string(&f.app.b).unwrap(), original);
        assert_eq!(
            serde_json::to_string(&f.app.store.load().unwrap().unwrap()).unwrap(),
            original
        );
        f.app.undo(true);
        assert_eq!(serde_json::to_string(&f.app.b).unwrap(), after);
        assert_eq!(
            serde_json::to_string(&f.app.store.load().unwrap().unwrap()).unwrap(),
            after
        );
    }
}
#[test]
fn transfer_editor_keeps_peer_and_saves_a_valid_pair() {
    let mut f = fixture(0);
    let a = f.app.b.accounts[0].id.clone();
    let other = f.app.b.accounts[1].id.clone();
    schedule_transfer(
        &mut f.app.b,
        TransferDraft {
            from: &a,
            to: &other,
            amount: 500,
            date: "2027-01-31",
            memo: "Before",
            repeat: Repeat::Monthly,
        },
    )
    .unwrap();
    let s = f.app.b.scheduled[0].clone();
    open_schedule_editor(&mut f.app, &s);
    let (mut d, _) = f.app.scheduled_editor.take().unwrap();
    assert_eq!(d.transfer_to, Some(other.clone()));
    d.outflow = "7.50".into();
    d.date = "2027-02-15".into();
    d.memo = "After".into();
    save_scheduled_draft(&mut f.app.b, &d, Repeat::Weekly).unwrap();
    assert_eq!(f.app.b.scheduled[1].transaction.amount, 750);
    assert_eq!(f.app.b.scheduled[1].transaction.memo, "After");
    assert_eq!(f.app.b.scheduled[1].next_date, "2027-02-15");
    store::validate(&f.app.b).unwrap();
}
#[test]
fn divider_drag_changes_layout_and_persists_height() {
    let mut f = fixture(7);
    let size = Vec2::new(1440., 900.);
    frame(&mut f, size, vec![]);
    let output = frame(&mut f, size, vec![]);
    let title = text_rect(&output, "Scheduled Transactions (7)");
    let start = title.center() - Vec2::new(0., 26.5);
    let mouse = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    frame(
        &mut f,
        size,
        vec![egui::Event::PointerMoved(start), mouse(start, true)],
    );
    let end = start - Vec2::new(0., 60.);
    frame(&mut f, size, vec![egui::Event::PointerMoved(end)]);
    frame(&mut f, size, vec![mouse(end, false)]);
    assert!(f.app.preferences.scheduled_panel_height > 300.);
    let stored = crate::preferences::Preferences::load(&f.app.store.path).unwrap();
    assert_eq!(
        stored.scheduled_panel_height,
        f.app.preferences.scheduled_panel_height
    );
    let output = frame(&mut f, size, vec![]);
    assert!(text_rect(&output, "Scheduled Transactions (7)").top() < title.top() - 40.);
}

#[test]
fn future_schedule_can_be_entered_from_the_menu() {
    let mut f = fixture(1);
    let due = format!("{}-15", month_shift(&current_month(), 1));
    f.app.b.scheduled[0].next_date = due.clone();
    f.app.b.scheduled[0].transaction.set_date(due);
    f.app
        .scheduled_selected
        .insert(f.app.b.scheduled[0].id.clone());
    let menu_frame = |f: &mut Fixture, events| {
        f.ctx.run(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let rows = f.app.b.scheduled.clone();
                    f.app.schedule_menu(ui, &rows);
                });
            },
        )
    };
    menu_frame(&mut f, vec![]);
    let output = menu_frame(&mut f, vec![]);
    let pos = text_rect(&output, "Enter in Register Now").center();
    let count = f.app.b.transactions.len();
    menu_frame(
        &mut f,
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    menu_frame(
        &mut f,
        vec![egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert_eq!(f.app.b.transactions.len(), count + 1);
    assert!(!f.app.is_error);
}

#[test]
fn switching_budget_discards_the_schedule_editor() {
    let mut f = fixture(1);
    let s = f.app.b.scheduled[0].clone();
    open_schedule_editor(&mut f.app, &s);
    f.app
        .switch_budget(&f.root.join("another.sqlite3"), Some(Budget::new("EUR")))
        .unwrap();
    assert!(f.app.scheduled_editor.is_none());
}
