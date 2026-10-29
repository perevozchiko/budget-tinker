use super::*;

#[test]
fn invalid_custom_edits_keep_saved_dates_and_stale_ids_never_broaden_selection() {
    let mut fixture = report_fixture();
    fixture.app.persist_report_preferences();
    let saved = fixture.app.preferences.reports.clone();
    fixture.app.report_period = 6;
    for (from, through) in [
        ("2026-", "2026-10-31"),
        ("2026-10-31", "2026-07-01"),
        ("1000-01-01", "3000-01-01"),
    ] {
        fixture.app.report_filter.from = from.into();
        fixture.app.report_filter.through = through.into();
        fixture.app.persist_report_preferences();
        assert_eq!(fixture.app.preferences.reports, saved);
        assert_eq!(
            crate::preferences::Preferences::load(&fixture.app.store.path)
                .unwrap()
                .reports,
            saved
        );
    }
    fixture.app.preferences.reports = Some(crate::preferences::ReportPreferences {
        report: usize::MAX,
        period: 6,
        filter: reports::Filter {
            from: "bad".into(),
            through: "bad".into(),
            category: Selection::Selected(vec!["deleted-category".into()]),
            ..Default::default()
        },
        spending_accounts: Selection::Selected(vec!["deleted-account".into()]),
        ..Default::default()
    });
    fixture.app.restore_report_preferences();
    assert_eq!(fixture.app.report, 0);
    assert_eq!(fixture.app.report_period, 5);
    assert_eq!(fixture.app.report_filter.category, Selection::NoneSelected);
    assert_eq!(fixture.app.report_filter.accounts, Selection::NoneSelected);
    assert!(fixture.app.report_filter.months().is_ok());
}

#[test]
fn relative_periods_follow_today_all_dates_follow_history_and_captions_are_readable() {
    let fixture = report_fixture();
    let mut filter = reports::Filter::default();
    apply_report_period(&fixture.app.b, 5, &mut filter, "2026-10-27");
    assert_eq!(
        (&*filter.from, &*filter.through),
        ("2026-07-01", "2026-10-31")
    );
    assert_eq!(
        report_period_caption(&filter, "2026-10-27"),
        "Jul 1, 2026 — Oct 31, 2026 · Oct 2026 is a partial month"
    );
    apply_report_period(&fixture.app.b, 0, &mut filter, "2027-01-05");
    assert_eq!(
        (&*filter.from, &*filter.through),
        ("2027-01-01", "2027-01-31")
    );
    apply_report_period(&fixture.app.b, 1, &mut filter, "2027-01-05");
    assert_eq!(
        (&*filter.from, &*filter.through),
        ("2026-12-01", "2026-12-31")
    );
    assert!(!report_period_caption(&filter, "2027-01-05").contains("partial"));
    apply_report_period(&fixture.app.b, 0, &mut filter, "2028-02-29");
    assert_eq!(filter.through, "2028-02-29");
    assert!(!report_period_caption(&filter, "2028-02-29").contains("partial"));
    apply_report_period(&Budget::new("USD"), 5, &mut filter, "2026-10-27");
    assert_eq!(filter.from, "2026-10-01");
    assert_eq!(filter.through, "2026-10-31");
}

#[test]
fn report_periods_include_whole_months_and_previous_leap_year() {
    let b = Budget::new("USD");
    let mut filter = reports::Filter::default();
    for (period, now, from, through) in [
        (7, "2025-02-10", "2024-01-01", "2024-12-31"),
        (2, "2025-01-15", "2024-11-01", "2025-01-31"),
        (4, "2024-02-10", "2024-01-01", "2024-02-29"),
    ] {
        apply_report_period(&b, period, &mut filter, now);
        assert_eq!(filter.from, from);
        assert_eq!(filter.through, through);
    }
}
