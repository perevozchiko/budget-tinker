use budgettinker::{
    bank,
    model::*,
    register::{Command, ImportState, MatchBackup},
    store,
};
use serde_json::Value;
use std::{collections::HashSet, path::Path};

fn text(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or("").into()
}
fn link(v: &Value) -> Option<String> {
    let identity = text(v, "entityId");
    let other = text(v, "transferTransactionId");
    if other.is_empty() {
        None
    } else {
        Some(
            if identity.contains("26") || other.contains("26") {
                "link26"
            } else {
                "link27"
            }
            .into(),
        )
    }
}
fn transaction(v: &Value) -> Transaction {
    Transaction {
        id: text(v, "entityId"),
        account: text(v, "accountId"),
        date: text(v, "date"),
        payee: text(v, "payee"),
        memo: text(v, "memo"),
        amount: v["amount_cents"].as_i64().unwrap(),
        base_amount: v["amount_cents"].as_i64().unwrap(),
        snapshot: None,
        category: (text(v, "categoryId") == "bank-coffee").then(|| "bank-coffee".into()),
        income_month: None,
        flag: None,
        operation_fx: None,
        check_number: v["checkNumber"].as_str().map(str::to_owned),
        cleared: text(v, "cleared") != "Uncleared",
        reconciled: text(v, "cleared") == "Reconciled",
        transfer: link(v),
        splits: v["subTransactions"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|s| Split {
                category: (text(s, "categoryId") == "bank-coffee").then(|| "bank-coffee".into()),
                amount: s["amount_cents"].as_i64().unwrap(),
                memo: text(s, "memo"),
                transfer: link(s).map(|link| SplitTransfer {
                    link,
                    account: text(s, "targetAccountId"),
                }),
                ..Default::default()
            })
            .collect(),
    }
}
fn reference() -> (Budget, Value) {
    let data: Value =
        serde_json::from_str(include_str!("fixtures/bank/matching/expected.json")).unwrap();
    let mut b = Budget::new("USD");
    for (identity, name) in [
        ("bank-checking", "Checking"),
        ("boundary-savings", "Savings"),
    ] {
        let a = b.add_account(name, "USD", true, 0, "2026-01-01").unwrap();
        b.accounts.iter_mut().find(|x| x.id == a).unwrap().id = identity.into();
    }
    b.categories[0].id = "bank-coffee".into();
    for v in data["initial"].as_array().unwrap() {
        let t = transaction(v);
        let source_id = v["FITID"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| format!("bank:{}:{s}", t.account));
        if source_id.is_some() || text(v, "source") == "Imported" {
            let matched = v["matchedTransactions"].as_array().map(|rows| {
                let mut manual = transaction(&rows[0]);
                // Craft retains the manual identity; legacyretains the bank identity.
                manual.id = t.id.clone();
                MatchBackup {
                    manual,
                    imported: transaction(&rows[1]),
                    pending: true,
                    manual_state: None,
                }
            });
            b.register_meta.insert(
                t.id.clone(),
                ImportState {
                    source_id,
                    pending: false,
                    matched,
                },
            );
        }
        b.transactions.push(t);
    }
    store::validate(&b).unwrap();
    (b, data)
}
fn json(b: &Budget) -> Value {
    serde_json::to_value(b).unwrap()
}

#[test]
fn all_84_reference_cases_preview_apply_persist_repeat_and_unmatch() {
    let (b, data) = reference();
    for (extension, matches, rows, balance) in [
        ("csv", 16, 47, 62502),
        ("qif", 18, 45, 64531),
        ("ofx", 19, 44, 65553),
    ] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "tests/fixtures/bank/matching/statement.{extension}"
        ));
        let before = json(&b);
        let preview = bank::preview(&b, &path, "bank-checking").unwrap();
        assert_eq!(preview.count(), 28);
        assert_eq!(preview.matched_count(), matches, "{extension}");
        assert_eq!(preview.transactions().len(), 28);
        assert_eq!(json(&b), before);
        let (mut next, _) = preview.apply(&b, &path, "bank-checking").unwrap();
        assert_eq!(next.transactions.len(), rows);
        assert_eq!(next.balance("bank-checking", None, false), balance);
        let mut selections = Vec::new();
        for case in data["cases"].as_array().unwrap() {
            let number = case["number"].as_u64().unwrap();
            let name = text(case, "name");
            let captured = data["results"][extension]["transactions"]
                .as_array()
                .unwrap();
            let expected = captured.iter().find_map(|r| {
                let children = r["matchedTransactions"].as_array()?;
                children
                    .iter()
                    .any(|s| text(s, "memo") == name)
                    .then_some(children)
            });
            let actual = next.register_meta.iter().find_map(|(id, m)| {
                let pair = m.matched.as_ref()?;
                (pair.imported.memo == name).then_some((id, pair))
            });
            assert_eq!(
                actual.is_some(),
                expected.is_some(),
                "{extension} case {number}"
            );
            if let Some((id, pair)) = actual {
                let expected = expected
                    .unwrap()
                    .iter()
                    .find(|r| text(r, "memo") == "My note")
                    .unwrap();
                assert!(
                    text(expected, "entityId").ends_with(id),
                    "{extension} {number}: {id}"
                );
                let manual = b.transactions.iter().find(|t| &t.id == id).unwrap();
                let result = next.transactions.iter().find(|t| &t.id == id).unwrap();
                assert_eq!(result.date, manual.date);
                assert_eq!(result.memo, manual.memo);
                assert_eq!(result.category, manual.category);
                assert_eq!(result.reconciled, manual.reconciled);
                assert!(result.cleared);
                assert_eq!(next.pending_import(id), !manual.reconciled);
                assert_eq!(
                    serde_json::to_value(&pair.manual).unwrap(),
                    serde_json::to_value(manual).unwrap()
                );
                assert_eq!(pair.imported.date, "2026-04-15");
                selections.push(HashSet::from([id.clone()]));
            }
        }
        // Reload and verify both Undo revisions and the saved merged originals.
        let db = std::env::temp_dir().join(format!("bank-reference-{}.sqlite3", id()));
        let mut store = store::Store::open(&db).unwrap();
        store.save(&b).unwrap();
        store.save(&next).unwrap();
        next = store.load().unwrap().unwrap();
        let old_statement = db.with_extension("ofx");
        std::fs::write(&old_statement, "<CURDEF>USD<STMTTRN><DTPOSTED>20260415<TRNAMT>-10.21<FITID>previous-bank-id<NAME>Old bank</STMTTRN>").unwrap();
        assert_eq!(
            bank::import(&next, &old_statement, "bank-checking")
                .unwrap()
                .1,
            0
        );
        let repeat = bank::preview(&next, &path, "bank-checking").unwrap();
        assert_eq!(repeat.count(), 0);
        assert_eq!(json(repeat.budget()), json(&next));
        assert_eq!(
            json(&store.version(store.versions().unwrap()[0].0).unwrap()),
            before
        );
        for selection in &selections {
            if next.pending_import(selection.iter().next().unwrap()) {
                next.register_command(selection, Command::Approve).unwrap();
            }
            next.register_command(selection, Command::Unmatch).unwrap();
        }
        for original in &b.transactions {
            let restored = next
                .transactions
                .iter()
                .find(|t| t.id == original.id)
                .unwrap();
            assert_eq!(
                serde_json::to_value(restored).unwrap(),
                serde_json::to_value(original).unwrap()
            );
            assert_eq!(
                serde_json::to_value(next.register_meta.get(&original.id)).unwrap(),
                serde_json::to_value(b.register_meta.get(&original.id)).unwrap()
            );
        }
        store::validate(&next).unwrap();
        assert_eq!(
            bank::preview(&next, &path, "bank-checking")
                .unwrap()
                .count(),
            0
        );
        assert_eq!(
            bank::import(&next, &old_statement, "bank-checking")
                .unwrap()
                .1,
            0
        );
        std::fs::remove_file(old_statement).unwrap();
        drop(store);
        std::fs::remove_file(db).unwrap();
    }
}

#[test]
fn legacy_json_defaults_and_check_notes_still_match_at_sixty_days() {
    let (b, _) = reference();
    let mut legacy = json(&b);
    for t in legacy["transactions"].as_array_mut().unwrap() {
        t.as_object_mut().unwrap().remove("check_number");
    }
    for m in legacy["register_meta"]
        .as_object_mut()
        .unwrap()
        .values_mut()
    {
        m.as_object_mut().unwrap().remove("source_id");
        if let Some(pair) = m["matched"].as_object_mut() {
            pair.remove("manual_state");
            for field in ["manual", "imported"] {
                pair[field].as_object_mut().unwrap().remove("check_number");
            }
        }
    }
    legacy["notes"]["transaction:boundary-14-0:check"] = "00123".into();
    let b: Budget = serde_json::from_value(legacy).unwrap();
    store::validate(&b).unwrap();
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bank/matching/statement.ofx");
    let preview = bank::preview(&b, &path, "bank-checking").unwrap();
    let pair = preview.budget().register_meta["boundary-14-0"]
        .matched
        .as_ref()
        .unwrap();
    assert_eq!(pair.imported.check_number.as_deref(), Some("123"));
    assert_eq!(pair.manual.date, "2026-02-14");
}

#[test]
fn identical_manual_row_is_matched_instead_of_mistaken_for_a_repeat() {
    for (extension, source) in [
        (
            "csv",
            "Date,Payee,Memo,Amount\n2026-04-15,Exact,Same,-22.22\n",
        ),
        (
            "qif",
            "!Type:Bank\nD04/15/2026\nT-22.22\nPExact\nMSame\n^\n",
        ),
        (
            "ofx",
            "<CURDEF>USD<STMTTRN><DTPOSTED>20260415<TRNAMT>-22.22<NAME>Exact<MEMO>Same</STMTTRN>",
        ),
    ] {
        let (mut b, _) = reference();
        let mut manual = b.transactions[1].clone();
        manual.id = "identical-manual".into();
        manual.date = "2026-04-15".into();
        manual.payee = "Exact".into();
        manual.memo = "Same".into();
        manual.amount = -2222;
        b.put_transaction(manual).unwrap();
        let path = std::env::temp_dir().join(format!("bank-identical-{}.{}", id(), extension));
        std::fs::write(&path, source).unwrap();
        let preview = bank::preview(&b, &path, "bank-checking").unwrap();
        assert_eq!(preview.count(), 1);
        assert_eq!(preview.matched_count(), 1);
        let mut next = preview.budget().clone();
        assert_eq!(
            next.balance("bank-checking", None, false),
            b.balance("bank-checking", None, false)
        );
        assert_eq!(
            bank::preview(&next, &path, "bank-checking")
                .unwrap()
                .count(),
            0
        );
        next.register_command(
            &HashSet::from(["identical-manual".into()]),
            Command::Unmatch,
        )
        .unwrap();
        assert_eq!(
            bank::preview(&next, &path, "bank-checking")
                .unwrap()
                .count(),
            0
        );
        std::fs::remove_file(path).unwrap();
    }
}
