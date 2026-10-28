use budgettinker::{
    model::*,
    reports::{self, Filter},
};

fn filter() -> Filter {
    Filter {
        from: "2026-01-10".into(),
        through: "2026-02-15".into(),
        ..Filter::default()
    }
}

fn transaction(
    account: &str,
    category: Option<&str>,
    amount: i64,
    date: &str,
    payee: &str,
) -> Transaction {
    Transaction {
        check_number: None,
        flag: None,
        id: id(),
        account: account.into(),
        date: date.into(),
        payee: payee.into(),
        memo: String::new(),
        amount,
        base_amount: 0,
        snapshot: None,
        income_month: None,
        category: category.map(str::to_owned),
        cleared: false,
        reconciled: false,
        transfer: None,
        splits: vec![],
    }
}

#[path = "reports/exports.rs"]
mod exports;
#[path = "reports/filters.rs"]
mod filters;
#[path = "reports/matrix.rs"]
mod matrix;
#[path = "reports/net_worth.rs"]
mod net_worth;
#[path = "reports/spending.rs"]
mod spending;

fn matrix_row<'a>(report: &'a reports::Report, key: &str) -> &'a reports::MatrixRow {
    report.matrix.iter().find(|row| row.key == key).unwrap()
}
