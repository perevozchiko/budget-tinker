//! Validate and load the shared synthetic fixture into a NEW persistent demo database.
//! Usage: cargo run --locked --example comparison -- /absolute/path/to/demo
use budgettinker::{
    model::*,
    reports::{self, Filter, Selection},
    store::{self, Store},
};
use std::{fs, path::PathBuf};

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("Pass fixture directory"));
    let path = root.join("budgettinker.budgettinker");
    assert!(
        !path.exists(),
        "Demo database already exists; refusing overwrite"
    );
    let budget = store::import_json(&root.join("budget.json")).expect("valid synthetic fixture");
    assert!(
        budget.name.starts_with("Four Months Lab"),
        "Only lab fixtures are allowed"
    );
    assert_eq!(budget.scheduled.len(), 9);
    let from = budget
        .transactions
        .iter()
        .map(|t| &t.date)
        .min()
        .unwrap()
        .clone();
    let through = budget
        .transactions
        .iter()
        .map(|t| &t.date)
        .max()
        .unwrap()
        .clone();
    let filter = Filter {
        from,
        through,
        category: Selection::All,
        payee: Selection::All,
        accounts: Selection::BudgetAccounts,
    };
    let mut reports_json = vec![];
    for kind in 0..5 {
        let mut scoped = filter.clone();
        if kind == 4 {
            scoped.accounts = Selection::All;
        }
        let report = reports::calculate(&budget, kind, &scoped).expect("report calculation");
        fs::write(
            root.join(format!("report-{kind}.csv")),
            report.csv(&budget.base, str::to_owned).unwrap(),
        )
        .unwrap();
        let chart_rows = report.spending_rows(None, None);
        let mut detail_rows = vec![];
        for (index, row) in chart_rows.iter().enumerate() {
            if kind < 2 {
                fs::write(
                    root.join(format!("report-{kind}-detail-{index}.csv")),
                    report.spending_csv(&row.scope, None, &budget.base).unwrap(),
                )
                .unwrap();
                detail_rows.push(serde_json::json!({
                    "label": row.label, "value": row.value,
                    "children": report.spending_rows(Some(&row.scope), None).iter().map(|c| serde_json::json!({"label": c.label, "value": c.value})).collect::<Vec<_>>()
                }));
            }
        }
        reports_json.push(serde_json::json!({
            "spending_chart": detail_rows,
            "title": reports::TITLES[kind], "total_cents": report.total, "rows": report.rows,
            "months": report.months.iter().map(|m| serde_json::json!({
                "month": m.month, "income_cents": m.income, "expense_cents": m.expense,
                "assets_cents": m.assets, "debts_cents": m.debts,
                "net_worth_cents": m.net_worth
            })).collect::<Vec<_>>()
        }));
    }
    let accounts = budget
        .accounts
        .iter()
        .map(|a| {
            serde_json::json!({
                "name": a.name, "on_budget": a.on_budget,
                "balance_cents": budget.balance(&a.id, None, false),
                "cleared_cents": budget.balance(&a.id, None, true),
            })
        })
        .collect::<Vec<_>>();
    let mut months = vec![];
    let mut month = filter.from[..7].to_owned();
    while month.as_str() <= &filter.through[..7] {
        let summary = budget.month_summary(&month).unwrap();
        assert_eq!(
            summary.available, 0,
            "Every received dollar must have a job in {month}"
        );
        months.push(serde_json::json!({
            "month": month, "income_cents": summary.income, "budgeted_cents": summary.budgeted,
            "overspent_previous_cents": summary.overspent, "available_cents": summary.available,
            "dining_balance_cents": budget.category_balance(&month, &budget.categories.iter().find(|c| c.name == "Dining Out").unwrap().id),
        }));
        month = month_shift(&month, 1);
    }
    let expected =
        serde_json::json!({"accounts": accounts, "budget_months": months, "reports": reports_json});
    fs::write(
        root.join("expected.json"),
        serde_json::to_vec_pretty(&expected).unwrap(),
    )
    .unwrap();
    let mut store = Store::open(&path).unwrap();
    store.save(&budget).unwrap();
    assert_eq!(
        store.load().unwrap().unwrap().transactions.len(),
        budget.transactions.len()
    );
    println!("{}", serde_json::to_string_pretty(&expected).unwrap());
    println!("Demo database: {}", path.display());
}
