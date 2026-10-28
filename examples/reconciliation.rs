//! Small, isolated statement-reconciliation fixture; refuses to overwrite a database.
use budgettinker::{model::*, store::Store};
fn main() -> Result<()> {
    let path = std::env::var("BUDGETTINKER_DATA")
        .map_err(|_| "Set BUDGETTINKER_DATA to a new isolated database")?;
    if std::path::Path::new(&path).exists() {
        return Err("The destination already exists.".into());
    }
    let mut b = Budget::new("USD");
    b.name = "Statement Reconciliation Lab".into();
    let account = b.add_account("Checking", "USD", true, 100_000, "2026-01-01")?;
    let savings = b.add_account("Savings", "USD", true, 0, "2026-01-01")?;
    b.reconcile_through(&account, 100_000, "2026-01-01")?;
    let category = b.categories[0].id.clone();
    for (date, payee, amount, cleared) in [
        ("2026-01-20", "Pending Coffee", -500, false),
        ("2026-01-15", "Internet", -3000, true),
        ("2026-02-01", "Later Payment", -2000, true),
    ] {
        b.put_transaction(Transaction {
            id: id(),
            account: account.clone(),
            date: date.into(),
            payee: payee.into(),
            memo: String::new(),
            amount,
            base_amount: 0,
            snapshot: None,
            income_month: None,
            category: Some(category.clone()),
            cleared,
            reconciled: false,
            transfer: None,
            splits: vec![],
            check_number: None,
            flag: None,
        })?;
    }
    b.transfer(
        &account,
        &savings,
        10_000,
        10_000,
        "2026-01-22",
        "Pending transfer",
    )?;
    Store::open(std::path::Path::new(&path))?.save(&b)?;
    println!("Statement date: 2026-01-31; cleared: $970.00; target after clearing coffee: $965.00");
    Ok(())
}
