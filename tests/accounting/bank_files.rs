use super::*;

#[test]
fn csv_legacy_import_duplicate_detection_and_atomic_failure() {
    let (b, eur, _) = fixture();
    let path = temp("csv");
    std::fs::write(
        &path,
        "Date,Payee,Category,Memo,Outflow,Inflow\n01/20/2026,Shop,Food: Groceries,hello,12.34,\n",
    )
    .unwrap();
    let (next, count) = import_csv(&b, &path, &eur).unwrap();
    assert_eq!(count, 1);
    assert_eq!(next.transactions.last().unwrap().amount, -1234);
    assert_eq!(import_csv(&next, &path, &eur).unwrap().1, 0);
    std::fs::write(
        &path,
        "Date,Payee,Amount\n2026-01-21,Shop,-10\nbad,Shop,-1\n",
    )
    .unwrap();
    assert!(import_csv(&b, &path, &eur).is_err());
    assert_eq!(b.transactions.len(), 2);
    std::fs::remove_file(path).unwrap();
}
#[test]
fn csv_export_preserves_quotes_and_currency() {
    let (mut b, eur, _) = fixture();
    let c = b.categories[0].id.clone();
    let mut t = expense(&eur, &c, -1234, "2026-01-20");
    t.memo = "a, b\n\"quoted\"".into();
    b.put_transaction(t).unwrap();
    let path = temp("csv");
    export_csv(&b, &path).unwrap();
    let mut csv = csv::Reader::from_path(&path).unwrap();
    assert_eq!(
        csv.headers().unwrap().iter().collect::<Vec<_>>(),
        vec![
            "Date",
            "Account",
            "Currency",
            "Payee",
            "Category",
            "Memo",
            "Outflow",
            "Inflow",
            "Cleared",
            "Transfer",
            "Income Month"
        ]
    );
    let last = csv.records().last().unwrap().unwrap();
    assert_eq!(&last[5], "a, b\n\"quoted\"");
    assert_eq!(&last[2], "EUR");
    std::fs::remove_file(path).unwrap();
}

#[test]
fn identical_csv_rows_are_preserved_and_reimport_is_idempotent() {
    let (b, eur, _) = fixture();
    let path = temp("csv");
    std::fs::write(
        &path,
        "Date,Payee,Amount\n2026-01-21,Coffee,-5\n2026-01-21,Coffee,-5\n",
    )
    .unwrap();
    let (next, count) = import_csv(&b, &path, &eur).unwrap();
    assert_eq!(count, 2);
    assert_eq!(import_csv(&next, &path, &eur).unwrap().1, 0);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn csv_short_years_are_modern_years_and_four_digit_years_stay_literal() {
    let (b, account, _) = fixture();
    let path = temp("csv");
    for (input, expected) in [
        ("01/20/26", "2026-01-20"),
        ("01/20/2026", "2026-01-20"),
        ("01/20/0026", "0026-01-20"),
    ] {
        std::fs::write(&path, format!("Date,Payee,Amount\n{input},Shop,-1\n")).unwrap();
        let (next, count) = import_csv(&b, &path, &account).unwrap();
        assert_eq!(count, 1);
        assert_eq!(next.transactions.last().unwrap().date, expected);
    }
    std::fs::remove_file(path).unwrap();
}
