use super::*;

#[test]
fn net_worth_uses_history_before_start_and_stops_at_custom_end_date() {
    let mut b = Budget::new("EUR");
    let a = b
        .add_account("Checking", "EUR", true, 1000, "2026-01-01")
        .unwrap();
    b.add_account("Tracking", "EUR", false, 5000, "2026-01-01")
        .unwrap();
    let c = b.categories[0].id.clone();
    b.put_transaction(transaction(&a, Some(&c), -100, "2026-02-10", "Shop"))
        .unwrap();
    b.put_transaction(transaction(&a, Some(&c), -200, "2026-02-20", "Shop"))
        .unwrap();
    let mut f = filter();
    f.accounts = Selection::All;
    // Payee and category filters intentionally do not filter a balance sheet.
    f.payee = Selection::Selected(vec!["Missing".into()]);
    f.category = Selection::Selected(vec!["Missing".into()]);
    let r = reports::calculate(&b, 4, &f).unwrap();
    assert_eq!(
        r.rows,
        vec![("2026-01".into(), 6000), ("2026-02".into(), 5900)]
    );
    assert_eq!(r.total, 5900);
    assert_eq!(r.months[0].assets, 6000);
    assert_eq!(r.months[0].debts, 0);
    assert_eq!(r.months[1].assets, 5900);
    assert_eq!(r.months[1].debts, 0);
    f.accounts = Selection::All;
    assert_eq!(reports::calculate(&b, 4, &f).unwrap().total, 5900);
}

#[test]
fn net_worth_keeps_debt_separate_from_assets_and_exports_each_month() {
    let mut b = Budget::new("EUR");
    let checking = b
        .add_account("Checking", "EUR", true, 10_000, "2026-01-01")
        .unwrap();
    let card = b
        .add_account("Credit Card", "EUR", true, -3_000, "2026-01-01")
        .unwrap();
    let loan = b
        .add_account("Loan", "EUR", false, -8_000, "2026-01-01")
        .unwrap();
    let mut f = filter();
    f.accounts = Selection::All;
    let r = reports::calculate(&b, 4, &f).unwrap();
    for month in &r.months {
        assert_eq!(
            (month.assets, month.debts, month.net_worth),
            (10_000, -11_000, -1_000)
        );
    }
    assert_eq!(r.total, -1_000);
    let csv = r.csv("EUR", str::to_owned).unwrap();
    let mut rows = csv::Reader::from_reader(csv.as_slice());
    assert_eq!(
        rows.headers().unwrap(),
        &[
            "From",
            "Through",
            "Currency",
            "Month",
            "Assets",
            "Debts",
            "Net Worth"
        ][..]
    );
    let january = rows.records().next().unwrap().unwrap();
    assert_eq!(
        january.iter().skip(3).collect::<Vec<_>>(),
        vec!["2026-01", "100.00", "-110.00", "-10.00"]
    );
    f.accounts = Selection::All;
    let all_accounts = reports::calculate(&b, 4, &f).unwrap();
    assert_eq!(
        (all_accounts.months[0].assets, all_accounts.months[0].debts),
        (10_000, -11_000)
    );
    f.accounts = Selection::Selected(vec![card]);
    let card_only = reports::calculate(&b, 4, &f).unwrap();
    assert_eq!(
        (card_only.months[0].assets, card_only.months[0].debts),
        (0, -3_000)
    );
    assert_eq!(b.balance(&checking, None, false), 10_000);
    assert_eq!(b.balance(&loan, None, false), -8_000);
}

#[test]
fn net_worth_with_all_accounts_includes_off_budget_balances() {
    let mut b = Budget::new("EUR");
    let _on = b
        .add_account("Checking", "EUR", true, 1000, "2026-01-01")
        .unwrap();
    let off = b
        .add_account("Tracking", "EUR", false, 5000, "2026-01-01")
        .unwrap();
    // With All accounts, both on-budget and off-budget balances are included.
    let mut f = filter();
    f.accounts = Selection::All;
    let r = reports::calculate(&b, 4, &f).unwrap();
    assert_eq!(r.total, 6000);
    // With BudgetAccounts, only on-budget account is included.
    let mut f2 = filter();
    f2.accounts = Selection::BudgetAccounts;
    let r2 = reports::calculate(&b, 4, &f2).unwrap();
    assert_eq!(r2.total, 1000);
    // With Selected, only the explicit account is included.
    let f3 = Filter {
        accounts: Selection::Selected(vec![off]),
        ..filter()
    };
    let r3 = reports::calculate(&b, 4, &f3).unwrap();
    assert_eq!(r3.total, 5000);
}
