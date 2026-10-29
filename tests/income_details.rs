use budgettinker::model::*;

fn income(
    b: &mut Budget,
    account: &str,
    date: &str,
    value: i64,
    assigned: Option<&str>,
) -> Transaction {
    let mut t = b.transactions[0].clone();
    t.id = id();
    t.account = account.into();
    t.date = date.into();
    t.amount = value;
    t.category = None;
    t.income_month = assigned.map(str::to_owned);
    t.splits.clear();
    t.transfer = None;
    t
}
#[test]
fn income_details_include_deferred_and_split_income_and_exclude_spending_and_internal_transfers() {
    let mut b = Budget::new("USD");
    let cash = b
        .add_account("Cash", "USD", true, 10_000, "2026-01-01")
        .unwrap();
    let other = b
        .add_account("Other", "USD", true, 0, "2026-01-01")
        .unwrap();
    let outside = b
        .add_account("Outside", "USD", false, 0, "2026-01-01")
        .unwrap();
    for (account, date, value, assigned) in [
        (&cash, "2026-01-20", 2000, Some("2026-02")),
        (&cash, "2026-02-02", 3000, None),
        (&cash, "2026-02-03", 1000, Some("2026-03")),
        (&outside, "2026-02-02", 99999, None),
    ] {
        let t = income(&mut b, account, date, value, assigned);
        b.put_transaction(t).unwrap();
    }
    let mut split = income(&mut b, &cash, "2026-02-04", 450, None);
    split.splits = vec![
        Split {
            amount: 500,
            income_month: Some("2026-02".into()),
            ..Default::default()
        },
        Split {
            amount: -50,
            category: Some(b.categories[0].id.clone()),
            ..Default::default()
        },
    ];
    b.put_transaction(split).unwrap();
    b.transfer(&cash, &other, 700, 700, "2026-02-05", "Internal")
        .unwrap();
    let rows = b.income_rows("2026-02").unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows.iter().map(|(_, value)| value).sum::<i64>(), 5500);
    assert_eq!(b.month_summary("2026-02").unwrap().income, 5500);
    assert_eq!(rows[0].0.date, "2026-01-20");
    assert_eq!(b.income_rows("2026-03").unwrap()[0].1, 1000);
    assert!(b.income_rows("bad").is_err());
}
#[test]
fn income_details_use_frozen_rate_and_expose_revaluation_separately() {
    let mut b = Budget::new("USD");
    let cash = b.add_account("Cash", "USD", true, 1, "2026-01-01").unwrap();
    let euro = b.add_account("Euro", "EUR", true, 0, "2026-01-01").unwrap();
    b.add_snapshot("2026-01", "EUR", 150_000_000, "2026-01-01", "test")
        .unwrap();
    b.add_snapshot("2026-02", "EUR", 200_000_000, "2026-02-01", "test")
        .unwrap();
    let t = income(&mut b, &euro, "2026-01-20", 1000, Some("2026-02"));
    b.put_transaction(t).unwrap();
    let rows = b.income_rows("2026-02").unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, 1500);
    assert_eq!(b.month_summary("2026-02").unwrap().income, 2000);
    let _ = cash;
}
