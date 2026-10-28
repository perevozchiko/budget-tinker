use super::*;

#[test]
fn category_keys_select_without_saving_regular_or_scheduled_rows() {
    fn frame(
        ctx: &egui::Context,
        draft: &mut Draft,
        b: &Budget,
        scheduled: bool,
        events: Vec<egui::Event>,
    ) -> (egui::FullOutput, Action, Option<egui::Id>) {
        let mut action = Action::None;
        let mut memo = None;
        let output = ctx.run(
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
                    memo = Some(ui.id().with(("new_transaction", "memo")));
                    let widths = [28., 0., 110., 170., 150., 140., 79., 79., 31., 28.];
                    action = if scheduled {
                        draft.show_scheduled(ui, &widths, b, &mut Repeat::Monthly)
                    } else {
                        draft.show(ui, &widths, b)
                    };
                });
            },
        );
        (output, action, memo)
    }
    for (scheduled, accept) in [false, true]
        .into_iter()
        .flat_map(|scheduled| [egui::Key::Tab, egui::Key::Enter].map(|key| (scheduled, key)))
    {
        let (mut b, account) = budget();
        let group = b.group_names()[0].clone();
        let coffee = b.add_category(&group, "Coffee test").unwrap();
        let ctx = egui::Context::default();
        let mut fonts = egui::FontDefinitions::default();
        fonts.families.insert(
            egui::FontFamily::Name("arial-bold".into()),
            fonts.families[&egui::FontFamily::Proportional].clone(),
        );
        ctx.set_fonts(fonts);
        let mut draft = Draft::new(&b, Some(&account), None).unwrap();
        frame(&ctx, &mut draft, &b, scheduled, vec![]);
        let output = frame(&ctx, &mut draft, &b, scheduled, vec![]).0;
        let pos = output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape
                    && text.galley.text() == "Uncategorized  ▾"
                {
                    return Some(text.pos + text.galley.size() * 0.5);
                }
                None
            })
            .expect("category trigger");
        for pressed in [true, false] {
            frame(
                &ctx,
                &mut draft,
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
        frame(&ctx, &mut draft, &b, scheduled, vec![]);
        // Escape dismisses the menu, while the transaction draft stays open.
        let (_, action, _) = frame(
            &ctx,
            &mut draft,
            &b,
            scheduled,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(action == Action::None);
        frame(
            &ctx,
            &mut draft,
            &b,
            scheduled,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        for pressed in [true, false] {
            frame(
                &ctx,
                &mut draft,
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
        frame(&ctx, &mut draft, &b, scheduled, vec![]);
        let (_, action, _) = frame(
            &ctx,
            &mut draft,
            &b,
            scheduled,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            }],
        );
        assert!(
            action == Action::None,
            "Command+Enter in a category menu must not save"
        );
        frame(
            &ctx,
            &mut draft,
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
        frame(
            &ctx,
            &mut draft,
            &b,
            scheduled,
            vec![egui::Event::Text("Coffee test".into())],
        );
        let (_, action, memo) = frame(
            &ctx,
            &mut draft,
            &b,
            scheduled,
            vec![egui::Event::Key {
                key: accept,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(
            action == Action::None,
            "Choosing a category must not save the row"
        );
        assert_eq!(draft.category, coffee, "scheduled={scheduled}");
        if accept == egui::Key::Tab {
            assert_eq!(ctx.memory(|m| m.focused()), memo, "Tab must focus memo");
        }
    }
}
