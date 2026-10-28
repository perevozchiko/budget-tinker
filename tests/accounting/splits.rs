use super::*;

#[test]
fn zero_net_foreign_split_freezes_rate_and_keeps_zero_parts_zero() {
    let (mut b, _, usd) = fixture();
    let categories: Vec<_> = b.categories.iter().take(3).map(|c| c.id.clone()).collect();
    let mut t = expense(&usd, &categories[0], 0, "2026-01-20");
    t.category = None;
    t.splits = vec![
        Split {
            category: Some(categories[0].clone()),
            amount: -6,
            ..Default::default()
        },
        Split {
            category: Some(categories[1].clone()),
            amount: 3,
            ..Default::default()
        },
        Split {
            category: Some(categories[1].clone()),
            amount: 3,
            ..Default::default()
        },
        Split {
            category: Some(categories[2].clone()),
            amount: 0,
            ..Default::default()
        },
    ];
    b.put_transaction(t.clone()).unwrap();
    let saved = b.transactions.last().unwrap();
    assert!(saved.snapshot.is_some());
    assert_eq!(saved.base_amount, 0);
    assert_eq!(b.activity("2026-01", &categories[0]), -5);
    assert_eq!(b.activity("2026-01", &categories[1]), 5);
    assert_eq!(b.activity("2026-01", &categories[2]), 0);
    let filter = budgettinker::reports::Filter {
        from: "2026-01-01".into(),
        through: "2026-01-31".into(),
        category: Selection::Selected(vec![categories[0].clone()]),
        ..Default::default()
    };
    assert_eq!(
        budgettinker::reports::calculate(&b, 0, &filter)
            .unwrap()
            .total,
        5
    );
    validate(&b).unwrap();
    let path = temp("sqlite3");
    let mut store = Store::open(&path).unwrap();
    store.save(&b).unwrap();
    let loaded = store.load().unwrap().unwrap();
    assert_eq!(
        serde_json::to_value(&loaded).unwrap(),
        serde_json::to_value(&b).unwrap()
    );
    drop(store);
    std::fs::remove_file(path).unwrap();

    let mut planned = t.clone();
    planned.id = id();
    let sid = id();
    b.scheduled.push(Scheduled {
        id: sid.clone(),
        transaction: planned,
        next_date: "2026-01-20".into(),
        monthly: true,
        repeat: None,
        anchor_day: None,
    });
    b.post_scheduled(&sid).unwrap();
    assert!(b.transactions.last().unwrap().snapshot.is_some());
    assert_eq!(b.activity("2026-01", &categories[0]), -10);
    assert_eq!(b.activity("2026-01", &categories[1]), 10);
    assert_eq!(b.activity("2026-01", &categories[2]), 0);
    let peer = b
        .add_account("Other Dollar", "USD", true, 0, "2026-01-01")
        .unwrap();
    b.register_command(
        &[t.id.clone()].into_iter().collect(),
        budgettinker::register::Command::MoveAccount(peer),
    )
    .unwrap();
    validate(&b).unwrap();

    // A memo-only zero needs no rate; converting it into a zero-net split does.
    let mut blank = t.clone();
    blank.id = id();
    blank.splits.clear();
    b.put_transaction(blank.clone()).unwrap();
    assert!(b.transactions.last().unwrap().snapshot.is_none());
    blank.splits = t.splits.clone();
    b.put_transaction(blank.clone()).unwrap();
    assert!(b.transactions.last().unwrap().snapshot.is_some());
    blank.splits.clear();
    b.put_transaction(blank).unwrap();
    assert!(b.transactions.last().unwrap().snapshot.is_none());
    t.date = "2026-03-20".into();
    let before = serde_json::to_value(&b).unwrap();
    assert!(b.put_transaction(t).is_err());
    assert_eq!(serde_json::to_value(&b).unwrap(), before);
}

#[test]
fn mixed_splits_enforce_part_limits_even_when_large_parts_cancel() {
    let (mut b, eur, _) = fixture();
    let category = b.categories[0].id.clone();
    for value in [100_000_000_000_001, i64::MAX] {
        let mut t = expense(&eur, &category, 0, "2026-01-20");
        t.category = None;
        t.splits = vec![
            Split {
                category: Some(category.clone()),
                amount: value,
                ..Default::default()
            },
            Split {
                category: Some(category.clone()),
                amount: -value,
                ..Default::default()
            },
        ];
        let before = serde_json::to_value(&b).unwrap();
        assert!(b.put_transaction(t.clone()).is_err());
        assert_eq!(serde_json::to_value(&b).unwrap(), before);
        let mut invalid = b.clone();
        invalid.transactions.push(t.clone());
        assert!(validate(&invalid).is_err());
        invalid.transactions.pop();
        invalid.scheduled.push(Scheduled {
            id: id(),
            next_date: t.date.clone(),
            transaction: t,
            monthly: false,
            repeat: None,
            anchor_day: None,
        });
        assert!(validate(&invalid).is_err());
    }
}
#[test]
fn split_rounding_preserves_total() {
    let (mut b, _, usd) = fixture();
    let c1 = b.categories[0].id.clone();
    let c2 = b.categories[1].id.clone();
    let mut t = expense(&usd, &c1, -3, "2026-01-20");
    t.category = None;
    t.splits = vec![
        Split {
            category: Some(c1.clone()),
            amount: -1,
            ..Default::default()
        },
        Split {
            category: Some(c2.clone()),
            amount: -2,
            ..Default::default()
        },
    ];
    b.put_transaction(t).unwrap();
    assert_eq!(b.activity("2026-01", &c1) + b.activity("2026-01", &c2), -3);
    validate(&b).unwrap();
}
#[test]
fn invalid_split_and_bad_dates_rejected() {
    let (mut b, eur, _) = fixture();
    let c = b.categories[0].id.clone();
    let mut t = expense(&eur, &c, -100, "2026-01-20");
    t.splits = vec![Split {
        category: Some(c.clone()),
        amount: -99,
        ..Default::default()
    }];
    assert!(b.put_transaction(t).is_err());
    assert!(
        b.put_transaction(expense(&eur, &c, -10, "2026-1-2"))
            .is_err()
    );
    assert!(
        b.put_transaction(expense(&eur, &c, -10, "2026-02-30"))
            .is_err()
    );
}
