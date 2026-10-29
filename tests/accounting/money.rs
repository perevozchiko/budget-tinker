use super::*;

#[test]
fn approved_currencies_and_native_precision() {
    assert_eq!(CURRENCIES.len(), 11);
    let expected = [
        "EUR", "USD", "RSD", "RUB", "TRY", "GBP", "CHF", "CAD", "AUD", "JPY", "KWD",
    ];
    for code in expected {
        let metadata = currency_metadata(code).unwrap();
        assert_eq!(metadata.code, code);
        assert_eq!(
            metadata.decimals,
            match code {
                "JPY" => 0,
                "KWD" => 3,
                _ => 2,
            }
        );
        for sign in ["", "-"] {
            let input = match code {
                "JPY" => "125 000",
                "KWD" => "125.345",
                _ => "1 234.56",
            };
            let value = amount(&format!("{sign}{input}"), code).unwrap();
            assert_eq!(money(value, code), format!("{sign}{input} {code}"));
            if code != "JPY" {
                assert!(amount(&format!("{sign}{input}1"), code).is_err());
            }
        }
    }
    assert!(currency_metadata("BTC").is_none());
    assert!(amount("1", "BTC").is_err());
    assert!(amount("125 000.0", "JPY").is_err());
    assert!(amount("125.3456", "KWD").is_err());
}

#[test]
fn space_grouping_is_strict_and_bounded() {
    for (input, expected) in [
        ("1 234.56", 123456),
        ("1 500.00", 150000),
        ("1234.56", 123456),
        (" -1 234.56 ", -123456),
        ("+1 234 567.89", 123456789),
        ("1 234", 123400),
        ("0.01", 1),
    ] {
        assert_eq!(amount(input, "EUR").unwrap(), expected, "{input}");
    }
    for invalid in [
        "",
        "-",
        "1,23",
        "1,234.56",
        "1,500",
        "1.234,56",
        "12 34.56",
        "1234 567",
        "1  234",
        "1 234.5 6",
        "1 23",
        "1 234,56",
        "1e3",
        "NaN",
        "--1",
        "１２３",
        "1.234",
        "1.2.3",
    ] {
        assert!(amount(invalid, "EUR").is_err(), "{invalid}");
    }
    assert_eq!(parse_fixed("0.00855000", 8).unwrap(), 855000);
    assert_eq!(parse_fixed("1 234.12345678", 8).unwrap(), 123412345678);
    assert!(parse_fixed("1", u32::MAX).is_err());
    for sign in ["", "-"] {
        assert_eq!(
            amount(&format!("{sign}1 000 000 000 000.00"), "EUR")
                .unwrap()
                .unsigned_abs(),
            100_000_000_000_000
        );
        for invalid in [
            "1 000 000 000 000.01",
            "9223372036854775808",
            "999999999999999999999999999999999999",
        ] {
            assert!(amount(&format!("{sign}{invalid}"), "EUR").is_err());
        }
    }
    assert_eq!(money(i64::MIN, "EUR"), "-92 233 720 368 547 758.08 EUR");
    assert_eq!(plain(123456, "EUR"), "1234.56");
}

#[test]
fn conversion_checks_overflow_and_rounds_symmetrically() {
    for from in CURRENCIES {
        for to in CURRENCIES {
            for value in [0, 1, 101, 999999] {
                assert_eq!(
                    convert(-value, from, to, 85_500_001).unwrap(),
                    -convert(value, from, to, 85_500_001).unwrap()
                );
            }
        }
    }
    assert_eq!(convert(1, "EUR", "EUR", RATE_SCALE / 2).unwrap(), 1);
    assert_eq!(convert(-1, "EUR", "EUR", RATE_SCALE / 2).unwrap(), -1);
    assert!(convert(i64::MAX, "JPY", "KWD", i64::MAX).is_err());
    assert!(convert(i64::MIN, "JPY", "KWD", i64::MAX).is_err());
    assert!(convert(i64::MAX, "JPY", "EUR", RATE_SCALE).is_err());
    assert!(convert(1, "EUR", "USD", 0).is_err());
    assert!(convert(1, "EUR", "BTC", RATE_SCALE).is_err());
}

#[test]
fn all_currencies_round_trip_current_document_without_migration() {
    let mut budget = Budget::new("TRY");
    for currency in CURRENCIES {
        if *currency != budget.base {
            budget
                .add_snapshot("2026-01", currency, RATE_SCALE, "2026-01-01", "Synthetic")
                .unwrap();
        }
        budget
            .add_account(currency, currency, true, 123, "2026-01-01")
            .unwrap();
    }
    validate(&budget).unwrap();
    let json = serde_json::to_value(&budget).unwrap();
    let restored: Budget = serde_json::from_value(json.clone()).unwrap();
    validate(&restored).unwrap();
    assert_eq!(serde_json::to_value(restored).unwrap(), json);
    assert_eq!(budget.version, 1);
}

#[test]
fn fixed_point_and_currency_precision() {
    assert_eq!(amount("-1.23", "EUR").unwrap(), -123);
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
