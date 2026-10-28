use super::*;

#[test]
fn expenses_net_refunds_and_exclude_income_transfers_and_outside_dates() {
    let mut b = Budget::new("EUR");
    let a = b
        .add_account("Checking", "EUR", true, 100_000, "2026-01-01")
        .unwrap();
    let other = b
        .add_account("Savings", "EUR", true, 0, "2026-01-01")
        .unwrap();
    let c = b.categories[0].id.clone();
    for (value, date) in [
        (-1000, "2026-01-10"),
        (200, "2026-02-15"),
        (-9000, "2026-02-16"),
        (-7000, "2026-01-09"),
    ] {
        b.put_transaction(transaction(&a, Some(&c), value, date, "Shop"))
            .unwrap();
    }
    b.put_transaction(transaction(&a, None, 5000, "2026-02-01", "Salary"))
        .unwrap();
    b.transfer(&a, &other, 300, 300, "2026-02-01", "Transfer")
        .unwrap();
    let report = reports::calculate(&b, 0, &filter()).unwrap();
    assert_eq!(report.total, 800);
    assert_eq!(report.rows, vec![(b.categories[0].group.clone(), 800)]);
    let income = reports::calculate(&b, 3, &filter()).unwrap();
    assert_eq!(
        income.rows,
        vec![
            ("Income".into(), 5000),
            ("Expenses".into(), 800),
            ("Net Income".into(), 4200)
        ]
    );
    assert_eq!(income.months[0].expense, 1000);
    assert_eq!(income.months[1].expense, -200);
}

#[test]
fn hidden_categories_are_grouped_and_refund_only_groups_remain_visible() {
    let mut b = Budget::new("EUR");
    let a = b
        .add_account("Checking", "EUR", true, 0, "2026-01-01")
        .unwrap();
    b.categories[0].hidden = true;
    let hidden = b.categories[0].id.clone();
    let visible = b.categories[1].id.clone();
    b.put_transaction(transaction(&a, Some(&hidden), -100, "2026-01-20", "Shop"))
        .unwrap();
    b.put_transaction(transaction(&a, Some(&visible), 200, "2026-01-20", "Refund"))
        .unwrap();
    let r = reports::calculate(&b, 0, &filter()).unwrap();
    assert_eq!(r.total, -100);
    assert!(r.rows.contains(&("Hidden Categories".into(), 100)));
    assert!(r.rows.contains(&(b.categories[1].group.clone(), -200)));
}

#[test]
fn split_conversion_preserves_the_transaction_rounding_remainder() {
    let mut b = Budget::new("EUR");
    b.add_snapshot("2026-01", "USD", 90_000_000, "2026-01-01", "test")
        .unwrap();
    let a = b.add_account("USD", "USD", true, 0, "2026-01-01").unwrap();
    let mut t = transaction(&a, None, -3, "2026-01-20", "Shop");
    t.splits = (0..3)
        .map(|index| Split {
            category: Some(b.categories[index].id.clone()),
            amount: -1,
            ..Default::default()
        })
        .collect();
    b.put_transaction(t).unwrap();
    assert_eq!(reports::calculate(&b, 0, &filter()).unwrap().total, 3);
    let mut f = filter();
    f.category = Selection::Selected(vec![b.categories[2].id.clone()]);
    assert_eq!(reports::calculate(&b, 1, &f).unwrap().total, 1);
}

#[test]
fn invalid_ranges_return_errors_and_empty_months_are_retained() {
    let b = Budget::new("EUR");
    let mut f = filter();
    let r = reports::calculate(&b, 2, &f).unwrap();
    assert_eq!(r.rows, vec![("2026-01".into(), 0), ("2026-02".into(), 0)]);
    f.from = "2026-02-30".into();
    assert!(reports::calculate(&b, 0, &f).is_err());
    f.from = "2026-03-01".into();
    assert!(reports::calculate(&b, 0, &f).is_err());
    f.from = "1900-01-01".into();
    assert!(reports::calculate(&b, 0, &f).is_err());
    f.from = "9999-12-01".into();
    f.through = "9999-12-31".into();
    assert_eq!(reports::calculate(&b, 4, &f).unwrap().total, 0);
}

#[test]
fn payee_other_preserves_all_members_refunds_and_reserved_names() {
    use reports::SpendingScope;
    let mut b = Budget::new("USD");
    let a = b
        .add_account("Checking", "USD", true, 0, "2026-01-01")
        .unwrap();
    let c = b.categories[0].id.clone();
    for index in 0..31 {
        let payee = match index {
            29 => "All Other Payees".into(),
            30 => String::new(),
            _ => format!("Payee {index:02}"),
        };
        b.put_transaction(transaction(
            &a,
            Some(&c),
            -(3100 - index * 100),
            "2026-01-20",
            &payee,
        ))
        .unwrap();
    }
    b.put_transaction(transaction(&a, Some(&c), 500, "2026-02-10", "Refund only"))
        .unwrap();
    let before = serde_json::to_string(&b).unwrap();
    let r = reports::calculate(&b, 1, &filter()).unwrap();
    let rows = r.spending_rows(None, None);
    assert_eq!(rows.len(), 10);
    assert_eq!(rows.iter().map(|r| r.value).sum::<i64>(), r.total);
    let other = rows
        .iter()
        .find(|r| matches!(r.scope, SpendingScope::Other(_)))
        .unwrap();
    let members = r.spending_rows(Some(&other.scope), None);
    assert_eq!(members.len(), 23);
    assert_eq!(members.iter().map(|r| r.value).sum::<i64>(), other.value);
    assert!(
        members
            .iter()
            .any(|r| r.scope == SpendingScope::Payee("All Other Payees".into()))
    );
    assert!(
        members
            .iter()
            .any(|r| r.scope == SpendingScope::Payee(String::new()))
    );
    assert!(members.iter().any(|r| r.value == -500));
    assert_eq!(
        r.spending_details(&other.scope, None)
            .iter()
            .map(|e| e.value)
            .sum::<i64>(),
        other.value
    );
    assert_eq!(serde_json::to_string(&b).unwrap(), before);
    let f = Filter {
        payee: Selection::Selected(vec!["Payee 00".into()]),
        ..filter()
    };
    let narrow = reports::calculate(&b, 1, &f).unwrap();
    assert_eq!(narrow.spending_rows(None, None).len(), 1);
    assert!(matches!(
        narrow.spending_rows(None, None)[0].scope,
        SpendingScope::Payee(_)
    ));
}

#[test]
fn multiple_accounts_payees_and_split_categories_combine_correctly() {
    let mut b = Budget::new("EUR");
    let checking = b
        .add_account("Checking", "EUR", true, 0, "2026-01-01")
        .unwrap();
    let savings = b
        .add_account("Savings", "EUR", true, 0, "2026-01-01")
        .unwrap();
    let tracking = b
        .add_account("Tracking", "EUR", false, 0, "2026-01-01")
        .unwrap();
    let groceries = b.categories[0].id.clone();
    let transport = b.categories[1].id.clone();
    let other = b.categories[2].id.clone();
    let mut split = transaction(&checking, None, -1000, "2026-01-20", "Shop");
    split.splits = vec![
        Split {
            category: Some(groceries.clone()),
            amount: -400,
            ..Default::default()
        },
        Split {
            category: Some(transport.clone()),
            amount: -600,
            ..Default::default()
        },
    ];
    b.put_transaction(split).unwrap();
    b.put_transaction(transaction(
        &tracking,
        Some(&groceries),
        -300,
        "2026-01-20",
        "Cafe",
    ))
    .unwrap();
    b.put_transaction(transaction(
        &savings,
        Some(&groceries),
        -200,
        "2026-01-20",
        "Other",
    ))
    .unwrap();
    b.put_transaction(transaction(
        &checking,
        Some(&other),
        -100,
        "2026-01-20",
        "Shop",
    ))
    .unwrap();

    let mut f = filter();
    f.accounts = Selection::Selected(vec![checking, tracking]);
    f.category = Selection::Selected(vec![groceries, transport.clone()]);
    f.payee = Selection::Selected(vec!["Shop".into(), "Cafe".into()]);
    assert_eq!(reports::calculate(&b, 0, &f).unwrap().total, 1300);

    f.category = Selection::Selected(vec![transport]);
    assert_eq!(reports::calculate(&b, 0, &f).unwrap().total, 600);
    f.payee = Selection::Selected(vec!["Cafe".into()]);
    assert_eq!(reports::calculate(&b, 0, &f).unwrap().total, 0);
}
