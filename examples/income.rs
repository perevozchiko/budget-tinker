//! Isolated income-month preview. Never replaces an existing database.
use budgettinker::{model::*, store::Store};
fn main() {
    let path = std::path::PathBuf::from(
        std::env::var("BUDGETTINKER_DATA")
            .expect("Set BUDGETTINKER_DATA to a new isolated preview database"),
    );
    assert!(!path.exists(), "Preview database already exists");
    let mut b = Budget::new("EUR");
    b.name = "Income Months Preview (Synthetic)".into();
    b.categories.truncate(2);
    let month = current_month();
    let next = month_shift(&month, 1);
    let date = format!("{month}-01");
    let a = b
        .add_account("Synthetic Checking", "EUR", true, 0, &date)
        .unwrap();
    for (payee, value, assigned, category) in [
        ("Salary", 300_000, Some(next), None),
        ("Side project", 50_000, Some(month.clone()), None),
        ("Groceries", -10_000, None, Some(b.categories[0].id.clone())),
    ] {
        b.put_transaction(Transaction {
            check_number: None,
            flag: None,
            id: id(),
            account: a.clone(),
            date: date.clone(),
            payee: payee.into(),
            memo: "Synthetic GUI verification".into(),
            amount: value,
            base_amount: 0,
            snapshot: None,
            category,
            income_month: assigned,
            cleared: true,
            reconciled: false,
            transfer: None,
            splits: vec![],
        })
        .unwrap();
    }
    b.allocate(&month, &b.categories[0].id.clone(), 30_000)
        .unwrap();
    let mut store = Store::open(&path).unwrap();
    store.save(&b).unwrap();
    for delta in 0..3 {
        let month = month_shift(&month, delta);
        println!("{month}: {:?}", b.month_summary(&month).unwrap());
    }
    println!("Synthetic preview: {}", path.display());
}
