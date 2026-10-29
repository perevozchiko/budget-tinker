use budgettinker::{
    model::*,
    register::{Command, Repeat},
    store::*,
};
use std::collections::HashSet;

fn fixture() -> (Budget, String) {
    let mut b = Budget::new("EUR");
    let a = b
        .add_account("Synthetic Cash", "EUR", true, 0, "2025-12-01")
        .unwrap();
    (b, a)
}
fn income(account: &str, date: &str, assigned: Option<&str>, amount: i64) -> Transaction {
    Transaction {
        operation_fx: None,
        check_number: None,
        flag: None,
        id: id(),
        account: account.into(),
        date: date.into(),
        payee: "Salary".into(),
        memo: String::new(),
        amount,
        base_amount: 0,
        snapshot: None,
        category: None,
        income_month: assigned.map(Into::into),
        cleared: false,
        reconciled: false,
        transfer: None,
        splits: vec![],
    }
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
fn next_month_changes_budget_timing_but_not_cash_capital_or_reports() {
    let (mut b, a) = fixture();
    b.put_transaction(income(&a, "2025-12-20", Some("2026-01"), 100_000))
        .unwrap();
    assert_eq!(b.balance(&a, Some("2025-12-31"), false), 100_000);
    assert_eq!(b.net_worth("2025-12", true).unwrap(), 100_000);
    summary(&b, "2025-12", [0, 0, 0, 0, 0]);
    summary(&b, "2026-01", [0, 0, 100_000, 0, 100_000]);
    summary(&b, "2026-02", [100_000, 0, 0, 0, 100_000]);
    let report = budgettinker::reports::calculate(
        &b,
        3,
        &budgettinker::reports::Filter {
            from: "2025-12-01".into(),
            through: "2025-12-31".into(),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(report.months[0].income, 100_000);
}
#[test]
fn current_and_legacy_income_refunds_and_overspending_stay_consistent() {
    let (mut b, a) = fixture();
    b.put_transaction(income(&a, "2025-12-01", None, 20_000))
        .unwrap();
    b.put_transaction(income(&a, "2025-12-20", Some("2025-12"), 30_000))
        .unwrap();
    b.put_transaction(income(&a, "2025-12-21", Some("2026-01"), 100_000))
        .unwrap();
    b.put_transaction(income(&a, "2025-12-22", Some("2026-01"), -10_000))
        .unwrap();
    let c = b.categories[0].id.clone();
    b.allocate("2025-12", &c, 10_000).unwrap();
    let mut expense = income(&a, "2025-12-23", None, -15_000);
    expense.category = Some(c.clone());
    b.put_transaction(expense).unwrap();
    summary(&b, "2025-12", [0, 0, 50_000, 10_000, 40_000]);
    summary(&b, "2026-01", [40_000, -5000, 90_000, 0, 125_000]);
    assert_eq!(b.category_balance("2026-01", &c), 0);
}
#[test]
fn off_budget_income_and_transfers_do_not_create_deferred_money() {
    let (mut b, a) = fixture();
    let off = b
        .add_account("Outside", "EUR", false, 0, "2025-12-01")
        .unwrap();
    b.put_transaction(income(&off, "2025-12-20", Some("2026-01"), 100_000))
        .unwrap();
    summary(&b, "2025-12", [0, 0, 0, 0, 0]);
    summary(&b, "2026-01", [0, 0, 0, 0, 0]);
    b.transfer(&off, &a, 10_000, 10_000, "2025-12-21", "transfer")
        .unwrap();
    summary(&b, "2025-12", [0, 0, 10_000, 0, 10_000]);
}
#[test]
fn foreign_income_holds_frozen_base_value_and_keeps_revaluation_timing() {
    let (mut b, _) = fixture();
    b.add_snapshot("2025-12", "USD", 90_000_000, "2025-12-01", "synthetic")
        .unwrap();
    b.add_snapshot("2026-01", "USD", 95_000_000, "2026-01-01", "synthetic")
        .unwrap();
    let a = b
        .add_account("Dollar", "USD", true, 0, "2025-12-01")
        .unwrap();
    b.put_transaction(income(&a, "2025-12-20", Some("2026-01"), 10_000))
        .unwrap();
    assert_eq!(b.deferred_income("2025-12"), 9000);
    summary(&b, "2025-12", [0, 0, 0, 0, 0]);
    summary(&b, "2026-01", [0, 0, 9500, 0, 9500]);
    assert_eq!(b.transactions[0].base_amount, 9000);
}
#[test]
fn income_commands_are_atomic_and_clear_old_category_or_income_assignment() {
    let (mut b, a) = fixture();
    let mut t = income(&a, "2025-12-20", None, 1000);
    let c = b.categories[0].id.clone();
    t.category = Some(c.clone());
    let ids = HashSet::from([t.id.clone()]);
    b.put_transaction(t).unwrap();
    b.register_command(
        &ids,
        Command::IncomeMonth {
            next_month: true,
            replace_splits: false,
        },
    )
    .unwrap();
    assert_eq!(b.transactions[0].income_month.as_deref(), Some("2026-01"));
    assert!(b.transactions[0].category.is_none());
    b.register_command(
        &ids,
        Command::Categorize {
            category: Some(c),
            replace_splits: false,
        },
    )
    .unwrap();
    assert!(b.transactions[0].income_month.is_none());
    b.transactions[0].cleared = true;
    b.transactions[0].reconciled = true;
    let before = serde_json::to_string(&b).unwrap();
    assert!(!b.can_register_command(
        &ids,
        Command::IncomeMonth {
            next_month: false,
            replace_splits: true
        }
    ));
    assert!(
        b.register_command(
            &ids,
            Command::IncomeMonth {
                next_month: false,
                replace_splits: true
            }
        )
        .is_err()
    );
    assert_eq!(serde_json::to_string(&b).unwrap(), before);
}
#[test]
fn invalid_income_assignments_never_reach_persistence() {
    let (b, a) = fixture();
    for month in ["2025-11", "2026-02", "2026-13", "bad"] {
        let mut invalid = b.clone();
        let t = income(&a, "2025-12-20", Some(month), 1000);
        assert!(invalid.put_transaction(t.clone()).is_err());
        invalid.transactions.push(t);
        assert!(validate(&invalid).is_err());
    }
    let mut t = income(&a, "2025-12-20", Some("2026-01"), 1000);
    t.category = Some(b.categories[0].id.clone());
    assert!(b.prepare_transaction(t.clone()).is_err());
    t.category = None;
    t.transfer = Some(id());
    // Income timing is valid on a budget/off-budget transfer. Pair validation
    // still rejects an orphan; the ordinary editor cannot create a transfer.
    t.validate_income_month().unwrap();
    assert!(b.prepare_transaction(t.clone()).is_err());
    let mut orphan = b.clone();
    orphan.transactions.push(t.clone());
    assert!(validate(&orphan).is_err());
    t.transfer = None;
    t.splits = vec![Split {
        category: Some(b.categories[0].id.clone()),
        amount: 1000,
        ..Default::default()
    }];
    assert!(b.prepare_transaction(t).is_err());
}
#[test]
fn old_sqlite_json_revisions_and_export_round_trip_income_month() {
    let (mut b, a) = fixture();
    b.put_transaction(income(&a, "2025-12-20", None, 1000))
        .unwrap();
    let path = std::env::temp_dir().join(format!("budgettinker-income-{}.sqlite3", id()));
    let mut old = serde_json::to_value(&b).unwrap();
    for row in old["transactions"].as_array_mut().unwrap() {
        row.as_object_mut().unwrap().remove("income_month");
    }
    {
        let mut store = Store::open(&path).unwrap();
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute(
            "INSERT INTO document(id,json) VALUES(1,?1)",
            [old.to_string()],
        )
        .unwrap();
        drop(conn);
        let mut loaded = store.load().unwrap().unwrap();
        summary(&loaded, "2025-12", [0, 0, 1000, 0, 1000]);
        loaded
            .register_command(
                &HashSet::from([loaded.transactions[0].id.clone()]),
                Command::IncomeMonth {
                    next_month: true,
                    replace_splits: false,
                },
            )
            .unwrap();
        store.save(&loaded).unwrap();
        let restored = store.load().unwrap().unwrap();
        summary(&restored, "2025-12", [0, 0, 0, 0, 0]);
        let revision = store.version(store.versions().unwrap()[0].0).unwrap();
        summary(&revision, "2025-12", [0, 0, 1000, 0, 1000]);
        let json = path.with_extension("json");
        export_json(&restored, &json).unwrap();
        assert_eq!(
            import_json(&json).unwrap().transactions[0]
                .income_month
                .as_deref(),
            Some("2026-01")
        );
        std::fs::remove_file(json).unwrap();
        let csv = path.with_extension("csv");
        export_csv(&restored, &csv).unwrap();
        let (empty, account) = fixture();
        let (imported, count) = import_csv(&empty, &csv, &account).unwrap();
        assert_eq!(count, 1);
        summary(&imported, "2026-01", [0, 0, 1000, 0, 1000]);
        std::fs::remove_file(csv).unwrap();
    }
    std::fs::remove_file(path).unwrap();
}
#[test]
fn schedule_and_match_dates_preserve_next_month_intent() {
    let (mut b, a) = fixture();
    let t = income(&a, "2025-12-20", Some("2026-01"), 1000);
    let ids = HashSet::from([t.id.clone()]);
    b.put_transaction(t).unwrap();
    b.register_command(
        &ids,
        Command::Schedule {
            date: "2026-01-20".into(),
            repeat: Repeat::Monthly,
            move_original: false,
        },
    )
    .unwrap();
    let sid = b.scheduled[0].id.clone();
    assert_eq!(
        b.scheduled[0].transaction.income_month.as_deref(),
        Some("2026-02")
    );
    b.post_scheduled(&sid).unwrap();
    b.post_scheduled(&sid).unwrap();
    assert_eq!(b.transactions[1].income_month.as_deref(), Some("2026-02"));
    assert_eq!(b.transactions[2].income_month.as_deref(), Some("2026-03"));
    validate(&b).unwrap();
    let mut moved = b.transactions[0].clone();
    moved.set_date("2026-03-01".into());
    assert_eq!(moved.income_month.as_deref(), Some("2026-04"));
}

#[test]
fn matching_import_across_month_boundary_keeps_manual_income_choice_and_unmatches() {
    let (mut b, a) = fixture();
    let manual = income(&a, "2025-12-31", Some("2026-01"), 1000);
    let imported = income(&a, "2026-01-01", None, 1000);
    let ids = HashSet::from([manual.id.clone(), imported.id.clone()]);
    let manual_id = manual.id.clone();
    let imported_id = imported.id.clone();
    b.put_transaction(manual).unwrap();
    b.put_transaction(imported).unwrap();
    b.register_meta.insert(imported_id, Default::default());
    b.register_command(&ids, Command::Match).unwrap();
    assert_eq!(b.transactions[0].date, "2025-12-31");
    assert_eq!(b.transactions[0].income_month.as_deref(), Some("2026-01"));
    b.register_command(&HashSet::from([manual_id]), Command::Unmatch)
        .unwrap();
    assert_eq!(b.transactions[0].date, "2025-12-31");
    assert_eq!(b.transactions[0].income_month.as_deref(), Some("2026-01"));
}
#[test]
fn split_replacement_requires_confirmation_and_transfers_cannot_be_income() {
    let (mut b, a) = fixture();
    let mut t = income(&a, "2025-12-20", None, 1000);
    t.splits = vec![Split {
        category: Some(b.categories[0].id.clone()),
        amount: 1000,
        ..Default::default()
    }];
    let ids = HashSet::from([t.id.clone()]);
    b.put_transaction(t).unwrap();
    let before = serde_json::to_string(&b).unwrap();
    assert!(
        b.register_command(
            &ids,
            Command::IncomeMonth {
                next_month: true,
                replace_splits: false
            }
        )
        .is_err()
    );
    assert_eq!(serde_json::to_string(&b).unwrap(), before);
    b.register_command(
        &ids,
        Command::IncomeMonth {
            next_month: true,
            replace_splits: true,
        },
    )
    .unwrap();
    assert!(b.transactions[0].splits.is_empty());
    let second = b
        .add_account("Second", "EUR", true, 0, "2025-12-01")
        .unwrap();
    b.transfer(&a, &second, 100, 100, "2025-12-21", "test")
        .unwrap();
    let ids = HashSet::from([b.transactions[1].id.clone()]);
    assert!(!b.can_register_command(
        &ids,
        Command::IncomeMonth {
            next_month: true,
            replace_splits: true
        }
    ));
}

#[test]
fn csv_and_qif_income_labels_import_without_losing_assignment_or_accepting_invalid_months() {
    let (b, a) = fixture();
    let path = std::env::temp_dir().join(format!("budgettinker-income-{}.csv", id()));
    std::fs::write(
        &path,
        "Date,Payee,Category,Amount\n12/20/25,Salary,Income for January,10\n",
    )
    .unwrap();
    let (imported, _) = import_csv(&b, &path, &a).unwrap();
    assert_eq!(
        imported.transactions[0].income_month.as_deref(),
        Some("2026-01")
    );
    std::fs::write(
        &path,
        "Date,Payee,Category,Amount,Income Month\n12/20/25,Salary,Income for January,10,2026-02\n",
    )
    .unwrap();
    assert!(import_csv(&b, &path, &a).is_err());
    assert!(b.transactions.is_empty());
    std::fs::remove_file(&path).unwrap();
    let path = path.with_extension("qif");
    std::fs::write(
        &path,
        "!Type:Bank\nD12/20/25\nT10\nPSalary\nLIncome for January\n^\n",
    )
    .unwrap();
    let (imported, _) = budgettinker::bank::import(&b, &path, &a).unwrap();
    assert_eq!(
        imported.transactions[0].income_month.as_deref(),
        Some("2026-01")
    );
    export_qif(&imported, &path).unwrap();
    assert!(
        std::fs::read_to_string(&path)
            .unwrap()
            .contains("LIncome for January\n")
    );
    std::fs::remove_file(path).unwrap();
}
