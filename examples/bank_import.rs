//! Synthetic USD account for tests/fixtures/bank/statement.{csv,qif,ofx}.
use budgettinker::{model::*, store::Store};

fn main() {
    let path = std::env::var("BUDGETTINKER_DATA")
        .expect("Set BUDGETTINKER_DATA to a new isolated database");
    let path = std::path::Path::new(&path);
    assert!(!path.exists(), "Use a new destination");
    let mut b = Budget::new("USD");
    b.name = "Bank Import Lab".into();
    let account = b
        .add_account("Checking", "USD", true, 120000, "2026-01-01")
        .unwrap();
    b.add_category_group("Everyday Life").unwrap();
    let category = b.add_category("Everyday Life", "Coffee").unwrap();
    b.payee_rules.insert(
        "Coffee Shop".into(),
        PayeeRule {
            listed: true,
            category: Some(category.clone()),
            rename_contains: "COFFEE-123".into(),
        },
    );
    b.put_transaction(Transaction {
        id: "manual-coffee".into(),
        account,
        date: "2026-01-02".into(),
        payee: "My coffee".into(),
        memo: "My note".into(),
        amount: -350,
        base_amount: 0,
        snapshot: None,
        category: Some(category),
        income_month: None,
        cleared: false,
        reconciled: false,
        transfer: None,
        splits: vec![],
        operation_fx: None,
        check_number: None,
        flag: None,
    })
    .unwrap();
    Store::open(path).unwrap().save(&b).unwrap();
    println!("Synthetic bank import budget: {}", path.display());
}
