use budgettinker::{model::*, store::*};
use std::path::PathBuf;

fn fixture() -> (Budget, String, String) {
    let mut b = Budget::new("EUR");
    b.add_snapshot("2026-01", "USD", 90_000_000, "2026-01-30", "test")
        .unwrap();
    b.add_snapshot("2026-02", "USD", 95_000_000, "2026-02-27", "test")
        .unwrap();
    let eur = b
        .add_account("Euro", "EUR", true, 100_000, "2026-01-01")
        .unwrap();
    let usd = b
        .add_account("Dollar", "USD", true, 100_000, "2026-01-01")
        .unwrap();
    (b, eur, usd)
}
fn expense(account: &str, category: &str, value: i64, date: &str) -> Transaction {
    Transaction {
        operation_fx: None,
        check_number: None,
        flag: None,
        id: id(),
        account: account.into(),
        date: date.into(),
        payee: "Shop".into(),
        memo: String::new(),
        amount: value,
        base_amount: 0,
        snapshot: None,
        income_month: None,
        category: Some(category.into()),
        cleared: false,
        reconciled: false,
        transfer: None,
        splits: vec![],
    }
}
fn temp(extension: &str) -> PathBuf {
    std::env::temp_dir().join(format!("budgettinker-test-{}.{}", id(), extension))
}

#[path = "accounting/bank_files.rs"]
mod bank_files;
#[path = "accounting/categories.rs"]
mod categories;
#[path = "accounting/money.rs"]
mod money;
#[path = "accounting/monthly_fx.rs"]
mod monthly_fx;
#[path = "accounting/persistence.rs"]
mod persistence;
#[path = "accounting/reconciliation.rs"]
mod reconciliation;
#[path = "accounting/schedules.rs"]
mod schedules;
#[path = "accounting/splits.rs"]
mod splits;
#[path = "accounting/transfers.rs"]
mod transfers;
#[path = "accounting/upgrade.rs"]
mod upgrade;
