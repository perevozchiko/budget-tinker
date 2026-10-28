use super::*;

#[test]
fn csv_exports_filtered_rows_with_quoted_labels_and_exact_signed_amounts() {
    let mut b = Budget::new("EUR");
    let a = b
        .add_account("Checking", "EUR", true, 0, "2026-01-01")
        .unwrap();
    let c = b.categories[0].id.clone();
    b.put_transaction(transaction(
        &a,
        Some(&c),
        -123,
        "2026-01-20",
        "Shop, \"A\"\nSecond line",
    ))
    .unwrap();
    let r = reports::calculate(&b, 1, &filter()).unwrap();
    let bytes = r.csv("EUR", str::to_owned).unwrap();
    let mut reader = csv::Reader::from_reader(bytes.as_slice());
    let row = reader.records().next().unwrap().unwrap();
    assert_eq!(&row[1], "2026-01-10");
    assert_eq!(&row[2], "2026-02-15");
    assert_eq!(&row[3], "Shop, \"A\"\nSecond line");
    assert_eq!(&row[4], "1.23");
    assert_eq!(&row[5], "EUR");
}
