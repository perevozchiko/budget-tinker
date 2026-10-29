use budgettinker::{model::*, store::*};

fn fixture() -> (Budget, String, String) {
    let mut b = Budget::new("EUR");
    let account = b
        .add_account("Cash", "EUR", true, 100_000, "2026-01-01")
        .unwrap();
    let category = b.categories[0].id.clone();
    (b, account, category)
}
fn post(b: &mut Budget, account: &str, category: &str, date: &str, amount: i64) {
    b.put_transaction(Transaction {
        operation_fx: None,
        check_number: None,
        flag: None,
        id: id(),
        account: account.into(),
        category: Some(category.into()),
        date: date.into(),
        amount,
        base_amount: 0,
        snapshot: None,
        income_month: None,
        payee: "Shop".into(),
        memo: String::new(),
        cleared: false,
        reconciled: false,
        transfer: None,
        splits: vec![],
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
#[test]
fn default_charges_next_month_once_and_keeps_current_month_available() {
    let (mut b, a, c) = fixture();
    b.allocate("2026-01", &c, 10_000).unwrap();
    post(&mut b, &a, &c, "2026-01-10", -13_000);
    assert_eq!(b.category_balance("2026-01", &c), -3000);
    assert_eq!(b.category_balance("2026-02", &c), 0);
    summary(&b, "2026-01", [0, 0, 100_000, 10_000, 90_000]);
    summary(&b, "2026-02", [90_000, -3000, 0, 0, 87_000]);
    summary(&b, "2026-03", [87_000, 0, 0, 0, 87_000]);
    assert_eq!(b.category_balance("2029-12", &c), 0);
    b.allocate("2026-02", &c, 5000).unwrap();
    assert_eq!(b.category_balance("2026-02", &c), 5000);
    summary(&b, "2026-02", [90_000, -3000, 0, 5000, 82_000]);
}
#[test]
fn category_rule_carries_until_changed_even_in_an_empty_month() {
    let (mut b, a, c) = fixture();
    b.set_overspending_rule("2026-01", &c, OverspendingRule::CategoryBalance)
        .unwrap();
    b.allocate("2026-01", &c, 10_000).unwrap();
    post(&mut b, &a, &c, "2026-01-10", -13_000);
    assert_eq!(b.category_balance("2026-03", &c), -3000);
    summary(&b, "2026-02", [90_000, 0, 0, 0, 90_000]);
    b.set_overspending_rule("2026-03", &c, OverspendingRule::AvailableToBudget)
        .unwrap();
    assert_eq!(b.category_balance("2026-03", &c), -3000);
    assert_eq!(b.category_balance("2026-04", &c), 0);
    summary(&b, "2026-04", [90_000, -3000, 0, 0, 87_000]);
    summary(&b, "2026-05", [87_000, 0, 0, 0, 87_000]);
    // A later explicit choice survives edits to an earlier rule.
    b.set_overspending_rule("2026-05", &c, OverspendingRule::CategoryBalance)
        .unwrap();
    b.set_overspending_rule("2026-01", &c, OverspendingRule::AvailableToBudget)
        .unwrap();
    assert_eq!(
        *b.overspending_rule("2026-06", &c),
        OverspendingRule::CategoryBalance
    );
    assert_eq!(b.category_balance("2026-02", &c), 0);
}
#[test]
fn refunds_positive_rollover_and_repeated_overspending() {
    let (mut b, a, c) = fixture();
    b.allocate("2026-01", &c, 10_000).unwrap();
    post(&mut b, &a, &c, "2026-01-10", -13_000);
    post(&mut b, &a, &c, "2026-01-11", 1000);
    post(&mut b, &a, &c, "2026-02-02", 1500);
    assert_eq!(b.overspent("2026-01"), -2000);
    assert_eq!(b.category_balance("2026-02", &c), 1500);
    post(&mut b, &a, &c, "2026-03-01", -2000);
    assert_eq!(b.category_balance("2026-03", &c), -500);
    assert_eq!(b.category_balance("2026-04", &c), 0);
    summary(&b, "2026-04", [88_000, -500, 0, 0, 87_500]);
    // A backdated refund recomputes all later months without cached balances.
    post(&mut b, &a, &c, "2026-01-15", 4000);
    assert_eq!(b.overspent("2026-01"), 0);
    assert_eq!(b.category_balance("2026-04", &c), 1500);
    assert_eq!(b.available("2026-04").unwrap(), 90_000);
}
#[test]
fn hidden_categories_and_mixed_rules_never_net_overspending_against_surpluses() {
    let (mut b, a, c) = fixture();
    let carry = b.categories[1].id.clone();
    let positive = b.categories[2].id.clone();
    b.categories[0].hidden = true;
    b.set_overspending_rule("2026-01", &carry, OverspendingRule::CategoryBalance)
        .unwrap();
    b.allocate("2026-01", &positive, 5000).unwrap();
    post(&mut b, &a, &c, "2026-01-02", -3000);
    post(&mut b, &a, &carry, "2026-01-02", -2000);
    assert_eq!(b.overspent("2026-01"), -3000);
    assert_eq!(b.category_balance("2026-02", &carry), -2000);
    assert_eq!(b.category_balance("2026-02", &positive), 5000);
    summary(&b, "2026-02", [95_000, -3000, 0, 0, 92_000]);
}
#[test]
fn foreign_splits_use_snapshot_rounding_and_summary_includes_revaluation() {
    let (mut b, _, c) = fixture();
    b.add_snapshot("2026-01", "USD", 90_000_000, "2026-01-01", "test")
        .unwrap();
    b.add_snapshot("2026-02", "USD", 95_000_000, "2026-02-01", "test")
        .unwrap();
    let usd = b
        .add_account("Dollar", "USD", true, 100, "2026-01-01")
        .unwrap();
    let other = b.categories[1].id.clone();
    let t = Transaction {
        operation_fx: None,
        check_number: None,
        flag: None,
        id: id(),
        account: usd,
        category: None,
        date: "2026-01-02".into(),
        amount: -3,
        base_amount: 0,
        snapshot: None,
        income_month: None,
        payee: "Split".into(),
        memo: String::new(),
        cleared: false,
        reconciled: false,
        transfer: None,
        splits: vec![
            Split {
                category: Some(c.clone()),
                amount: -1,
                ..Default::default()
            },
            Split {
                category: Some(other.clone()),
                amount: -2,
                ..Default::default()
            },
        ],
    };
    b.put_transaction(t).unwrap();
    assert_eq!(b.overspent("2026-01"), -3);
    assert_eq!(b.category_balance("2026-02", &c), 0);
    assert_eq!(b.category_balance("2026-02", &other), 0);
    summary(&b, "2026-02", [100_090, -3, 5, 0, 100_092]);
    assert!(b.month_summary("2026-03").is_err());
}
#[test]
fn legacy_json_and_sqlite_preserve_negative_rollover_and_migrate_on_save() {
    let (mut b, a, c) = fixture();
    post(&mut b, &a, &c, "2026-01-10", -3000);
    let mut json = serde_json::to_value(&b).unwrap();
    json.as_object_mut()
        .unwrap()
        .remove("default_overspending_rule");
    json.as_object_mut().unwrap().remove("overspending_rules");
    let path = std::env::temp_dir().join(format!("budgettinker-rollover-{}.sqlite3", id()));
    {
        let mut store = Store::open(&path).unwrap();
        // Write an actual old-format document, then load through the normal reader.
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute(
            "INSERT INTO document(id,json) VALUES(1,?1)",
            [json.to_string()],
        )
        .unwrap();
        drop(conn);
        let mut legacy = store.load().unwrap().unwrap();
        assert_eq!(legacy.category_balance("2026-03", &c), -3000);
        assert_eq!(legacy.available("2026-03").unwrap(), 100_000);
        legacy
            .set_overspending_rule("2026-02", &c, OverspendingRule::AvailableToBudget)
            .unwrap();
        store.save(&legacy).unwrap();
        let restored = store.load().unwrap().unwrap();
        assert_eq!(restored.category_balance("2026-02", &c), -3000);
        assert_eq!(restored.category_balance("2026-03", &c), 0);
        assert_eq!(restored.available("2026-03").unwrap(), 97_000);
        // Stored revisions retain their original rules (the same states used by Undo/Redo).
        let revision = store.version(store.versions().unwrap()[0].0).unwrap();
        assert_eq!(revision.category_balance("2026-03", &c), -3000);
        let backup = path.with_extension("json");
        export_json(&restored, &backup).unwrap();
        assert_eq!(
            import_json(&backup)
                .unwrap()
                .category_balance("2026-03", &c),
            0
        );
        std::fs::remove_file(backup).unwrap();
        let fresh = restored.fresh_start().unwrap();
        assert!(fresh.overspending_rules.is_empty());
        assert_eq!(
            fresh.default_overspending_rule,
            OverspendingRule::AvailableToBudget
        );
    }
    std::fs::remove_file(path).unwrap();
}
#[test]
fn invalid_rules_are_rejected_atomically_and_unused_group_deletion_cleans_rules() {
    let (mut b, _, c) = fixture();
    let before = serde_json::to_string(&b).unwrap();
    for (month, category) in [
        ("2026-13", c.as_str()),
        ("bad", c.as_str()),
        ("2026-01", "missing"),
    ] {
        assert!(
            b.set_overspending_rule(month, category, OverspendingRule::CategoryBalance)
                .is_err()
        );
        assert_eq!(serde_json::to_string(&b).unwrap(), before);
        let mut invalid = b.clone();
        invalid.overspending_rules.insert(
            format!("{month}/{category}"),
            OverspendingRule::CategoryBalance,
        );
        assert!(validate(&invalid).is_err());
    }
    b.overspending_rules
        .insert("bad".into(), OverspendingRule::CategoryBalance);
    assert!(validate(&b).is_err());
    b.overspending_rules.clear();
    b.set_overspending_rule("2026-01", &c, OverspendingRule::CategoryBalance)
        .unwrap();
    let group = b.categories[0].group.clone();
    b.delete_group(&group).unwrap();
    assert!(b.overspending_rules.is_empty());
    validate(&b).unwrap();
}

#[test]
fn negative_allocations_and_positive_balances_cross_the_year_boundary() {
    let (mut b, _, c) = fixture();
    b.allocate("2026-12", &c, 10_000).unwrap();
    assert_eq!(b.category_balance("2027-01", &c), 10_000);
    b.allocate("2027-01", &c, -13_000).unwrap();
    assert_eq!(b.category_balance("2027-01", &c), -3000);
    summary(&b, "2027-01", [90_000, 0, 0, -13_000, 103_000]);
    summary(&b, "2027-02", [103_000, -3000, 0, 0, 100_000]);
    assert_eq!(b.category_balance("2027-02", &c), 0);
    b.set_overspending_rule("2027-01", &c, OverspendingRule::CategoryBalance)
        .unwrap();
    assert_eq!(b.category_balance("2027-02", &c), -3000);
    summary(&b, "2027-02", [103_000, 0, 0, 0, 103_000]);
}
