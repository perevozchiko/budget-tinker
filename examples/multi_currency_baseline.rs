//! Repeatable synthetic workload; never opens the default or an existing budget.
//! cargo run --release --locked --example multi_currency_baseline
use budgettinker::{
    model::*,
    reports,
    store::{self, Store},
};
use std::{hint::black_box, time::Instant};

fn measure<T>(name: &str, operation: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let result = operation();
    println!("{name}: {:.3} ms", start.elapsed().as_secs_f64() * 1000.0);
    black_box(result)
}

fn main() {
    let mut budget = Budget::new("EUR");
    budget.name = "Synthetic multi-currency performance lab".into();
    let months: Vec<_> = (0..240).map(|n| month_shift("2006-01", n)).collect();
    for month in &months {
        for currency in CURRENCIES.iter().filter(|c| **c != budget.base) {
            // Fictitious exact rates; variation exercises monthly valuation.
            budget.snapshots.push(Snapshot {
                id: format!("rate-{month}-{currency}"),
                month: month.clone(),
                currency: (*currency).into(),
                rate: 90_000_000,
                rate_date: format!("{month}-01"),
                captured_at: "synthetic".into(),
                source: "Synthetic benchmark".into(),
            });
        }
    }
    for n in 0..100 {
        budget
            .add_account(
                &format!("Synthetic account {n}"),
                CURRENCIES[n % CURRENCIES.len()],
                n % 3 != 0,
                0,
                "2006-01-01",
            )
            .unwrap();
    }
    for n in 0..100_000 {
        let account = &budget.accounts[n % 100];
        let month = &months[(n / 100) % months.len()];
        let amount = if n % 7 == 0 { 100_001 } else { -101 };
        let (base_amount, snapshot) = budget.to_base(amount, &account.currency, month).unwrap();
        budget.transactions.push(Transaction {
            id: format!("synthetic-{n}"),
            account: account.id.clone(),
            date: format!("{month}-15"),
            payee: format!("Synthetic payee {}", n % 200),
            memo: String::new(),
            amount,
            base_amount,
            snapshot,
            category: Some(budget.categories[n % budget.categories.len()].id.clone()),
            income_month: None,
            cleared: n % 2 == 0,
            reconciled: false,
            transfer: None,
            splits: vec![],
            operation_fx: None,
            check_number: None,
            flag: None,
        });
    }
    println!(
        "accounts=100 transactions=100000 months=240 currencies={} custom_account_groups=unsupported",
        CURRENCIES.len()
    );
    measure("validate", || store::validate(&budget).unwrap());
    let root = std::env::temp_dir().join(format!("budgettinker-benchmark-{}", id()));
    std::fs::create_dir(&root).unwrap();
    let mut db = Store::open(&root.join("synthetic.budgettinker")).unwrap();
    measure("sqlite_save", || db.save(&budget).unwrap());
    let loaded = measure("sqlite_load", || db.load().unwrap().unwrap());
    assert_eq!(loaded.transactions.len(), 100_000);
    measure("month_summary", || budget.month_summary("2025-12").unwrap());
    let account = &budget.accounts[0].id;
    measure("register_filter", || {
        budget
            .transactions
            .iter()
            .filter(|t| &t.account == account && t.payee.contains("Synthetic"))
            .count()
    });
    measure("net_worth", || budget.net_worth("2025-12", false).unwrap());
    let filter = reports::Filter {
        from: "2006-01-01".into(),
        through: "2025-12-31".into(),
        ..Default::default()
    };
    for kind in 0..5 {
        measure(&format!("report_{kind}"), || {
            reports::calculate(&budget, kind, &filter).unwrap()
        });
    }
    let path = root.join("synthetic.json");
    measure("json_export", || {
        store::export_json(&budget, &path).unwrap()
    });
    let restored = measure("json_import", || store::import_json(&path).unwrap());
    assert_eq!(
        serde_json::to_value(&budget).unwrap(),
        serde_json::to_value(&restored).unwrap()
    );
    // Matches the existing whole-document Undo representation; not a GUI timing.
    let prior = measure("undo_clone", || budget.clone());
    let mut changed = prior.clone();
    changed
        .allocate("2025-12", &budget.categories[0].id, 123)
        .unwrap();
    measure("save_with_revision", || db.save(&changed).unwrap());
    measure("undo_save", || db.save(&prior).unwrap());
    measure("redo_save", || db.save(&changed).unwrap());
    // Existing snapshots cannot be corrected. Measure candidate valuation only.
    measure("rate_correction_preview_surrogate", || {
        let mut candidate = budget.clone();
        candidate
            .snapshots
            .iter_mut()
            .find(|s| s.month == "2025-12" && s.currency == "RSD")
            .unwrap()
            .rate = 95_000_000;
        candidate.net_worth("2025-12", false).unwrap()
    });
    drop(db);
    std::fs::remove_dir_all(root).unwrap();
}
