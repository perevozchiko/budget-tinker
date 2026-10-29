use budgettinker::{
    bank,
    model::*,
    store::{self, Store},
};
use std::{fs, path::PathBuf};

fn file(extension: &str) -> PathBuf {
    std::env::temp_dir().join(format!("budgettinker-command-{}.{}", id(), extension))
}
fn budget() -> (Budget, String) {
    let mut b = Budget::new("USD");
    let account = b
        .add_account("Checking", "USD", true, 120_000, "2026-01-01")
        .unwrap();
    (b, account)
}
#[test]
fn ofx_import_handles_sgml_and_entities_and_bank_ids_prevent_duplicates() {
    let (b, a) = budget();
    let path = file("ofx");
    fs::write(&path, "OFXHEADER:100\n<OFX><CURDEF>USD\n<BANKTRANLIST><STMTTRN><DTPOSTED>20260102120000[-5:EST]\n<TRNAMT>-12.34\n<FITID>bank-123\n<NAME>Shop &amp; Caf&#233;\n<MEMO>Lunch\n</STMTTRN></BANKTRANLIST></OFX>").unwrap();
    let (next, count) = bank::import(&b, &path, &a).unwrap();
    assert_eq!(count, 1);
    assert_eq!(next.balance(&a, None, false), 118_766);
    assert_eq!(next.transactions.last().unwrap().payee, "Shop & Café");
    assert_eq!(bank::import(&next, &path, &a).unwrap().1, 0);
    assert_eq!(b.transactions.len(), 1);
    fs::remove_file(path).unwrap();
}
#[test]
fn qif_split_import_and_reimport_preserve_amounts_and_categories() {
    let (b, a) = budget();
    let path = file("qif");
    fs::write(&path, "!Type:Bank\nD01/02'26\nT-25.00\nPShop\nSFood:Groceries\n$-20.00\nSHome:Supplies\n$-5.00\n^\n").unwrap();
    let (next, count) = bank::import(&b, &path, &a).unwrap();
    assert_eq!(count, 1);
    let t = next.transactions.last().unwrap();
    assert_eq!(t.date, "2026-01-02");
    assert_eq!(t.splits.len(), 2);
    assert_eq!(t.splits.iter().map(|s| s.amount).sum::<i64>(), t.amount);
    assert_eq!(
        next.category_balance("2026-01", t.splits[0].category.as_ref().unwrap()),
        -2000
    );
    assert_eq!(bank::import(&next, &path, &a).unwrap().1, 0);
    fs::remove_file(path).unwrap();
}
#[test]
fn malformed_or_wrong_currency_statement_never_changes_budget() {
    let (b, a) = budget();
    let path = file("qfx");
    let before = serde_json::to_string(&b).unwrap();
    for source in [
        "<CURDEF>EUR<STMTTRN><DTPOSTED>20260102<TRNAMT>-1<FITID>1</STMTTRN>",
        "<CURDEF>USD<STMTTRN><DTPOSTED>20260102<TRNAMT>-1<FITID>1",
    ] {
        fs::write(&path, source).unwrap();
        assert!(bank::import(&b, &path, &a).is_err());
        assert_eq!(serde_json::to_string(&b).unwrap(), before);
    }
    fs::remove_file(path).unwrap();
}
#[test]
fn fresh_start_keeps_balances_and_structure_without_changing_original() {
    let (mut b, a) = budget();
    let c = b.categories[0].id.clone();
    b.allocate("2026-01", &c, 10_000).unwrap();
    b.put_transaction(Transaction {
        operation_fx: None,
        check_number: None,
        flag: None,
        id: id(),
        account: a.clone(),
        date: "2026-01-02".into(),
        payee: "Shop".into(),
        memo: "Old history".into(),
        amount: -2300,
        base_amount: 0,
        snapshot: None,
        income_month: None,
        category: Some(c),
        cleared: false,
        reconciled: false,
        transfer: None,
        splits: vec![],
    })
    .unwrap();
    let fresh = b.fresh_start().unwrap();
    assert_eq!(fresh.balance(&a, None, false), b.balance(&a, None, false));
    assert_eq!(fresh.categories.len(), b.categories.len());
    assert!(fresh.allocations.is_empty());
    assert_eq!(fresh.transactions.len(), 1);
    assert!(fresh.transactions[0].category.is_none());
    assert_eq!(b.transactions.len(), 2);
    assert!(!b.allocations.is_empty());
    store::validate(&fresh).unwrap();
}
#[test]
fn version_restoration_keeps_the_state_that_was_replaced() {
    let (mut b, _) = budget();
    let path = file("sqlite3");
    let mut store = Store::open(&path).unwrap();
    store.save(&b).unwrap();
    b.name = "Changed".into();
    store.save(&b).unwrap();
    let old = store.version(store.versions().unwrap()[0].0).unwrap();
    assert_ne!(old.name, b.name);
    store.save(&old).unwrap();
    assert_eq!(
        store.version(store.versions().unwrap()[0].0).unwrap().name,
        "Changed"
    );
    drop(store);
    fs::remove_file(path).unwrap();
}
#[test]
fn opening_unknown_or_missing_files_does_not_create_or_modify_them() {
    let missing = file("sqlite3");
    assert!(store::read_budget(&missing).is_err());
    assert!(!missing.exists());
    let foreign = file("sqlite3");
    fs::write(&foreign, b"This is not a budget").unwrap();
    assert!(store::read_budget(&foreign).is_err());
    assert_eq!(fs::read(&foreign).unwrap(), b"This is not a budget");
    fs::remove_file(foreign).unwrap();
}
#[test]
fn exports_include_real_amounts_and_qif_escapes_newlines() {
    let (mut b, a) = budget();
    b.accounts[0].name = "My\nChecking".into();
    let qif = file("qif");
    let csv = file("csv");
    store::export_qif(&b, &qif).unwrap();
    store::export_budget_csv(&b, &csv).unwrap();
    let output = fs::read_to_string(&qif).unwrap();
    assert!(output.contains("NMy Checking\n"));
    assert!(output.contains("T1200.00\n"));
    assert!(
        fs::read_to_string(&csv)
            .unwrap()
            .starts_with("Month,Master Category,Sub Category,Budgeted,Outflows,Balance,Currency")
    );
    let mut empty = b.clone();
    empty.transactions.clear();
    let (roundtrip, n) = bank::import(&empty, &qif, &a).unwrap();
    assert_eq!(n, 1);
    assert_eq!(roundtrip.balance(&a, None, false), 120_000);
    fs::remove_file(qif).unwrap();
    fs::remove_file(csv).unwrap();
}

#[test]
fn bank_import_applies_saved_payee_rename_and_category_rules() {
    let (mut b, a) = budget();
    let cat = b.categories[0].id.clone();
    b.payee_rules.insert(
        "Coffee Shop".into(),
        PayeeRule {
            listed: true,
            category: Some(cat.clone()),
            rename_contains: "COFFEE-123".into(),
        },
    );
    let path = file("qif");
    fs::write(
        &path,
        "!Type:Bank\nD01/02/2026\nT-3.50\nPCARD COFFEE-123 BELGRADE\n^\n",
    )
    .unwrap();
    let (next, n) = bank::import(&b, &path, &a).unwrap();
    assert_eq!(n, 1);
    let transaction = next.transactions.last().unwrap();
    assert_eq!(transaction.payee, "Coffee Shop");
    assert_eq!(transaction.category.as_ref(), Some(&cat));
    assert_eq!(bank::import(&next, &path, &a).unwrap().1, 0);
    fs::remove_file(path).unwrap();
}

#[test]
fn older_budgets_load_without_new_desktop_metadata() {
    let (b, _) = budget();
    let mut json = serde_json::to_value(&b).unwrap();
    for key in ["payee_rules", "notes"] {
        json.as_object_mut().unwrap().remove(key);
    }
    for account in json["accounts"].as_array_mut().unwrap() {
        account.as_object_mut().unwrap().remove("account_type");
    }
    for category in json["categories"].as_array_mut().unwrap() {
        category.as_object_mut().unwrap().remove("hidden");
    }
    let restored: Budget = serde_json::from_value(json).unwrap();
    store::validate(&restored).unwrap();
    assert_eq!(restored.accounts[0].account_type, "checking");
    assert!(!restored.categories[0].hidden);
    assert!(restored.payee_rules.is_empty());
    assert!(restored.notes.is_empty());
}

#[test]
fn branded_budget_extension_preserves_legacy_sqlite_document_and_versions() {
    let root = std::env::temp_dir().join(format!("budgettinker-extension-{}", id()));
    fs::create_dir_all(&root).unwrap();
    let legacy = root.join("old.sqlite3");
    let branded = root.join("budget.budgettinker");
    let (original, _) = budget();
    {
        let mut db = Store::open(&legacy).unwrap();
        db.save(&original).unwrap();
    }
    let original_bytes = fs::read(&legacy).unwrap();
    fs::copy(&legacy, &branded).unwrap();
    {
        let mut db = Store::open(&branded).unwrap();
        let loaded = db.load().unwrap().unwrap();
        assert_eq!(
            serde_json::to_value(&loaded).unwrap(),
            serde_json::to_value(&original).unwrap()
        );
        let mut edited = loaded.clone();
        edited.name = "Renamed budget".into();
        db.save(&edited).unwrap();
        assert_eq!(db.load().unwrap().unwrap().name, "Renamed budget");
        db.save(&loaded).unwrap();
        assert_eq!(
            serde_json::to_value(db.load().unwrap().unwrap()).unwrap(),
            serde_json::to_value(&original).unwrap()
        );
    }
    assert_eq!(fs::read(&legacy).unwrap(), original_bytes);
    assert_eq!(&fs::read(&branded).unwrap()[..16], b"SQLite format 3\0");
    fs::remove_dir_all(root).unwrap();
}
