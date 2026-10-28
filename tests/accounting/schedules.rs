use super::*;

#[test]
fn monthly_schedule_and_no_double_post() {
    let (mut b, eur, _) = fixture();
    let c = b.categories[0].id.clone();
    let sid = id();
    b.scheduled.push(Scheduled {
        id: sid.clone(),
        transaction: expense(&eur, &c, -100, "2026-01-31"),
        next_date: "2026-01-31".into(),
        monthly: true,
        repeat: None,
        anchor_day: None,
    });
    b.post_scheduled(&sid).unwrap();
    assert_eq!(b.scheduled[0].next_date, "2026-02-28");
    assert_eq!(b.transactions.len(), 3);
    b.scheduled[0].monthly = false;
    b.post_scheduled(&sid).unwrap();
    assert!(b.post_scheduled(&sid).is_err());
}
