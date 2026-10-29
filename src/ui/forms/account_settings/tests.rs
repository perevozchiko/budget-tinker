use super::*;

struct Harness {
    app: App,
    ctx: egui::Context,
    root: std::path::PathBuf,
    time: f64,
}

impl Harness {
    fn new(balance: i64) -> Self {
        let root = std::env::temp_dir().join(format!("account-popup-{}", id()));
        let mut b = Budget::new("USD");
        b.add_account("Synthetic checking", "USD", true, balance, "2026-10-01")
            .unwrap();
        let ctx = egui::Context::default();
        let mut store = Store::open(&root.join("demo.budgettinker")).unwrap();
        store.save(&b).unwrap();
        let app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            store,
            b,
            false,
        );
        let mut h = Self {
            app,
            ctx,
            root,
            time: 0.,
        };
        h.open();
        h
    }

    fn open(&mut self) {
        self.app.open_at(
            DialogKind::AccountSettings(self.app.b.accounts[0].id.clone()),
            egui::Rect::from_min_size(egui::pos2(8., 130.), Vec2::new(240., 26.)),
        );
        self.frame(vec![]);
        self.frame(vec![]);
    }

    fn frame(&mut self, events: Vec<egui::Event>) {
        self.render(events, false);
    }

    fn render(&mut self, events: Vec<egui::Event>, sidebar: bool) -> egui::FullOutput {
        self.time += 0.05;
        self.ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(1050., 712.),
                )),
                time: Some(self.time),
                events,
                focused: true,
                ..Default::default()
            },
            |ctx| {
                if sidebar {
                    egui::CentralPanel::default().show(ctx, |ui| self.app.sidebar(ui));
                }
                self.app.dialog(ctx);
            },
        )
    }

    fn rect(&self, name: &str) -> egui::Rect {
        self.ctx.data(|data| {
            data.get_temp(egui::Id::new(("account_settings", name)))
                .unwrap()
        })
    }

    fn click(&mut self, name: &str) {
        let rect = self.rect(name);
        // The close link occupies the left part of its allocated footer area.
        let pos = if name == "close" {
            rect.left_center() + Vec2::new(6., 0.)
        } else {
            rect.center()
        };
        for pressed in [true, false] {
            self.frame(vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ]);
        }
        self.frame(vec![]);
    }

    fn stored(&mut self) -> serde_json::Value {
        serde_json::to_value(self.app.store.load().unwrap().unwrap()).unwrap()
    }
}

#[test]
fn sidebar_hover_and_right_click_keep_the_target_account_highlighted() {
    let mut h = Harness::new(12345);
    h.app.dialog = None;
    let selected = h
        .app
        .b
        .add_account("Other synthetic account", "USD", true, 0, "2026-10-01")
        .unwrap();
    h.app.page = Page::Accounts;
    h.app.account = Some(selected);
    h.render(vec![], true);
    let output = h.render(vec![], true);
    let point = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == "Synthetic checking" => {
                Some(text.pos + text.galley.size() * 0.5)
            }
            _ => None,
        })
        .unwrap();
    let selected_point = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == "Other synthetic account" => {
                Some(text.pos + text.galley.size() * 0.5)
            }
            _ => None,
        })
        .unwrap();
    let highlighted = |output: &egui::FullOutput| {
        output.shapes.iter().any(|shape| {
        matches!(&shape.shape, egui::Shape::Rect(rect) if rect.fill == crate::palette::ACCOUNT_HOVER && rect.rect.contains(point))
    })
    };
    let output = h.render(vec![egui::Event::PointerMoved(point)], true);
    assert!(highlighted(&output), "hover must visibly highlight the row");
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(rect) if rect.fill == crate::palette::SELECTED && rect.rect.contains(selected_point))), "the selected account must keep its darker color");

    for pressed in [true, false] {
        h.render(
            vec![egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }],
            true,
        );
    }
    assert!(
        matches!(&h.app.dialog.as_ref().unwrap().kind, DialogKind::AccountSettings(id) if id == &h.app.b.accounts[0].id)
    );
    assert!(h.app.dialog.as_ref().unwrap().anchor.is_some());
    let output = h.render(
        vec![egui::Event::PointerMoved(egui::pos2(600., 500.))],
        true,
    );
    assert!(
        highlighted(&output),
        "the edited account must stay highlighted after moving off its row"
    );
    assert!(h.app.undo.is_empty());
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn compact_popup_actions_fit_and_cancel_discards_draft() {
    let mut h = Harness::new(12345);
    let original = h.stored();
    let popup = h.rect("popup");
    assert!(popup.width() <= 380. && popup.height() <= 320., "{popup:?}");
    assert_eq!(popup.left_top(), egui::pos2(48., 168.));
    for action in ["budget", "adjust", "close", "cancel", "done"] {
        assert!(
            popup.contains_rect(h.rect(action)),
            "{action} outside popup"
        );
    }
    assert!(h.rect("close").right() < h.rect("cancel").left());
    assert!(h.rect("cancel").right() < h.rect("done").left());
    h.click("budget");
    assert!(!h.app.dialog.as_ref().unwrap().second);
    assert_eq!(h.stored(), original);
    h.click("cancel");
    assert!(h.app.dialog.is_none());
    assert_eq!(h.stored(), original);
    assert!(h.app.undo.is_empty());
}

#[test]
fn done_saves_account_fields_and_budget_toggle_in_one_undoable_change() {
    let mut h = Harness::new(12345);
    let original = h.stored();
    let d = h.app.dialog.as_mut().unwrap();
    d.fields[0] = "Renamed synthetic account".into();
    d.fields[1] = "savings".into();
    d.fields[2] = "Synthetic note".into();
    h.click("budget");
    h.click("done");
    assert!(h.app.dialog.is_none());
    let account = &h.app.b.accounts[0];
    assert_eq!(account.name, "Renamed synthetic account");
    assert_eq!(account.account_type, "savings");
    assert!(!account.on_budget);
    assert_eq!(
        h.app.b.notes[&format!("account:{}", account.id)],
        "Synthetic note"
    );
    let saved = h.stored();
    h.app.undo(false);
    assert_eq!(h.stored(), original);
    assert!(h.app.undo.is_empty());
    h.app.undo(true);
    assert_eq!(h.stored(), saved);
}

#[test]
fn close_rejects_nonzero_balance_and_zero_balance_can_close_and_reopen() {
    let mut h = Harness::new(12345);
    let original = h.stored();
    h.click("close");
    let d = h.app.dialog.as_ref().unwrap();
    assert!(!d.error.is_empty());
    assert!(!d.check);
    assert_eq!(h.stored(), original);
    assert!(h.app.undo.is_empty());

    let mut h = Harness::new(0);
    let original = h.stored();
    h.click("close");
    assert!(h.app.dialog.is_none());
    assert!(h.app.b.accounts[0].closed);
    let closed = h.stored();
    h.app.undo(false);
    assert_eq!(h.stored(), original);
    h.app.undo(true);
    assert_eq!(h.stored(), closed);
    h.open();
    h.click("close");
    assert!(!h.app.b.accounts[0].closed);
    assert_eq!(h.stored(), original);
}

#[test]
fn balance_adjustment_returns_to_the_unsaved_account_draft() {
    let mut h = Harness::new(12345);
    let original = h.stored();
    h.app.dialog.as_mut().unwrap().fields[2] = "Keep this draft".into();
    h.click("adjust");
    let d = h.app.dialog.as_ref().unwrap();
    assert!(matches!(d.kind, DialogKind::AdjustBalance(_)));
    assert_eq!(d.parent.as_ref().unwrap().fields[2], "Keep this draft");
    // Escape from the nested adjustment returns to the account editor.
    h.frame(vec![egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }]);
    h.frame(vec![]);
    assert!(matches!(
        h.app.dialog.as_ref().unwrap().kind,
        DialogKind::AccountSettings(_)
    ));
    assert_eq!(h.app.dialog.as_ref().unwrap().fields[2], "Keep this draft");
    h.click("cancel");
    assert_eq!(h.stored(), original);
    assert!(h.app.undo.is_empty());
}
