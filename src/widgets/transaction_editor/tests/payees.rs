use super::*;

#[test]
fn payee_completion_uses_rules_preferences_and_account_history() {
    let (mut b, account, other, category) = payee_fixture();
    let preferred = b.categories[1].id.clone();
    b.payee_rules.insert(
        "Corner Coffee".into(),
        PayeeRule {
            category: Some(preferred.clone()),
            rename_contains: "CARD CORNER".into(),
            ..Default::default()
        },
    );
    let prefs = crate::preferences::Preferences {
        autocomplete_memo: true,
        autocomplete_amount: true,
        ..Default::default()
    };
    let mut d = Draft::new(&b, Some(&account), None).unwrap();
    d.payee = "card corner #123".into();
    d.complete_payee = true;
    d.autocomplete(&b, &prefs);
    assert_eq!(d.payee, "Corner Coffee");
    assert_eq!(d.category, preferred);
    assert_ne!(d.category, category);
    assert_eq!(d.memo, "latest");
    assert_eq!(d.outflow, "3.50");
    d.account = other;
    d.autocomplete(&b, &prefs);
    assert_eq!(d.outflow, "9.00");
    d.account = b
        .add_account("No history", "EUR", true, 0, &today())
        .unwrap();
    d.autocomplete(&b, &prefs);
    assert!(d.outflow.is_empty() && d.inflow.is_empty());
    d.outflow = "123".into();
    d.account = account;
    d.autocomplete(&b, &prefs);
    assert_eq!(d.outflow, "123", "manual amount survives account changes");
}

#[test]
fn disabled_completion_hidden_payees_and_splits_keep_manual_fields() {
    let (mut b, account, _, category) = payee_fixture();
    let mut d = Draft::new(&b, Some(&account), None).unwrap();
    d.payee = "Corner Coffee".into();
    d.memo = "manual".into();
    d.outflow = "12".into();
    d.complete_payee = true;
    let prefs = crate::preferences::Preferences {
        autocomplete_category: false,
        ..Default::default()
    };
    d.autocomplete(&b, &prefs);
    assert!(d.category.is_empty());
    assert_eq!(d.memo, "manual");
    assert_eq!(d.outflow, "12");
    b.payee_rules.insert(
        d.payee.clone(),
        PayeeRule {
            listed: false,
            category: Some(category.clone()),
            ..Default::default()
        },
    );
    d.complete_payee = true;
    d.autocomplete(&b, &crate::preferences::Preferences::default());
    assert!(d.category.is_empty());
    b.payee_rules.clear();
    d.splits = vec![(category, "-12".into())];
    d.complete_payee = true;
    d.autocomplete(&b, &crate::preferences::Preferences::default());
    assert!(d.category.is_empty());
    assert_eq!(d.splits.len(), 1);
    d.splits.clear();
    d.category = b.categories[0].id.clone();
    b.transactions.last_mut().unwrap().category = None;
    d.complete_payee = true;
    d.autocomplete(&b, &crate::preferences::Preferences::default());
    assert!(
        d.category.is_empty(),
        "uncategorized history must not keep the previous payee's category"
    );
}

#[test]
fn rule_only_payee_gets_category_without_history() {
    let (mut b, account) = budget();
    let category = b.categories[0].id.clone();
    b.payee_rules.insert(
        "New shop".into(),
        PayeeRule {
            category: Some(category.clone()),
            ..Default::default()
        },
    );
    let mut d = Draft::new(&b, Some(&account), None).unwrap();
    d.payee = "New shop".into();
    d.complete_payee = true;
    d.autocomplete(&b, &crate::preferences::Preferences::default());
    assert_eq!(d.category, category);
}

#[test]
fn payee_keys_complete_regular_and_scheduled_rows_without_saving() {
    for scheduled in [false, true] {
        for accept in [egui::Key::Enter, egui::Key::Tab] {
            let (b, account, _, category) = payee_fixture();
            let mut d = Draft::new(&b, Some(&account), None).unwrap();
            d.focus = false;
            let ctx = egui::Context::default();
            let mut fonts = egui::FontDefinitions::default();
            fonts.families.insert(
                egui::FontFamily::Name("arial-bold".into()),
                fonts.families[&egui::FontFamily::Proportional].clone(),
            );
            ctx.set_fonts(fonts);
            let mut frame = |events: Vec<egui::Event>, focus: bool| {
                let input_description = format!("{events:?}");
                let mut action = Action::None;
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            egui::Pos2::ZERO,
                            Vec2::new(1200., 700.),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            if focus {
                                ui.memory_mut(|m| {
                                    m.request_focus(ui.id().with(("new_transaction", "payee")))
                                });
                            }
                            let widths = [28., 0., 110., 170., 150., 140., 79., 79., 31., 28.];
                            action = if scheduled {
                                d.show_scheduled(ui, &widths, &b, &mut Repeat::Monthly)
                            } else {
                                d.show(ui, &widths, &b)
                            };
                            d.autocomplete(&b, &crate::preferences::Preferences::default());
                        });
                    },
                );
                assert!(
                    action == Action::None,
                    "payee keys must not save/cancel: {input_description}"
                );
            };
            frame(vec![], true);
            frame(vec![egui::Event::Text("Cor".into())], false);
            frame(vec![], false);
            for (key, modifiers) in [
                (egui::Key::Enter, egui::Modifiers::COMMAND),
                (egui::Key::Escape, egui::Modifiers::NONE),
            ] {
                frame(
                    vec![egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers,
                    }],
                    false,
                );
                frame(
                    vec![egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: false,
                        repeat: false,
                        modifiers,
                    }],
                    false,
                );
            }
            frame(vec![egui::Event::Text("n".into())], false);
            frame(vec![], false);
            for key in [egui::Key::ArrowDown, egui::Key::ArrowUp, accept] {
                frame(
                    vec![egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                    false,
                );
                frame(
                    vec![egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: false,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                    false,
                );
            }
            assert_eq!(d.payee, "Corner Coffee");
            assert_eq!(d.category, category);
            assert!(
                d.build(&b).is_err(),
                "completion did not fill disabled amount"
            );
        }
    }
}

#[test]
fn split_payee_keyboard_completion_keeps_the_editor_open_in_both_registers() {
    for scheduled in [false, true] {
        let (b, account, _, _) = payee_fixture();
        let ctx = date_context();
        let mut d = Draft::new(&b, Some(&account), None).unwrap();
        d.focus = false;
        d.date = "2026-01-31".into();
        d.outflow = "1".into();
        d.splits = vec![(String::new(), "-1".into())];
        d.split_details = vec![SplitDetails {
            payee: "Cor".into(),
            ..Default::default()
        }];
        date_frame(&ctx, &mut d, &b, scheduled, vec![]);
        let out = date_frame(&ctx, &mut d, &b, scheduled, vec![]);
        let pos = date_text_position(&out, "Cor");
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
        date_frame(&ctx, &mut d, &b, scheduled, vec![date_key(egui::Key::End)]);
        date_frame(
            &ctx,
            &mut d,
            &b,
            scheduled,
            vec![egui::Event::Text("n".into())],
        );
        date_frame(&ctx, &mut d, &b, scheduled, vec![]);
        date_frame(&ctx, &mut d, &b, scheduled, vec![date_key(egui::Key::Tab)]);
        assert_eq!(d.split_details[0].payee, "Corner Coffee");
        let t = if scheduled {
            d.build_scheduled(&b)
        } else {
            d.build(&b)
        }
        .unwrap();
        assert_eq!(t.splits[0].payee.as_deref(), Some("Corner Coffee"));
    }
}
