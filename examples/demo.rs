//! Synthetic data for visual inspection; never writes to the default budget.
use budgettinker::{model::*, store::Store};
fn main() {
    let path = std::env::var("BUDGETTINKER_DATA")
        .expect("Set BUDGETTINKER_DATA to an isolated preview database");
    let mut b = Budget::new("EUR");
    b.name = "My Budget (Demo)".into();
    let month = current_month();
    let day = format!("{month}-01");
    let bank = b
        .add_account("Checking", "EUR", true, 320_000, &day)
        .unwrap();
    let cash = b.add_account("Cash", "EUR", true, 42_600, &day).unwrap();
    let savings = b
        .add_account("Savings", "EUR", true, 150_000, &day)
        .unwrap();
    for (name, allocated, spent, payee, account) in [
        ("Аренда / ипотека", 90_000, -85_000, "Rent", &bank),
        ("Продукты", 40_000, -7114, "Supermarket", &cash),
        ("Кафе и рестораны", 15_000, -4200, "Coffee Shop", &bank),
        ("Транспорт", 8000, -1448, "Transit", &cash),
        ("Интернет", 3500, -2990, "Internet provider", &bank),
        ("Резервный фонд", 70_000, 0, "", &bank),
        ("Отпуск", 40_000, 0, "", &bank),
    ] {
        let c = b
            .categories
            .iter()
            .find(|c| c.name == name)
            .unwrap()
            .id
            .clone();
        b.allocate(&month, &c, allocated).unwrap();
        if spent != 0 {
            b.put_transaction(Transaction {
                operation_fx: None,
                check_number: None,
                flag: None,
                id: id(),
                account: account.clone(),
                date: day.clone(),
                payee: payee.into(),
                memo: "Demo transaction".into(),
                amount: spent,
                base_amount: 0,
                snapshot: None,
                income_month: None,
                category: Some(c),
                cleared: true,
                reconciled: false,
                transfer: None,
                splits: vec![],
            })
            .unwrap();
        }
    }
    b.transfer(&savings, &bank, 10_000, 10_000, &day, "Savings transfer")
        .unwrap();
    let c = b
        .categories
        .iter()
        .find(|c| c.name == "Интернет")
        .unwrap()
        .id
        .clone();
    b.scheduled.push(Scheduled {
        id: id(),
        next_date: format!("{}-01", month_shift(&month, 1)),
        monthly: true,
        repeat: None,
        anchor_day: None,
        transaction: Transaction {
            operation_fx: None,
            check_number: None,
            flag: None,
            id: id(),
            account: bank,
            date: day,
            payee: "Internet provider".into(),
            memo: "Monthly bill".into(),
            amount: -2990,
            base_amount: 0,
            snapshot: None,
            income_month: None,
            category: Some(c),
            cleared: false,
            reconciled: false,
            transfer: None,
            splits: vec![],
        },
    });
    b.add_account("Brokerage", "EUR", false, 1_222_200, &today())
        .unwrap();
    let closed = b
        .add_account("Closed account", "EUR", true, 0, &today())
        .unwrap();
    b.accounts
        .iter_mut()
        .find(|a| a.id == closed)
        .unwrap()
        .closed = true;
    let mut store = Store::open(std::path::Path::new(&path)).unwrap();
    store.save(&b).unwrap();
    println!("Synthetic preview budget: {path}");
}
