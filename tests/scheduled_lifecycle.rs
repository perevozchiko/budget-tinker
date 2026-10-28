use budgettinker::{model::*, register::Repeat, store};
fn fixture() -> (Budget, String, String, String) {
    let mut b = Budget::new("EUR");
    let a = b.add_account("A", "EUR", true, 1000, "2026-01-01").unwrap();
    let other = b.add_account("B", "EUR", true, 0, "2026-01-01").unwrap();
    let c = b.add_account("C", "EUR", true, 0, "2026-01-01").unwrap();
    (b, a, other, c)
}
fn schedule(account: &str, amount: i64, link: Option<String>) -> Scheduled {
    Scheduled {
        id: id(),
        next_date: "2026-03-31".into(),
        monthly: true,
        repeat: Some(Repeat::Monthly),
        anchor_day: Some(31),
        transaction: Transaction {
            id: id(),
            account: account.into(),
            date: "2026-03-31".into(),
            payee: "Review".into(),
            memo: String::new(),
            amount,
            base_amount: 0,
            snapshot: None,
            income_month: None,
            category: None,
            cleared: false,
            reconciled: false,
            transfer: link,
            splits: vec![],
            check_number: None,
            flag: None,
        },
    }
}
fn pair(b: &mut Budget, a: &str, other: &str) -> String {
    let link = Some(id());
    b.scheduled.push(schedule(a, -100, link.clone()));
    b.scheduled.push(schedule(other, 100, link));
    store::validate(b).unwrap();
    b.scheduled[0].id.clone()
}
#[test]
fn recurring_transfer_skip_must_preserve_valid_pair() {
    let (mut b, a, other, _) = fixture();
    let sid = pair(&mut b, &a, &other);
    b.skip_scheduled(&sid).unwrap();
    assert_eq!(b.scheduled[0].next_date, b.scheduled[1].next_date);
    store::validate(&b).unwrap();
}
#[test]
fn moving_transfer_must_keep_destination_account() {
    let (mut b, a, other, c) = fixture();
    let sid = pair(&mut b, &a, &other);
    b.move_scheduled_account(&sid, &c).unwrap();
    assert_eq!(b.scheduled[1].transaction.account, other);
    store::validate(&b).unwrap();
}
#[test]
fn editing_date_must_change_next_occurrence() {
    let (mut b, a, _, _) = fixture();
    b.scheduled.push(schedule(&a, -100, None));
    let mut changed = b.scheduled[0].transaction.clone();
    changed.date = "2026-04-15".into();
    let sid = b.scheduled[0].id.clone();
    b.edit_scheduled(&sid, changed, Repeat::Monthly, Some(15))
        .unwrap();
    assert_eq!(b.scheduled[0].next_date, "2026-04-15");
}
#[test]
fn moving_to_foreign_account_must_not_panic() {
    let (mut b, a, _, _) = fixture();
    let foreign = b.add_account("USD", "USD", false, 0, "2026-01-01").unwrap();
    b.scheduled.push(schedule(&a, -100, None));
    let sid = b.scheduled[0].id.clone();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        b.move_scheduled_account(&sid, &foreign)
    }));
    assert!(
        outcome.is_ok(),
        "must return a validation result, not panic"
    );
}
#[test]
fn failed_early_post_must_leave_model_unchanged() {
    let (mut b, a, _, _) = fixture();
    let foreign = b.add_account("USD", "USD", false, 0, "2026-01-01").unwrap();
    let sid = pair(&mut b, &a, &foreign);
    let before = serde_json::to_string(&b).unwrap();
    assert!(b.enter_scheduled_now(&sid).is_err());
    assert_eq!(serde_json::to_string(&b).unwrap(), before);
}

#[test]
fn invalid_transfer_edits_preserve_the_complete_budget() {
    let (mut b, a, other, _) = fixture();
    let sid = pair(&mut b, &a, &other);
    let before = serde_json::to_string(&b).unwrap();
    let mut t = b.scheduled[0].transaction.clone();
    t.account = other.clone();
    assert!(b.edit_scheduled(&sid, t, Repeat::Weekly, None).is_err());
    assert_eq!(serde_json::to_string(&b).unwrap(), before);
    assert!(b.move_scheduled_account(&sid, &other).is_err());
    assert_eq!(serde_json::to_string(&b).unwrap(), before);
}

#[test]
fn transfer_edit_updates_both_sides_but_not_posted_transactions() {
    let (mut b, a, other, c) = fixture();
    let sid = pair(&mut b, &a, &other);
    let posted = serde_json::to_string(&b.transactions).unwrap();
    let mut t = b.scheduled[0].transaction.clone();
    t.date = "2026-04-15".into();
    t.account = c.clone();
    t.amount = -250;
    t.memo = "Changed".into();
    b.edit_scheduled_transfer(&sid, t, &other, Repeat::Weekly)
        .unwrap();
    assert_eq!(b.scheduled[0].transaction.account, c);
    assert_eq!(b.scheduled[1].transaction.account, other);
    assert_eq!(b.scheduled[1].transaction.amount, 250);
    assert_eq!(b.scheduled[1].transaction.memo, "Changed");
    for s in &b.scheduled {
        assert_eq!(s.next_date, "2026-04-15");
        assert_eq!(s.transaction.date, "2026-04-15");
        assert_eq!(s.frequency(), Repeat::Weekly);
        assert_eq!(s.anchor_day, Some(15));
    }
    assert_eq!(b.scheduled[1].transaction.payee, "Transfer : C");
    assert_eq!(serde_json::to_string(&b.transactions).unwrap(), posted);
    store::validate(&b).unwrap();
}

#[test]
fn changing_transfer_destination_keeps_pair_valid() {
    let (mut b, a, other, c) = fixture();
    let sid = pair(&mut b, &a, &other);
    let t = b.scheduled[0].transaction.clone();
    b.edit_scheduled_transfer(&sid, t, &c, Repeat::Monthly)
        .unwrap();
    assert_eq!(b.scheduled[0].transaction.payee, "Transfer : C");
    assert_eq!(b.scheduled[1].transaction.account, c);
    store::validate(&b).unwrap();
}

#[test]
fn memo_edit_after_a_short_month_keeps_anchor_and_due_date() {
    let (mut b, a, _, _) = fixture();
    let mut s = schedule(&a, -100, None);
    s.next_date = "2024-01-31".into();
    s.transaction.set_date(s.next_date.clone());
    let sid = s.id.clone();
    b.scheduled.push(s);
    b.skip_scheduled(&sid).unwrap();
    assert_eq!(b.scheduled[0].next_date, "2024-02-29");
    let mut t = b.scheduled[0].transaction.clone();
    t.memo = "Only a memo".into();
    b.edit_scheduled(&sid, t, Repeat::Monthly, None).unwrap();
    assert_eq!(b.scheduled[0].anchor_day, Some(31));
    b.skip_scheduled(&sid).unwrap();
    assert_eq!(b.scheduled[0].next_date, "2024-03-31");
}

#[test]
fn early_post_advances_from_nominal_date_and_cannot_duplicate_that_occurrence() {
    for transfer in [false, true] {
        let (mut b, a, other, _) = fixture();
        let sid = if transfer {
            pair(&mut b, &a, &other)
        } else {
            b.scheduled.push(schedule(&a, -100, None));
            b.scheduled[0].id.clone()
        };
        let due = format!("{}-15", month_shift(&current_month(), 1));
        for s in &mut b.scheduled {
            s.next_date = due.clone();
            s.transaction.set_date(due.clone());
            s.anchor_day = Some(15);
        }
        let count = b.transactions.len();
        b.enter_scheduled_now(&sid).unwrap();
        assert_eq!(b.transactions.len(), count + if transfer { 2 } else { 1 });
        for t in &b.transactions[count..] {
            assert_eq!(t.date, today());
        }
        for s in &b.scheduled {
            assert_eq!(
                s.next_date,
                format!("{}-15", month_shift(&current_month(), 2))
            );
        }
        let after = serde_json::to_string(&b).unwrap();
        assert!(b.post_scheduled(&sid).is_err());
        assert_eq!(serde_json::to_string(&b).unwrap(), after);
        store::validate(&b).unwrap();
    }
}

#[test]
fn one_time_post_or_skip_removes_the_whole_logical_schedule() {
    for transfer in [false, true] {
        for post in [false, true] {
            let (mut b, a, other, _) = fixture();
            let sid = if transfer {
                pair(&mut b, &a, &other)
            } else {
                b.scheduled.push(schedule(&a, -100, None));
                b.scheduled[0].id.clone()
            };
            for s in &mut b.scheduled {
                s.repeat = Some(Repeat::Never);
                s.monthly = false;
            }
            let count = b.transactions.len();
            if post {
                b.enter_scheduled_now(&sid).unwrap();
            } else {
                b.skip_scheduled(&sid).unwrap();
            }
            assert!(b.scheduled.is_empty());
            assert_eq!(
                b.transactions.len(),
                count
                    + if !post {
                        0
                    } else if transfer {
                        2
                    } else {
                        1
                    }
            );
            store::validate(&b).unwrap();
        }
    }
}

#[test]
fn closed_transfer_account_rejects_posting_atomically() {
    let (mut b, a, other, _) = fixture();
    let sid = pair(&mut b, &a, &other);
    b.accounts
        .iter_mut()
        .find(|a| a.id == other)
        .unwrap()
        .closed = true;
    let before = serde_json::to_string(&b).unwrap();
    assert!(b.enter_scheduled_now(&sid).is_err());
    assert!(b.post_scheduled(&sid).is_err());
    assert_eq!(serde_json::to_string(&b).unwrap(), before);
}

#[test]
fn missing_transfer_partner_returns_an_error_without_panicking() {
    let (mut b, a, other, _) = fixture();
    let sid = pair(&mut b, &a, &other);
    b.scheduled.pop();
    let before = serde_json::to_string(&b).unwrap();
    assert!(b.enter_scheduled_now(&sid).is_err());
    assert!(b.skip_scheduled(&sid).is_err());
    assert_eq!(serde_json::to_string(&b).unwrap(), before);
}

#[test]
fn skip_uses_each_existing_frequency_without_posting() {
    for repeat in Repeat::ALL {
        let (mut b, a, _, _) = fixture();
        let mut s = schedule(&a, -100, None);
        s.repeat = Some(repeat);
        s.monthly = repeat == Repeat::Monthly;
        let expected = repeat.next(&s.next_date, 31).unwrap();
        let sid = s.id.clone();
        b.scheduled.push(s);
        let posted = serde_json::to_string(&b.transactions).unwrap();
        b.skip_scheduled(&sid).unwrap();
        assert_eq!(b.scheduled.first().map(|s| s.next_date.clone()), expected);
        assert_eq!(serde_json::to_string(&b.transactions).unwrap(), posted);
    }
}
