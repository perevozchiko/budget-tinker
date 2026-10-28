use super::*;

#[test]
fn transfer_has_two_legs_and_no_spending() {
    let (mut b, eur, usd) = fixture();
    let before = b.available("2026-01").unwrap();
    b.transfer(&usd, &eur, 10_000, 9000, "2026-01-20", "Exchange")
        .unwrap();
    assert_eq!(b.balance(&usd, None, false), 90_000);
    assert_eq!(b.balance(&eur, None, false), 109_000);
    assert_eq!(b.available("2026-01").unwrap(), before);
    assert_eq!(
        b.categories
            .iter()
            .map(|c| b.activity("2026-01", &c.id))
            .sum::<i64>(),
        0
    );
    let tid = b.transactions.last().unwrap().id.clone();
    b.delete_transaction(&tid).unwrap();
    assert_eq!(b.transactions.len(), 2);
}
#[test]
fn transfer_uses_actual_received_amount() {
    let (mut b, eur, usd) = fixture();
    b.transfer(&usd, &eur, 10_000, 8800, "2026-01-20", "")
        .unwrap();
    assert_eq!(b.available("2026-01").unwrap(), 189_800);
}
