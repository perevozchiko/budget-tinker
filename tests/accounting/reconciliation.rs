use super::*;

#[test]
fn reconcile_protects_locked_transactions_and_transfers() {
    let (mut b, eur, usd) = fixture();
    assert!(b.reconcile(&eur, 0).is_err());
    b.reconcile(&eur, 100_000).unwrap();
    let t = b.transactions[0].clone();
    assert!(b.delete_transaction(&t.id).is_err());
    assert!(b.put_transaction(t).is_err());
    b.transfer(&eur, &usd, 900, 1000, "2026-01-20", "").unwrap();
    let tid = b.transactions.last().unwrap().id.clone();
    b.transactions.last_mut().unwrap().cleared = true;
    b.reconcile(&usd, 101_000).unwrap();
    assert!(b.delete_transaction(&tid).is_err());
}
#[test]
fn dated_reconciliation_only_locks_cleared_transactions_through_statement_date() {
    let (mut b, eur, _) = fixture();
    let category = b.categories[0].id.clone();
    let mut later = expense(&eur, &category, -1000, "2026-02-10");
    later.cleared = true;
    let later_id = later.id.clone();
    b.put_transaction(later).unwrap();
    let pending = expense(&eur, &category, -500, "2026-01-20");
    let pending_id = pending.id.clone();
    b.put_transaction(pending).unwrap();
    b.reconcile_through(&eur, 100_000, "2026-01-31").unwrap();
    assert!(
        b.transactions
            .iter()
            .find(|t| t.account == eur && t.date == "2026-01-01")
            .unwrap()
            .reconciled
    );
    assert!(
        !b.transactions
            .iter()
            .find(|t| t.id == later_id)
            .unwrap()
            .reconciled
    );
    assert!(
        !b.transactions
            .iter()
            .find(|t| t.id == pending_id)
            .unwrap()
            .reconciled
    );
    b.delete_transaction(&later_id).unwrap();
    b.delete_transaction(&pending_id).unwrap();
    let before = serde_json::to_string(&b).unwrap();
    for (account, date, target) in [
        (&eur[..], "2026-01-31", 0),
        (&eur, "2026-02-30", 100_000),
        (&eur, "2099-01-01", 100_000),
        ("missing", "2026-01-31", 0),
    ] {
        assert!(b.reconcile_through(account, target, date).is_err());
        assert_eq!(serde_json::to_string(&b).unwrap(), before);
    }
}

#[test]
fn balance_adjustment_preserves_reconciled_history_and_uses_dated_working_balance() {
    let (mut b, eur, _) = fixture();
    let category = b.categories[0].id.clone();
    b.reconcile(&eur, 100_000).unwrap();
    let original = serde_json::to_string(&b.transactions).unwrap();
    b.put_transaction(expense(&eur, &category, -500, "2026-02-10"))
        .unwrap();
    let before = b.transactions.len();
    b.adjust_balance(&eur, 98_000, "2026-01-31").unwrap();
    assert_eq!(b.balance(&eur, Some("2026-01-31"), false), 98_000);
    assert_eq!(b.balance(&eur, None, false), 97_500);
    assert_eq!(
        serde_json::to_string(&b.transactions[..2]).unwrap(),
        original
    );
    let adjustment = b.transactions.last().unwrap();
    assert_eq!(adjustment.amount, -2000);
    assert_eq!(adjustment.base_amount, -2000);
    assert!(adjustment.cleared);
    assert!(!adjustment.reconciled);
    assert!(adjustment.category.is_none());
    b.adjust_balance(&eur, 98_000, "2026-01-31").unwrap();
    assert_eq!(b.transactions.len(), before + 1);
    let path = temp("sqlite3");
    let mut store = Store::open(&path).unwrap();
    store.save(&b).unwrap();
    let loaded = store.load().unwrap().unwrap();
    assert_eq!(loaded.balance(&eur, None, false), 97_500);
    validate(&loaded).unwrap();
    drop(store);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn foreign_balance_adjustment_requires_snapshot_and_failure_is_atomic() {
    let (mut b, _, usd) = fixture();
    let before = serde_json::to_string(&b).unwrap();
    for date in ["2026-03-01", "2026-02-30", "2099-01-01"] {
        assert!(b.adjust_balance(&usd, 98_000, date).is_err());
        assert_eq!(serde_json::to_string(&b).unwrap(), before);
    }
    assert!(b.adjust_balance("missing", 0, "2026-01-31").is_err());
    assert_eq!(serde_json::to_string(&b).unwrap(), before);
    b.adjust_balance(&usd, 98_000, "2026-02-10").unwrap();
    let adjustment = b.transactions.last().unwrap();
    assert_eq!(adjustment.amount, -2000);
    assert_eq!(adjustment.base_amount, -1900);
    assert!(adjustment.snapshot.is_some());
    b.accounts.iter_mut().find(|a| a.id == usd).unwrap().closed = true;
    let before = serde_json::to_string(&b).unwrap();
    assert!(b.adjust_balance(&usd, 98_000, "2026-02-10").is_err());
    assert_eq!(serde_json::to_string(&b).unwrap(), before);
}

#[test]
fn reconciliation_adjustment_uses_cleared_balance_and_preserves_pending_and_transfer_peer() {
    for target in [101_000, 98_000] {
        let (mut b, eur, usd) = fixture();
        let category = b.categories[0].id.clone();
        b.reconcile_through(&eur, 100_000, "2026-01-01").unwrap();
        let historical = b.transactions[0].clone();
        let pending = expense(&eur, &category, -700, "2026-01-15");
        let pending_id = pending.id.clone();
        b.put_transaction(pending).unwrap();
        let mut later = expense(&eur, &category, -300, "2026-02-01");
        later.cleared = true;
        let later_id = later.id.clone();
        b.put_transaction(later).unwrap();
        b.transfer(&eur, &usd, 1000, 1000, "2026-01-20", "Bank transfer")
            .unwrap();
        let leg = b
            .transactions
            .iter_mut()
            .find(|t| t.account == eur && t.transfer.is_some())
            .unwrap();
        leg.cleared = true;
        let peer = b
            .transactions
            .iter()
            .find(|t| t.account == usd && t.transfer.is_some())
            .unwrap()
            .clone();
        assert_eq!(
            b.reconciliation_difference(&eur, target, "2026-01-31")
                .unwrap(),
            target - 99_000
        );
        b.reconcile_with_adjustment(&eur, target, "2026-01-31")
            .unwrap();
        validate(&b).unwrap();
        assert_eq!(b.balance(&eur, Some("2026-01-31"), true), target);
        assert_eq!(b.balance(&eur, Some("2026-01-31"), false), target - 700);
        assert_eq!(b.balance(&eur, None, false), target - 1000);
        let adjustment = b.transactions.last().unwrap();
        assert_eq!(adjustment.amount, target - 99_000);
        assert_eq!(adjustment.base_amount, adjustment.amount);
        assert_eq!(adjustment.date, "2026-01-31");
        assert!(adjustment.reconciled && adjustment.cleared);
        for original in [&historical, &peer] {
            let after = b.transactions.iter().find(|t| t.id == original.id).unwrap();
            assert_eq!(
                serde_json::to_value(after).unwrap(),
                serde_json::to_value(original).unwrap()
            );
        }
        for tid in [&pending_id, &later_id] {
            assert!(
                !b.transactions
                    .iter()
                    .find(|t| &t.id == tid)
                    .unwrap()
                    .reconciled
            );
        }
        let count = b.transactions.len();
        b.reconcile_with_adjustment(&eur, target, "2026-01-31")
            .unwrap();
        assert_eq!(b.transactions.len(), count);
    }
}

#[test]
fn reconciliation_adjustment_failures_are_atomic_and_foreign_precision_is_exact() {
    let (mut b, eur, usd) = fixture();
    let before = serde_json::to_string(&b).unwrap();
    for (account, target, date) in [
        (&usd[..], 99_999, "2026-03-31"), // missing snapshot
        (&eur, i64::MAX, "2026-01-31"),
        (&eur, 1, "2026-02-30"),
        (&eur, 1, "2099-01-01"),
        ("missing", 1, "2026-01-31"),
    ] {
        assert!(b.reconcile_with_adjustment(account, target, date).is_err());
        assert_eq!(serde_json::to_string(&b).unwrap(), before);
    }
    b.accounts.iter_mut().find(|a| a.id == eur).unwrap().closed = true;
    let before = serde_json::to_string(&b).unwrap();
    assert!(
        b.reconcile_with_adjustment(&eur, 100_000, "2026-01-31")
            .is_err()
    );
    assert_eq!(serde_json::to_string(&b).unwrap(), before);
    b.reconcile_with_adjustment(&usd, 99_999, "2026-01-31")
        .unwrap();
    let adjustment = b.transactions.last().unwrap();
    assert_eq!(adjustment.amount, -1);
    assert_eq!(adjustment.base_amount, -1);
    assert!(adjustment.snapshot.is_some());
    validate(&b).unwrap();

    for (currency, target) in [("JPY", -123), ("KWD", -1234)] {
        let mut b = Budget::new(currency);
        let a = b
            .add_account("Bank", currency, true, 0, "2026-01-01")
            .unwrap();
        b.reconcile_with_adjustment(&a, target, "2026-01-31")
            .unwrap();
        assert_eq!(b.transactions[0].amount, target);
        assert_eq!(b.balance(&a, None, true), target);
        validate(&b).unwrap();
    }
}
