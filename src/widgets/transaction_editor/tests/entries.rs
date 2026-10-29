use super::*;

#[test]
fn grouped_input_posts_exactly_and_ambiguous_comma_preserves_draft() {
    let (mut b, a) = budget();
    let mut draft = Draft::new(&b, Some(&a), None).unwrap();
    draft.payee = "Synthetic grouped amount".into();
    draft.outflow = "1,23".into();
    assert!(draft.build(&b).is_err());
    assert_eq!(draft.outflow, "1,23");
    assert_eq!(draft.payee, "Synthetic grouped amount");
    assert_eq!(b.transactions.len(), 1);
    draft.outflow = "1 234.56".into();
    b.put_transaction(draft.build(&b).unwrap()).unwrap();
    assert_eq!(b.transactions.last().unwrap().amount, -123456);
    assert_eq!(b.balance(&a, None, false), -23456);
}

#[test]
fn register_expense_uses_account_currency_and_persists_as_negative() {
    let (mut b, a) = budget();
    let mut d = Draft::new(&b, Some(&a), None).unwrap();
    d.outflow = "12.34".into();
    d.payee = "Shop".into();
    let t = d.build(&b).unwrap();
    assert_eq!((t.amount, t.base_amount), (-1234, -1234));
    b.put_transaction(t).unwrap();
    assert_eq!(b.balance(&a, None, false), 98_766);
}
#[test]
fn invalid_row_never_changes_the_budget() {
    let (b, a) = budget();
    let mut d = Draft::new(&b, Some(&a), None).unwrap();
    d.outflow = "12".into();
    d.inflow = "20".into();
    assert!(d.build(&b).is_err());
    d.inflow.clear();
    d.outflow = "-10".into();
    assert!(d.build(&b).is_err());
    d.outflow = "5".into();
    d.date = "2026-99-01".into();
    assert!(d.build(&b).is_err());
    assert_eq!(b.transactions.len(), 1);
}
#[test]
fn edit_replaces_existing_transaction_and_preserves_its_identity() {
    let (mut b, a) = budget();
    let mut d = Draft::new(&b, Some(&a), None).unwrap();
    d.outflow = "10".into();
    let t = d.build(&b).unwrap();
    let tid = t.id.clone();
    b.put_transaction(t).unwrap();
    b.transactions.last_mut().unwrap().flag = Some(budgettinker::model::TransactionFlag::Green);
    let mut edit = Draft::new(&b, None, Some(&tid)).unwrap();
    edit.outflow = "25".into();
    edit.memo = "edited".into();
    b.put_transaction(edit.build(&b).unwrap()).unwrap();
    assert_eq!(b.transactions.len(), 2);
    assert_eq!(b.transactions.last().unwrap().id, tid);
    assert_eq!(
        b.transactions.last().unwrap().flag,
        Some(budgettinker::model::TransactionFlag::Green)
    );
    assert_eq!(b.balance(&a, None, false), 97_500);
}
#[test]
fn row_requires_monthly_snapshot_for_foreign_currency() {
    let mut b = Budget::new("EUR");
    let a = b.add_account("Dollar", "USD", true, 0, &today()).unwrap();
    let mut d = Draft::new(&b, Some(&a), None).unwrap();
    d.inflow = "100".into();
    assert!(d.build(&b).is_err());
    b.add_snapshot(&current_month(), "USD", 90_000_000, &today(), "test")
        .unwrap();
    let t = d.build(&b).unwrap();
    assert_eq!(t.base_amount, 9000);
    assert!(t.snapshot.is_some());
}
