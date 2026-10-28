use budgettinker::{credit::*, model::*, store::*};

fn post(b: &mut Budget, account: &str, category: &str, date: &str, amount: i64) {
    b.put_transaction(Transaction {
        id: id(),
        account: account.into(),
        category: Some(category.into()),
        date: date.into(),
        amount,
        base_amount: 0,
        snapshot: None,
        income_month: None,
        payee: "Card merchant".into(),
        memo: String::new(),
        cleared: false,
        reconciled: false,
        transfer: None,
        splits: vec![],
        check_number: None,
        flag: None,
    })
    .unwrap();
}
fn summary(b: &Budget, month: &str, expected: [i64; 5]) {
    let s = b.month_summary(month).unwrap();
    assert_eq!(
        [
            s.not_budgeted,
            s.overspent,
            s.income,
            s.budgeted,
            s.available
        ],
        expected
    );
    assert_eq!(
        s.not_budgeted + s.overspent + s.income - s.budgeted,
        s.available
    );
}
fn document(b: &Budget) -> serde_json::Value {
    serde_json::to_value(b).unwrap()
}

#[test]
fn old_debt_payments_interest_and_refund_after_paying_in_full() {
    let mut b = Budget::new("USD");
    let groceries = b.categories[0].id.clone();
    let interest = b.categories[1].id.clone();
    let cash = b
        .add_account("Checking", "USD", true, 200_000, "2026-01-01")
        .unwrap();
    let card = b
        .add_typed_account("Visa", "USD", true, -100_000, "2026-01-01", "credit")
        .unwrap();
    let debt = b.initial_debt_category(&card).unwrap().id.clone();
    assert_eq!(b.category_balance("2026-01", &debt), -100_000);
    summary(&b, "2026-01", [0, 0, 200_000, 0, 200_000]);
    for (c, value) in [(&debt, 20_000), (&groceries, 10_000), (&interest, 2000)] {
        b.allocate("2026-01", c, value).unwrap();
    }
    post(&mut b, &card, &groceries, "2026-01-10", -10_000);
    post(&mut b, &card, &interest, "2026-01-15", -2000);
    let before_payment = b.available("2026-01").unwrap();
    b.transfer(&cash, &card, 32_000, 32_000, "2026-01-20", "Payment")
        .unwrap();
    assert_eq!(b.available("2026-01").unwrap(), before_payment);
    assert_eq!(b.category_balance("2026-01", &debt), -80_000);
    assert_eq!(b.balance(&card, Some("2026-01-31"), false), -80_000);
    assert_eq!(b.activity("2026-01", &debt), -100_000);
    summary(&b, "2026-01", [0, 0, 200_000, 32_000, 168_000]);
    // Empty intervening months preserve old debt, without a second ATB charge.
    summary(&b, "2026-02", [168_000, 0, 0, 0, 168_000]);
    assert_eq!(b.category_balance("2026-03", &debt), -80_000);
    for (c, value) in [(&debt, 30_000), (&groceries, 5000), (&interest, 1000)] {
        b.allocate("2026-03", c, value).unwrap();
    }
    post(&mut b, &card, &groceries, "2026-03-10", -5000);
    post(&mut b, &card, &interest, "2026-03-15", -1000);
    b.transfer(&cash, &card, 36_000, 36_000, "2026-03-20", "Payment")
        .unwrap();
    summary(&b, "2026-03", [168_000, 0, 0, 36_000, 132_000]);
    assert_eq!(b.category_balance("2026-03", &debt), -50_000);
    b.allocate("2026-04", &debt, 50_000).unwrap();
    b.transfer(&cash, &card, 50_000, 50_000, "2026-04-10", "Final payment")
        .unwrap();
    assert_eq!(b.balance(&card, None, false), 0);
    post(&mut b, &card, &groceries, "2026-04-15", 4000);
    assert_eq!(b.balance(&card, None, false), 4000);
    assert_eq!(b.category_balance("2026-04", &groceries), 4000);
    assert_eq!(b.category_balance("2026-04", &debt), 0);
    summary(&b, "2026-04", [132_000, 0, 0, 50_000, 82_000]);
    validate(&b).unwrap();
}

#[test]
fn opening_sign_budget_status_and_independent_cards() {
    let mut b = Budget::new("USD");
    for (name, kind, on_budget, opening, expected) in [
        ("Visa", "credit", true, -100, true),
        ("Mastercard", "credit", true, -200, true),
        ("Zero card", "credit", true, 0, true),
        ("Credit line", "line_of_credit", true, -300, true),
        ("Positive card", "credit", true, 400, false),
        ("Off budget", "credit", false, -500, false),
        ("Overdrawn", "checking", true, -600, true),
        ("Zero checking", "checking", true, 0, false),
    ] {
        let a = b
            .add_typed_account(name, "USD", on_budget, opening, "2026-01-01", kind)
            .unwrap();
        assert_eq!(b.initial_debt_category(&a).is_some(), expected);
        if expected {
            let c = b.initial_debt_category(&a).unwrap();
            assert_eq!(b.category_balance("2026-04", &c.id), opening);
            assert_eq!(
                *b.overspending_rule("2026-04", &c.id),
                OverspendingRule::CategoryBalance
            );
        }
    }
    assert_eq!(
        b.category_groups
            .iter()
            .filter(|g| g.name == INITIAL_DEBT_GROUP)
            .count(),
        1
    );
    assert_eq!(b.available("2026-04").unwrap(), 400);
    validate(&b).unwrap();
}

#[test]
fn failed_creation_is_atomic_and_foreign_old_debt_retains_precision() {
    for (currency, opening, rate, expected) in [
        ("JPY", -101, 625_000, -63),
        ("KWD", -1234, 300_000_000, -370),
    ] {
        let mut b = Budget::new("EUR");
        let original = document(&b);
        for (value, date) in [
            (opening, "2026-01-01"),
            (i64::MIN, "2026-01-01"),
            (opening, "invalid"),
        ] {
            assert!(
                b.add_typed_account("Card", currency, true, value, date, "credit")
                    .is_err()
            );
            assert_eq!(document(&b), original);
        }
        b.add_snapshot("2026-01", currency, rate, "2026-01-01", "test")
            .unwrap();
        let a = b
            .add_typed_account("Card", currency, true, opening, "2026-01-01", "credit")
            .unwrap();
        let c = &b.initial_debt_category(&a).unwrap().id;
        assert_eq!(b.category_balance("2026-01", c), expected);
        assert_eq!(b.available("2026-01").unwrap(), 0);
        validate(&b).unwrap();
    }
}

#[test]
fn stable_category_identity_rename_hidden_sqlite_json_and_legacy_documents() {
    let mut b = Budget::new("USD");
    let a = b
        .add_typed_account("Visa", "USD", true, -10_000, "2026-01-01", "credit")
        .unwrap();
    let c = b.initial_debt_category(&a).unwrap().id.clone();
    b.allocate("2026-02", &c, 3000).unwrap();
    b.rename_account(&a, "Family: Visa").unwrap();
    assert_eq!(b.initial_debt_category(&a).unwrap().name, "Family- Visa");
    assert_eq!(b.initial_debt_category(&a).unwrap().id, c);
    b.set_group_hidden(INITIAL_DEBT_GROUP, true).unwrap();
    assert_eq!(b.category_balance("2026-04", &c), -7000);
    let root = std::env::temp_dir().join(format!("credit-debt-{}", id()));
    std::fs::create_dir_all(&root).unwrap();
    {
        let mut s = Store::open(&root.join("budget.sqlite3")).unwrap();
        s.save(&b).unwrap();
        assert_eq!(document(&s.load().unwrap().unwrap()), document(&b));
        export_json(&b, &root.join("budget.json")).unwrap();
        assert_eq!(
            document(&import_json(&root.join("budget.json")).unwrap()),
            document(&b)
        );
        // Old credit-account documents have no special category. Loading keeps
        // their intentional categorization and totals exactly as saved.
        b.transactions[0].category = None;
        b.categories.retain(|cat| cat.id != c);
        b.allocations.clear();
        b.overspending_rules.clear();
        s.save(&b).unwrap();
        let restored = s.load().unwrap().unwrap();
        assert!(restored.initial_debt_category(&a).is_none());
        assert_eq!(document(&restored), document(&b));
        assert_eq!(restored.available("2026-04").unwrap(), -10_000);
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn fresh_start_replaces_old_debt_with_the_current_balance() {
    let mut b = Budget::new("USD");
    let cash = b
        .add_account("Checking", "USD", true, 200_000, "2026-01-01")
        .unwrap();
    let card = b
        .add_typed_account("Visa", "USD", true, -100_000, "2026-01-01", "credit")
        .unwrap();
    let c = b.initial_debt_category(&card).unwrap().id.clone();
    b.allocate("2026-01", &c, 20_000).unwrap();
    b.transfer(&cash, &card, 20_000, 20_000, "2026-01-20", "Payment")
        .unwrap();
    let original = document(&b);
    let next = b.fresh_start().unwrap();
    assert_eq!(document(&b), original);
    assert_eq!(
        next.categories
            .iter()
            .filter(|c| debt_account(&c.id) == Some(&card))
            .count(),
        1
    );
    assert_eq!(next.category_balance(&current_month(), &c), -80_000);
    assert_eq!(
        next.category_balance(&month_shift(&current_month(), 3), &c),
        -80_000
    );
    assert_eq!(next.available(&current_month()).unwrap(), 180_000);
    validate(&next).unwrap();
}
