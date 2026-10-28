use super::*;

#[test]
fn zero_and_mixed_sign_schedules_post_skip_and_keep_recurring_parts() {
    let f = Fixture::new();
    f.edit(|v| {
        v["scheduledTransactions"][0]["amount"] = json!(0);
        let mut split = v["transactions"][3].clone();
        split["entityId"] = json!("mixed-plan");
        split["amount"] = json!(0);
        split["subTransactions"][1]["amount"] = json!(25);
        split["frequency"] = json!("Monthly");
        v["scheduledTransactions"]
            .as_array_mut()
            .unwrap()
            .push(split);
    });
    let mut b = legacy_budget::preview(&f.root).unwrap().budget;
    let balance = b.balance("card", None, false);
    let food = b.activity("2026-01", "food");
    let fees = b.activity("2026-01", "fees");
    b.post_scheduled("mixed-plan").unwrap();
    assert_eq!(b.balance("card", None, false), balance);
    assert_eq!(b.activity("2026-01", "food"), food - 2500);
    assert_eq!(b.activity("2026-01", "fees"), fees + 2500);
    let plan = b.scheduled.iter().find(|s| s.id == "mixed-plan").unwrap();
    assert_eq!(plan.next_date, "2026-02-10");
    assert_eq!(plan.transaction.splits[1].amount, 2500);
    b.skip_scheduled("mixed-plan").unwrap();
    assert_eq!(
        b.scheduled
            .iter()
            .find(|s| s.id == "mixed-plan")
            .unwrap()
            .next_date,
        "2026-03-10"
    );
    let count = b.transactions.len();
    b.enter_scheduled_now("rent").unwrap();
    assert_eq!(b.transactions.len(), count + 1);
    assert_eq!(b.transactions.last().unwrap().amount, 0);
    store::validate(&b).unwrap();
}

#[test]
fn categorized_boundary_schedule_generates_uncategorized_peer_and_preserves_edits() {
    let f = Fixture::new();
    f.edit(|v| {
        v["accounts"][2]["hidden"] = json!(false);
        v["scheduledTransactions"][1]["targetAccountId"] = json!("asset");
        v["scheduledTransactions"][1]["payeeId"] = Value::Null;
        v["scheduledTransactions"][1]["categoryId"] = json!("fees");
    });
    let mut b = legacy_budget::preview(&f.root).unwrap().budget;
    let peer_id = b
        .scheduled
        .iter()
        .find(|s| s.id.starts_with("legacy-counterpart/"))
        .unwrap()
        .id
        .clone();
    assert!(
        b.scheduled
            .iter()
            .find(|s| s.id == peer_id)
            .unwrap()
            .transaction
            .category
            .is_none()
    );
    let mut edited = b
        .scheduled
        .iter()
        .find(|s| s.id == "payment")
        .unwrap()
        .transaction
        .clone();
    edited.amount = -7500;
    edited.memo = "Boundary payment".into();
    b.edit_scheduled("payment", edited, Repeat::Monthly, None)
        .unwrap();
    b.post_scheduled(&peer_id).unwrap();
    assert_eq!(b.balance("cash", None, false), 112500);
    assert_eq!(b.balance("asset", None, false), 7500);
    assert_eq!(b.activity("2026-02", "fees"), -7500);
    assert_eq!(
        b.scheduled
            .iter()
            .find(|s| s.id == "payment")
            .unwrap()
            .transaction
            .category
            .as_deref(),
        Some("fees")
    );
    assert!(
        b.scheduled
            .iter()
            .all(|s| s.id == "rent" || s.next_date == "2026-03-28")
    );
    b.skip_scheduled("payment").unwrap();
    assert!(
        b.scheduled
            .iter()
            .all(|s| s.id == "rent" || s.next_date == "2026-04-28")
    );
    store::validate(&b).unwrap();
}
