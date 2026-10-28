use budgettinker::{model::*, register::Repeat};

fn transaction(account: &str, category: &str, amount: i64, date: &str) -> Transaction {
    Transaction {
        id: id(),
        account: account.into(),
        date: date.into(),
        payee: "Shop".into(),
        memo: String::new(),
        check_number: None,
        amount,
        base_amount: 0,
        snapshot: None,
        category: Some(category.into()),
        cleared: false,
        flag: None,
        income_month: None,
        reconciled: false,
        transfer: None,
        splits: vec![],
    }
}
fn schedule(t: Transaction, repeat: Repeat) -> Scheduled {
    Scheduled {
        id: id(),
        next_date: t.date.clone(),
        monthly: repeat == Repeat::Monthly,
        repeat: Some(repeat),
        anchor_day: Some(t.date[8..].parse().unwrap()),
        transaction: t,
    }
}
#[test]
fn average_uses_available_complete_months_including_empty_months() {
    let mut b = Budget::new("USD");
    let account = b
        .add_account("Cash", "USD", true, 10000, "2026-01-01")
        .unwrap();
    let category = b.categories[0].id.clone();
    b.put_transaction(transaction(&account, &category, -1000, "2026-01-05"))
        .unwrap();
    b.put_transaction(transaction(&account, &category, -500, "2026-03-05"))
        .unwrap();
    assert_eq!(b.average_category_outflow("2026-04", &category), (500, 3));
    assert_eq!(b.average_category_outflow("2026-01", &category), (0, 0));
    assert_eq!(b.average_category_outflow("2027-03", &category), (42, 12));
}
#[test]
fn scheduled_outflows_count_recurrences_and_splits_and_ignore_off_budget() {
    let mut b = Budget::new("USD");
    let account = b
        .add_account("Cash", "USD", true, 10000, "2026-01-01")
        .unwrap();
    let outside = b
        .add_account("Outside", "USD", false, 0, "2026-01-01")
        .unwrap();
    let category = b.categories[0].id.clone();
    let other = b.categories[1].id.clone();
    let mut split = transaction(&account, &category, -300, "2026-02-02");
    split.category = None;
    split.splits = vec![
        Split {
            category: Some(category.clone()),
            amount: -200,
            ..Default::default()
        },
        Split {
            category: Some(other),
            amount: -100,
            ..Default::default()
        },
    ];
    b.scheduled.push(schedule(split, Repeat::Weekly));
    b.scheduled.push(schedule(
        transaction(&outside, &category, -9999, "2026-02-02"),
        Repeat::Never,
    ));
    b.scheduled.push(schedule(
        transaction(&account, &category, 300, "2026-02-02"),
        Repeat::Never,
    ));
    assert_eq!(
        b.scheduled_category_outflows("2026-02", &category).unwrap(),
        800
    );
    assert_eq!(
        b.scheduled_category_outflows("2026-03", &category).unwrap(),
        1000
    );
}
#[test]
fn foreign_schedule_uses_latest_available_rate_or_reports_missing_rate() {
    let mut b = Budget::new("USD");
    let account = b.add_account("Euro", "EUR", true, 0, "2026-01-01").unwrap();
    let category = b.categories[0].id.clone();
    b.scheduled.push(schedule(
        transaction(&account, &category, -1000, "2026-04-10"),
        Repeat::Never,
    ));
    assert!(b.scheduled_category_outflows("2026-04", &category).is_err());
    b.add_snapshot("2026-02", "EUR", 150_000_000, "2026-02-01", "test")
        .unwrap();
    assert_eq!(
        b.scheduled_category_outflows("2026-04", &category).unwrap(),
        1500
    );
}

#[test]
fn historical_net_outflow_includes_refunds_and_excludes_off_budget() {
    let mut b = Budget::new("USD");
    let cash = b
        .add_account("Cash", "USD", true, 10_000, "2026-01-01")
        .unwrap();
    let outside = b
        .add_account("Outside", "USD", false, 0, "2026-01-01")
        .unwrap();
    let category = b.categories[0].id.clone();
    for (account, amount, date) in [
        (&cash, -10_000, "2026-01-05"),
        (&cash, 2_000, "2026-01-06"),
        (&outside, -90_000, "2026-01-07"),
        (&cash, 3_000, "2026-02-05"),
    ] {
        b.put_transaction(transaction(account, &category, amount, date))
            .unwrap();
    }
    assert_eq!(b.activity("2026-01", &category), -8_000);
    assert_eq!(b.average_category_outflow("2026-03", &category), (4_000, 2));
}
