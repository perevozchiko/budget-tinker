use super::*;

#[test]
fn filters_select_split_portion_and_payee_and_account_together() {
    let mut b = Budget::new("EUR");
    let a = b
        .add_account("Checking", "EUR", true, 0, "2026-01-01")
        .unwrap();
    let off = b
        .add_account("Tracking", "EUR", false, 0, "2026-01-01")
        .unwrap();
    let c = b.categories[0].id.clone();
    let d = b.categories[1].id.clone();
    let mut t = transaction(&a, None, -1000, "2026-01-20", "Split Shop");
    t.splits = vec![
        Split {
            category: Some(c.clone()),
            amount: -400,
            ..Default::default()
        },
        Split {
            category: Some(d),
            amount: -600,
            ..Default::default()
        },
    ];
    b.put_transaction(t).unwrap();
    b.put_transaction(transaction(
        &off,
        Some(&c),
        -800,
        "2026-01-20",
        "Split Shop",
    ))
    .unwrap();
    b.put_transaction(transaction(
        &a,
        Some(&c),
        -200,
        "2026-01-20",
        "Different Shop",
    ))
    .unwrap();
    let mut f = filter();
    f.category = Selection::Selected(vec![c]);
    f.payee = Selection::Selected(vec!["Split Shop".into()]);
    f.accounts = Selection::All;
    assert_eq!(
        reports::calculate(&b, 1, &f).unwrap().rows,
        vec![("Split Shop".into(), 1200)]
    );
    assert_eq!(reports::calculate(&b, 1, &f).unwrap().total, 1200);
    f.accounts = Selection::Selected(vec![off]);
    assert_eq!(reports::calculate(&b, 1, &f).unwrap().total, 800);
}

#[test]
fn spending_series_preserve_split_filters_hidden_groups_and_refunds() {
    let mut b = Budget::new("USD");
    let a = b
        .add_account("Checking", "USD", true, 0, "2026-01-01")
        .unwrap();
    let other = b
        .add_account("Other", "USD", true, 0, "2026-01-01")
        .unwrap();
    let c = b.categories[0].id.clone();
    let d = b.categories[1].id.clone();
    b.categories[0].group = "Travel".into();
    b.categories[1].group = "Hidden group".into();
    b.category_groups.push(CategoryGroup {
        name: "Hidden group".into(),
        hidden: true,
    });
    let mut t = transaction(&a, None, -1000, "2026-01-20", "Trip");
    t.splits = vec![
        Split {
            category: Some(c.clone()),
            amount: -750,
            ..Default::default()
        },
        Split {
            category: Some(d),
            amount: -250,
            ..Default::default()
        },
    ];
    b.put_transaction(t).unwrap();
    b.put_transaction(transaction(&a, Some(&c), 200, "2026-02-10", "Trip"))
        .unwrap();
    b.put_transaction(transaction(&a, None, -50, "2026-02-10", "Trip"))
        .unwrap();
    b.put_transaction(transaction(&a, None, 5000, "2026-02-10", "Income"))
        .unwrap();
    b.transfer(&a, &other, 100, 100, "2026-01-20", "Internal")
        .unwrap();
    let r = reports::calculate(&b, 2, &filter()).unwrap();
    assert_eq!(r.rows, [("2026-01".into(), 1000), ("2026-02".into(), -150)]);
    for (label, values) in [
        ("Travel", vec![750, -200]),
        ("Hidden Categories", vec![250, 0]),
        ("Uncategorized", vec![0, 50]),
    ] {
        assert_eq!(
            r.spending_series
                .iter()
                .find(|s| s.label == label)
                .unwrap()
                .values,
            values
        );
    }
    for (i, month) in r.months.iter().enumerate() {
        assert_eq!(
            r.spending_series.iter().map(|s| s.values[i]).sum::<i64>(),
            month.expense
        );
    }
    let bytes = r.csv("USD", str::to_owned).unwrap();
    let records = csv::Reader::from_reader(bytes.as_slice())
        .records()
        .map(|r| r.unwrap())
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 8);
    assert!(
        records
            .iter()
            .any(|r| &r[3] == "2026-02" && &r[4] == "Travel" && &r[5] == "-2.00")
    );
    let f = Filter {
        category: Selection::Selected(vec![c]),
        payee: Selection::Selected(vec!["Trip".into()]),
        accounts: Selection::Selected(vec![a]),
        ..filter()
    };
    let filtered = reports::calculate(&b, 2, &f).unwrap();
    assert_eq!(filtered.spending_series.len(), 1);
    assert_eq!(filtered.total, 550);
    assert_eq!(filtered.spending_series[0].values, [750, -200]);
}

#[test]
fn drilldown_preserves_split_amounts_category_identity_period_and_filters() {
    use reports::SpendingScope;
    let mut b = Budget::new("USD");
    let a = b
        .add_account("Checking", "USD", true, 0, "2026-01-01")
        .unwrap();
    let other = b
        .add_account("Other", "USD", true, 0, "2026-01-01")
        .unwrap();
    let c = b.categories[0].id.clone();
    let d = b.categories[1].id.clone();
    for category in &mut b.categories[..2] {
        category.group = "Home".into();
        category.name = "Same name".into();
    }
    let mut t = transaction(&a, None, -1001, "2026-01-20", "Shop, Inc.");
    t.memo = "A \"quoted\" memo".into();
    t.splits = vec![
        Split {
            category: Some(c.clone()),
            amount: -700,
            ..Default::default()
        },
        Split {
            category: Some(d.clone()),
            amount: -301,
            ..Default::default()
        },
    ];
    let transaction_id = t.id.clone();
    b.put_transaction(t).unwrap();
    b.put_transaction(transaction(&a, Some(&c), 200, "2026-02-10", "Shop, Inc."))
        .unwrap();
    b.put_transaction(transaction(
        &other,
        Some(&c),
        -9000,
        "2026-01-20",
        "Shop, Inc.",
    ))
    .unwrap();
    b.put_transaction(transaction(&a, Some(&c), -9000, "2026-01-20", "Elsewhere"))
        .unwrap();
    b.transfer(&a, &other, 100, 100, "2026-01-20", "Internal")
        .unwrap();
    let f = Filter {
        accounts: Selection::Selected(vec![a]),
        payee: Selection::Selected(vec!["Shop, Inc.".into()]),
        ..filter()
    };
    let r = reports::calculate(&b, 0, &f).unwrap();
    assert_eq!(r.total, 801);
    let group = SpendingScope::Group("Home".into());
    let children = r.spending_rows(Some(&group), None);
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].scope, SpendingScope::Category(Some(c.clone())));
    assert_eq!(children[0].value, 500);
    assert_eq!(children[1].scope, SpendingScope::Category(Some(d)));
    assert_eq!(children[1].value, 301);
    let entries = r.spending_details(&children[0].scope, None);
    assert_eq!(
        entries.iter().map(|e| e.value).collect::<Vec<_>>(),
        [700, -200]
    );
    assert_eq!(entries[0].transaction_id, transaction_id);
    assert_eq!(
        r.spending_rows(Some(&group), Some("2026-02"))[0].value,
        -200
    );
    let bytes = r
        .spending_csv(&children[0].scope, Some("2026-01"), "USD")
        .unwrap();
    let records = csv::Reader::from_reader(bytes.as_slice())
        .records()
        .map(|r| r.unwrap())
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 1);
    assert_eq!(&records[0][6], "Shop, Inc.");
    assert_eq!(&records[0][9], "A \"quoted\" memo");
    assert_eq!(&records[0][10], "7.00");
    assert_eq!(
        f,
        Filter {
            accounts: f.accounts.clone(),
            payee: Selection::Selected(vec!["Shop, Inc.".into()]),
            ..filter()
        }
    );
    let narrowed = reports::calculate(
        &b,
        0,
        &Filter {
            category: Selection::Selected(vec![c]),
            ..f
        },
    )
    .unwrap();
    assert_eq!(narrowed.spending_rows(Some(&group), None).len(), 1);
    assert_eq!(narrowed.total, 500);
}

#[test]
fn budget_accounts_default_excludes_off_budget_tracking_accounts() {
    let mut b = Budget::new("EUR");
    let on = b
        .add_account("Checking", "EUR", true, 0, "2026-01-01")
        .unwrap();
    let off = b
        .add_account("Tracking", "EUR", false, 0, "2026-01-01")
        .unwrap();
    let c = b.categories[0].id.clone();
    b.put_transaction(transaction(&on, Some(&c), -500, "2026-01-20", "Shop"))
        .unwrap();
    b.put_transaction(transaction(&off, Some(&c), -300, "2026-01-20", "Shop"))
        .unwrap();
    // Default filter uses BudgetAccounts, so only on-budget account is included.
    let r = reports::calculate(&b, 0, &filter()).unwrap();
    assert_eq!(r.total, 500);
    // Explicit All includes both.
    let mut f = filter();
    f.accounts = Selection::All;
    let r2 = reports::calculate(&b, 0, &f).unwrap();
    assert_eq!(r2.total, 800);
}

#[test]
fn selected_accounts_filter_only_explicit_ids() {
    let mut b = Budget::new("EUR");
    let a1 = b
        .add_account("Checking", "EUR", true, 0, "2026-01-01")
        .unwrap();
    let a2 = b
        .add_account("Savings", "EUR", true, 0, "2026-01-01")
        .unwrap();
    let c = b.categories[0].id.clone();
    b.put_transaction(transaction(&a1, Some(&c), -500, "2026-01-20", "Shop"))
        .unwrap();
    b.put_transaction(transaction(&a2, Some(&c), -300, "2026-01-20", "Shop"))
        .unwrap();
    // Select only a1.
    let f = Filter {
        accounts: Selection::Selected(vec![a1]),
        ..filter()
    };
    let r = reports::calculate(&b, 0, &f).unwrap();
    assert_eq!(r.total, 500);
}

#[test]
fn none_selected_and_empty_selected_are_empty_for_each_dimension() {
    let mut b = Budget::new("EUR");
    let checking = b
        .add_account("Checking", "EUR", true, 0, "2026-01-01")
        .unwrap();
    let category = b.categories[0].id.clone();
    b.put_transaction(transaction(
        &checking,
        Some(&category),
        -500,
        "2026-01-20",
        "Shop",
    ))
    .unwrap();
    let mut f = filter();
    assert_eq!(reports::calculate(&b, 0, &f).unwrap().total, 500);
    f.category = Selection::NoneSelected;
    assert_eq!(reports::calculate(&b, 0, &f).unwrap().total, 0);
    f.category = Selection::Selected(vec![]);
    assert_eq!(reports::calculate(&b, 0, &f).unwrap().total, 0);
    f.category = Selection::All;
    f.payee = Selection::NoneSelected;
    assert_eq!(reports::calculate(&b, 0, &f).unwrap().total, 0);
    f.payee = Selection::Selected(vec![]);
    assert_eq!(reports::calculate(&b, 0, &f).unwrap().total, 0);
    f.payee = Selection::All;
    f.accounts = Selection::NoneSelected;
    assert_eq!(reports::calculate(&b, 0, &f).unwrap().total, 0);
    f.accounts = Selection::Selected(vec![]);
    assert_eq!(reports::calculate(&b, 4, &f).unwrap().total, 0);
}

#[test]
fn empty_payee_can_be_selected_in_spending_reports() {
    let mut b = Budget::new("EUR");
    let checking = b
        .add_account("Checking", "EUR", true, 0, "2026-01-01")
        .unwrap();
    let category = b.categories[0].id.clone();
    b.put_transaction(transaction(
        &checking,
        Some(&category),
        -250,
        "2026-01-20",
        "",
    ))
    .unwrap();
    b.put_transaction(transaction(
        &checking,
        Some(&category),
        -150,
        "2026-01-20",
        "Shop",
    ))
    .unwrap();
    let f = Filter {
        payee: Selection::Selected(vec![String::new()]),
        ..filter()
    };
    assert_eq!(reports::calculate(&b, 1, &f).unwrap().total, 250);
}

#[test]
fn uncategorized_selection_excludes_income_transfers_and_categorized_parts() {
    let mut b = Budget::new("USD");
    let account = b
        .add_account("Checking", "USD", true, 10000, "2026-01-01")
        .unwrap();
    let other = b
        .add_account("Savings", "USD", true, 0, "2026-01-01")
        .unwrap();
    let category = b.categories[0].id.clone();
    b.put_transaction(transaction(&account, None, -700, "2026-01-20", "Unknown"))
        .unwrap();
    b.put_transaction(transaction(&account, None, 5000, "2026-01-20", "Salary"))
        .unwrap();
    let mut split = transaction(&account, None, -1000, "2026-01-20", "Split");
    split.splits = vec![
        Split {
            category: Some(category.clone()),
            amount: -400,
            ..Default::default()
        },
        Split {
            category: Some(b.categories[1].id.clone()),
            amount: -600,
            ..Default::default()
        },
    ];
    b.put_transaction(split).unwrap();
    b.transfer(&account, &other, 300, 300, "2026-01-20", "Transfer")
        .unwrap();
    let mut f = filter();
    f.category = Selection::Selected(vec![UNCATEGORIZED_CATEGORY.into()]);
    let before = serde_json::to_value(&b).unwrap();
    for kind in 0..4 {
        let report = reports::calculate(&b, kind, &f).unwrap();
        assert_eq!(report.months[0].expense, 700);
        assert_eq!(report.months[0].income, 0);
    }
    f.category = Selection::Selected(vec![UNCATEGORIZED_CATEGORY.into(), category]);
    assert_eq!(reports::calculate(&b, 0, &f).unwrap().total, 1100);
    // New selection keys use the existing serialized representation.
    let encoded = serde_json::to_string(&f.category).unwrap();
    assert_eq!(
        serde_json::from_str::<Selection>(&encoded).unwrap(),
        f.category
    );
    assert_eq!(serde_json::to_value(&b).unwrap(), before);
}
