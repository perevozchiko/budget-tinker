//! Isolated, reproducible rollover preview. Refuses to replace an existing database.
use budgettinker::{model::*, store::Store};
fn main() {
    let path = std::path::PathBuf::from(
        std::env::var("BUDGETTINKER_DATA")
            .expect("Set BUDGETTINKER_DATA to a new isolated preview database"),
    );
    assert!(!path.exists(), "Preview database already exists");
    let mut b = Budget::new("EUR");
    b.name = "Overspending Preview (Synthetic)".into();
    b.categories.truncate(2);
    b.categories[0].name = "Reduce available".into();
    b.categories[1].name = "Carry in category".into();
    for c in &mut b.categories {
        c.group = "Rollover examples".into();
    }
    let month = current_month();
    let date = format!("{month}-01");
    let a = b
        .add_account("Synthetic Cash", "EUR", true, 100_000, &date)
        .unwrap();
    for c in b.categories.clone() {
        b.allocate(&month, &c.id, 10_000).unwrap();
        b.put_transaction(Transaction {
            check_number: None,
            flag: None,
            id: id(),
            account: a.clone(),
            category: Some(c.id),
            date: date.clone(),
            payee: "Synthetic purchase".into(),
            amount: -13_000,
            base_amount: 0,
            snapshot: None,
            income_month: None,
            memo: "For GUI verification only".into(),
            cleared: true,
            reconciled: false,
            transfer: None,
            splits: vec![],
        })
        .unwrap();
    }
    b.set_overspending_rule(
        &month,
        &b.categories[1].id.clone(),
        OverspendingRule::CategoryBalance,
    )
    .unwrap();
    let mut store = Store::open(&path).unwrap();
    store.save(&b).unwrap();
    for delta in 0..3 {
        println!(
            "{}: {:?}",
            month_shift(&month, delta),
            b.month_summary(&month_shift(&month, delta)).unwrap()
        );
    }
    println!("Synthetic preview: {}", path.display());
}
