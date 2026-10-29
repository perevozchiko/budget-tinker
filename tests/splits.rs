use budgettinker::{
    model::*,
    register::{Command, Repeat},
    reports::{self, Filter, Selection},
    store::{self, Store},
};
use std::collections::HashSet;

fn fixture() -> (Budget, String, String, String) {
    let mut b = Budget::new("EUR");
    let source = b
        .add_account("Checking", "EUR", true, 0, "2026-01-01")
        .unwrap();
    let target = b
        .add_account("Savings", "EUR", true, 0, "2026-01-01")
        .unwrap();
    let outside = b
        .add_account("Outside", "EUR", false, 0, "2026-01-01")
        .unwrap();
    (b, source, target, outside)
}
fn row(account: &str, parts: Vec<Split>) -> Transaction {
    Transaction {
        id: id(),
        account: account.into(),
        date: "2026-01-31".into(),
        payee: "Parent".into(),
        memo: "Parent memo".into(),
        amount: parts.iter().map(|s| s.amount).sum(),
        base_amount: 0,
        snapshot: None,
        category: None,
        cleared: false,
        operation_fx: None,
        check_number: None,
        flag: None,
        income_month: None,
        reconciled: false,
        transfer: None,
        splits: parts,
    }
}
fn snapshot(b: &Budget) -> serde_json::Value {
    serde_json::to_value(b).unwrap()
}
fn select(tid: &str) -> HashSet<String> {
    HashSet::from([tid.into()])
}
fn transfer(account: &str, amount: i64, category: Option<String>) -> Split {
    Split {
        amount,
        category,
        transfer: Some(SplitTransfer {
            link: id(),
            account: account.into(),
        }),
        memo: "Part transfer".into(),
        ..Default::default()
    }
}

#[test]
fn split_income_payees_uncategorized_and_negative_adjustments_have_independent_flows() {
    let (mut b, a, _, _) = fixture();
    let category = b.categories[0].id.clone();
    let t = row(
        &a,
        vec![
            Split {
                amount: 10000,
                income_month: Some("2026-02".into()),
                payee: Some("Employer".into()),
                ..Default::default()
            },
            Split {
                amount: -1000,
                income_month: Some("2026-02".into()),
                payee: Some("Adjustment".into()),
                ..Default::default()
            },
            Split {
                amount: 3000,
                income_month: Some("2026-01".into()),
                ..Default::default()
            },
            Split {
                amount: -2000,
                category: Some(category.clone()),
                payee: Some("Grocer".into()),
                memo: "Vegetables".into(),
                ..Default::default()
            },
            Split {
                amount: -500,
                ..Default::default()
            },
            Split {
                amount: 0,
                ..Default::default()
            },
        ],
    );
    b.put_transaction(t.clone()).unwrap();
    assert_eq!(b.balance(&a, None, false), 9500);
    assert_eq!(b.activity("2026-01", &category), -2000);
    assert_eq!(b.deferred_income("2026-01"), 9000);
    assert_eq!(b.available("2026-01").unwrap(), 2500);
    assert_eq!(b.available("2026-02").unwrap(), 9500);
    let f = Filter {
        from: "2026-01-01".into(),
        through: "2026-02-28".into(),
        ..Default::default()
    };
    let report = reports::calculate(&b, 3, &f).unwrap();
    assert_eq!(
        (report.months[0].income, report.months[0].expense),
        (12000, 2500)
    );
    assert_eq!(report.months[1].income, 0);
    let selected = reports::calculate(
        &b,
        1,
        &Filter {
            payee: Selection::Selected(["Grocer".into()].into_iter().collect()),
            ..f
        },
    )
    .unwrap();
    assert_eq!(selected.total, 2000);
    assert_eq!(selected.spending_entries[0].memo, "Vegetables");
    let mut dated = t.clone();
    dated.set_date("2026-02-28".into());
    assert_eq!(dated.splits[0].income_month.as_deref(), Some("2026-03"));
    assert_eq!(dated.splits[2].income_month.as_deref(), Some("2026-02"));
    let legacy: Split = serde_json::from_str(r#"{"category":null,"amount":0}"#).unwrap();
    assert!(legacy.payee.is_none() && legacy.transfer.is_none());
    assert_eq!(
        serde_json::to_string(&legacy).unwrap(),
        r#"{"category":null,"amount":0}"#
    );
    let dir = std::env::temp_dir().join(format!("split-persistence-{}", id()));
    std::fs::create_dir(&dir).unwrap();
    let mut store = Store::open(&dir.join("budget.sqlite3")).unwrap();
    store.save(&b).unwrap();
    assert_eq!(snapshot(&store.load().unwrap().unwrap()), snapshot(&b));
    store::export_json(&b, &dir.join("backup.json")).unwrap();
    assert_eq!(
        snapshot(&store::import_json(&dir.join("backup.json")).unwrap()),
        snapshot(&b)
    );
    drop(store);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn split_transfers_update_delete_move_and_replace_atomically_without_double_expenses() {
    let (mut b, a, target, outside) = fixture();
    let category = b.categories[0].id.clone();
    let t = row(
        &a,
        vec![
            Split {
                amount: -2000,
                category: Some(category.clone()),
                ..Default::default()
            },
            transfer(&target, -3000, None),
            transfer(&outside, -1000, Some(category.clone())),
        ],
    );
    let tid = t.id.clone();
    b.put_transaction(t).unwrap();
    assert_eq!(b.transactions.len(), 3);
    assert_eq!(b.balance(&target, None, false), 3000);
    assert_eq!(b.activity("2026-01", &category), -3000);
    assert_eq!(
        reports::calculate(
            &b,
            0,
            &Filter {
                from: "2026-01-01".into(),
                through: "2026-01-31".into(),
                accounts: Selection::All,
                ..Default::default()
            }
        )
        .unwrap()
        .total,
        3000
    );
    let original = snapshot(&b);
    let mut edit = b.transactions.iter().find(|t| t.id == tid).unwrap().clone();
    edit.splits[1].amount = -4500;
    edit.amount -= 1500;
    edit.set_date("2026-02-01".into());
    b.put_transaction(edit).unwrap();
    assert_eq!(b.balance(&target, None, false), 4500);
    assert!(b.transactions.iter().all(|t| t.date == "2026-02-01"));
    let peer = b
        .transactions
        .iter_mut()
        .find(|t| t.account == outside)
        .unwrap();
    peer.cleared = true;
    peer.reconciled = true;
    let locked = snapshot(&b);
    assert!(b.delete_transaction(&tid).is_err());
    assert_eq!(snapshot(&b), locked);
    let mut edit = b.transactions.iter().find(|t| t.id == tid).unwrap().clone();
    edit.splits[2].amount -= 1;
    edit.amount -= 1;
    assert!(b.put_transaction(edit).is_err());
    assert_eq!(snapshot(&b), locked);
    let mut restored: Budget = serde_json::from_value(original).unwrap();
    restored
        .register_command(&select(&tid), Command::MoveAccount(target.clone()))
        .unwrap_err();
    let third = restored
        .add_account("Third", "EUR", true, 0, "2026-01-01")
        .unwrap();
    let peer_id = restored
        .transactions
        .iter()
        .find(|t| t.account == target)
        .unwrap()
        .id
        .clone();
    restored
        .register_command(&select(&peer_id), Command::MoveAccount(third.clone()))
        .unwrap();
    assert_eq!(
        restored
            .transactions
            .iter()
            .find(|t| t.id == tid)
            .unwrap()
            .splits[1]
            .transfer
            .as_ref()
            .unwrap()
            .account,
        third
    );
    assert!(!restored.can_register_command(&select(&tid), Command::Match));
    let mut replaced = restored.clone();
    replaced
        .register_command(
            &select(&tid),
            Command::Categorize {
                category: Some(category),
                replace_splits: true,
            },
        )
        .unwrap();
    assert_eq!(replaced.transactions.len(), 1);
    assert!(!replaced.transactions[0].has_transfers());
    restored
        .register_command(&select(&peer_id), Command::Delete)
        .unwrap();
    assert_eq!(restored.transactions.len(), 2);
    let parent = restored.transactions.iter().find(|t| t.id == tid).unwrap();
    assert_eq!(parent.splits.len(), 2);
    assert_eq!(parent.amount, -3000);
    assert_eq!(restored.balance(&outside, None, false), 1000);
}

#[test]
fn split_schedule_posts_skips_edits_and_moves_the_whole_group_from_either_side() {
    let (mut b, a, target, outside) = fixture();
    let category = b.categories[0].id.clone();
    let mut t = row(
        &a,
        vec![
            Split {
                amount: 10000,
                income_month: Some("2026-02".into()),
                ..Default::default()
            },
            transfer(&target, -3000, None),
            transfer(&outside, -1000, Some(category)),
        ],
    );
    t.set_date("2026-01-31".into());
    b.add_split_schedule(t, Repeat::Monthly).unwrap();
    let sid = b
        .scheduled
        .iter()
        .find(|s| s.transaction.account == a)
        .unwrap()
        .id
        .clone();
    let peer = b
        .scheduled
        .iter()
        .find(|s| s.transaction.account == target)
        .unwrap()
        .id
        .clone();
    b.skip_scheduled(&peer).unwrap();
    assert!(b.scheduled.iter().all(|s| s.next_date == "2026-02-28"));
    assert_eq!(
        b.scheduled
            .iter()
            .find(|s| s.id == sid)
            .unwrap()
            .transaction
            .splits[0]
            .income_month
            .as_deref(),
        Some("2026-03")
    );
    let mut t = b
        .scheduled
        .iter()
        .find(|s| s.id == peer)
        .unwrap()
        .transaction
        .clone();
    t.amount = 4000;
    t.set_date("2026-03-31".into());
    b.edit_scheduled(&peer, t, Repeat::Monthly, None).unwrap();
    let parent = &b
        .scheduled
        .iter()
        .find(|s| s.id == sid)
        .unwrap()
        .transaction;
    assert_eq!(parent.amount, 5000);
    assert_eq!(parent.splits[1].amount, -4000);
    let third = b
        .add_account("Third", "EUR", true, 0, "2026-01-01")
        .unwrap();
    b.move_scheduled_account(&peer, &third).unwrap();
    let old = snapshot(&b);
    b.move_scheduled_account(&peer, &a).unwrap_err();
    assert_eq!(snapshot(&b), old);
    b.enter_scheduled_now(&peer).unwrap();
    assert_eq!(b.transactions.len(), 3);
    assert_eq!(b.balance(&third, None, false), 4000);
    assert!(b.transactions.iter().all(|t| t.date == today()));
    assert!(b.scheduled.iter().all(|s| s.next_date == "2026-04-30"));
    store::validate(&b).unwrap();
    let (mut b, a, target, _) = fixture();
    let t = row(&a, vec![transfer(&target, -3000, None)]);
    let tid = t.id.clone();
    b.put_transaction(t).unwrap();
    b.register_command(
        &select(&tid),
        Command::Schedule {
            date: "2027-01-31".into(),
            repeat: Repeat::Monthly,
            move_original: true,
        },
    )
    .unwrap();
    assert!(b.transactions.is_empty());
    assert_eq!(b.scheduled.len(), 2);
    let peer = b
        .scheduled
        .iter()
        .find(|s| s.transaction.account == target)
        .unwrap()
        .id
        .clone();
    b.skip_scheduled(&peer).unwrap();
    assert!(b.scheduled.iter().all(|s| s.next_date == "2027-02-28"));
}

#[test]
fn zero_net_foreign_parts_freeze_income_and_assign_rounding_once() {
    let (mut b, _, _, _) = fixture();
    b.add_snapshot("2026-01", "USD", 90_000_000, "2026-01-01", "test")
        .unwrap();
    let a = b
        .add_account("Dollars", "USD", true, 0, "2026-01-01")
        .unwrap();
    let category = b.categories[0].id.clone();
    let t = row(
        &a,
        vec![
            Split {
                amount: 101,
                income_month: Some("2026-02".into()),
                ..Default::default()
            },
            Split {
                amount: -101,
                category: Some(category.clone()),
                ..Default::default()
            },
            Split::default(),
        ],
    );
    b.put_transaction(t).unwrap();
    assert!(b.transactions[0].snapshot.is_some());
    assert_eq!(
        b.split_base_amounts(&b.transactions[0]).unwrap(),
        vec![91, -91, 0]
    );
    assert_eq!(b.deferred_income("2026-01"), 91);
    assert_eq!(b.activity("2026-01", &category), -91);
    assert_eq!(b.available("2026-01").unwrap(), 0);
    for invalid in 0..5 {
        let before = snapshot(&b);
        let mut t = b.transactions[0].clone();
        match invalid {
            0 => t.splits[0].income_month = Some("2026-03".into()),
            1 => t.splits[0].category = Some(category.clone()),
            2 => t.splits[0].amount += 1,
            3 => {
                t.splits[0].transfer = Some(SplitTransfer {
                    link: id(),
                    account: a.clone(),
                })
            }
            _ => t.splits[0].category = Some("missing".into()),
        }
        assert!(b.put_transaction(t).is_err());
        assert_eq!(snapshot(&b), before);
    }
}

#[test]
fn split_to_split_links_preserve_other_parts_and_propagate_date_through_all_peers() {
    let (mut b, a, target, outside) = fixture();
    let category = b.categories[0].id.clone();
    let t = row(
        &a,
        vec![
            transfer(&target, -3000, None),
            Split {
                amount: -100,
                category: Some(category.clone()),
                ..Default::default()
            },
        ],
    );
    let tid = t.id.clone();
    b.put_transaction(t).unwrap();
    let mut peer = b
        .transactions
        .iter()
        .find(|t| t.account == target)
        .unwrap()
        .clone();
    let link = peer.transfer.take().unwrap();
    peer.splits = vec![
        Split {
            amount: 3000,
            transfer: Some(SplitTransfer {
                link,
                account: a.clone(),
            }),
            ..Default::default()
        },
        transfer(&outside, -1000, Some(category.clone())),
        Split {
            amount: -200,
            category: Some(category.clone()),
            ..Default::default()
        },
    ];
    peer.amount = 1800;
    let pid = peer.id.clone();
    b.put_transaction(peer).unwrap();
    let mut t = b.transactions.iter().find(|t| t.id == tid).unwrap().clone();
    t.splits[0].amount = -4000;
    t.amount = -4100;
    t.set_date("2026-02-28".into());
    b.put_transaction(t).unwrap();
    let peer = b.transactions.iter().find(|t| t.id == pid).unwrap();
    assert_eq!(peer.amount, 2800);
    assert_eq!(peer.splits[0].amount, 4000);
    assert!(b.transactions.iter().all(|t| t.date == "2026-02-28"));
    b.delete_transaction(&tid).unwrap();
    let peer = b.transactions.iter().find(|t| t.id == pid).unwrap();
    assert_eq!(peer.splits.len(), 2);
    assert_eq!(peer.amount, -1200);
    assert_eq!(b.activity("2026-02", &category), -1200);
    store::validate(&b).unwrap();
}
