//! Synthetic register with cleared/reconciled/flagged rows; refuses to overwrite.
use budgettinker::{model::*, store::Store};
fn main() {
    let path = std::path::PathBuf::from(
        std::env::var("BUDGETTINKER_DATA").expect("Set a new isolated BUDGETTINKER_DATA path"),
    );
    assert!(!path.exists(), "Preview database already exists");
    let mut b = Budget::new("EUR");
    b.name = "Register Preview (Synthetic)".into();
    let date = format!("{}-01", current_month());
    let a = b
        .add_account("Checking", "EUR", true, 100_000, &date)
        .unwrap();
    b.transactions[0].payee = "Starting Balance".into();
    b.transactions[0].reconciled = true;
    for (payee, amount, cleared, category, flag) in [
        (
            "Groceries",
            -5000,
            false,
            Some(b.categories[0].id.clone()),
            None,
        ),
        ("Refund", 10_000, true, None, Some(TransactionFlag::Blue)),
    ] {
        b.put_transaction(Transaction {
            id: id(),
            account: a.clone(),
            date: date.clone(),
            payee: payee.into(),
            memo: "Synthetic register check".into(),
            amount,
            base_amount: 0,
            snapshot: None,
            category,
            income_month: None,
            check_number: None,
            flag,
            cleared,
            reconciled: false,
            transfer: None,
            splits: vec![],
        })
        .unwrap();
    }
    let mut store = Store::open(&path).unwrap();
    store.save(&b).unwrap();
    println!(
        "{}: working {}, cleared {}",
        path.display(),
        b.balance(&a, None, false),
        b.balance(&a, None, true)
    );
}
