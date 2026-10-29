use super::*;
use budgettinker::store::upgrade::FrozenBudget;

fn quote(rate: i64, date: &str) -> MonthlyQuote {
    MonthlyQuote {
        rate,
        rate_date: date.into(),
        source: "Synthetic manual".into(),
    }
}
fn document(b: &Budget) -> serde_json::Value {
    serde_json::to_value(b).unwrap()
}
fn upgraded() -> (Budget, String) {
    let (b, _, usd) = fixture();
    (FrozenBudget::preflight(&b).unwrap().into_v2().unwrap(), usd)
}

#[test]
fn historical_preview_reconciles_adjacent_months_without_repricing_postings() {
    let (mut b, usd) = upgraded();
    let category = b.categories[0].id.clone();
    b.put_transaction(expense(&usd, &category, -10000, "2026-01-15"))
        .unwrap();
    let original = document(&b);
    let activity = b.activity("2026-01", &category);
    let preview = b
        .preview_monthly_correction(
            "2026-01",
            "USD",
            quote(100_000_000, "2026-01-30"),
            "Correct synthetic close",
        )
        .unwrap();
    assert_eq!(document(&b), original);
    assert_eq!(preview.before[0].net_worth, Some(181000));
    assert_eq!(preview.after[0].net_worth, Some(190000));
    assert_eq!(preview.before[0].revaluation_all, Some(0));
    assert_eq!(preview.after[0].revaluation_all, Some(9000));
    assert_eq!(preview.before[1].revaluation_all, Some(4500));
    assert_eq!(preview.after[1].revaluation_all, Some(-4500));
    assert_eq!(
        preview.after[0].available.unwrap() - preview.before[0].available.unwrap(),
        9000
    );
    b.apply_monthly_correction(preview).unwrap();
    assert_eq!(document(&b)["transactions"], original["transactions"]);
    assert_eq!(document(&b)["register_meta"], original["register_meta"]);
    assert_eq!(b.activity("2026-01", &category), activity);
    assert_eq!(
        b.monthly_fx.corrections[0].before.as_ref().unwrap().rate,
        90_000_000
    );
    assert_eq!(b.monthly_fx.corrections.len(), 1);
    validate(&b).unwrap();
    let path = temp("budgettinker");
    let mut store = Store::open(&path).unwrap();
    store.save(&b).unwrap();
    drop(store);
    assert_eq!(document(&read_budget(&path).unwrap()), document(&b));
    let _ = std::fs::remove_file(path);
}

#[test]
fn open_month_edits_are_provisional_and_finalize_is_explicit() {
    let mut b = Budget::new("EUR");
    b.version = 2;
    let month = current_month();
    b.edit_current_monthly_quote("USD", quote(90_000_000, &today()))
        .unwrap();
    let usd = b
        .add_account("Synthetic", "USD", true, 100000, &today())
        .unwrap();
    let postings = document(&b)["transactions"].clone();
    b.edit_current_monthly_quote("USD", quote(95_000_000, &today()))
        .unwrap();
    assert_eq!(b.net_worth(&month, false).unwrap(), 95000);
    assert_eq!(b.balance(&usd, None, false), 100000);
    assert_eq!(document(&b)["transactions"], postings);
    assert_eq!(
        b.monthly_valuation_status(&month).unwrap(),
        ValuationStatus::Provisional
    );
    let before = document(&b);
    assert!(b.finalize_valuation_month(&month).is_err());
    assert!(
        b.preview_monthly_correction(&month, "USD", quote(96_000_000, &today()), "wrong flow")
            .is_err()
    );
    assert_eq!(document(&b), before);
    let (mut b, _) = upgraded();
    assert!(b.monthly_fx.finalized.is_empty()); // Calendar rollover does not finalize.
    b.finalize_valuation_month("2026-01").unwrap();
    assert_eq!(
        b.monthly_valuation_status("2026-01").unwrap(),
        ValuationStatus::Finalized
    );
    let preview = b
        .preview_monthly_correction(
            "2026-01",
            "USD",
            quote(91_000_000, "2026-01-30"),
            "Revised close",
        )
        .unwrap();
    b.apply_monthly_correction(preview).unwrap();
    assert_eq!(
        b.monthly_valuation_status("2026-01").unwrap(),
        ValuationStatus::Finalized
    );
    assert!(b.finalize_valuation_month("2026-01").is_err());
}

#[test]
fn missing_quotes_use_earlier_preview_only_and_do_not_finalize() {
    let (mut b, usd) = upgraded();
    b.snapshots.retain(|s| s.month == "2026-01");
    assert_eq!(
        b.monthly_valuation_status("2026-02").unwrap(),
        ValuationStatus::Incomplete
    );
    let preview = b.account_valuation_preview(&usd, "2026-02").unwrap();
    assert!(preview.estimated);
    assert_eq!(preview.base_amount, Some(90000));
    assert_eq!(preview.quote_month.as_deref(), Some("2026-01"));
    assert!(b.net_worth("2026-02", false).is_err());
    assert!(b.finalize_valuation_month("2026-02").is_err());
    let correction = b
        .preview_monthly_correction(
            "2026-02",
            "USD",
            quote(95_000_000, "2026-02-27"),
            "Supply missing close",
        )
        .unwrap();
    assert_eq!(correction.before[0].net_worth, None);
    assert_eq!(correction.after[0].net_worth, Some(195000));
    b.apply_monthly_correction(correction).unwrap();
    b.finalize_valuation_month("2026-02").unwrap();
    b.snapshots.clear();
    b.add_snapshot(
        "2026-03",
        "USD",
        96_000_000,
        "2026-03-30",
        "Future relative to preview",
    )
    .unwrap();
    let unavailable = b.account_valuation_preview(&usd, "2026-01").unwrap();
    assert_eq!(unavailable.base_amount, None);
    assert!(!unavailable.estimated);
    assert!(validate(&b).is_err());
    let mut zero = Budget::new("EUR");
    zero.version = 2;
    zero.add_account("Zero", "KWD", true, 0, "2026-01-01")
        .unwrap();
    zero.finalize_valuation_month("2026-01").unwrap();
}

#[test]
fn corrections_reject_stale_approval_bad_audit_and_legacy_edits_atomically() {
    let (mut b, _) = upgraded();
    let preview = b
        .preview_monthly_correction(
            "2026-01",
            "USD",
            quote(91_000_000, "2026-01-30"),
            "Synthetic",
        )
        .unwrap();
    b.transactions[0].memo = "Changed after preview".into();
    let before = document(&b);
    assert!(b.apply_monthly_correction(preview).is_err());
    for (rate, date, reason) in [
        (0, "2026-01-30", "x"),
        (1, "2026-02-01", "x"),
        (1, "2026-01-30", " "),
    ] {
        assert!(
            b.preview_monthly_correction("2026-01", "USD", quote(rate, date), reason)
                .is_err()
        );
    }
    assert_eq!(document(&b), before);
    let mut legacy = fixture().0;
    let original = document(&legacy);
    assert!(
        legacy
            .edit_current_monthly_quote("USD", quote(90_000_000, &today()))
            .is_err()
    );
    assert!(
        legacy
            .preview_monthly_correction("2026-01", "USD", quote(91_000_000, "2026-01-30"), "x")
            .is_err()
    );
    assert_eq!(document(&legacy), original);
    let preview = b
        .preview_monthly_correction(
            "2026-01",
            "USD",
            quote(91_000_000, "2026-01-30"),
            "Synthetic",
        )
        .unwrap();
    b.apply_monthly_correction(preview).unwrap();
    b.monthly_fx.corrections[0].reason.clear();
    assert!(validate(&b).is_err());
    let mut json = original;
    json.as_object_mut().unwrap().remove("monthly_fx");
    let old: Budget = serde_json::from_value(json).unwrap();
    validate(&old).unwrap();
}

#[test]
fn historical_liability_and_off_budget_preview_keep_populations_separate() {
    let mut b = Budget::new("EUR");
    b.add_snapshot("2026-01", "USD", 90_000_000, "2026-01-30", "Synthetic")
        .unwrap();
    b.add_snapshot("2026-02", "USD", 95_000_000, "2026-02-27", "Synthetic")
        .unwrap();
    b.add_account("Off-budget debt", "USD", false, -100000, "2026-01-01")
        .unwrap();
    b.add_account("On-budget asset", "USD", true, 10000, "2026-01-01")
        .unwrap();
    let b = FrozenBudget::preflight(&b).unwrap().into_v2().unwrap();
    let preview = b
        .preview_monthly_correction(
            "2026-01",
            "USD",
            quote(100_000_000, "2026-01-30"),
            "Debt correction",
        )
        .unwrap();
    assert_eq!(
        preview.after[0].net_worth.unwrap() - preview.before[0].net_worth.unwrap(),
        -9000
    );
    assert_eq!(
        preview.after[0].budget_value.unwrap() - preview.before[0].budget_value.unwrap(),
        1000
    );
    assert_eq!(
        preview.after[0].available.unwrap() - preview.before[0].available.unwrap(),
        1000
    );
    assert_eq!(preview.after[1].revaluation_all, Some(4500));
    assert_eq!(preview.after[1].revaluation_budget, Some(-500));
}

#[test]
fn corrections_preserve_legacy_empty_source_and_validate_lifecycle_metadata() {
    let (mut b, _) = upgraded();
    b.snapshots[0].source.clear();
    let preview = b
        .preview_monthly_correction(
            "2026-01",
            "USD",
            quote(91_000_000, "2026-01-30"),
            "Supply documented source",
        )
        .unwrap();
    b.apply_monthly_correction(preview).unwrap();
    assert!(
        b.monthly_fx.corrections[0]
            .before
            .as_ref()
            .unwrap()
            .source
            .is_empty()
    );
    validate(&b).unwrap();
    let mut invalid = b.clone();
    invalid
        .monthly_fx
        .finalized
        .insert("2026-01".into(), "2026-01-31T12:00:00+00:00".into());
    assert!(validate(&invalid).is_err());
    let mut invalid = b.clone();
    invalid.monthly_fx.corrections[0].recorded_at = "invalid".into();
    assert!(validate(&invalid).is_err());
    let mut invalid = b.clone();
    invalid.monthly_fx.corrections[0].after.source.clear();
    assert!(validate(&invalid).is_err());
    let before = document(&b);
    assert!(
        b.preview_monthly_correction(
            "2026-01",
            "USD",
            MonthlyQuote {
                source: " ".into(),
                ..quote(91_000_000, "2026-01-30")
            },
            "Invalid source"
        )
        .is_err()
    );
    assert_eq!(document(&b), before);
}

#[test]
fn historical_correction_rejects_total_valuation_overflow_without_changes() {
    let mut b = Budget::new("EUR");
    b.add_snapshot("2026-01", "USD", 90_000_000, "2026-01-30", "Synthetic")
        .unwrap();
    for name in ["Large synthetic A", "Large synthetic B"] {
        b.add_account(name, "USD", false, 10_000_000_000_000, "2026-01-01")
            .unwrap();
    }
    let b = FrozenBudget::preflight(&b).unwrap().into_v2().unwrap();
    let original = document(&b);
    // Each account conversion fits i64; their combined stock does not.
    assert!(
        b.preview_monthly_correction(
            "2026-01",
            "USD",
            quote(90_000_000_000_000, "2026-01-30"),
            "Synthetic overflow"
        )
        .unwrap_err()
        .contains("overflow")
    );
    assert_eq!(document(&b), original);
}
