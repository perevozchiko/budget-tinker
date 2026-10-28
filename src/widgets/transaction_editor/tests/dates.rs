use super::*;

#[test]
fn date_typing_and_tab_are_identical_in_both_registers() {
    for scheduled in [false, true] {
        let (b, account) = budget();
        let ctx = date_context();
        let mut d = Draft::new(&b, Some(&account), None).unwrap();
        d.outflow = "1".into();
        date_frame(&ctx, &mut d, &b, scheduled, vec![]);
        for ch in "02/29/2024".chars() {
            date_frame(
                &ctx,
                &mut d,
                &b,
                scheduled,
                vec![egui::Event::Text(ch.to_string())],
            );
            assert_eq!(d.date_input.text().chars().last(), Some(ch));
            if d.date_input.text().len() < 8 {
                assert!(d.build(&b).is_err());
                assert!(d.build_scheduled(&b).is_err());
            }
        }
        assert_eq!(d.date, "2024-02-29");
        assert_eq!(d.date_input.text(), "02/29/2024");
        date_frame(&ctx, &mut d, &b, scheduled, vec![date_key(egui::Key::Tab)]);
        assert_eq!(d.date_input.text(), "02/29/24");
        assert_eq!(d.build(&b).unwrap().date, "2024-02-29");
        assert_eq!(d.build_scheduled(&b).unwrap().date, "2024-02-29");
        date_frame(
            &ctx,
            &mut d,
            &b,
            scheduled,
            vec![egui::Event::Text("Coffee".into())],
        );
        assert_eq!(d.payee, "Coffee", "Tab must go to payee in either register");
    }
}

#[test]
fn tab_preserves_invalid_dates_and_never_saves_a_stale_date() {
    for scheduled in [false, true] {
        for input in ["02/", "02/29/26", "02/28/2"] {
            let (b, account) = budget();
            let ctx = date_context();
            let mut d = Draft::new(&b, Some(&account), None).unwrap();
            d.outflow = "1".into();
            date_frame(&ctx, &mut d, &b, scheduled, vec![]);
            date_frame(
                &ctx,
                &mut d,
                &b,
                scheduled,
                vec![egui::Event::Text(input.into())],
            );
            date_frame(&ctx, &mut d, &b, scheduled, vec![date_key(egui::Key::Tab)]);
            assert_eq!(d.date_input.text(), input);
            assert!(d.build(&b).is_err());
            assert!(d.build_scheduled(&b).is_err());
            date_frame(
                &ctx,
                &mut d,
                &b,
                scheduled,
                vec![egui::Event::Text("Coffee".into())],
            );
            assert_eq!(d.payee, "Coffee");
        }
    }
}

#[test]
fn both_calendars_select_dates_and_escape_preserves_the_draft() {
    for scheduled in [false, true] {
        let (b, account) = budget();
        let ctx = date_context();
        let mut d = Draft::new(&b, Some(&account), None).unwrap();
        d.date = "2024-02-01".into();
        let output = date_frame(&ctx, &mut d, &b, scheduled, vec![]);
        let calendar = date_text_position(&output, "▦");
        let click = |pos| {
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ]
        };
        date_frame(&ctx, &mut d, &b, scheduled, click(calendar));
        let output = date_frame(&ctx, &mut d, &b, scheduled, vec![]);
        assert!(egui::Popup::is_any_open(&ctx));
        let next = date_text_position(&output, "▶");
        date_frame(&ctx, &mut d, &b, scheduled, click(next));
        let output = date_frame(&ctx, &mut d, &b, scheduled, vec![]);
        let previous = date_text_position(&output, "◀");
        date_frame(&ctx, &mut d, &b, scheduled, click(previous));
        let output = date_frame(&ctx, &mut d, &b, scheduled, vec![]);
        let day = date_text_position(&output, "29");
        date_frame(&ctx, &mut d, &b, scheduled, click(day));
        assert_eq!(d.date, "2024-02-29");
        assert_eq!(d.date_input.text(), "02/29/24");
        assert!(!egui::Popup::is_any_open(&ctx));
        date_frame(&ctx, &mut d, &b, scheduled, vec![]);
        date_frame(&ctx, &mut d, &b, scheduled, click(calendar));
        date_frame(&ctx, &mut d, &b, scheduled, vec![]);
        assert!(egui::Popup::is_any_open(&ctx), "calendar must reopen");
        date_frame(
            &ctx,
            &mut d,
            &b,
            scheduled,
            vec![date_key(egui::Key::Escape)],
        );
        assert!(!egui::Popup::is_any_open(&ctx));
        assert_eq!(d.date, "2024-02-29");
    }
}

#[test]
fn schedule_rejects_invalid_dates_and_splits() {
    let (b, a) = budget();
    let mut draft = Draft::new(&b, Some(&a), None).unwrap();
    draft.outflow = "30".into();
    draft.date = "2026-99-15".into();
    assert!(draft.build_scheduled(&b).is_err());
    draft.date = format!("{}-15", month_shift(&current_month(), 1));
    draft.splits = vec![(b.categories[0].id.clone(), "-10".into())];
    assert!(draft.build_scheduled(&b).is_err());
    draft.splits[0].1 = "-30".into();
    assert!(draft.build_scheduled(&b).is_ok());
}
#[test]
fn income_month_survives_inline_edits_date_changes_and_recategorization() {
    let (mut b, a) = budget();
    let legacy = Draft::new(&b, None, Some(&b.transactions[0].id)).unwrap();
    assert_eq!(legacy.income_next, Some(false));
    let mut d = Draft::new(&b, Some(&a), None).unwrap();
    d.date = "2025-12-20".into();
    d.inflow = "100".into();
    d.income_next = Some(true);
    let t = d.build(&b).unwrap();
    assert_eq!(t.income_month.as_deref(), Some("2026-01"));
    let tid = t.id.clone();
    b.put_transaction(t).unwrap();
    let mut edit = Draft::new(&b, None, Some(&tid)).unwrap();
    assert_eq!(edit.income_next, Some(true));
    edit.date = "2026-01-20".into();
    assert_eq!(
        edit.build(&b).unwrap().income_month.as_deref(),
        Some("2026-02")
    );
    edit.category = b.categories[0].id.clone();
    assert!(edit.build(&b).unwrap().income_month.is_none());
    edit.category.clear();
    edit.income_next = Some(false);
    assert_eq!(
        edit.build(&b).unwrap().income_month.as_deref(),
        Some("2026-01")
    );
}
