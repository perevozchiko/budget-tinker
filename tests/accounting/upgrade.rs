use super::*;
use budgettinker::{
    model::operation_fx::{FrozenOperation, RateOrigin},
    register::{ImportState, MatchBackup},
    store::{self, upgrade::FrozenBudget},
};
use rusqlite::Connection;

fn manual_rate(value: i64) -> budgettinker::model::operation_fx::OperationRate {
    budgettinker::model::operation_fx::OperationRate {
        rate: value,
        rate_date: "2026-01-01".into(),
        source: "Synthetic manual quote".into(),
        origin: RateOrigin::Manual,
    }
}

#[test]
fn manual_operation_fx_posts_without_monthly_quotes_and_survives_second_validation() {
    let mut b = Budget::new("EUR");
    b.version = 2;
    let rsd = b.add_account("RSD", "RSD", true, 0, "2026-01-01").unwrap();
    let category = b.categories[0].id.clone();
    let t = expense(&rsd, &category, -118000, "2026-01-02");
    let t = b
        .prepare_operation_fx(t, manual_rate(855000), Some((-1009, vec![])))
        .unwrap();
    let fx = t.operation_fx.clone();
    b.put_transaction(t.clone()).unwrap();
    assert_eq!(b.transactions.last().unwrap().operation_fx, fx);
    assert_eq!(b.balance(&rsd, None, false), -118000);
    assert!(b.snapshots.is_empty());
    store::validate(&b).unwrap();
    let mut edit = t;
    edit.memo = "Metadata only".into();
    b.put_transaction(edit).unwrap();
    assert_eq!(b.transactions.last().unwrap().operation_fx, fx);
    let root = std::env::temp_dir().join(format!("manual-fx-{}", id()));
    let path = root.join("test.budgettinker");
    let mut db = store::Store::open(&path).unwrap();
    db.save(&b).unwrap();
    assert_eq!(json(&db.load().unwrap().unwrap()), json(&b));
    drop(db);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn manual_fx_split_sums_and_invalid_quotes_are_atomic() {
    let mut b = Budget::new("EUR");
    b.version = 2;
    let rsd = b.add_account("RSD", "RSD", true, 0, "2026-01-01").unwrap();
    let category = b.categories[0].id.clone();
    let mut t = expense(&rsd, &category, -118000, "2026-01-02");
    t.category = None;
    t.splits = [-70800, -47200]
        .into_iter()
        .map(|amount| Split {
            amount,
            category: Some(category.clone()),
            ..Default::default()
        })
        .collect();
    let before = json(&b);
    for (base, parts) in [
        (-1009, vec![-605, -403]),
        (1009, vec![605, 404]),
        (-1009, vec![-1009]),
    ] {
        assert!(
            b.prepare_operation_fx(t.clone(), manual_rate(855000), Some((base, parts)))
                .is_err()
        );
        assert_eq!(json(&b), before);
    }
    for rate in [0, -1, 100_000_000_000_001] {
        assert!(
            b.prepare_operation_fx(t.clone(), manual_rate(rate), None)
                .is_err()
        );
        assert_eq!(json(&b), before);
    }
    let t = b
        .prepare_operation_fx(t, manual_rate(855000), Some((-1009, vec![-605, -404])))
        .unwrap();
    let frozen = t.operation_fx.clone();
    b.put_transaction(t).unwrap();
    assert_eq!(b.transactions.last().unwrap().operation_fx, frozen);
    assert_eq!(
        b.split_base_amounts(b.transactions.last().unwrap())
            .unwrap(),
        [-605, -404]
    );
    store::validate(&b).unwrap();
}

fn json(b: &Budget) -> serde_json::Value {
    serde_json::to_value(b).unwrap()
}

#[test]
fn detached_fx_preserves_every_currency_and_ignores_monthly_changes() {
    for currency in CURRENCIES {
        let mut b = Budget::new("EUR");
        if *currency != "EUR" {
            b.add_snapshot("2026-01", currency, 91_234_567, "2026-01-31", "synthetic")
                .unwrap();
        }
        let account = b
            .add_account("Test", currency, true, 12345, "2026-01-03")
            .unwrap();
        let category = b.categories[0].id.clone();
        b.put_transaction(expense(&account, &category, -537, "2026-01-04"))
            .unwrap();
        let before = json(&b);
        let candidate = FrozenBudget::preflight(&b).unwrap();
        assert_eq!(json(&candidate.budget), before);
        assert_eq!(candidate.operations.len(), b.transactions.len());
        let repeat = FrozenBudget::preflight(&b).unwrap();
        assert_eq!(candidate.operations, repeat.operations);
        let locks = serde_json::to_value(&candidate.operations).unwrap();
        b.snapshots.clear();
        for t in &candidate.budget.transactions {
            let lock = &candidate.operations[&t.id];
            assert_eq!(lock.native_amount, t.amount);
            assert_eq!(lock.base_amount, t.base_amount);
            assert_eq!(lock.rate.is_some(), *currency != "EUR");
            if let Some(rate) = &lock.rate {
                assert_eq!(rate.rate, 91_234_567);
                assert_eq!(rate.rate_date, "2026-01-31");
                assert_eq!(rate.source, "synthetic");
                assert!(
                    matches!(&rate.origin, RateOrigin::MonthlyDefault { snapshot_id } if Some(snapshot_id) == t.snapshot.as_ref())
                );
            }
            lock.validate().unwrap();
            let restored: FrozenOperation =
                serde_json::from_value(serde_json::to_value(lock).unwrap()).unwrap();
            assert_eq!(&restored, lock);
            restored.validate().unwrap();
        }
        assert_eq!(serde_json::to_value(&candidate.operations).unwrap(), locks);
        assert_eq!(json(&candidate.budget), before);
    }
}

#[test]
fn detached_split_rounding_zero_net_and_empty_movements_are_lossless() {
    let (mut b, _, usd) = fixture();
    let category = b.categories[0].id.clone();
    for parts in [vec![-6, 3, 3, 0], vec![-1, -1, -1, 0], vec![0, 0]] {
        let mut t = expense(&usd, &category, parts.iter().sum(), "2026-01-20");
        t.category = None;
        t.splits = parts
            .into_iter()
            .map(|amount| Split {
                amount,
                category: Some(category.clone()),
                ..Default::default()
            })
            .collect();
        b.put_transaction(t).unwrap();
    }
    let candidate = FrozenBudget::preflight(&b).unwrap();
    for t in &b.transactions {
        let frozen = &candidate.operations[&t.id];
        assert_eq!(
            frozen
                .splits
                .iter()
                .map(|s| s.base_amount)
                .collect::<Vec<_>>(),
            b.split_base_amounts(t).unwrap()
        );
        frozen.validate().unwrap();
        if t.amount == 0 && t.splits.iter().all(|s| s.amount == 0) {
            assert!(frozen.rate.is_none());
        }
        if !t.splits.is_empty() {
            assert_eq!(frozen.splits.last().unwrap().base_amount, 0);
            let mut broken = frozen.clone();
            broken.splits[0].base_amount += 1;
            assert!(broken.validate().is_err());
            broken = frozen.clone();
            broken.splits[1].id = broken.splits[0].id.clone();
            assert!(broken.validate().is_err());
        }
    }
}

#[test]
fn preflight_keeps_schedule_templates_and_match_originals() {
    let (mut b, _, usd) = fixture();
    let category = b.categories[0].id.clone();
    let t = b
        .prepare_transaction(expense(&usd, &category, -123, "2026-01-20"))
        .unwrap();
    let mut imported = t.clone();
    imported.id = id();
    imported.memo = "Bank original".into();
    b.register_meta.insert(
        t.id.clone(),
        ImportState {
            pending: false,
            source_id: Some("synthetic".into()),
            matched: Some(MatchBackup {
                manual_state: None,
                manual: t.clone(),
                imported: imported.clone(),
                pending: true,
            }),
        },
    );
    b.transactions.push(t.clone());
    let template = b
        .prepare_scheduled_transaction(expense(&usd, &category, -123, "2026-02-20"))
        .unwrap();
    b.scheduled.push(Scheduled {
        id: id(),
        transaction: template.clone(),
        next_date: template.date.clone(),
        monthly: true,
        repeat: None,
        anchor_day: None,
    });
    let before = json(&b);
    let candidate = FrozenBudget::preflight(&b).unwrap();
    let mut active = candidate.clone().into_v2().unwrap();
    active.snapshots.clear();
    validate(&active).unwrap();
    let ids = std::collections::HashSet::from([t.id.clone()]);
    active
        .register_command(&ids, budgettinker::register::Command::Unmatch)
        .unwrap();
    validate(&active).unwrap();
    assert!(active.transactions.iter().all(|t| t.operation_fx.is_some()));
    assert_eq!(json(&candidate.budget), before);
    assert!(!candidate.operations.contains_key(&template.id));
    assert_eq!(
        candidate.matched_originals[&t.id][0].base_amount,
        t.base_amount
    );
    assert_eq!(
        candidate.matched_originals[&t.id][1].base_amount,
        imported.base_amount
    );
    b.register_meta
        .get_mut(&t.id)
        .unwrap()
        .matched
        .as_mut()
        .unwrap()
        .imported
        .base_amount += 1;
    assert!(FrozenBudget::preflight(&b).is_err());
}

#[test]
fn malformed_current_schema_is_rejected_without_inventing_fx() {
    let (b, _, _) = fixture();
    let before = json(&b);
    let mut invalid = b.clone();
    invalid.snapshots.clear();
    assert!(FrozenBudget::preflight(&invalid).is_err());
    invalid = b.clone();
    invalid.transactions[1].base_amount += 1;
    assert!(FrozenBudget::preflight(&invalid).is_err());
    invalid = b.clone();
    invalid.transactions[1].snapshot = Some("broken-reference".into());
    assert!(FrozenBudget::preflight(&invalid).is_err());
    invalid = b.clone();
    invalid.accounts.push(invalid.accounts[0].clone());
    assert!(FrozenBudget::preflight(&invalid).is_err());
    assert_eq!(json(&b), before);
}

#[test]
fn operation_validator_rejects_missing_quote_and_tampered_postings() {
    let (b, _, _) = fixture();
    let candidate = FrozenBudget::preflight(&b).unwrap();
    let lock = &candidate.operations[&b.transactions[1].id];
    let mut bad = lock.clone();
    bad.rate = None;
    assert!(bad.validate().is_err());
    bad = lock.clone();
    bad.base_amount += 1;
    assert!(bad.validate().is_err());
    bad = lock.clone();
    bad.rate.as_mut().unwrap().rate = i64::MAX;
    assert!(bad.validate().is_err());
    bad = lock.clone();
    bad.rate.as_mut().unwrap().rate_date = "2026-02-30".into();
    assert!(bad.validate().is_err());
    bad = lock.clone();
    bad.account_currency = "XXX".into();
    assert!(bad.validate().is_err());
}

#[test]
fn wal_backup_preserves_revisions_extra_tables_and_reopens_independently() {
    let (mut b, _, _) = fixture();
    let directory = temp("upgrade");
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("live.budgettinker");
    let mut store = Store::open(&path).unwrap();
    let extra = Connection::open(&path).unwrap();
    extra.execute_batch("PRAGMA wal_autocheckpoint=0; CREATE TABLE extra(value TEXT); INSERT INTO extra VALUES('preserve me');").unwrap();
    store.save(&b).unwrap();
    b.name = "Second revision".into();
    store.save(&b).unwrap();
    assert!(
        path.with_extension("budgettinker-wal")
            .metadata()
            .unwrap()
            .len()
            > 0
    );
    let preview = store.preflight_upgrade().unwrap();
    assert_eq!(preview.revisions.len(), 1);
    let backup = store.backup_for_upgrade(&preview, &directory).unwrap();
    assert_eq!(json(&read_budget(&backup).unwrap()), json(&b));
    let independent =
        Connection::open_with_flags(&backup, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let extra_value: String = independent
        .query_row("SELECT value FROM extra", [], |r| r.get(0))
        .unwrap();
    assert_eq!(extra_value, "preserve me");
    drop(independent);
    let recovery = Store::open(&backup).unwrap();
    assert_eq!(recovery.versions().unwrap(), store.versions().unwrap());
    for (id, candidate) in &preview.revisions {
        assert_eq!(
            json(&recovery.version(*id).unwrap()),
            json(&candidate.budget)
        );
    }
    // Later live writes cannot change the recovery file.
    b.name = "After backup".into();
    store.save(&b).unwrap();
    assert_eq!(recovery.load().unwrap().unwrap().name, "Second revision");
    assert_eq!(store.load().unwrap().unwrap().name, "After backup");
    drop(recovery);
    drop(extra);
    drop(store);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn failed_or_stale_backup_leaves_source_unchanged() {
    let (mut b, _, _) = fixture();
    let directory = temp("upgrade");
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("live.budgettinker");
    let mut store = Store::open(&path).unwrap();
    store.save(&b).unwrap();
    let preview = store.preflight_upgrade().unwrap();
    assert!(
        store
            .backup_for_upgrade(&preview, &directory.join("missing"))
            .is_err()
    );
    assert_eq!(json(&store.load().unwrap().unwrap()), json(&b));
    assert!(store.versions().unwrap().is_empty());
    b.name = "Changed".into();
    store.save(&b).unwrap();
    assert!(
        store
            .backup_for_upgrade(&preview, &directory)
            .unwrap_err()
            .contains("changed")
    );
    assert!(
        std::fs::read_dir(&directory).unwrap().all(|entry| !entry
            .unwrap()
            .file_type()
            .unwrap()
            .is_dir())
    );
    assert_eq!(json(&store.load().unwrap().unwrap()), json(&b));
    drop(store);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn invalid_retained_revision_blocks_preflight_without_deleting_history() {
    let (b, _, _) = fixture();
    let path = temp("sqlite3");
    let mut store = Store::open(&path).unwrap();
    store.save(&b).unwrap();
    let writer = Connection::open(&path).unwrap();
    writer
        .execute("INSERT INTO revisions(json) VALUES(?1)", ["{malformed}"])
        .unwrap();
    let before = store.versions().unwrap();
    assert!(store.preflight_upgrade().unwrap_err().contains("revision"));
    assert_eq!(store.versions().unwrap(), before);
    assert_eq!(json(&store.load().unwrap().unwrap()), json(&b));
    drop(writer);
    drop(store);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn split_conversion_checks_final_range_without_intermediate_i64_overflow() {
    let mut b = Budget::new("EUR");
    b.add_snapshot(
        "2026-01",
        "JPY",
        100_000_000_000_000,
        "2026-01-31",
        "synthetic extreme",
    )
    .unwrap();
    let account = b.add_account("Yen", "JPY", true, 0, "2026-01-01").unwrap();
    let category = b.categories[0].id.clone();
    let mut t = expense(&account, &category, 0, "2026-01-20");
    t.category = None;
    t.splits = [
        60_000_000_000,
        60_000_000_000,
        -60_000_000_000,
        -60_000_000_000,
    ]
    .into_iter()
    .map(|amount| Split {
        amount,
        category: Some(category.clone()),
        ..Default::default()
    })
    .collect();
    b.put_transaction(t).unwrap();
    validate(&b).unwrap();
    let candidate = FrozenBudget::preflight(&b).unwrap();
    assert_eq!(
        candidate
            .operations
            .values()
            .next()
            .unwrap()
            .splits
            .iter()
            .map(|s| s.base_amount as i128)
            .sum::<i128>(),
        0
    );
    let row = b.transactions.last_mut().unwrap();
    row.splits.pop();
    row.splits[2].amount = -120_000_000_000;
    assert!(validate(&b).unwrap_err().contains("overflow"));
    assert!(FrozenBudget::preflight(&b).is_err());
}

#[test]
fn atomic_upgrade_preserves_postings_history_and_reopens_with_v2_readers() {
    let (mut b, eur, usd) = fixture();
    let category = b.categories[0].id.clone();
    let mut split = expense(&usd, &category, -3, "2026-01-20");
    split.category = None;
    split.splits = [-1, -1, -1]
        .into_iter()
        .map(|amount| Split {
            amount,
            category: Some(category.clone()),
            ..Default::default()
        })
        .collect();
    b.put_transaction(split).unwrap();
    let before_activity = b.activity("2026-01", &category);
    let directory = temp("activation");
    let path = directory.join("live.budgettinker");
    let mut store = Store::open(&path).unwrap();
    store.save(&b).unwrap();
    b.name = "Latest".into();
    store.save(&b).unwrap();
    let versions = store.versions().unwrap();
    let mut preview = store.preflight_upgrade().unwrap();
    // Public presentation data cannot tamper with the private validated source.
    preview.current.budget.name = "Tampered preview".into();
    let (mut upgraded, backup) = store.upgrade(&preview, &directory).unwrap();
    assert_eq!(upgraded.version, 2);
    assert_eq!(upgraded.name, "Latest");
    assert_eq!(upgraded.activity("2026-01", &category), before_activity);
    assert_eq!(
        upgraded.balance(&eur, None, false),
        b.balance(&eur, None, false)
    );
    assert_eq!(
        upgraded.balance(&usd, None, false),
        b.balance(&usd, None, false)
    );
    assert_eq!(json(&store.load().unwrap().unwrap()), json(&upgraded));
    assert_eq!(json(&read_budget(&backup).unwrap()), json(&b));
    assert_eq!(store.versions().unwrap(), versions);
    for (id, _) in &versions {
        assert_eq!(store.version(*id).unwrap().version, 2);
    }
    let locks: Vec<_> = upgraded
        .transactions
        .iter()
        .map(|t| t.operation_fx.clone())
        .collect();
    for snapshot in &mut upgraded.snapshots {
        snapshot.rate /= 2;
    }
    validate(&upgraded).unwrap();
    assert_eq!(upgraded.activity("2026-01", &category), before_activity);
    assert_eq!(
        locks,
        upgraded
            .transactions
            .iter()
            .map(|t| t.operation_fx.clone())
            .collect::<Vec<_>>()
    );
    store.save(&upgraded).unwrap();
    let json_path = directory.join("roundtrip.json");
    export_json(&upgraded, &json_path).unwrap();
    assert_eq!(json(&import_json(&json_path).unwrap()), json(&upgraded));
    drop(store);
    let reopened = Store::open(&path).unwrap();
    assert_eq!(json(&reopened.load().unwrap().unwrap()), json(&upgraded));
    drop(reopened);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn failed_upgrade_rolls_back_document_and_every_revision_and_keeps_recovery() {
    for trigger in [
        "CREATE TRIGGER fail_upgrade BEFORE UPDATE ON revisions BEGIN SELECT RAISE(ABORT, 'synthetic failure'); END;",
        "CREATE TRIGGER fail_upgrade BEFORE UPDATE ON revisions BEGIN SELECT RAISE(IGNORE); END;",
    ] {
        let (mut b, _, _) = fixture();
        let directory = temp("rollback");
        let path = directory.join("live.budgettinker");
        let mut store = Store::open(&path).unwrap();
        for index in 0..3 {
            b.name = format!("Revision {index}");
            store.save(&b).unwrap();
        }
        let writer = Connection::open(&path).unwrap();
        writer.execute_batch(trigger).unwrap();
        let before = store.versions().unwrap();
        let originals: Vec<_> = before
            .iter()
            .map(|(id, _)| (*id, json(&store.version(*id).unwrap())))
            .collect();
        let preview = store.preflight_upgrade().unwrap();
        assert!(
            store
                .upgrade(&preview, &directory)
                .unwrap_err()
                .contains("Recovery copy")
        );
        assert_eq!(json(&store.load().unwrap().unwrap()), json(&b));
        assert_eq!(store.versions().unwrap(), before);
        for (id, original) in originals {
            assert_eq!(json(&store.version(id).unwrap()), original);
        }
        let backup = std::fs::read_dir(&directory)
            .unwrap()
            .map(|p| p.unwrap().path())
            .find(|p| p.is_dir())
            .unwrap()
            .join("original.budgettinker");
        assert_eq!(json(&read_budget(&backup).unwrap()), json(&b));
        drop(writer);
        drop(store);
        assert_eq!(read_budget(&path).unwrap().version, 1);
        std::fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn activation_rejects_stale_preview_backup_failure_downgrade_and_base_switch() {
    let (mut b, _, _) = fixture();
    let directory = temp("activation-guards");
    let mut store = Store::open(&directory.join("live.budgettinker")).unwrap();
    store.save(&b).unwrap();
    let preview = store.preflight_upgrade().unwrap();
    assert!(store.upgrade(&preview, &directory.join("missing")).is_err());
    assert_eq!(json(&store.load().unwrap().unwrap()), json(&b));
    b.name = "Changed".into();
    store.save(&b).unwrap();
    assert!(
        store
            .upgrade(&preview, &directory)
            .unwrap_err()
            .contains("changed")
    );
    let preview = store.preflight_upgrade().unwrap();
    let (upgraded, _) = store.upgrade(&preview, &directory).unwrap();
    assert!(store.save(&b).unwrap_err().contains("Schema"));
    let mut empty = Budget::new("EUR");
    empty.version = 2;
    let other_path = directory.join("empty.budgettinker");
    let mut other = Store::open(&other_path).unwrap();
    other.save(&empty).unwrap();
    empty.base = "USD".into();
    assert!(other.save(&empty).unwrap_err().contains("immutable"));
    assert_eq!(json(&store.load().unwrap().unwrap()), json(&upgraded));
    drop(other);
    drop(store);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn active_locks_validate_without_monthly_references_and_reject_corruption() {
    let (b, _, _) = fixture();
    let b = FrozenBudget::preflight(&b).unwrap().into_v2().unwrap();
    let mut independent = b.clone();
    independent.snapshots.clear();
    validate(&independent).unwrap();
    let mut bad = independent.clone();
    bad.transactions[1].operation_fx = None;
    assert!(validate(&bad).is_err());
    bad = independent.clone();
    bad.transactions[1].amount += 1;
    assert!(validate(&bad).is_err());
    bad = independent.clone();
    bad.accounts[1].currency = "TRY".into();
    assert!(validate(&bad).is_err());
    bad = independent.clone();
    bad.base = "USD".into();
    assert!(validate(&bad).is_err());
    bad = b.clone();
    bad.version = 1;
    assert!(validate(&bad).is_err());
}

#[test]
fn exact_base_and_explicit_split_overrides_are_atomic_and_independent() {
    let (mut b, _, usd) = fixture();
    let category = b.categories[0].id.clone();
    let mut t = expense(&usd, &category, -3, "2026-01-20");
    t.category = None;
    t.splits = [-1, -1, -1]
        .into_iter()
        .map(|amount| Split {
            amount,
            category: Some(category.clone()),
            ..Default::default()
        })
        .collect();
    let tid = t.id.clone();
    b.put_transaction(t).unwrap();
    let mut b = FrozenBudget::preflight(&b).unwrap().into_v2().unwrap();
    let before = json(&b);
    assert!(b.set_operation_base(&tid, -2, &[-1, -1, -1]).is_err());
    assert_eq!(json(&b), before);
    b.set_operation_base(&tid, -2, &[-1, 0, -1]).unwrap();
    let t = b.transactions.iter().find(|t| t.id == tid).unwrap().clone();
    assert_eq!(b.split_base_amounts(&t).unwrap(), [-1, 0, -1]);
    b.snapshots.clear();
    validate(&b).unwrap();
    let before = json(&b);
    let mut metadata = t.clone();
    metadata.memo = "New memo".into();
    metadata.operation_fx = None; // actual editor rebuilds the draft this way
    b.put_transaction(metadata).unwrap();
    assert_eq!(
        b.transactions
            .iter()
            .find(|t| t.id == tid)
            .unwrap()
            .operation_fx,
        t.operation_fx
    );
    let mut invalid_edit = t;
    invalid_edit.amount = -4;
    invalid_edit.splits[0].amount = -2;
    let unchanged = json(&b);
    assert!(b.put_transaction(invalid_edit).is_err());
    assert_eq!(json(&b), unchanged);
    assert_ne!(json(&b), before);
}

#[test]
fn v2_commands_post_edit_transfer_schedule_and_keep_split_identity() {
    let (b, eur, usd) = fixture();
    let mut b = FrozenBudget::preflight(&b).unwrap().into_v2().unwrap();
    let category = b.categories[0].id.clone();
    b.add_account("New foreign", "USD", true, 500, "2026-01-10")
        .unwrap();
    b.transfer(&eur, &usd, 100, 110, "2026-01-12", "Synthetic")
        .unwrap();
    let mut t = expense(&usd, &category, -6, "2026-01-20");
    t.category = None;
    t.splits = [-1, -2, -3]
        .into_iter()
        .map(|amount| Split {
            amount,
            category: Some(category.clone()),
            ..Default::default()
        })
        .collect();
    let tid = t.id.clone();
    b.put_transaction(t).unwrap();
    validate(&b).unwrap();
    let mut edit = b.transactions.iter().find(|t| t.id == tid).unwrap().clone();
    let survivor = edit.splits[2].id.clone();
    edit.amount += 1;
    edit.splits.remove(0);
    b.put_transaction(edit).unwrap();
    let edited = b.transactions.iter().find(|t| t.id == tid).unwrap();
    assert_eq!(edited.operation_fx.as_ref().unwrap().splits[1].id, survivor);
    let template = b
        .prepare_scheduled_transaction(expense(&usd, &category, -123, "2026-02-01"))
        .unwrap();
    let sid = id();
    b.scheduled.push(Scheduled {
        id: sid.clone(),
        transaction: template,
        next_date: "2026-02-01".into(),
        monthly: false,
        repeat: None,
        anchor_day: None,
    });
    b.post_scheduled(&sid).unwrap();
    validate(&b).unwrap();
    b.snapshots.clear();
    validate(&b).unwrap();
}
