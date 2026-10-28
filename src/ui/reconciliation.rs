use super::*;

/// Statement details are transient UI state; cleared edits use the normal command history.
pub(super) struct Session {
    pub account: String,
    pub date: String,
    pub statement: i64,
    confirm_adjustment: bool,
}

impl App {
    pub(super) fn begin_reconciliation(
        &mut self,
        account: &str,
        statement: i64,
        date: &str,
    ) -> Result<()> {
        if self.transaction_editor.is_some() || self.scheduled_editor.is_some() {
            return Err("Save or cancel the current transaction before reconciling.".into());
        }
        if self
            .reconciliation
            .as_ref()
            .is_some_and(|s| s.account != account)
        {
            return Err("Finish or cancel the current reconciliation first.".into());
        }
        self.b.reconciliation_difference(account, statement, date)?;
        self.reconciliation = Some(Session {
            account: account.into(),
            date: date.into(),
            statement,
            confirm_adjustment: false,
        });
        self.page = Page::Accounts;
        self.account = Some(account.into());
        self.search.clear();
        self.selected.clear();
        self.notify(
            "Check each transaction against your statement, then finish reconciliation.",
            false,
        );
        Ok(())
    }

    fn finish_reconciliation(&mut self, adjustment: bool) -> Result<()> {
        if self.transaction_editor.is_some() || self.scheduled_editor.is_some() {
            return Err(
                "Save or cancel the current transaction before finishing reconciliation.".into(),
            );
        }
        let session = self
            .reconciliation
            .as_ref()
            .ok_or("No reconciliation is in progress.")?;
        let mut next = self.b.clone();
        if adjustment {
            next.reconcile_with_adjustment(&session.account, session.statement, &session.date)?;
        } else {
            next.reconcile_through(&session.account, session.statement, &session.date)?;
        }
        self.commit(next)?;
        self.reconciliation = None;
        self.selected.clear();
        self.notify("Reconciliation complete. Use Undo to reverse it.", false);
        Ok(())
    }

    fn cancel_reconciliation(&mut self) {
        self.reconciliation = None;
        self.selected.clear();
        self.notify(
            "Reconciliation cancelled. Cleared changes are kept and can be undone.",
            false,
        );
    }

    pub(super) fn reconciliation_panel(&mut self, ui: &mut egui::Ui) {
        let Some(session) = &self.reconciliation else {
            return;
        };
        let Ok(account) = self.b.account(&session.account) else {
            return;
        };
        let account_id = account.id.clone();
        let name = account.name.clone();
        let currency = account.currency.clone();
        let date = session.date.clone();
        let statement = session.statement;
        let active = self.account.as_ref() == Some(&account_id);
        let difference = self
            .b
            .reconciliation_difference(&account_id, statement, &date);
        let mut finish = false;
        let mut change = false;
        let mut cancel = false;
        let mut resume = false;
        let idle = self.transaction_editor.is_none() && self.scheduled_editor.is_none();
        Frame::new()
            .fill(Color32::from_rgb(238, 246, 229))
            .stroke(Stroke::new(1_f32, Color32::from_rgb(167, 190, 142)))
            .inner_margin(9)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(format!("Reconciling {name} · through {}", crate::register_view::date(&date))).strong());
                match &difference {
                    Ok(amount) => {
                        let cleared = statement.checked_sub(*amount).expect("checked reconciliation balance");
                        ui.horizontal_wrapped(|ui| {
                            ui.label(format!("Statement Balance: {}", i18n::money(statement, &currency)));
                            ui.label(format!("Cleared Balance: {}", i18n::money(cleared, &currency)));
                            // legacy displays cleared minus statement; the adjustment has the opposite sign.
                            if let Some(difference) = amount.checked_neg() {
                                ui.colored_label(if difference == 0 { GREEN } else { RED }, RichText::new(format!("Difference: {}", i18n::money(difference, &currency))).strong());
                            } else {
                                ui.colored_label(RED, "Difference is too large.");
                            }
                        });
                    }
                    Err(error) => { ui.colored_label(RED, crate::i18n::tr(ui.ctx(), error)); }
                }
                if active {
                    ui.label("Mark matching transactions Cleared using C. Later and reconciled transactions are hidden; totals include reconciled history.");
                }
                ui.horizontal_wrapped(|ui| {
                    if active {
                        finish = ui.add_enabled(idle && difference.is_ok(), classic::Button::new("Finish Reconciliation").primary()).clicked();
                        change = ui.add_enabled(idle, classic::Button::new("Change Statement…")).clicked();
                    } else {
                        resume = ui.add_enabled(idle, classic::Button::new(format!("Continue reconciling {name}"))).clicked();
                    }
                    cancel = classic::button(ui, "Cancel Reconciliation").clicked();
                    ui.label("Cancel keeps Cleared changes. Undo reverses them.");
                });
            });
        ui.add_space(6.);
        if cancel {
            self.cancel_reconciliation();
            return;
        }
        if resume {
            self.account = Some(account_id.clone());
            self.search.clear();
            self.selected.clear();
        }
        if change {
            self.open(DialogKind::Reconcile(account_id));
        }
        if finish {
            if difference == Ok(0) {
                if let Err(error) = self.finish_reconciliation(false) {
                    self.notify(error, true);
                }
                return;
            }
            self.reconciliation.as_mut().unwrap().confirm_adjustment = true;
        }
        if self
            .reconciliation
            .as_ref()
            .is_some_and(|s| s.confirm_adjustment)
        {
            let mut create = false;
            let mut keep_checking = false;
            let response = egui::Modal::new(egui::Id::new("reconciliation_adjustment"))
                .show(ui.ctx(), |ui| {
                    ui.set_width(440.);
                    ui.heading("BudgetTinker and your bank aren't balanced");
                    ui.label("Continue checking your statement, or create an adjustment transaction to match its balance.");
                    if let Ok(amount) = difference {
                        ui.label(format!("Adjustment: {} on {}", i18n::money(amount, &currency), crate::register_view::date(&date)));
                    }
                    ui.label("The adjustment and reconciliation can be reversed together with Undo.");
                    ui.horizontal_wrapped(|ui| {
                        create = ui.add_enabled(idle, classic::Button::new("Create Adjustment and Finish").primary()).clicked();
                        keep_checking = classic::button(ui, "Continue Reconciling").clicked();
                    });
                    if self.is_error { ui.colored_label(RED, crate::i18n::tr(ui.ctx(), &self.notice)); }
                });
            if create {
                if let Err(error) = self.finish_reconciliation(true) {
                    self.notify(error, true);
                }
            } else if keep_checking || response.should_close() {
                self.reconciliation.as_mut().unwrap().confirm_adjustment = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture {
        app: App,
        ctx: egui::Context,
        root: std::path::PathBuf,
        account: String,
        pending: String,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }
    fn fixture() -> Fixture {
        let root = std::env::temp_dir().join(format!("reconciliation-ui-{}", id()));
        let mut b = Budget::new("USD");
        let account = b
            .add_account("Checking", "USD", true, 10_000, "2026-01-01")
            .unwrap();
        b.reconcile_through(&account, 10_000, "2026-01-01").unwrap();
        let mut draft = RegisterDraft::new(&b, Some(&account), None).unwrap();
        draft.date = "2026-01-20".into();
        draft.payee = "Pending coffee".into();
        draft.outflow = "5".into();
        let row = draft.build(&b).unwrap();
        let pending = row.id.clone();
        b.put_transaction(row).unwrap();
        draft.date = "2026-02-01".into();
        draft.payee = "Later coffee".into();
        b.put_transaction(draft.build(&b).unwrap()).unwrap();
        let ctx = egui::Context::default();
        let mut store = Store::open(&root.join("budget.sqlite3")).unwrap();
        store.save(&b).unwrap();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            store,
            b,
            false,
        );
        app.account = Some(account.clone());
        app.page = Page::Accounts;
        Fixture {
            app,
            ctx,
            root,
            account,
            pending,
        }
    }
    fn document(b: &Budget) -> serde_json::Value {
        serde_json::to_value(b).unwrap()
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
    fn rect(output: &egui::FullOutput, label: &str) -> Option<egui::Rect> {
        output.shapes.iter().find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text() == label => {
                let rect = egui::Rect::from_min_size(t.pos, t.galley.size());
                s.clip_rect.contains_rect(rect).then_some(rect)
            }
            _ => None,
        })
    }
    fn click(f: &mut Fixture, size: Vec2, pos: egui::Pos2) {
        frame(
            f,
            size,
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
        frame(
            f,
            size,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        frame(f, size, vec![]);
    }
    fn click_text(f: &mut Fixture, size: Vec2, label: &str) {
        frame(f, size, vec![]);
        let output = frame(f, size, vec![]);
        click(
            f,
            size,
            rect(&output, label)
                .unwrap_or_else(|| panic!("Missing control: {label}"))
                .center(),
        );
    }

    #[test]
    fn begin_and_change_statement_do_not_save_or_lock_and_invalid_input_keeps_session() {
        let mut f = fixture();
        let before = document(&f.app.b);
        let mut d = Dialog::new(DialogKind::Reconcile(f.account.clone()));
        d.fields[0] = "95".into();
        d.fields[1] = "2026-01-31".into();
        f.app.save_dialog(&d).unwrap();
        assert!(f.app.reconciliation.is_some());
        assert_eq!(document(&f.app.b), before);
        assert_eq!(document(&f.app.store.load().unwrap().unwrap()), before);
        assert!(f.app.undo.is_empty());
        f.app.open(DialogKind::Reconcile(f.account.clone()));
        let d = f.app.dialog.take().unwrap();
        assert_eq!(d.fields[0], "95.00");
        assert_eq!(d.fields[1], "2026-01-31");
        assert!(
            f.app
                .begin_reconciliation(&f.account, 9500, "2099-01-01")
                .is_err()
        );
        assert_eq!(f.app.reconciliation.as_ref().unwrap().date, "2026-01-31");
        f.app
            .begin_reconciliation(&f.account, 9400, "2026-01-30")
            .unwrap();
        assert_eq!(f.app.reconciliation.as_ref().unwrap().statement, 9400);
        f.app.cancel_reconciliation();
        assert_eq!(document(&f.app.b), before);
        assert!(f.app.reconciliation.is_none());
    }

    #[test]
    fn register_cleared_click_updates_difference_finish_and_undo_redo_persist() {
        // 760 px models the register space left beside the sidebar in a 1050 px window.
        for size in [
            Vec2::new(1440., 900.),
            Vec2::new(1050., 712.),
            Vec2::new(760., 600.),
        ] {
            let mut f = fixture();
            f.app
                .begin_reconciliation(&f.account, 9500, "2026-01-31")
                .unwrap();
            frame(&mut f, size, vec![]);
            let output = frame(&mut f, size, vec![]);
            let row = rect(&output, "Pending coffee").unwrap();
            assert!(rect(&output, "Later coffee").is_none());
            assert!(rect(&output, "Difference: $5.00").is_some());
            let column = rect(&output, "C").unwrap();
            click(&mut f, size, egui::pos2(column.center().x, row.center().y));
            assert!(
                f.app
                    .b
                    .transactions
                    .iter()
                    .find(|t| t.id == f.pending)
                    .unwrap()
                    .cleared
            );
            let output = frame(&mut f, size, vec![]);
            assert!(rect(&output, "Difference: $0.00").is_some());
            let before_finish = document(&f.app.b);
            click_text(&mut f, size, "Finish Reconciliation");
            assert!(f.app.reconciliation.is_none());
            assert!(
                f.app
                    .b
                    .transactions
                    .iter()
                    .find(|t| t.id == f.pending)
                    .unwrap()
                    .reconciled
            );
            let done = document(&f.app.b);
            assert_eq!(document(&f.app.store.load().unwrap().unwrap()), done);
            f.app.undo(false);
            assert_eq!(document(&f.app.b), before_finish);
            assert_eq!(
                document(&f.app.store.load().unwrap().unwrap()),
                before_finish
            );
            f.app.undo(true);
            assert_eq!(document(&f.app.b), done);
            assert_eq!(document(&f.app.store.load().unwrap().unwrap()), done);
        }
    }

    #[test]
    fn adjustment_requires_confirmation_cancel_keeps_cleared_edits_and_undo_reverses_completion() {
        let mut f = fixture();
        let size = Vec2::new(1050., 712.);
        let original = document(&f.app.b);
        f.app
            .begin_reconciliation(&f.account, 9200, "2026-01-31")
            .unwrap();
        click_text(&mut f, size, "Finish Reconciliation");
        assert!(f.app.reconciliation.as_ref().unwrap().confirm_adjustment);
        assert_eq!(document(&f.app.b), original);
        click_text(&mut f, size, "Continue Reconciling");
        assert!(!f.app.reconciliation.as_ref().unwrap().confirm_adjustment);
        f.app.run_register_command(
            &HashSet::from([f.pending.clone()]),
            RegisterCommand::Cleared(true),
        );
        let cleared = document(&f.app.b);
        click_text(&mut f, size, "Cancel Reconciliation");
        assert_eq!(document(&f.app.b), cleared);
        f.app.undo(false);
        assert_eq!(document(&f.app.b), original);
        f.app
            .begin_reconciliation(&f.account, 9200, "2026-01-31")
            .unwrap();
        f.app.search = "no results".into(); // searches must never change the reconciliation total
        click_text(&mut f, size, "Finish Reconciliation");
        click_text(&mut f, size, "Create Adjustment and Finish");
        assert!(f.app.reconciliation.is_none());
        assert_eq!(f.app.b.balance(&f.account, Some("2026-01-31"), true), 9200);
        assert_eq!(f.app.b.transactions.last().unwrap().amount, -800);
        assert!(
            !f.app
                .b
                .transactions
                .iter()
                .find(|t| t.id == f.pending)
                .unwrap()
                .cleared
        );
        let done = document(&f.app.b);
        assert_eq!(document(&f.app.store.load().unwrap().unwrap()), done);
        f.app.undo(false);
        assert_eq!(document(&f.app.b), original);
        assert_eq!(document(&f.app.store.load().unwrap().unwrap()), original);
        f.app.undo(true);
        assert_eq!(document(&f.app.b), done);
        assert_eq!(document(&f.app.store.load().unwrap().unwrap()), done);
    }
}
