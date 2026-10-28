use super::*;

#[test]
fn income_matrix_preserves_categories_refunds_sources_and_transaction_month() {
    let mut b = Budget::new("EUR");
    let a = b
        .add_account("Checking", "EUR", true, 0, "2026-01-01")
        .unwrap();
    let other = b
        .add_account("Savings", "EUR", true, 0, "2026-01-01")
        .unwrap();
    let c = b.categories[0].id.clone();
    let d = b.categories[1].id.clone();
    b.categories[0].name = "Same name".into();
    b.categories[1].name = "Same name".into();
    b.categories[1].group = "Another group".into();
    let hidden = b.categories[2].id.clone();
    b.categories[2].hidden = true;
    for (cat, value, date) in [
        (Some(c.as_str()), -101, "2026-01-20"),
        (Some(c.as_str()), 101, "2026-02-10"),
        (Some(d.as_str()), -301, "2026-01-20"),
        (Some(hidden.as_str()), -42, "2026-01-20"),
        (None, -10, "2026-01-20"),
    ] {
        b.put_transaction(transaction(&a, cat, value, date, "Shop"))
            .unwrap();
    }
    let mut salary = transaction(&a, None, 1001, "2026-01-25", "Employer, \"A\"");
    salary.income_month = Some("2026-02".into());
    b.put_transaction(salary).unwrap();
    b.transfer(&a, &other, 100, 100, "2026-01-20", "Internal")
        .unwrap();
    let r = reports::calculate(&b, 3, &filter()).unwrap();
    let income = matrix_row(&r, "income");
    assert_eq!(income.values, [1001, 0]);
    assert_eq!(income.average(), 501);
    assert_eq!(income.children[0].label, "Employer, \"A\"");
    let first = r
        .matrix
        .iter()
        .flat_map(|row| &row.children)
        .find(|row| row.key == format!("category:{c}"))
        .unwrap();
    assert_eq!(first.values, [-101, 101]);
    assert_eq!(first.total(), 0); // Do not discard activity that nets to zero.
    assert_eq!(
        matrix_row(&r, "group:Another group").children[0].label,
        "Same name"
    );
    assert_eq!(matrix_row(&r, "hidden").values, [-42, 0]);
    assert_eq!(matrix_row(&r, "uncategorized").values, [-10, 0]);
    assert_eq!(matrix_row(&r, "total-expenses").values, [-454, 101]);
    assert_eq!(matrix_row(&r, "net-income").values, [547, 101]);
    for month in 0..r.months.len() {
        let detail_sum: i64 = r
            .matrix
            .iter()
            .filter(|row| {
                row.key.starts_with("group:") || row.key == "hidden" || row.key == "uncategorized"
            })
            .map(|row| row.values[month])
            .sum();
        assert_eq!(detail_sum, -r.months[month].expense);
    }
    let bytes = r.csv("EUR", str::to_owned).unwrap();
    let mut csv = csv::Reader::from_reader(bytes.as_slice());
    assert_eq!(
        csv.headers().unwrap().iter().collect::<Vec<_>>(),
        [
            "From", "Through", "Currency", "Group", "Label", "2026-01", "2026-02", "Average",
            "Total"
        ]
    );
    let records = csv.records().map(|row| row.unwrap()).collect::<Vec<_>>();
    let salary = records
        .iter()
        .find(|row| &row[4] == "Employer, \"A\"")
        .unwrap();
    assert_eq!(&salary[3], "All Income Sources");
    assert_eq!(&salary[5], "10.01");
    assert_eq!(&salary[7], "5.01");
    let group = records
        .iter()
        .find(|row| &row[4] == "Another group")
        .unwrap();
    assert_eq!(&group[7], "-1.50");
    assert_eq!(&group[8], "-3.01");
}

#[test]
fn matrix_filters_split_parts_and_preserves_foreign_rounding_and_empty_months() {
    let mut b = Budget::new("EUR");
    b.add_snapshot("2026-01", "USD", 90_000_000, "2026-01-01", "test")
        .unwrap();
    let a = b.add_account("USD", "USD", true, 0, "2026-01-01").unwrap();
    let other = b
        .add_account("Other", "EUR", true, 0, "2026-01-01")
        .unwrap();
    let mut t = transaction(&a, None, -3, "2026-01-20", "Shop");
    t.splits = (0..3)
        .map(|index| Split {
            category: Some(b.categories[index].id.clone()),
            amount: -1,
            ..Default::default()
        })
        .collect();
    b.put_transaction(t).unwrap();
    let c = b.categories[2].id.clone();
    b.put_transaction(transaction(&other, Some(&c), -500, "2026-01-20", "Shop"))
        .unwrap();
    let f = Filter {
        accounts: Selection::Selected(vec![a]),
        category: Selection::Selected(vec![c.clone()]),
        payee: Selection::Selected(vec!["Shop".into()]),
        ..filter()
    };
    let r = reports::calculate(&b, 3, &f).unwrap();
    assert_eq!(matrix_row(&r, "total-expenses").values, [-1, 0]);
    assert_eq!(matrix_row(&r, "total-expenses").average(), 0);
    let children = r
        .matrix
        .iter()
        .flat_map(|row| &row.children)
        .collect::<Vec<_>>();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].key, format!("category:{c}"));
    assert_eq!(children[0].values, [-1, 0]);
    let empty = reports::calculate(&Budget::new("EUR"), 3, &filter()).unwrap();
    assert_eq!(empty.matrix.len(), 4);
    assert!(
        empty
            .matrix
            .iter()
            .all(|row| row.values == [0, 0] && row.average() == 0)
    );
}

#[test]
fn original_average_rounds_ties_up_and_sums_leaf_averages_for_net_income() {
    let mut b = Budget::new("USD");
    let a = b
        .add_account("Checking", "USD", true, 0, "2026-01-01")
        .unwrap();
    for index in 0..2 {
        let c = b.categories[index].id.clone();
        b.put_transaction(transaction(&a, Some(&c), -1, "2026-01-20", "Shop"))
            .unwrap();
    }
    for payee in ["A", "B"] {
        b.put_transaction(transaction(&a, None, 1, "2026-01-20", payee))
            .unwrap();
    }
    let r = reports::calculate(&b, 3, &filter()).unwrap();
    assert_eq!(matrix_row(&r, "total-income").average(), 1);
    assert_eq!(matrix_row(&r, "total-expenses").average(), -1);
    let net = matrix_row(&r, "net-income");
    assert_eq!(net.total(), 0);
    assert_eq!(net.average(), 2); // +0.5 -> +1 per source; -0.5 -> 0 per category.
    let csv = String::from_utf8(r.csv("USD", str::to_owned).unwrap()).unwrap();
    assert!(csv.contains("Net Income,0.00,0.00,0.02,0.00"));
}
