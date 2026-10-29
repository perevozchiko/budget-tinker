use budgettinker::{
    bank,
    model::*,
    register::{Command, ImportState, Repeat},
    store::{self, Store},
};
use std::{collections::HashSet, fs};

fn ids(values: &[&str]) -> HashSet<String> {
    values.iter().map(|s| s.to_string()).collect()
}
fn fixture() -> (Budget, String, String, String) {
    let mut b = Budget::new("EUR");
    let a = b
        .add_account("Checking", "EUR", true, 0, "2026-01-01")
        .unwrap();
    let other = b
        .add_account("Savings", "EUR", true, 0, "2026-01-01")
        .unwrap();
    let category = b.categories[0].id.clone();
    b.put_transaction(Transaction {
        operation_fx: None,
        check_number: None,
        flag: None,
        id: "manual".into(),
        account: a.clone(),
        date: "2026-01-30".into(),
        payee: "Shop".into(),
        memo: "Keep this note".into(),
        amount: -1000,
        base_amount: 0,
        snapshot: None,
        income_month: None,
        category: Some(category.clone()),
        cleared: false,
        reconciled: false,
        transfer: None,
        splits: vec![],
    })
    .unwrap();
    (b, a, other, category)
}
fn with_import() -> (Budget, String, String, String) {
    let (mut b, a, other, c) = fixture();
    let mut t = b.transactions[0].clone();
    t.id = "imported".into();
    t.date = "2026-01-31".into();
    t.payee = "BANK SHOP".into();
    t.memo = "Bank note".into();
    t.category = None;
    b.put_transaction(t).unwrap();
    b.register_meta.insert(
        "imported".into(),
        ImportState {
            source_id: None,
            pending: true,
            matched: None,
        },
    );
    (b, a, other, c)
}
fn json(b: &Budget) -> String {
    serde_json::to_string(b).unwrap()
}
#[test]
fn clear_and_unclear_leave_amounts_and_other_rows_untouched() {
    let (mut b, a, _, _) = with_import();
    let balance = b.balance(&a, None, false);
    b.register_command(&ids(&["manual"]), Command::Cleared(true))
        .unwrap();
    assert_eq!(b.balance(&a, None, true), -1000);
    assert!(!b.transactions[1].cleared);
    b.register_command(&ids(&["manual"]), Command::Cleared(false))
        .unwrap();
    assert_eq!(b.balance(&a, None, false), balance);
    assert!(!b.can_register_command(&ids(&["manual"]), Command::Cleared(false)));
}
#[test]
fn approve_does_not_clear_and_reject_only_deletes_pending_imports() {
    let (mut b, _, _, _) = with_import();
    let before = json(&b);
    assert!(
        b.register_command(&ids(&["manual", "imported"]), Command::Reject)
            .is_err()
    );
    assert_eq!(json(&b), before);
    b.register_command(&ids(&["manual", "imported"]), Command::Approve)
        .unwrap();
    assert!(!b.pending_import("imported"));
    assert!(!b.transactions[1].cleared);
    assert!(!b.can_register_command(&ids(&["imported"]), Command::Approve));
    b.register_meta.get_mut("imported").unwrap().pending = true;
    b.register_command(&ids(&["imported"]), Command::Reject)
        .unwrap();
    assert_eq!(b.transactions.len(), 1);
    assert!(b.register_meta.is_empty());
}
#[test]
fn match_and_unmatch_restore_both_originals_after_save_reload() {
    let (mut b, a, _, c) = with_import();
    let originals = json(&b);
    b.register_command(&ids(&["manual", "imported"]), Command::Match)
        .unwrap();
    assert_eq!(b.transactions.len(), 1);
    let t = &b.transactions[0];
    assert_eq!(t.id, "manual");
    assert_eq!(t.payee, "Shop");
    assert_eq!(t.memo, "Keep this note");
    assert_eq!(t.category, Some(c));
    assert_eq!(t.date, "2026-01-30");
    assert!(t.cleared);
    assert_eq!(b.balance(&a, None, false), -1000);
    assert!(b.imported_id_exists("imported"));
    let path = std::env::temp_dir().join(format!("budgettinker-match-{}.sqlite3", id()));
    {
        let mut store = Store::open(&path).unwrap();
        store.save(&b).unwrap();
        b = store.load().unwrap().unwrap();
    }
    b.register_command(&ids(&["manual"]), Command::Unmatch)
        .unwrap();
    assert_eq!(json(&b), originals);
    fs::remove_file(path).unwrap();
}
#[test]
fn matched_bank_import_does_not_reappear_when_statement_is_reimported() {
    let (b, a, _, _) = fixture();
    let path = std::env::temp_dir().join(format!("budgettinker-import-{}.ofx", id()));
    fs::write(&path, "<OFX><CURDEF>EUR<BANKTRANLIST><STMTTRN><DTPOSTED>20260131<TRNAMT>-10.00<FITID>test-match<NAME>BANK SHOP</STMTTRN></BANKTRANLIST></OFX>").unwrap();
    let (mut b, n) = bank::import(&b, &path, &a).unwrap();
    assert_eq!(n, 1);
    assert!(b.register_meta["manual"].matched.is_some());
    b.register_command(&ids(&["manual"]), Command::Approve)
        .unwrap();
    assert_eq!(bank::import(&b, &path, &a).unwrap().1, 0);
    b.register_command(&ids(&["manual"]), Command::Unmatch)
        .unwrap();
    assert_eq!(bank::import(&b, &path, &a).unwrap().1, 0);
    fs::remove_file(path).unwrap();
}
#[test]
fn rejecting_a_match_preserves_the_original_manual_transaction() {
    let (mut b, _, _, _) = with_import();
    let manual = serde_json::to_string(&b.transactions[0]).unwrap();
    b.register_command(&ids(&["manual", "imported"]), Command::Match)
        .unwrap();
    b.register_command(&ids(&["manual"]), Command::Reject)
        .unwrap();
    assert_eq!(b.transactions.len(), 1);
    assert_eq!(serde_json::to_string(&b.transactions[0]).unwrap(), manual);
    assert!(b.register_meta.is_empty());
}
#[test]
fn match_rejects_unequal_amounts_other_accounts_and_two_manual_entries_atomically() {
    let (mut b, _, other, _) = with_import();
    for bad in [0, 1, 2] {
        let mut next = b.clone();
        if bad == 0 {
            next.transactions[1].amount -= 1;
        }
        if bad == 1 {
            next.transactions[1].account = other.clone();
        }
        if bad == 2 {
            next.register_meta.clear();
        }
        let before = json(&next);
        assert!(
            next.register_command(&ids(&["manual", "imported"]), Command::Match)
                .is_err()
        );
        assert_eq!(json(&next), before);
    }
    b.transactions[0].reconciled = true;
    b.transactions[0].cleared = true;
    assert!(b.can_register_command(&ids(&["manual", "imported"]), Command::Match));
    b.register_command(&ids(&["manual", "imported"]), Command::Match)
        .unwrap();
    assert!(b.transactions[0].reconciled);
    assert!(!b.pending_import("manual"));
}
#[test]
fn categorizing_splits_requires_confirmation_and_preserves_total() {
    let (mut b, a, _, c) = fixture();
    b.transactions[0].category = None;
    b.transactions[0].splits = vec![
        Split {
            category: Some(c.clone()),
            amount: -600,
            ..Default::default()
        },
        Split {
            category: Some(c.clone()),
            amount: -400,
            ..Default::default()
        },
    ];
    let before = json(&b);
    assert!(
        b.register_command(
            &ids(&["manual"]),
            Command::Categorize {
                category: Some(c.clone()),
                replace_splits: false
            }
        )
        .is_err()
    );
    assert_eq!(json(&b), before);
    b.register_command(
        &ids(&["manual"]),
        Command::Categorize {
            category: Some(c),
            replace_splits: true,
        },
    )
    .unwrap();
    assert!(b.transactions[0].splits.is_empty());
    assert_eq!(b.balance(&a, None, false), -1000);
    b.register_command(
        &ids(&["manual"]),
        Command::Categorize {
            category: None,
            replace_splits: true,
        },
    )
    .unwrap();
    assert!(b.transactions[0].category.is_none());
}
#[test]
fn move_to_account_updates_balances_and_transfer_peer() {
    let (mut b, a, other, _) = fixture();
    b.register_command(&ids(&["manual"]), Command::MoveAccount(other.clone()))
        .unwrap();
    assert_eq!(b.balance(&a, None, false), 0);
    assert_eq!(b.balance(&other, None, false), -1000);
    let third = b
        .add_account("Third", "EUR", true, 0, "2026-01-01")
        .unwrap();
    b.transfer(&a, &other, 500, 500, "2026-01-31", "Transfer memo")
        .unwrap();
    let tid = b.transactions[1].id.clone();
    b.register_command(&ids(&[&tid]), Command::MoveAccount(third.clone()))
        .unwrap();
    assert_eq!(b.balance(&a, None, false), 0);
    assert_eq!(b.balance(&third, None, false), -500);
    assert!(b.transactions[2].payee.contains("Third"));
    store::validate(&b).unwrap();
    let before = json(&b);
    assert!(
        b.register_command(&ids(&[&tid]), Command::MoveAccount(other))
            .is_err()
    );
    assert_eq!(json(&b), before);
}
#[test]
fn batch_delete_with_locked_transfer_peer_is_atomic() {
    let (mut b, a, other, _) = fixture();
    b.transfer(&a, &other, 500, 500, "2026-01-31", "").unwrap();
    let tid = b.transactions[1].id.clone();
    b.transactions[2].cleared = true;
    b.transactions[2].reconciled = true;
    let before = json(&b);
    assert!(!b.can_register_command(&ids(&["manual", &tid]), Command::Delete));
    assert!(
        b.register_command(&ids(&["manual", &tid]), Command::Delete)
            .is_err()
    );
    assert_eq!(json(&b), before);
}
#[test]
fn rename_like_payees_updates_all_history_and_schedules() {
    let (mut b, _, _, _) = with_import();
    b.register_command(
        &ids(&["manual"]),
        Command::Schedule {
            date: "2026-01-31".into(),
            repeat: Repeat::Monthly,
            move_original: false,
        },
    )
    .unwrap();
    b.payee_rules.insert("Shop".into(), PayeeRule::default());
    b.register_command(
        &ids(&["manual", "imported"]),
        Command::RenamePayees("Combined".into()),
    )
    .unwrap();
    assert!(b.transactions.iter().all(|t| t.payee == "Combined"));
    assert_eq!(b.scheduled[0].transaction.payee, "Combined");
    assert!(b.payee_rules.contains_key("Combined"));
}
#[test]
fn recurring_copy_preserves_original_and_month_end_anchor() {
    let (mut b, a, _, _) = fixture();
    let original = json(&b);
    b.register_command(
        &ids(&["manual"]),
        Command::Schedule {
            date: "2026-01-31".into(),
            repeat: Repeat::Monthly,
            move_original: false,
        },
    )
    .unwrap();
    assert_eq!(b.balance(&a, None, false), -1000);
    let sid = b.scheduled[0].id.clone();
    let mut without_schedule = b.clone();
    without_schedule.scheduled.clear();
    assert_eq!(json(&without_schedule), original);
    b.post_scheduled(&sid).unwrap();
    assert_eq!(b.scheduled[0].next_date, "2026-02-28");
    b.post_scheduled(&sid).unwrap();
    assert_eq!(b.scheduled[0].next_date, "2026-03-31");
    assert_eq!(b.balance(&a, None, false), -3000);
}
#[test]
fn recurrence_intervals_cover_days_weeks_months_and_leap_years() {
    for (repeat, date, day, expected) in [
        (Repeat::Daily, "2026-01-31", 31, "2026-02-01"),
        (Repeat::Weekly, "2026-01-31", 31, "2026-02-07"),
        (Repeat::EveryOtherWeek, "2026-01-31", 31, "2026-02-14"),
        (Repeat::Every4Weeks, "2026-01-31", 31, "2026-02-28"),
        (Repeat::EveryOtherMonth, "2026-01-31", 31, "2026-03-31"),
        (Repeat::Every3Months, "2026-01-31", 31, "2026-04-30"),
        (Repeat::Every4Months, "2026-01-31", 31, "2026-05-31"),
        (Repeat::TwiceAYear, "2026-01-31", 31, "2026-07-31"),
        (Repeat::Yearly, "2024-02-29", 29, "2025-02-28"),
        (Repeat::EveryOtherYear, "2024-02-29", 29, "2026-02-28"),
        (Repeat::TwiceAMonth, "2026-01-01", 1, "2026-01-16"),
        (Repeat::TwiceAMonth, "2026-01-16", 1, "2026-02-01"),
    ] {
        assert_eq!(repeat.next(date, day).unwrap().as_deref(), Some(expected));
    }
    assert!(Repeat::Never.next("2026-01-01", 1).unwrap().is_none());
}
#[test]
fn moving_a_transfer_to_scheduler_keeps_both_legs_together() {
    let (mut b, a, other, _) = fixture();
    b.transfer(&a, &other, 500, 500, "2026-01-31", "memo")
        .unwrap();
    let tid = b.transactions[1].id.clone();
    let tomorrow = (chrono::Local::now().date_naive() + chrono::Duration::days(1)).to_string();
    b.register_command(
        &ids(&[&tid]),
        Command::Schedule {
            date: tomorrow,
            repeat: Repeat::Weekly,
            move_original: true,
        },
    )
    .unwrap();
    assert_eq!(b.transactions.len(), 1);
    assert_eq!(b.scheduled.len(), 2);
    assert_eq!(b.balance(&a, None, false), -1000);
    let sid = b.scheduled[0].id.clone();
    let before = json(&b);
    assert!(b.post_scheduled(&sid).is_err());
    assert_eq!(json(&b), before);
    for s in &mut b.scheduled {
        s.next_date = "2026-01-31".into();
    }
    b.post_scheduled(&sid).unwrap();
    assert_eq!(b.transactions.len(), 3);
    assert_eq!(b.balance(&a, None, false), -1500);
    assert_eq!(b.balance(&other, None, false), 500);
    assert!(b.scheduled.iter().all(|s| s.next_date == "2026-02-07"));
    store::validate(&b).unwrap();
}
#[test]
fn legacy_json_defaults_new_metadata_and_schedule_fields() {
    let (mut b, _, _, _) = fixture();
    b.register_command(
        &ids(&["manual"]),
        Command::Schedule {
            date: "2026-01-31".into(),
            repeat: Repeat::Monthly,
            move_original: false,
        },
    )
    .unwrap();
    let mut value = serde_json::to_value(b).unwrap();
    value.as_object_mut().unwrap().remove("register_meta");
    for s in value["scheduled"].as_array_mut().unwrap() {
        s.as_object_mut().unwrap().remove("repeat");
        s.as_object_mut().unwrap().remove("anchor_day");
    }
    let b: Budget = serde_json::from_value(value).unwrap();
    assert!(b.register_meta.is_empty());
    assert_eq!(b.scheduled[0].frequency(), Repeat::Monthly);
    store::validate(&b).unwrap();
}

#[test]
fn csv_match_counts_each_import_once_and_preserves_real_duplicate_rows() {
    let (mut b, a, _, _) = fixture();
    b.transactions[0].date = "2026-01-31".into();
    b.transactions[0].payee = "Shop".into();
    b.transactions[0].memo.clear();
    let path = std::env::temp_dir().join(format!("budgettinker-duplicates-{}.csv", id()));
    // Import after entering a manual transaction with another payee, to force an explicit match.
    b.transactions[0].payee = "Manual Shop".into();
    fs::write(&path, "Date,Payee,Amount\n2026-01-31,Shop,-10.00\n").unwrap();
    let (mut b, n) = store::import_csv(&b, &path, &a).unwrap();
    assert_eq!(n, 1);
    let imported = b.transactions[1].id.clone();
    b.register_command(&ids(&["manual", &imported]), Command::Match)
        .unwrap();
    assert_eq!(store::import_csv(&b, &path, &a).unwrap().1, 0);
    fs::write(
        &path,
        "Date,Payee,Amount\n2026-01-31,Shop,-10.00\n2026-01-31,Shop,-10.00\n",
    )
    .unwrap();
    assert_eq!(store::import_csv(&b, &path, &a).unwrap().1, 1);
    fs::remove_file(path).unwrap();
}
#[test]
fn saved_versions_can_undo_and_redo_a_matched_import() {
    let (mut b, _, _, _) = with_import();
    let original = json(&b);
    let path = std::env::temp_dir().join(format!("budgettinker-undo-{}.sqlite3", id()));
    {
        let mut store = Store::open(&path).unwrap();
        store.save(&b).unwrap();
        b.register_command(&ids(&["manual", "imported"]), Command::Match)
            .unwrap();
        let matched = json(&b);
        store.save(&b).unwrap();
        let revision = store.versions().unwrap()[0].0;
        let undo = store.version(revision).unwrap();
        assert_eq!(json(&undo), original);
        store.save(&undo).unwrap();
        let redo_revision = store.versions().unwrap()[0].0;
        let redo = store.version(redo_revision).unwrap();
        assert_eq!(json(&redo), matched);
        store.save(&redo).unwrap();
        assert_eq!(json(&store.load().unwrap().unwrap()), matched);
    }
    fs::remove_file(path).unwrap();
}
#[test]
fn stale_selection_and_invalid_schedule_never_partially_apply() {
    let (mut b, _, _, _) = with_import();
    let before = json(&b);
    for command in [Command::Delete, Command::Cleared(true), Command::Approve] {
        assert!(
            b.register_command(&ids(&["manual", "missing"]), command)
                .is_err()
        );
        assert_eq!(json(&b), before);
    }
    for date in ["bad", "2026-02-30", "2026-01-31"] {
        assert!(
            b.register_command(
                &ids(&["manual"]),
                Command::Schedule {
                    date: date.into(),
                    repeat: Repeat::Weekly,
                    move_original: true
                }
            )
            .is_err()
        );
        assert_eq!(json(&b), before);
    }
}
#[test]
fn moving_a_matched_record_preserves_the_originals_for_unmatch() {
    let (mut b, a, other, _) = with_import();
    b.register_command(&ids(&["manual", "imported"]), Command::Match)
        .unwrap();
    b.register_command(&ids(&["manual"]), Command::MoveAccount(other.clone()))
        .unwrap();
    b.register_command(&ids(&["manual"]), Command::Unmatch)
        .unwrap();
    assert!(b.transactions.iter().all(|t| t.account == other));
    assert_eq!(b.balance(&a, None, false), 0);
    store::validate(&b).unwrap();
}

#[test]
fn flags_are_atomic_metadata_even_on_reconciled_transactions_and_survive_versions() {
    let (mut b, account, _, _) = fixture();
    let original = b.transactions[0].id.clone();
    b.transactions[0].reconciled = true;
    b.transactions[0].cleared = true;
    let balance = b.balance(&account, None, false);
    let ids = HashSet::from([original.clone()]);
    b.register_command(&ids, Command::Flag(Some(TransactionFlag::Blue)))
        .unwrap();
    assert_eq!(b.transactions[0].flag, Some(TransactionFlag::Blue));
    assert!(b.transactions[0].reconciled);
    assert_eq!(b.balance(&account, None, false), balance);
    let before = serde_json::to_string(&b).unwrap();
    assert!(
        b.register_command(
            &HashSet::from([original.clone(), "missing".into()]),
            Command::Flag(Some(TransactionFlag::Red))
        )
        .is_err()
    );
    assert_eq!(serde_json::to_string(&b).unwrap(), before);
    let path = std::env::temp_dir().join(format!("budgettinker-flags-{}.sqlite3", id()));
    {
        let mut store = Store::open(&path).unwrap();
        store.save(&b).unwrap();
        b.register_command(&ids, Command::Flag(None)).unwrap();
        store.save(&b).unwrap();
        assert!(
            store.load().unwrap().unwrap().transactions[0]
                .flag
                .is_none()
        );
        let old = store.version(store.versions().unwrap()[0].0).unwrap();
        assert_eq!(old.transactions[0].flag, Some(TransactionFlag::Blue));
        let mut legacy = serde_json::to_value(&old).unwrap();
        for t in legacy["transactions"].as_array_mut().unwrap() {
            t.as_object_mut().unwrap().remove("flag");
        }
        let loaded: Budget = serde_json::from_value(legacy).unwrap();
        assert!(loaded.transactions[0].flag.is_none());
    }
    std::fs::remove_file(path).unwrap();
}
