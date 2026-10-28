use super::*;
#[test]
fn boundary_transfer_inline_edits_from_either_side_keep_category_and_income_timing() {
    for income in [None, Some(false), Some(true)] {
        for edit_budget_side in [false, true] {
            let mut b = Budget::new("EUR");
            let a = b
                .add_account("Checking", "EUR", true, 0, "2026-01-01")
                .unwrap();
            let off = b
                .add_account("Outside", "EUR", false, 0, "2026-01-01")
                .unwrap();
            let c = b.categories[0].id.clone();
            schedule_transfer(
                &mut b,
                TransferDraft {
                    from: &a,
                    to: &off,
                    amount: 5000,
                    date: "2026-02-28",
                    memo: "Boundary",
                    repeat: Repeat::Monthly,
                },
            )
            .unwrap();
            let source = b
                .scheduled
                .iter_mut()
                .find(|s| s.transaction.account == a)
                .unwrap();
            if let Some(next) = income {
                source.transaction.income_month = Some(month_shift("2026-02", i32::from(next)));
            } else {
                source.transaction.category = Some(c.clone());
            }
            store::validate(&b).unwrap();
            let selected = b
                .scheduled
                .iter()
                .find(|s| {
                    s.transaction.account
                        == if edit_budget_side {
                            a.as_str()
                        } else {
                            off.as_str()
                        }
                })
                .unwrap()
                .clone();
            let mut draft = RegisterDraft::from_scheduled(&b, &selected).unwrap();
            draft.date = "2026-03-28".into();
            draft.memo = "Edited boundary".into();
            if edit_budget_side {
                draft.outflow = "75".into();
            } else {
                draft.inflow = "75".into();
            }
            save_scheduled_draft(&mut b, &draft, Repeat::Monthly).unwrap();
            let source = b
                .scheduled
                .iter()
                .find(|s| s.transaction.account == a)
                .unwrap();
            assert_eq!(source.transaction.amount, -7500);
            assert_eq!(source.transaction.memo, "Edited boundary");
            if let Some(next) = income {
                assert_eq!(
                    source.transaction.income_month,
                    Some(month_shift("2026-03", i32::from(next)))
                );
            } else {
                assert_eq!(source.transaction.category.as_deref(), Some(c.as_str()));
            }
            let sid = source.id.clone();
            b.post_scheduled(&sid).unwrap();
            assert_eq!(b.balance(&a, None, false), -7500);
            assert_eq!(b.balance(&off, None, false), 7500);
            assert_eq!(
                b.activity("2026-03", &c),
                if income.is_none() { -7500 } else { 0 }
            );
            assert_eq!(
                b.deferred_income("2026-03"),
                if income == Some(true) { -7500 } else { 0 }
            );
            store::validate(&b).unwrap();
        }
    }
}
#[test]
fn inline_transfer_saves_both_directions_without_posting_and_rejects_invalid_rows() {
    for incoming in [false, true] {
        let mut b = Budget::new("EUR");
        let from = b
            .add_account("Checking", "EUR", true, 10_000, &today())
            .unwrap();
        let to = b.add_account("Savings", "EUR", true, 0, &today()).unwrap();
        let mut draft = RegisterDraft::new(&b, Some(&from), None).unwrap();
        draft.transfer_to = Some(to.clone());
        draft.date = format!("{}-15", month_shift(&current_month(), 1));
        draft.memo = "Planned savings".into();
        if incoming {
            draft.inflow = "25".into();
        } else {
            draft.outflow = "25".into();
        }
        let before = serde_json::to_string(&b).unwrap();
        // Both amount columns populated is invalid and must preserve the draft/budget.
        if incoming {
            draft.outflow = "10".into();
        } else {
            draft.inflow = "10".into();
        }
        assert!(save_scheduled_draft(&mut b, &draft, Repeat::Monthly).is_err());
        assert_eq!(serde_json::to_string(&b).unwrap(), before);
        if incoming {
            draft.outflow.clear();
        } else {
            draft.inflow.clear();
        }
        save_scheduled_draft(&mut b, &draft, Repeat::Monthly).unwrap();
        assert_eq!(b.scheduled.len(), 2);
        let source = b
            .scheduled
            .iter()
            .find(|s| s.transaction.account == from)
            .unwrap();
        assert_eq!(
            source.transaction.amount,
            if incoming { 2500 } else { -2500 }
        );
        assert_eq!(source.transaction.payee, "Transfer : Savings");
        assert_eq!(source.transaction.memo, draft.memo);
        assert_eq!(source.frequency(), Repeat::Monthly);
        assert_eq!(b.balance(&from, None, false), 10_000);
        assert_eq!(b.balance(&to, None, false), 0);
        store::validate(&b).unwrap();
    }
}
#[test]
fn scheduled_transfer_keeps_balances_until_both_sides_are_posted() {
    let mut b = Budget::new("EUR");
    let a = b
        .add_account("Cash", "EUR", true, 10_000, "2026-01-01")
        .unwrap();
    let other = b
        .add_account("Savings", "EUR", true, 0, "2026-01-01")
        .unwrap();
    schedule_transfer(
        &mut b,
        TransferDraft {
            from: &a,
            to: &other,
            amount: 3000,
            date: "2026-01-02",
            memo: "Saved",
            repeat: Repeat::Monthly,
        },
    )
    .unwrap();
    assert_eq!(b.scheduled.len(), 2);
    assert_eq!(b.balance(&a, None, false), 10_000);
    assert_eq!(b.balance(&other, None, false), 0);
    store::validate(&b).unwrap();
    let sid = b.scheduled[0].id.clone();
    b.post_scheduled(&sid).unwrap();
    assert_eq!(b.balance(&a, None, false), 7000);
    assert_eq!(b.balance(&other, None, false), 3000);
    assert!(b.scheduled.iter().all(|s| s.next_date == "2026-02-02"));
}
#[test]
fn invalid_scheduled_transfer_never_changes_budget() {
    let mut b = Budget::new("EUR");
    let a = b
        .add_account("Cash", "EUR", true, 100, "2026-01-01")
        .unwrap();
    let other = b
        .add_account("Savings", "EUR", true, 0, "2026-01-01")
        .unwrap();
    let before = serde_json::to_string(&b).unwrap();
    for (to, value, date) in [
        (&a, 100, "2026-01-02"),
        (&other, -100, "2026-01-02"),
        (&other, 100, "bad"),
    ] {
        assert!(
            schedule_transfer(
                &mut b,
                TransferDraft {
                    from: &a,
                    to,
                    amount: value,
                    date,
                    memo: "",
                    repeat: Repeat::Never
                }
            )
            .is_err()
        );
        assert_eq!(serde_json::to_string(&b).unwrap(), before);
    }
}
