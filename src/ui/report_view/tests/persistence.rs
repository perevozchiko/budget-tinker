use super::*;

#[test]
fn report_preferences_restore_custom_filters_and_separate_account_choices_after_restart() {
    let mut fixture = report_fixture();
    let app = &mut fixture.app;
    assert_eq!(app.report_period, 5);
    assert_eq!(app.report_filter.from, "2026-07-01");
    assert_eq!(app.report_filter.accounts, Selection::BudgetAccounts);
    let original = serde_json::to_value(&app.b).unwrap();
    app.report_period = 6;
    app.report_filter.from = "2026-07-15".into();
    app.report_filter.through = "2026-10-21".into();
    app.report_filter.category = Selection::Selected(vec![app.b.categories[0].id.clone()]);
    app.report_filter.payee = Selection::Selected(vec!["Shop".into()]);
    app.report_filter.accounts = Selection::Selected(vec![app.b.accounts[0].id.clone()]);
    app.report_accounts.switch(0, 4, &mut app.report_filter);
    app.report = 4;
    app.report_filter.accounts = Selection::Selected(vec![app.b.accounts[1].id.clone()]);
    app.persist_report_preferences();
    let saved = app.preferences.reports.clone().unwrap();
    let path = app.store.path.clone();
    let budget = store::read_budget(&path).unwrap();
    let mut restarted = App::new(
        &eframe::CreationContext::_new_kittest(fixture.ctx.clone()),
        Store::open(&path).unwrap(),
        budget,
        false,
    );
    assert_eq!(restarted.report, 4);
    assert_eq!(restarted.report_period, 6);
    assert_eq!(restarted.report_filter, saved.filter);
    restarted
        .report_accounts
        .switch(4, 0, &mut restarted.report_filter);
    assert_eq!(restarted.report_filter.accounts, saved.spending_accounts);
    assert_eq!(restarted.report_filter.category, saved.filter.category);
    assert_eq!(restarted.report_filter.payee, saved.filter.payee);
    assert_eq!(
        serde_json::to_value(store::read_budget(&path).unwrap()).unwrap(),
        original
    );
}

#[test]
fn switching_budgets_restores_only_their_own_report_preferences() {
    let mut fixture = report_fixture();
    let first_path = fixture.app.store.path.clone();
    fixture.app.report_period = 6;
    fixture.app.report_filter.from = "2026-08-01".into();
    fixture.app.report_filter.through = "2026-08-31".into();
    fixture.app.report_filter.category = Selection::NoneSelected;
    fixture.app.report_filter.accounts = Selection::NoneSelected;
    let second_path = fixture.root.join("second.sqlite3");
    fixture
        .app
        .switch_budget(&second_path, Some(Budget::new("EUR")))
        .unwrap();
    assert_eq!(fixture.app.report_period, 5);
    assert_eq!(fixture.app.report_filter.category, Selection::All);
    assert_eq!(
        fixture.app.report_filter.accounts,
        Selection::BudgetAccounts
    );
    fixture.app.report_filter.payee = Selection::NoneSelected;
    fixture
        .app
        .report_accounts
        .switch(0, 4, &mut fixture.app.report_filter);
    fixture.app.report = 4;
    fixture.app.switch_budget(&first_path, None).unwrap();
    assert_eq!(fixture.app.report, 0);
    assert_eq!(fixture.app.report_period, 6);
    assert_eq!(fixture.app.report_filter.from, "2026-08-01");
    assert_eq!(fixture.app.report_filter.category, Selection::NoneSelected);
    assert_eq!(fixture.app.report_filter.accounts, Selection::NoneSelected);
    assert_eq!(fixture.app.report_filter.payee, Selection::All);
    fixture.app.switch_budget(&second_path, None).unwrap();
    assert_eq!(fixture.app.report, 4);
    assert_eq!(fixture.app.report_filter.payee, Selection::NoneSelected);
    assert_eq!(fixture.app.report_filter.accounts, Selection::All);
}
