use super::*;

#[test]
fn categorized_boundary_transfers_affect_only_the_budget_side_and_delete_atomically() {
    for value in [-100, 100] {
        let f = Fixture::new();
        boundary_transfer(&f, value, "food", Some("food"));
        let source = fs::read(&f.full).unwrap();
        let mut b = legacy_budget::preview(&f.root).unwrap().budget;
        assert_eq!(b.activity("2026-01", "food"), -1500 + value * 100);
        assert_eq!(b.balance("cash", None, false), 130000 + value * 100);
        assert_eq!(b.balance("asset", None, false), -value * 100);
        assert_eq!(b.available("2026-01").unwrap(), 90000);
        for accounts in [Selection::All, Selection::BudgetAccounts] {
            let filter = budgettinker::reports::Filter {
                from: "2026-01-01".into(),
                through: "2026-01-31".into(),
                accounts,
                ..Default::default()
            };
            let r = budgettinker::reports::calculate(&b, 3, &filter).unwrap();
            assert_eq!(r.months[0].expense, 22500 - value * 100);
            assert_eq!(r.months[0].income, 130000);
        }
        let mut store = Store::open(&f.root.parent().unwrap().join("boundary.sqlite3")).unwrap();
        store.save(&b).unwrap();
        assert_eq!(
            serde_json::to_value(store.load().unwrap().unwrap()).unwrap(),
            serde_json::to_value(&b).unwrap()
        );
        let ids = ["pay-out".into()].into_iter().collect();
        let before = serde_json::to_value(&b).unwrap();
        let invalid_target = budgettinker::register::Command::MoveAccount("card".into());
        assert!(!b.can_register_command(
            &["pay-in".into()].into_iter().collect(),
            invalid_target.clone()
        ));
        assert!(
            b.register_command(&["pay-in".into()].into_iter().collect(), invalid_target)
                .is_err()
        );
        assert_eq!(serde_json::to_value(&b).unwrap(), before);
        b.register_command(&ids, budgettinker::register::Command::Delete)
            .unwrap();
        assert!(
            !b.transactions
                .iter()
                .any(|t| matches!(t.id.as_str(), "pay-out" | "pay-in"))
        );
        assert_eq!(b.balance("asset", None, false), 0);
        store::validate(&b).unwrap();
        assert_eq!(fs::read(&f.full).unwrap(), source);
    }
}

#[test]
fn boundary_income_transfers_keep_current_next_month_and_negative_corrections() {
    for next in [false, true] {
        for value in [-100, 100] {
            let f = Fixture::new();
            boundary_transfer(
                &f,
                value,
                if next {
                    "Category/__DeferredIncome__"
                } else {
                    "Category/__ImmediateIncome__"
                },
                None,
            );
            let b = legacy_budget::preview(&f.root).unwrap().budget;
            assert_eq!(b.activity("2026-01", "food"), -1500);
            assert_eq!(
                b.deferred_income("2026-01"),
                30000 + if next { value * 100 } else { 0 }
            );
            assert_eq!(
                b.available("2026-01").unwrap(),
                90000 + if next { 0 } else { value * 100 }
            );
            assert_eq!(b.available("2026-02").unwrap(), 120000 + value * 100);
            let t = b.transactions.iter().find(|t| t.id == "pay-out").unwrap();
            assert_eq!(
                t.income_month.as_deref(),
                Some(if next { "2026-02" } else { "2026-01" })
            );
            assert!(b.transaction_category_name(t).starts_with("Income for "));
            let r = budgettinker::reports::calculate(
                &b,
                3,
                &budgettinker::reports::Filter {
                    from: "2026-01-01".into(),
                    through: "2026-01-31".into(),
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(r.months[0].income, 130000 + value * 100);
            assert_eq!(r.months[0].expense, 22500);
        }
    }
}

#[test]
fn categorized_internal_and_zero_transfer_plans_fail_before_writes() {
    for boundary in [false, true] {
        for zero in [false, true] {
            let f = Fixture::new();
            f.edit(|v| {
                v["scheduledTransactions"][1]["categoryId"] = json!("fees");
                if boundary {
                    v["scheduledTransactions"][1]["targetAccountId"] = json!("asset");
                    v["scheduledTransactions"][1]["payeeId"] = Value::Null;
                }
                if zero {
                    v["scheduledTransactions"][1]["amount"] = json!(0);
                }
            });
            let before = fs::read(&f.full).unwrap();
            assert_eq!(legacy_budget::preview(&f.root).is_ok(), boundary && !zero);
            assert_eq!(fs::read(&f.full).unwrap(), before);
        }
    }
}

#[test]
fn payee_only_transfer_targets_do_not_duplicate_schedules_and_pairs_are_reciprocal() {
    let f = Fixture::new();
    f.edit(|v| {
        let mut peer = v["scheduledTransactions"][1].clone();
        peer["entityId"] = json!("payment-peer");
        peer["accountId"] = json!("card");
        peer["payeeId"] = json!("to-cash");
        peer["amount"] = json!(50);
        peer["targetAccountId"] = Value::Null;
        v["scheduledTransactions"][1]["targetAccountId"] = Value::Null;
        v["scheduledTransactions"]
            .as_array_mut()
            .unwrap()
            .push(peer);
    });
    let p = legacy_budget::preview(&f.root).unwrap();
    assert_eq!(p.budget.scheduled.len(), 3);
    assert_eq!(
        p.budget
            .scheduled
            .iter()
            .filter(|s| s.transaction.transfer.is_some())
            .count(),
        2
    );
    let mut b = p.budget;
    b.enter_scheduled_now("payment").unwrap();
    assert_eq!(b.transactions.len(), 10);
    assert_eq!(b.balance("cash", None, false), 115_000);
    assert_eq!(b.balance("card", None, false), -7_500);
    f.edit(|v| v["transactions"][7]["transferTransactionId"] = Value::Null);
    assert!(legacy_budget::preview(&f.root).is_err());
    f.edit(|v| {
        v["transactions"][7]["transferTransactionId"] = json!("pay-out");
        v["transactions"][7]["targetAccountId"] = json!("asset");
    });
    assert!(
        legacy_budget::preview(&f.root)
            .err()
            .unwrap()
            .contains("conflicts")
    );
}
