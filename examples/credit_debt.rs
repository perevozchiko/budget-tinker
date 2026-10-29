//! Synthetic legacy old-debt lab. Never overwrites an existing database.
use budgettinker::{model::*, store::Store};
fn main() -> Result<()> {
    let path = std::env::var("BUDGETTINKER_DATA")
        .map_err(|_| "Set BUDGETTINKER_DATA to a new isolated database")?;
    if std::path::Path::new(&path).exists() {
        return Err("The destination already exists.".into());
    }
    let mut b = Budget::new("USD");
    b.name = "BudgetTinker Credit Debt Lab".into();
    let groceries = b.categories[0].id.clone();
    let interest = b.categories[1].id.clone();
    b.categories[0].name = "Groceries".into();
    b.categories[1].name = "Card Interest".into();
    let cash = b.add_account("Checking", "USD", true, 200_000, "2026-10-01")?;
    let card = b.add_typed_account("Visa", "USD", true, -100_000, "2026-10-01", "credit")?;
    let debt = b.initial_debt_category(&card).unwrap().id.clone();
    for (category, value) in [(&debt, 20_000), (&groceries, 10_000), (&interest, 2000)] {
        b.allocate("2026-10", category, value)?;
    }
    for (date, payee, category, value) in [
        ("2026-10-05", "Grocery Shop", &groceries, -10_000),
        ("2026-10-06", "Card Interest", &interest, -2000),
        ("2026-10-15", "Grocery Refund", &groceries, 4000),
    ] {
        b.put_transaction(Transaction {
            id: id(),
            account: card.clone(),
            date: date.into(),
            payee: payee.into(),
            memo: String::new(),
            amount: value,
            base_amount: 0,
            snapshot: None,
            income_month: None,
            category: Some(category.clone()),
            cleared: false,
            reconciled: false,
            transfer: None,
            splits: vec![],
            operation_fx: None,
            check_number: None,
            flag: None,
        })?;
    }
    b.transfer(&cash, &card, 32_000, 32_000, "2026-10-10", "Card payment")?;
    Store::open(std::path::Path::new(&path))?.save(&b)?;
    println!(
        "Checking $1,680; Visa -$760; old debt -$800; Groceries $40; Available to Budget $1,680."
    );
    Ok(())
}
