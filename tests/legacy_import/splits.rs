use super::*;

#[test]
fn mixed_sign_zero_net_and_zero_parts_preserve_category_activity_and_money() {
    for (total, food, fees) in [(-30, -50, 20), (0, -25, 25), (30, 50, -20)] {
        let f = Fixture::new();
        f.edit(|v| {
            v["transactions"][3]["amount"] = json!(total);
            v["transactions"][3]["subTransactions"][0]["amount"] = json!(food);
            v["transactions"][3]["subTransactions"][1]["amount"] = json!(fees);
            v["transactions"][3]["subTransactions"]
                .as_array_mut()
                .unwrap()
                .push(json!({
                    "entityId": "zero-part", "amount": 0, "categoryId": "old", "memo": "Kept zero"
                }));
            let mut zero = v["transactions"][5].clone();
            zero["entityId"] = json!("zero");
            zero["amount"] = json!(0);
            zero["memo"] = json!("No cash movement");
            v["transactions"].as_array_mut().unwrap().push(zero);
        });
        let before = fs::read(&f.full).unwrap();
        let b = legacy_budget::preview(&f.root).unwrap().budget;
        assert_eq!(b.transactions.len(), 9);
        assert_eq!(b.activity("2026-01", "food"), food * 100 + 1000);
        assert_eq!(b.activity("2026-01", "fees"), fees * 100 - 500);
        assert_eq!(b.activity("2026-01", "old"), 0);
        assert_eq!(b.balance("card", None, false), -9500 + total * 100);
        assert_eq!(b.available("2026-01").unwrap(), 90_000);
        assert_eq!(b.notes["transaction:split:split:2"], "Kept zero");
        assert!(b.register_meta["zero"].pending);
        let zero = b.transactions.iter().find(|t| t.id == "zero").unwrap();
        assert_eq!(zero.memo, "No cash movement");
        assert_eq!(zero.amount, 0);
        let r = budgettinker::reports::calculate(
            &b,
            3,
            &budgettinker::reports::Filter {
                from: "2026-01-01".into(),
                through: "2026-01-31".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(r.months[0].expense, 19_500 - total * 100);
        let mut store = Store::open(&f.root.parent().unwrap().join("zero.sqlite3")).unwrap();
        store.save(&b).unwrap();
        assert_eq!(
            serde_json::to_value(store.load().unwrap().unwrap()).unwrap(),
            serde_json::to_value(&b).unwrap()
        );
        assert_eq!(fs::read(&f.full).unwrap(), before);
    }
}

#[test]
fn expanded_split_snapshot_preserves_each_part_and_links_scheduled_and_posted_peers() {
    use budgettinker::{
        register::Command,
        reports::{self, Filter, Selection},
    };
    let f = Fixture::new();
    fs::write(
        &f.full,
        include_bytes!("../fixtures/legacy/ExpandedSplits.yfull"),
    )
    .unwrap();
    let bytes = fs::read(&f.full).unwrap();
    let p = legacy_budget::preview(&f.root).unwrap();
    let b = &p.budget;
    let t = b.transactions.iter().find(|t| t.id == "split").unwrap();
    assert_eq!(t.amount, 8000);
    assert_eq!(t.splits.len(), 8);
    assert_eq!(t.splits[2].income_month.as_deref(), Some("2026-01"));
    assert_eq!(t.splits[3].income_month.as_deref(), Some("2026-02"));
    assert_eq!(t.splits[4].amount, -1000);
    assert_eq!(t.splits[5].payee.as_deref(), Some("Shop"));
    assert_eq!(t.splits[5].memo, "Uncategorized item");
    assert!(t.splits[5].category.is_none());
    assert_eq!(b.balance("cash", None, false), 122000);
    assert_eq!(b.balance("card", None, false), -1500);
    assert_eq!(b.balance("asset", None, false), 800);
    assert_eq!(b.activity("2026-01", "food"), -2300);
    assert_eq!(b.deferred_income("2026-01"), 34000);
    assert_eq!(b.available("2026-01").unwrap(), 99800);
    assert_eq!(b.scheduled.len(), 6);
    let report = reports::calculate(
        b,
        1,
        &Filter {
            from: "2026-01-01".into(),
            through: "2026-01-31".into(),
            payee: Selection::Selected(["Shop".into()].into_iter().collect()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(report.total, 2200); // Shop: 25 + 5 + 2 - 10 refund; transfer has its own payee.
    let mut posted = b.clone();
    posted
        .enter_scheduled_now("legacy-counterpart/plan-s7")
        .unwrap();
    assert_eq!(posted.transactions.len(), 13);
    assert!(
        posted
            .scheduled
            .iter()
            .filter(|s| s.id == "expanded-plan" || s.id.starts_with("legacy-counterpart/plan-"))
            .all(|s| s.next_date == "2026-03-28")
    );
    let mut deleted = b.clone();
    deleted
        .register_command(&["peer-s7".into()].into_iter().collect(), Command::Delete)
        .unwrap();
    let parent = deleted
        .transactions
        .iter()
        .find(|t| t.id == "split")
        .unwrap();
    assert_eq!(parent.splits.len(), 7);
    assert_eq!(parent.amount, 10000);
    assert!(deleted.transactions.iter().any(|t| t.id == "peer-s8"));
    let mut store = Store::open(&f.root.parent().unwrap().join("expanded.sqlite3")).unwrap();
    store.save(b).unwrap();
    assert_eq!(
        serde_json::to_value(store.load().unwrap().unwrap()).unwrap(),
        serde_json::to_value(b).unwrap()
    );
    p.verify_source(&f.root).unwrap();
    assert_eq!(fs::read(&f.full).unwrap(), bytes);
}
