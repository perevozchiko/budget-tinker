use super::*;

#[test]
fn fixed_point_and_currency_precision() {
    assert_eq!(amount("-1,23", "EUR").unwrap(), -123);
    assert_eq!(amount("123", "JPY").unwrap(), 123);
    assert_eq!(amount("1.234", "KWD").unwrap(), 1234);
    assert!(amount("1.234", "EUR").is_err());
    assert!(amount("NaN", "EUR").is_err());
    assert!(amount("100000000000000000000000", "EUR").is_err());
    assert_eq!(convert(100, "JPY", "EUR", 625_000).unwrap(), 63);
    assert_eq!(convert(-100, "JPY", "EUR", 625_000).unwrap(), -63);
    assert_eq!(convert(1234, "KWD", "EUR", 300_000_000).unwrap(), 370);
}

#[test]
fn snapshots_are_immutable_and_history_is_stable() {
    let (mut b, _, usd) = fixture();
    let c = b.categories[0].id.clone();
    b.put_transaction(expense(&usd, &c, -10_000, "2026-01-20"))
        .unwrap();
    assert_eq!(b.activity("2026-01", &c), -9000);
    assert!(
        b.add_snapshot("2026-01", "USD", 110_000_000, "2026-01-30", "overwrite")
            .is_err()
    );
    assert_eq!(b.transactions.last().unwrap().base_amount, -9000);
    assert_eq!(b.activity("2026-01", &c), -9000);
    assert_eq!(b.net_worth("2026-02", false).unwrap(), 185_500);
}
#[test]
fn no_silent_fallback_for_missing_rate() {
    let (mut b, _, usd) = fixture();
    let c = b.categories[0].id.clone();
    assert!(
        b.put_transaction(expense(&usd, &c, -100, "2026-03-01"))
            .is_err()
    );
    assert!(b.net_worth("2026-03", false).is_err());
    assert_eq!(b.transactions.len(), 2);
}
#[test]
fn zero_opening_does_not_need_rate() {
    let mut b = Budget::new("EUR");
    assert!(b.add_account("Cash", "RSD", true, 0, "2026-01-01").is_ok());
    assert_eq!(b.net_worth("2026-01", true).unwrap(), 0);
}
#[test]
fn allocations_roll_over_with_overspending() {
    let (mut b, eur, _) = fixture();
    let c = b.categories[0].id.clone();
    b.set_overspending_rule("2026-01", &c, OverspendingRule::CategoryBalance)
        .unwrap();
    b.allocate("2026-01", &c, 20_000).unwrap();
    b.put_transaction(expense(&eur, &c, -25_000, "2026-01-20"))
        .unwrap();
    assert_eq!(b.category_balance("2026-01", &c), -5000);
    assert_eq!(b.category_balance("2026-02", &c), -5000);
    b.allocate("2026-02", &c, 10_000).unwrap();
    assert_eq!(b.category_balance("2026-02", &c), 5000);
    assert_eq!(b.available("2026-01").unwrap(), 170_000);
}
#[test]
fn off_budget_excluded_from_budget_but_in_net_worth() {
    let mut b = Budget::new("EUR");
    b.add_account("Broker", "EUR", false, 100_000, "2026-01-01")
        .unwrap();
    assert_eq!(b.available("2026-01").unwrap(), 0);
    assert_eq!(b.net_worth("2026-01", false).unwrap(), 100_000);
}
