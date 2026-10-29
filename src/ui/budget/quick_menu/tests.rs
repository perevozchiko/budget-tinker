use super::*;

struct Harness {
    app: App,
    ctx: egui::Context,
    root: std::path::PathBuf,
    time: f64,
    texts: Vec<String>,
}
impl Harness {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("month-menu-{}", id()));
        let mut b = Budget::new("USD");
        let account = b
            .add_account("Cash", "USD", true, 100_000, "2026-01-01")
            .unwrap();
        let category = b.categories[0].id.clone();
        let mut t = b.transactions[0].clone();
        t.category = Some(category.clone());
        t.income_month = None;
        t.payee = "Synthetic shop".into();
        for (date, value) in [
            ("2026-01-05", -100),
            ("2026-02-05", -200),
            ("2026-03-05", -300),
            ("2026-04-05", -400),
            ("2026-05-05", -500),
        ] {
            t.id = id();
            t.date = date.into();
            t.amount = value;
            b.put_transaction(t.clone()).unwrap();
        }
        b.allocate("2026-05", &category, 1200).unwrap();
        // Include a stored zero and a hidden category, alongside positive and negative cells.
        b.allocate("2026-06", &category, 0).unwrap();
        b.categories[0].hidden = true;
        b.allocate("2026-06", &b.categories[1].id.clone(), 777)
            .unwrap();
        b.allocate("2026-06", &b.categories[2].id.clone(), -50)
            .unwrap();
        t.id = id();
        t.date = "2026-06-10".into();
        t.amount = -90;
        b.scheduled.push(Scheduled {
            id: id(),
            next_date: t.date.clone(),
            transaction: t,
            monthly: false,
            repeat: Some(Repeat::Weekly),
            anchor_day: Some(10),
        });
        assert_eq!(b.accounts[0].id, account);
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
            texts: vec![],
        };
        h.frame(vec![]);
        h.frame(vec![]);
        h
    }
    fn frame(&mut self, events: Vec<egui::Event>) {
        self.time += 0.02;
        let output = self.ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(1000., 700.),
                )),
                time: Some(self.time),
                events,
                focused: true,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    self.app
                        .budget_summary(ui, &["2026-06".into(), "2026-07".into()], 0., 420.)
                });
            },
        );
        self.texts = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(t) => Some(t.galley.text().to_owned()),
                _ => None,
            })
            .collect();
    }
    fn click(&mut self, id: egui::Id) {
        let rect: egui::Rect = self.ctx.data(|d| d.get_temp(id).unwrap());
        for pressed in [true, false] {
            self.frame(vec![
                egui::Event::PointerMoved(rect.center()),
                egui::Event::PointerButton {
                    pos: rect.center(),
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ]);
        }
        self.frame(vec![]);
    }
    fn persisted(&mut self) -> serde_json::Value {
        serde_json::to_value(self.app.store.load().unwrap().unwrap()).unwrap()
    }
}
impl Drop for Harness {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn every_month_menu_choice_persists_as_one_undoable_change() {
    for (action, expected) in [
        (QuickBudget::PreviousBudget, 1200),
        (QuickBudget::PreviousOutflows, 500),
        (QuickBudget::AverageThreeMonths, 400),
        (QuickBudget::AverageYear, 300),
        (QuickBudget::Scheduled, 270),
        (QuickBudget::ZeroBalance, -700),
        (QuickBudget::ZeroAllocations, 0),
    ] {
        let mut h = Harness::new();
        let original = h.persisted();
        let categories: Vec<_> = h.app.b.categories.iter().map(|c| c.id.clone()).collect();
        assert_eq!(h.app.b.category_balance("2026-06", &categories[0]), 700);
        h.click(egui::Id::new(("month_quick_button", "2026-06")));
        assert!(egui::Popup::is_any_open(&h.ctx));
        h.click(egui::Id::new((
            "month_quick_option",
            "2026-06",
            format!("{action:?}"),
        )));
        assert!(!h.app.is_error, "{}", h.app.notice);
        assert_eq!(
            h.app.b.allocated("2026-06", &categories[0]),
            expected,
            "{action:?}"
        );
        for (i, value) in [(1, 777), (2, -50)] {
            assert_eq!(
                h.app.b.allocated("2026-06", &categories[i]),
                if action == QuickBudget::ZeroAllocations {
                    0
                } else {
                    value
                }
            );
        }
        if action == QuickBudget::ZeroBalance {
            assert_eq!(h.app.b.category_balance("2026-06", &categories[0]), 0);
        }
        assert_eq!(h.app.undo.len(), 1);
        let changed = serde_json::to_value(&h.app.b).unwrap();
        assert_eq!(h.persisted(), changed);
        h.app.undo(false);
        assert_eq!(h.persisted(), original);
        h.app.undo(true);
        assert_eq!(h.persisted(), changed);
    }
}

#[test]
fn summary_click_collapses_all_months_restores_on_restart_and_does_not_edit_budget() {
    let mut h = Harness::new();
    let original = h.persisted();
    assert!(h.texts.iter().any(|t| t == "Income for Jun"));
    let expanded: egui::Rect = h.ctx.data(|d| {
        d.get_temp(egui::Id::new(("month_quick_button", "2026-06")))
            .unwrap()
    });
    h.click(egui::Id::new(("summary_toggle", "2026-06")));
    assert!(h.app.preferences.budget_summary_collapsed);
    assert!(
        crate::preferences::Preferences::load(&h.app.store.path)
            .unwrap()
            .budget_summary_collapsed
    );
    assert!(!h.texts.iter().any(|t| t.starts_with("Income for")));
    let collapsed: egui::Rect = h.ctx.data(|d| {
        d.get_temp(egui::Id::new(("summary_toggle", "2026-07")))
            .unwrap()
    });
    assert!(collapsed.bottom() < expanded.top() + 60.);
    // The second month expands the whole strip too.
    h.click(egui::Id::new(("summary_toggle", "2026-07")));
    assert!(!h.app.preferences.budget_summary_collapsed);
    assert!(h.texts.iter().any(|t| t == "Income for Jun"));
    assert_eq!(h.persisted(), original);
    assert!(h.app.undo.is_empty());
}

#[test]
fn income_popup_opens_and_account_view_uses_exact_month_filter() {
    let mut h = Harness::new();
    let mut income = h.app.b.transactions[0].clone();
    income.id = id();
    income.date = "2026-05-20".into();
    income.amount = 12345;
    income.income_month = Some("2026-06".into());
    income.payee = "Synthetic deferred paycheck".into();
    h.app.b.put_transaction(income).unwrap();
    h.app.store.save(&h.app.b).unwrap();
    let original = h.persisted();
    h.click(egui::Id::new(("month_income", "2026-06")));
    assert!(egui::Popup::is_any_open(&h.ctx));
    h.click(egui::Id::new("income_show_accounts"));
    assert!(matches!(h.app.page, Page::Accounts));
    assert_eq!(h.app.search, "income:2026-06");
    assert!(h.app.account.is_none());
    let matches: Vec<_> = h
        .app
        .b
        .transactions
        .iter()
        .filter(|t| crate::register_search::matches(&h.app.b, t, &h.app.search))
        .collect();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].payee, "Synthetic deferred paycheck");
    assert!(h.app.register_timeframe.includes(&matches[0].date));
    assert_eq!(h.persisted(), original);
    assert!(h.app.undo.is_empty());
}

#[test]
fn missing_scheduled_rate_reports_error_without_saving_or_adding_undo() {
    let mut h = Harness::new();
    let euro = h
        .app
        .b
        .add_account("Euro", "EUR", true, 0, "2026-01-01")
        .unwrap();
    let mut scheduled = h.app.b.scheduled[0].clone();
    scheduled.id = id();
    scheduled.transaction.id = id();
    scheduled.transaction.account = euro;
    scheduled.transaction.category = Some(h.app.b.categories[3].id.clone());
    h.app.b.scheduled.push(scheduled);
    h.app.store.save(&h.app.b).unwrap();
    let original = h.persisted();
    h.click(egui::Id::new(("month_quick_button", "2026-06")));
    h.click(egui::Id::new((
        "month_quick_option",
        "2026-06",
        "Scheduled".to_owned(),
    )));
    assert!(h.app.is_error);
    assert!(h.app.notice.contains("exchange rate"));
    assert_eq!(serde_json::to_value(&h.app.b).unwrap(), original);
    assert_eq!(h.persisted(), original);
    assert!(h.app.undo.is_empty());
}
