use budgettinker::{bank, model::*, register::Command, store};
use std::{collections::HashSet, fs, path::Path};

fn fixture() -> (Budget, String, String) {
    let mut b = Budget::new("USD");
    let account = b
        .add_account("Checking", "USD", true, 120000, "2026-01-01")
        .unwrap();
    let category = b.categories[0].id.clone();
    b.payee_rules.insert(
        "Coffee Shop".into(),
        PayeeRule {
            listed: true,
            category: Some(category.clone()),
            rename_contains: "COFFEE-123".into(),
        },
    );
    (b, account, category)
}
fn path(extension: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/bank/statement.{extension}"))
}
fn json(b: &Budget) -> serde_json::Value {
    serde_json::to_value(b).unwrap()
}

#[test]
fn same_statements_preview_rules_match_reimport_approve_unmatch_and_reject() {
    for extension in ["csv", "qif", "ofx"] {
        let (mut b, account, category) = fixture();
        b.put_transaction(Transaction {
            id: "manual".into(),
            account: account.clone(),
            date: "2026-01-02".into(),
            payee: "My coffee".into(),
            memo: "My note".into(),
            amount: -350,
            base_amount: 0,
            snapshot: None,
            category: Some(category.clone()),
            income_month: None,
            cleared: false,
            reconciled: false,
            transfer: None,
            splits: vec![],
            check_number: None,
            flag: None,
        })
        .unwrap();
        let before = json(&b);
        let source = path(extension);
        let preview = bank::preview(&b, &source, &account).unwrap();
        assert_eq!(json(&b), before);
        assert_eq!(preview.count(), 4);
        assert_eq!(preview.matched_count(), 1);
        let rows = preview.transactions();
        assert_eq!(
            rows.iter().map(|t| t.amount).collect::<Vec<_>>(),
            [-350, -350, 100, 20000]
        );
        assert_eq!(rows[0].payee, "My coffee");
        let bank_original = &preview.budget().register_meta["manual"]
            .matched
            .as_ref()
            .unwrap()
            .imported;
        assert_eq!(bank_original.payee, "Coffee Shop");
        assert_eq!(rows[0].category.as_ref(), Some(&category));
        assert_eq!(rows[0].memo, "My note");
        assert_eq!(bank_original.memo, "Lunch");
        assert!(
            rows.iter()
                .all(|t| t.cleared && !t.reconciled && preview.budget().pending_import(&t.id))
        );
        let imported = bank_original.id.clone();
        let (mut next, count) = preview.apply(&b, &source, &account).unwrap();
        assert_eq!(count, 4);
        assert_eq!(next.balance(&account, None, false), 139400);
        next.register_command(&HashSet::from(["manual".into()]), Command::Unmatch)
            .unwrap();
        assert_eq!(next.balance(&account, None, false), 139050);
        assert_eq!(bank::preview(&next, &source, &account).unwrap().count(), 0);
        let full = json(&next);
        next.register_command(
            &HashSet::from(["manual".into(), imported.clone()]),
            Command::Match,
        )
        .unwrap();
        assert_eq!(next.balance(&account, None, false), 139400);
        assert_eq!(
            next.transactions
                .iter()
                .find(|t| t.id == "manual")
                .unwrap()
                .memo,
            "My note"
        );
        assert_eq!(bank::import(&next, &source, &account).unwrap().1, 0);
        let selection = HashSet::from(["manual".into()]);
        next.register_command(&selection, Command::Approve).unwrap();
        assert!(!next.pending_import("manual"));
        assert_eq!(bank::import(&next, &source, &account).unwrap().1, 0);
        // Persist the match backups, then recover the original bank and manual rows.
        let db = std::env::temp_dir().join(format!("bank-lifecycle-{}.sqlite3", id()));
        let mut store = store::Store::open(&db).unwrap();
        store.save(&next).unwrap();
        next = store.load().unwrap().unwrap();
        next.register_command(&selection, Command::Unmatch).unwrap();
        let mut actual = json(&next);
        let mut expected = full;
        for value in [&mut actual, &mut expected] {
            value["transactions"]
                .as_array_mut()
                .unwrap()
                .sort_by_key(|t| t["id"].as_str().unwrap().to_owned());
        }
        assert_eq!(actual, expected);
        assert_eq!(bank::import(&next, &source, &account).unwrap().1, 0);
        next.register_command(&HashSet::from([imported]), Command::Reject)
            .unwrap();
        assert_eq!(bank::import(&next, &source, &account).unwrap().1, 1);
        store::validate(&next).unwrap();
        drop(store);
        fs::remove_file(db).unwrap();
    }
}

#[test]
fn preview_rejects_changed_file_account_path_and_budget_without_mutation() {
    let (mut b, account, _) = fixture();
    let source = std::env::temp_dir().join(format!("bank-preview-{}.csv", id()));
    let bytes = fs::read(path("csv")).unwrap();
    fs::write(&source, &bytes).unwrap();
    let preview = bank::preview(&b, &source, &account).unwrap();
    assert!(preview.apply(&b, &source, "other-account").is_err());
    assert!(preview.apply(&b, &path("csv"), &account).is_err());
    b.name = "Changed".into();
    let before = json(&b);
    assert!(preview.apply(&b, &source, &account).is_err());
    assert_eq!(json(&b), before);
    let preview = bank::preview(&b, &source, &account).unwrap();
    fs::write(&source, [bytes.as_slice(), b"\n"].concat()).unwrap();
    assert!(preview.apply(&b, &source, &account).is_err());
    assert_eq!(json(&b), before);
    fs::write(&source, &bytes).unwrap();
    assert_eq!(preview.apply(&b, &source, &account).unwrap().1, 4);
    fs::remove_file(source).unwrap();
}
