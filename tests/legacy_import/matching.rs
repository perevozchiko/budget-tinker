use super::*;

#[test]
fn matched_snapshot_keeps_visible_row_and_originals_through_sqlite_and_unmatch() {
    use budgettinker::register::Command;
    for reverse in [false, true] {
        let f = Fixture::matched();
        if reverse {
            f.edit(|v| {
                v["transactions"][3]["matchedTransactions"]
                    .as_array_mut()
                    .unwrap()
                    .reverse()
            });
        }
        let source = fs::read(&f.full).unwrap();
        let mut b = legacy_budget::preview(&f.root).unwrap().budget;
        assert_eq!(b.transactions.len(), 8);
        assert_eq!(b.balance("card", None, false), -12_500);
        assert_eq!(b.activity("2026-01", "food"), -1500);
        let merged = b.transactions.iter().find(|t| t.id == "split").unwrap();
        assert_eq!(merged.memo, "Combined entry");
        assert_eq!(merged.date, "2026-01-08");
        assert_eq!(merged.splits.len(), 2);
        let state = &b.register_meta["split"];
        assert!(state.pending);
        let backup = state.matched.clone().unwrap();
        assert_eq!(backup.imported.id, "split");
        assert_eq!(backup.manual.id, "manual/split");
        assert_eq!(backup.imported.payee, "Bank");
        assert_eq!(backup.imported.date, "2026-01-12");
        assert_eq!(backup.manual.date, "2026-01-08");
        assert!(backup.pending);
        assert!(b.imported_id_exists("split"));
        assert!(b.imported_id_exists("manual/split"));
        assert_eq!(
            b.notes["transaction:MT/split/MatchedTransaction/split:check"],
            "B-42"
        );
        assert_eq!(
            b.notes["transaction:MT/split/MatchedTransaction/manual/split:check"],
            "M-42"
        );
        let path = f.root.parent().unwrap().join("matched.sqlite3");
        let mut store = Store::open(&path).unwrap();
        store.save(&b).unwrap();
        b = store.load().unwrap().unwrap();
        assert_eq!(
            serde_json::to_value(&b.register_meta["split"].matched).unwrap(),
            serde_json::to_value(Some(backup.clone())).unwrap()
        );
        let ids = ["split".into()].into_iter().collect();
        b.register_command(&ids, Command::Approve).unwrap();
        assert!(!b.pending_import("split"));
        assert!(b.register_meta["split"].matched.is_some());
        b.register_command(&ids, Command::Unmatch).unwrap();
        assert_eq!(b.transactions.len(), 9);
        assert_eq!(b.balance("card", None, false), -15_500);
        assert!(!b.register_meta.contains_key("manual/split"));
        assert!(b.pending_import("split"));
        for original in [&backup.manual, &backup.imported] {
            let actual = b.transactions.iter().find(|t| t.id == original.id).unwrap();
            assert_eq!(
                serde_json::to_value(actual).unwrap(),
                serde_json::to_value(original).unwrap()
            );
        }
        store.save(&b).unwrap();
        assert_eq!(
            serde_json::to_value(store.load().unwrap()).unwrap(),
            serde_json::to_value(Some(&b)).unwrap()
        );
        assert_eq!(fs::read(&f.full).unwrap(), source);
    }
}

#[test]
fn matched_snapshot_reject_move_and_id_collisions_are_atomic() {
    use budgettinker::register::Command;
    let f = Fixture::matched();
    let b = legacy_budget::preview(&f.root).unwrap().budget;
    let ids = ["split".into()].into_iter().collect();
    let manual = b.register_meta["split"]
        .matched
        .as_ref()
        .unwrap()
        .manual
        .clone();
    let mut rejected = b.clone();
    rejected.register_command(&ids, Command::Reject).unwrap();
    assert!(rejected.transactions.iter().all(|t| t.id != "split"));
    assert!(!rejected.register_meta.contains_key("split"));
    assert_eq!(
        serde_json::to_value(
            rejected
                .transactions
                .iter()
                .find(|t| t.id == manual.id)
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(&manual).unwrap()
    );
    let mut moved = b.clone();
    moved
        .register_command(&ids, Command::MoveAccount("cash".into()))
        .unwrap();
    moved.register_command(&ids, Command::Unmatch).unwrap();
    assert!(
        moved
            .transactions
            .iter()
            .filter(|t| t.id == "split" || t.id == "manual/split")
            .all(|t| t.account == "cash")
    );
    let mut corrupt = b.clone();
    corrupt.transactions.push(manual);
    assert!(store::validate(&corrupt).is_err());
    let before = serde_json::to_value(&corrupt).unwrap();
    assert!(!corrupt.can_register_command(&ids, Command::Unmatch));
    assert!(corrupt.register_command(&ids, Command::Unmatch).is_err());
    assert_eq!(serde_json::to_value(&corrupt).unwrap(), before);
}

#[test]
fn approved_zero_and_income_matches_keep_timing_and_reconciled_locks() {
    use budgettinker::register::Command;
    for amount in [-30, 0, 30] {
        let f = Fixture::matched();
        f.edit(|v| {
            let parent = &mut v["transactions"][3];
            parent["amount"] = json!(amount);
            parent["categoryId"] = json!("Category/__DeferredIncome__");
            parent["subTransactions"] = json!([]);
            parent["accepted"] = json!(true);
            for i in 0..2 {
                parent["matchedTransactions"][i]["amount"] = json!(amount);
                parent["matchedTransactions"][i]["categoryId"] = json!(if i == 0 {
                    "Category/__ImmediateIncome__"
                } else {
                    "Category/__DeferredIncome__"
                });
                parent["matchedTransactions"][i]["subTransactions"] = json!([]);
            }
        });
        let mut b = legacy_budget::preview(&f.root).unwrap().budget;
        assert!(!b.pending_import("split"));
        assert!(b.register_meta["split"].matched.is_some());
        // The other fixture income deferred until February is $300.
        assert_eq!(b.deferred_income("2026-01"), 30_000 + amount * 100);
        let ids = ["split".into()].into_iter().collect();
        assert!(!b.can_register_command(&ids, Command::Reject));
        b.register_command(&ids, Command::Unmatch).unwrap();
        assert_eq!(b.deferred_income("2026-01"), 30_000 + amount * 100);
        assert!(b.pending_import("split"));
        f.edit(|v| v["transactions"][3]["cleared"] = json!("Reconciled"));
        let mut b = legacy_budget::preview(&f.root).unwrap().budget;
        let before = serde_json::to_value(&b).unwrap();
        assert!(!b.can_register_command(&ids, Command::Unmatch));
        assert!(b.register_command(&ids, Command::Unmatch).is_err());
        assert_eq!(serde_json::to_value(&b).unwrap(), before);
    }
}

#[test]
fn malformed_or_unsupported_matches_fail_without_changing_source() {
    for case in 0..12 {
        let f = Fixture::matched();
        f.edit(|v| {
            let parent = &mut v["transactions"][3];
            match case {
                0 => {
                    parent["matchedTransactions"].as_array_mut().unwrap().pop();
                }
                1 => {
                    parent["matchedTransactions"][0]["parentTransactionIdIfMatched"] =
                        json!("wrong")
                }
                2 => parent["matchedTransactions"][0]["entityId"] = json!("wrong"),
                3 => parent["matchedTransactions"][0]["amount"] = json!(-29),
                4 => parent["matchedTransactions"][0]["accountId"] = json!("cash"),
                5 => parent["matchedTransactions"][0]["targetAccountId"] = json!("cash"),
                6 => parent["matchedTransactions"][0]["matchedTransactions"] = json!([{}]),
                7 => {
                    parent["matchedTransactions"][0]["entityId"] =
                        json!("MT/split/MatchedTransaction/other")
                }
                8 => {
                    parent["matchedTransactions"][1]["entityId"] =
                        json!("MT/split/MatchedTransaction/interest")
                }
                9 => parent["matchedTransactions"][0]["transferTransactionId"] = json!("peer"),
                10 => parent["matchedTransactions"][1]["isTombstone"] = json!(true),
                _ => parent["parentTransactionIdIfMatched"] = json!("orphan"),
            }
        });
        let source = fs::read(&f.full).unwrap();
        assert!(legacy_budget::preview(&f.root).is_err(), "case {case}");
        assert_eq!(fs::read(&f.full).unwrap(), source);
    }
}

#[test]
fn broken_split_transfers_and_match_transfers_fail_before_import() {
    for case in 0..9 {
        let f = Fixture::new();
        fs::write(
            &f.full,
            include_bytes!("../fixtures/legacy/ExpandedSplits.yfull"),
        )
        .unwrap();
        f.edit(|v| match case {
            0 => v["transactions"][8]["transferTransactionId"] = json!("split"),
            1 => v["transactions"][3]["subTransactions"][6]["transferTransactionId"] = Value::Null,
            2 => v["transactions"][3]["subTransactions"][6]["targetAccountId"] = json!("card"),
            3 => v["transactions"][3]["subTransactions"][6]["categoryId"] = json!("food"),
            4 => v["transactions"][3]["subTransactions"][5]["payeeId"] = json!("missing"),
            5 => v["transactions"][3]["subTransactions"][7]["entityId"] = json!("s7"),
            6 => {
                v["scheduledTransactions"][2]["subTransactions"][6]["transferTransactionId"] =
                    json!("missing")
            }
            7 => v["transactions"][3]["subTransactions"][6]["matchedTransactions"] = json!([{}]),
            _ => {
                let original = v["transactions"][3].clone();
                v["transactions"][3]["matchedTransactions"] = json!([original.clone(), original]);
            }
        });
        let before = fs::read(&f.full).unwrap();
        assert!(legacy_budget::preview(&f.root).is_err(), "case {case}");
        assert_eq!(fs::read(&f.full).unwrap(), before);
    }
}
