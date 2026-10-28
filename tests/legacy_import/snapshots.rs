use super::*;

#[test]
fn complete_snapshot_mapping_preserves_money_timing_debt_and_sqlite() {
    let f = Fixture::new();
    let before = fs::read(&f.full).unwrap();
    let p = legacy_budget::preview(&f.root).unwrap();
    let b = &p.budget;
    assert_eq!(b.name, "Synthetic");
    assert_eq!(b.accounts.len(), 3);
    assert!(b.account("asset").unwrap().closed);
    assert!(!b.account("asset").unwrap().on_budget);
    assert_eq!(b.balance("cash", None, false), 120_000);
    assert_eq!(b.balance("card", None, false), -12_500);
    assert_eq!(
        b.category_balance("2026-02", concat!("Category/PreY", "NABDebt/card")),
        -15_000
    );
    assert_eq!(b.category_balance("2026-01", "food"), 2_500);
    assert_eq!(b.available("2026-01").unwrap(), 90_000);
    assert_eq!(b.available("2026-02").unwrap(), 120_000);
    assert_eq!(b.notes["account:cash"], "Account note");
    assert_eq!(b.notes["group:Living"], "Group note");
    assert_eq!(b.notes["food"], "Food note");
    assert_eq!(b.notes["transaction:split:split:0"], "Vegetables");
    let c = b.categories.iter().find(|c| c.id == "old").unwrap();
    assert_eq!(c.group, "Living");
    assert_eq!(c.name, "Old Hobby");
    assert!(c.hidden);
    assert!(!b.payee_rules["Shop"].listed);
    assert_eq!(b.payee_rules["Shop"].category.as_deref(), Some("food"));
    assert!(b.transactions[0].reconciled);
    assert!(b.register_meta["interest"].pending);
    assert_eq!(
        b.transactions
            .iter()
            .find(|t| t.id == "split")
            .unwrap()
            .flag,
        Some(TransactionFlag::Green)
    );
    assert_eq!(b.scheduled.len(), 3);
    assert!(b.scheduled.iter().all(|s| s.frequency() == Repeat::Monthly));
    let payment = b.scheduled.iter().find(|s| s.id == "payment").unwrap();
    let peer = b
        .scheduled
        .iter()
        .find(|s| s.id.starts_with("legacy-counterpart/"))
        .unwrap();
    assert_eq!(payment.transaction.transfer, peer.transaction.transfer);
    assert_eq!(payment.anchor_day, Some(28));
    let mut posted = b.clone();
    posted.enter_scheduled_now("payment").unwrap();
    assert_eq!(posted.balance("cash", None, false), 115_000);
    assert_eq!(posted.balance("card", None, false), -7_500);
    assert_eq!(
        posted.category_balance("2026-02", concat!("Category/PreY", "NABDebt/card")),
        -15_000
    );
    store::validate(&posted).unwrap();
    p.verify_source(&f.root).unwrap();
    assert_eq!(fs::read(&f.full).unwrap(), before);
    let mut store = Store::open(&f.root.parent().unwrap().join("target.sqlite3")).unwrap();
    store.save(b).unwrap();
    assert_eq!(
        serde_json::to_value(store.load().unwrap().unwrap()).unwrap(),
        serde_json::to_value(b).unwrap()
    );
    let json_path = f.root.parent().unwrap().join("backup.json");
    store::export_json(b, &json_path).unwrap();
    store::import_json(&json_path).unwrap();
}

#[test]
fn package_checks_versions_conflicts_journals_and_preview_changes() {
    let f = Fixture::new();
    let p = legacy_budget::preview(&f.root).unwrap();
    let diff = f.full.parent().unwrap().join("A-20_A-21.ydiff");
    fs::write(&diff, r#"{"endVersion":"A-21"}"#).unwrap();
    assert!(p.verify_source(&f.root).is_err());
    assert!(
        legacy_budget::preview(&f.root)
            .err()
            .unwrap()
            .contains("all known changes")
    );
    f.edit(|v| v["fileMetaData"]["currentKnowledge"] = json!("A-21"));
    let p = legacy_budget::preview(&f.root).unwrap();
    p.verify_source(&f.root).unwrap();
    fs::create_dir(f.root.join("data/devices")).unwrap();
    fs::write(
        f.root.join("data/devices/B.ydevice"),
        r#"{"knowledge":"A-21,B-1"}"#,
    )
    .unwrap();
    assert!(legacy_budget::preview(&f.root).is_err());
    f.edit(|v| v["fileMetaData"]["currentKnowledge"] = json!("A-21,B-1"));
    legacy_budget::preview(&f.root).unwrap();
    fs::create_dir(f.root.join("data/other")).unwrap();
    fs::copy(&f.full, f.root.join("data/other/Budget.yfull")).unwrap();
    legacy_budget::preview(&f.root).unwrap();
    let other = f.root.join("data/other/Budget.yfull");
    let mut v: Value = serde_json::from_slice(&fs::read(&other).unwrap()).unwrap();
    v["transactions"][0]["memo"] = json!("Conflicting edit");
    fs::write(other, serde_json::to_vec(&v).unwrap()).unwrap();
    assert!(
        legacy_budget::preview(&f.root)
            .err()
            .unwrap()
            .contains("Conflicting")
    );
}

#[test]
fn precision_is_exact_and_unsupported_data_fails_before_writes() {
    for (currency, raw, expected) in [
        ("USD", "1234.56", 123456),
        ("JPY", "1234.0", 1234),
        ("KWD", "1234.567", 1234567),
        ("USD", "1.23e2", 12300),
    ] {
        let f = Fixture::new();
        f.edit(|v| {
            v["budgetMetaData"]["currencyISOSymbol"] = json!(currency);
            v["transactions"][0]["amount"] = serde_json::from_str(raw).unwrap();
        });
        assert_eq!(
            legacy_budget::preview(&f.root).unwrap().budget.transactions[0].amount,
            expected
        );
    }
    let changes: Vec<(&str, Value)> = vec![
        ("amount", json!(1.001)),
        ("amount", serde_json::from_str("1e2147483647").unwrap()),
        ("categoryId", json!("missing")),
        ("accountId", json!("missing")),
        ("cleared", json!("invalid")),
        ("date", json!("bad")),
        ("matchedTransactions", json!([{}])),
    ];
    for (key, value) in changes {
        let f = Fixture::new();
        f.edit(|v| v["transactions"][0][key] = value);
        let original = fs::read(&f.full).unwrap();
        assert!(legacy_budget::preview(&f.root).is_err(), "{key}");
        assert_eq!(fs::read(&f.full).unwrap(), original);
    }
    for mutate in 0..8 {
        let f = Fixture::new();
        f.edit(|v| match mutate {
            0 => v["transactions"][3]["subTransactions"][0]["amount"] = json!(25),
            1 => v["transactions"][7]["amount"] = json!(99),
            2 => v["transactions"][7]["transferTransactionId"] = json!("wrong"),
            3 => v["transactions"][7]["categoryId"] = json!("food"),
            4 => v["scheduledTransactions"][0]["frequency"] = json!("unsupported"),
            5 => v["fileMetaData"]["budgetDataVersion"] = json!("5"),
            6 => {
                let row = v["accounts"][0].clone();
                v["accounts"].as_array_mut().unwrap().push(row);
            }
            _ => {
                v["monthlyBudgets"][0]["monthlySubCategoryBudgets"][0]["overspendingHandling"] =
                    json!("unsupported")
            }
        });
        assert!(legacy_budget::preview(&f.root).is_err(), "case {mutate}");
    }
}
