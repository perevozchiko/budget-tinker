use super::*;

#[test]
fn explicit_zero_and_mixed_splits_can_be_saved_and_edited() {
    let (mut b, a) = budget();
    let mut d = Draft::new(&b, Some(&a), None).unwrap();
    assert!(d.build(&b).is_err());
    d.inflow = "0".into();
    d.memo = "Zero net purchase and refund".into();
    d.splits = vec![
        (b.categories[0].id.clone(), "-25".into()),
        (b.categories[1].id.clone(), "25".into()),
        (b.categories[2].id.clone(), "0".into()),
    ];
    let t = d.build(&b).unwrap();
    let tid = t.id.clone();
    b.put_transaction(t).unwrap();
    let mut edit = Draft::new(&b, Some(&a), Some(&tid)).unwrap();
    edit.memo = "Updated memo".into();
    b.put_transaction(edit.build(&b).unwrap()).unwrap();
    let saved = b.transactions.iter().find(|t| t.id == tid).unwrap();
    assert_eq!(saved.amount, 0);
    assert_eq!(saved.memo, "Updated memo");
    assert_eq!(saved.splits[0].amount, -2500);
    assert_eq!(saved.splits[1].amount, 2500);
    assert_eq!(saved.splits[2].amount, 0);
    budgettinker::store::validate(&b).unwrap();
}
#[test]
fn split_transaction_round_trips_through_the_inline_row() {
    let (mut b, a) = budget();
    let mut d = Draft::new(&b, Some(&a), None).unwrap();
    d.outflow = "30".into();
    d.splits = vec![
        (b.categories[0].id.clone(), "-10".into()),
        (b.categories[1].id.clone(), "-20".into()),
    ];
    let t = d.build(&b).unwrap();
    let tid = t.id.clone();
    b.put_transaction(t).unwrap();
    let edit = Draft::new(&b, None, Some(&tid)).unwrap();
    assert_eq!(edit.splits.len(), 2);
    b.put_transaction(edit.build(&b).unwrap()).unwrap();
    assert_eq!(
        b.transactions
            .last()
            .unwrap()
            .splits
            .iter()
            .map(|s| s.amount)
            .sum::<i64>(),
        -3000
    );
}
#[test]
fn split_category_keyboard_selects_income_and_uncategorized_without_saving() {
    for scheduled in [false, true] {
        let (b, account) = budget();
        let ctx = date_context();
        let mut d = Draft::new(&b, Some(&account), None).unwrap();
        d.focus = false;
        d.date = "2026-01-31".into();
        d.inflow = "10".into();
        d.splits = vec![(String::new(), "10".into())];
        for (search, assigned) in [
            ("Income for February", Some("2026-02")),
            ("Uncategorized", None),
        ] {
            let out = date_frame(&ctx, &mut d, &b, scheduled, vec![]);
            let label = if d.split_details[0].income_next.is_some() {
                "Income for February  ▾"
            } else {
                "Uncategorized  ▾"
            };
            let pos = date_text_position(&out, label);
            for pressed in [true, false] {
                date_frame(
                    &ctx,
                    &mut d,
                    &b,
                    scheduled,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
            date_frame(&ctx, &mut d, &b, scheduled, vec![]);
            date_frame(
                &ctx,
                &mut d,
                &b,
                scheduled,
                vec![egui::Event::Text(search.into())],
            );
            date_frame(
                &ctx,
                &mut d,
                &b,
                scheduled,
                vec![date_key(egui::Key::Enter)],
            );
            let t = if scheduled {
                d.build_scheduled(&b)
            } else {
                d.build(&b)
            }
            .unwrap();
            assert_eq!(t.splits[0].income_month.as_deref(), assigned);
            assert!(t.splits[0].category.is_none());
            date_frame(
                &ctx,
                &mut d,
                &b,
                scheduled,
                vec![egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
        }
    }
}
