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

#[test]
fn three_month_average_caps_history_and_counts_empty_months_with_integer_rounding() {
    let mut b = Budget::new("USD");
    let account = b.add_account("Cash", "USD", true, 0, "2025-01-01").unwrap();
    let category = b.categories[0].id.clone();
    for (date, value) in [
        ("2025-01-05", -9999),
        ("2026-01-05", -100),
        ("2026-03-05", -102),
    ] {
        b.put_transaction(transaction(&account, &category, value, date))
            .unwrap();
    }
    assert_eq!(
        b.average_category_outflow_over("2026-04", &category, 3),
        (67, 3)
    );
    assert_eq!(b.average_category_outflow("2026-04", &category), (17, 12));
    let mut fresh = Budget::new("USD");
    fresh
        .allocate("2026-03", &category_for(&fresh), 123)
        .unwrap();
    assert_eq!(
        fresh.average_category_outflow_over("2026-04", &category_for(&fresh), 3),
        (0, 1)
    );
    assert_eq!(
        fresh.average_category_outflow_over("2026-03", &category_for(&fresh), 3),
        (0, 0)
    );
}
fn category_for(b: &Budget) -> String {
    b.categories[0].id.clone()
}

#[test]
fn month_schedule_missing_rate_is_atomic_and_filled_cells_do_not_need_rates() {
    let mut b = Budget::new("USD");
    let cash = b.add_account("Cash", "USD", true, 0, "2026-01-01").unwrap();
    let euro = b.add_account("Euro", "EUR", true, 0, "2026-01-01").unwrap();
    let first = b.categories[0].id.clone();
    let second = b.categories[1].id.clone();
    for (account, category) in [(&cash, &first), (&euro, &second)] {
        b.scheduled.push(schedule(
            transaction(account, category, -100, "2026-04-10"),
            Repeat::Never,
        ));
    }
    let original = serde_json::to_value(&b).unwrap();
    assert!(
        b.quick_budget_month("2026-04", QuickBudget::Scheduled)
            .is_err()
    );
    assert_eq!(serde_json::to_value(&b).unwrap(), original);
    assert!(
        b.quick_budget_month("invalid", QuickBudget::PreviousBudget)
            .is_err()
    );
    assert_eq!(serde_json::to_value(&b).unwrap(), original);
    b.allocate("2026-04", &second, -200).unwrap();
    b.quick_budget_month("2026-04", QuickBudget::Scheduled)
        .unwrap();
    assert_eq!(b.allocated("2026-04", &first), 100);
    assert_eq!(b.allocated("2026-04", &second), -200);
}
