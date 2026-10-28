use super::*;

#[test]
fn future_schedule_stays_out_of_register_and_needs_no_future_exchange_rate() {
    let mut b = Budget::new("EUR");
    let account = b.add_account("Dollar", "USD", true, 0, &today()).unwrap();
    let mut draft = Draft::new(&b, Some(&account), None).unwrap();
    draft.date = format!("{}-15", month_shift(&current_month(), 2));
    draft.outflow = "25".into();
    draft.cleared = true;
    assert!(draft.build(&b).is_err());
    let transaction = draft.build_scheduled(&b).unwrap();
    assert!(!transaction.cleared);
    assert_eq!(transaction.base_amount, 0);
    assert!(transaction.snapshot.is_none());
    b.scheduled.push(Scheduled {
        id: id(),
        next_date: transaction.date.clone(),
        transaction,
        monthly: true,
        repeat: Some(Repeat::Monthly),
        anchor_day: None,
    });
    budgettinker::store::validate(&b).unwrap();
    assert!(b.transactions.is_empty());
    assert_eq!(b.balance(&account, None, false), 0);
}
