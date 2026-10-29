use super::*;
use crate::{
    model::*,
    reports::{self, Filter, Selection},
};

fn fixture(months: usize) -> (Budget, Filter) {
    let mut b = Budget::new("USD");
    let account = b
        .add_account("Checking", "USD", true, 0, "2020-01-01")
        .unwrap();
    let category = b.categories[0].id.clone();
    for index in 0..months {
        b.put_transaction(Transaction {
            id: id(),
            account: account.clone(),
            date: format!("{}-15", month_shift("2020-01", index as i32)),
            payee: format!("Shop {index}"),
            memo: String::new(),
            amount: -(10001 + index as i64),
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
        })
        .unwrap();
    }
    let f = Filter {
        from: "2020-01-01".into(),
        through: Budget::month_end(&month_shift("2020-01", months as i32 - 1)),
        ..Filter::default()
    };
    (b, f)
}
fn document(report: &Report) -> Document {
    layout(
        report,
        "Test budget",
        "Jan 1, 2020 – Jan 31, 2026",
        &["Accounts: Checking".into()],
        "USD",
        None,
        |v| money(v, "USD"),
    )
}
fn texts(d: &Document) -> Vec<&str> {
    d.pages
        .iter()
        .flatten()
        .filter(|i| i.kind == "text")
        .map(|i| i.text.as_str())
        .collect()
}
fn check_bounds(d: &Document) {
    for (index, page) in d.pages.iter().enumerate() {
        assert!(
            page.iter()
                .any(|i| i.text
                    == format!("BudgetTinker  |  Page {} of {}", index + 1, d.pages.len()))
        );
        for i in page {
            assert!(
                i.x >= 0. && i.y >= 0. && i.x + i.w <= WIDTH + 0.01 && i.y + i.h <= HEIGHT + 0.01,
                "out of bounds: {i:?}"
            );
            for p in &i.points {
                assert!(p[0] >= 0. && p[0] <= WIDTH && p[1] >= 0. && p[1] <= HEIGHT);
            }
        }
    }
}
#[test]
fn matrix_keeps_all_months_exact_values_and_full_period_averages_across_pages() {
    let (b, f) = fixture(13);
    let r = reports::calculate(&b, 3, &f).unwrap();
    let d = document(&r);
    let t = texts(&d);
    assert!(d.pages.len() >= 3);
    for m in &r.months {
        assert!(t.contains(&m.month.as_str()));
    }
    for row in &r.matrix {
        for v in row
            .values
            .iter()
            .copied()
            .chain([row.average(), row.total()])
        {
            assert!(t.contains(&money(v, "USD").as_str()));
        }
    }
    for page in &d.pages {
        assert!(page.iter().any(|i| i.text == "Category / Source"));
    }
    check_bounds(&d);
}
#[test]
fn trends_and_net_worth_paginate_without_losing_signed_values() {
    let (b, mut f) = fixture(25);
    f.accounts = Selection::All;
    for kind in [2, 4] {
        let r = reports::calculate(&b, kind, &f).unwrap();
        let d = document(&r);
        let t = texts(&d);
        for m in &r.months {
            assert!(t.contains(&m.month.as_str()));
            let values = if kind == 4 {
                vec![m.assets, m.debts, m.net_worth]
            } else {
                vec![m.expense]
            };
            for v in values {
                assert!(t.contains(&money(v, "USD").as_str()));
            }
        }
        assert!(d.pages.len() >= 3);
        check_bounds(&d);
    }
}
#[test]
fn print_details_use_filtered_split_entries_and_preserve_long_unicode_memos() {
    let (mut b, mut f) = fixture(2);
    let a = b.accounts[0].id.clone();
    let c = b.categories[0].id.clone();
    let other = b.categories[1].id.clone();
    let mut t = b.transactions.last().unwrap().clone();
    t.id = id();
    t.amount = -700;
    t.account = a;
    t.payee = "Chosen".into();
    t.memo = "Москва café 東京 ".repeat(100);
    t.splits = vec![
        Split {
            category: Some(c.clone()),
            amount: -300,
            ..Default::default()
        },
        Split {
            category: Some(other),
            amount: -400,
            ..Default::default()
        },
    ];
    b.put_transaction(t.clone()).unwrap();
    f.category = Selection::Selected(vec![c.clone()]);
    f.payee = Selection::Selected(vec!["Chosen".into()]);
    let before = serde_json::to_string(&b).unwrap();
    let r = reports::calculate(&b, 0, &f).unwrap();
    let scope = SpendingScope::Category(Some(c));
    let d = layout(
        &r,
        "Budget",
        "Period",
        &[],
        "USD",
        Some(Detail {
            scope: &scope,
            month: Some("2020-02"),
            label: "Chosen category",
        }),
        |v| money(v, "USD"),
    );
    let text = texts(&d);
    assert!(text.contains(&"3.00 USD"));
    assert!(!text.contains(&"7.00 USD"));
    assert!(text.contains(&"2020-02-15"));
    let memo: String = d
        .pages
        .iter()
        .flatten()
        .filter(|i| i.kind == "text" && i.x > 500. && i.x < 600. && i.text != "Memo")
        .map(|i| i.text.as_str())
        .collect();
    assert_eq!(memo, t.memo);
    assert!(d.pages.len() > 1);
    check_bounds(&d);
    assert_eq!(serde_json::to_string(&b).unwrap(), before);
}
#[test]
fn other_members_and_empty_reports_remain_printable() {
    let (b, f) = fixture(31);
    let r = reports::calculate(&b, 1, &f).unwrap();
    let d = document(&r);
    let text = texts(&d);
    assert!(text.contains(&"All Other Payees"));
    for entry in &r.spending_entries {
        assert!(text.iter().any(|s| s.trim() == entry.payee));
    }
    check_bounds(&d);
    let empty = Budget::new("JPY");
    for kind in 0..5 {
        check_bounds(&document(&reports::calculate(&empty, kind, &f).unwrap()));
    }
}
#[cfg(target_os = "macos")]
#[test]
fn native_pdf_export_writes_pdf_and_failed_export_keeps_existing_file() {
    let (b, f) = fixture(13);
    let d = document(&reports::calculate(&b, 3, &f).unwrap());
    let root = std::env::temp_dir().join(format!("budgettinker-pdf-test-{}", id()));
    std::fs::create_dir(&root).unwrap();
    let path = root.join("report.pdf");
    d.export_pdf(&path).unwrap();
    assert!(std::fs::read(&path).unwrap().starts_with(b"%PDF-"));
    // A directory destination cannot be replaced; partial output must be cleaned up.
    let directory = root.join("keep.pdf");
    std::fs::create_dir(&directory).unwrap();
    std::fs::write(directory.join("marker"), "keep").unwrap();
    assert!(d.export_pdf(&directory).is_err());
    assert_eq!(
        std::fs::read_to_string(directory.join("marker")).unwrap(),
        "keep"
    );
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 2);
    std::fs::remove_dir_all(root).unwrap();
}
