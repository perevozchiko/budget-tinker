use super::*;

#[test]
fn category_popup_keeps_draft_and_supports_group_done_cancel_and_dismissal() {
    let ctx = egui::Context::default();
    let mut fonts = egui::FontDefinitions::default();
    crate::palette::ensure_font_families(&mut fonts);
    ctx.set_fonts(fonts);
    let mut budget = Budget::new("USD");
    budget.categories.truncate(3);
    for (index, category) in budget.categories.iter_mut().enumerate() {
        category.group = if index < 2 { "A Group" } else { "B Group" }.into();
        category.name = format!("Category {index}");
    }
    budget.categories[2].hidden = true;
    let mut selection = Selection::All;
    let render = |selection: &mut Selection, events| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(800., 600.),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    multi_select_combo(
                        ui,
                        "test_categories",
                        "Categories",
                        selection,
                        false,
                        |ui, draft| {
                            category_selection_items(ui, draft, &budget);
                        },
                    );
                });
            },
        )
    };
    let click = |point| {
        vec![
            egui::Event::PointerMoved(point),
            egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    };
    let click_text = |selection: &mut Selection, text: &str| {
        render(selection, vec![]);
        let output = render(selection, vec![]);
        let point = text_shapes(&output.shapes)
            .iter()
            .find(|(label, _)| label == text)
            .unwrap_or_else(|| panic!("missing {text}"))
            .1
            + Vec2::splat(3.);
        render(selection, click(point));
    };
    click_text(&mut selection, "Categories");
    click_text(&mut selection, "Select None");
    assert_eq!(selection, Selection::All);
    click_text(&mut selection, "A Group");
    click_text(&mut selection, "Done");
    assert_eq!(
        selection,
        Selection::Selected(
            budget.categories[..2]
                .iter()
                .map(|c| c.id.clone())
                .collect()
        )
    );
    let applied = selection.clone();
    click_text(&mut selection, "Categories");
    click_text(&mut selection, "Category 0"); // partial group
    click_text(&mut selection, "A Group"); // partial -> all
    click_text(&mut selection, "Done");
    assert_eq!(selection, applied);
    click_text(&mut selection, "Categories");
    click_text(&mut selection, "Uncategorized Transactions");
    click_text(&mut selection, "Cancel");
    assert_eq!(selection, applied);
    click_text(&mut selection, "Categories");
    click_text(&mut selection, "Select All");
    render(
        &mut selection,
        vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert_eq!(selection, applied);
    click_text(&mut selection, "Categories");
    click_text(&mut selection, "Select None");
    render(&mut selection, click(egui::pos2(700., 550.)));
    assert_eq!(selection, applied);
    click_text(&mut selection, "Categories");
    click_text(&mut selection, "Select None");
    click_text(&mut selection, "Uncategorized Transactions");
    click_text(&mut selection, "Done");
    assert_eq!(
        selection,
        Selection::Selected(vec![budgettinker::model::UNCATEGORIZED_CATEGORY.into()])
    );
    click_text(&mut selection, "Categories");
    click_text(&mut selection, "Select None");
    click_text(&mut selection, "Hidden Categories");
    click_text(&mut selection, "Done");
    assert_eq!(
        selection,
        Selection::Selected(vec![budget.categories[2].id.clone()])
    );
}

#[test]
fn checkbox_toggles_work_from_all_and_none() {
    let items = choices();
    let mut selection = Selection::All;
    toggle_selection_item(&mut selection, "savings", false, &items);
    assert_eq!(
        selection,
        Selection::Selected(vec!["checking".into(), "tracking".into()])
    );
    toggle_selection_item(&mut selection, "checking", false, &items);
    assert_eq!(selection, Selection::Selected(vec!["tracking".into()]));
    toggle_selection_item(&mut selection, "tracking", false, &items);
    assert_eq!(selection, Selection::NoneSelected);
    toggle_selection_item(&mut selection, "savings", true, &items);
    assert_eq!(selection, Selection::Selected(vec!["savings".into()]));
    toggle_selection_item(&mut selection, "tracking", true, &items);
    assert_eq!(
        selection,
        Selection::Selected(vec!["savings".into(), "tracking".into()])
    );
    toggle_selection_item(&mut selection, "tracking", true, &items);
    assert_eq!(
        selection,
        Selection::Selected(vec!["savings".into(), "tracking".into()])
    );
}

#[test]
fn budget_accounts_toggles_materialize_only_on_budget_ids() {
    let items = choices();
    let mut selection = Selection::BudgetAccounts;
    assert!(selection_item_checked(&selection, "checking", true));
    assert!(selection_item_checked(&selection, "savings", true));
    assert!(!selection_item_checked(&selection, "tracking", false));
    toggle_selection_item(&mut selection, "tracking", true, &items);
    assert_eq!(
        selection,
        Selection::Selected(vec!["checking".into(), "savings".into(), "tracking".into(),])
    );
    selection = Selection::BudgetAccounts;
    toggle_selection_item(&mut selection, "checking", false, &items);
    assert_eq!(selection, Selection::Selected(vec!["savings".into()]));
    selection = Selection::All;
    assert!(
        items
            .iter()
            .all(|(id, _, on_budget)| selection_item_checked(&selection, id, *on_budget))
    );
    selection = Selection::NoneSelected;
    assert!(
        items
            .iter()
            .all(|(id, _, on_budget)| !selection_item_checked(&selection, id, *on_budget))
    );
}

#[test]
fn toggling_discards_stale_ids_and_keeps_valid_items() {
    let items = choices();
    let mut selection = Selection::Selected(vec!["removed".into(), "checking".into()]);
    toggle_selection_item(&mut selection, "savings", true, &items);
    assert_eq!(
        selection,
        Selection::Selected(vec!["checking".into(), "savings".into()])
    );
}

#[test]
fn report_tabs_remember_spending_and_net_worth_accounts_independently() {
    let mut accounts = ReportAccounts::default();
    let mut filter = reports::Filter {
        from: "2026-07-01".into(),
        through: "2026-10-31".into(),
        category: Selection::All,
        payee: Selection::All,
        ..Default::default()
    };
    accounts.switch(0, 4, &mut filter);
    assert_eq!(filter.accounts, Selection::All);
    filter.accounts = Selection::Selected(vec!["brokerage".into()]);
    accounts.switch(4, 1, &mut filter);
    assert_eq!(filter.accounts, Selection::BudgetAccounts);
    filter.accounts = Selection::Selected(vec!["checking".into()]);
    accounts.switch(1, 2, &mut filter);
    assert_eq!(
        filter.accounts,
        Selection::Selected(vec!["checking".into()])
    );
    accounts.switch(2, 4, &mut filter);
    assert_eq!(
        filter.accounts,
        Selection::Selected(vec!["brokerage".into()])
    );
    accounts.switch(4, 0, &mut filter);
    assert_eq!(
        filter.accounts,
        Selection::Selected(vec!["checking".into()])
    );
    assert_eq!(filter.from, "2026-07-01");
    assert_eq!(filter.through, "2026-10-31");
    assert_eq!(filter.category, Selection::All);
    assert_eq!(filter.payee, Selection::All);

    let mut reset = ReportAccounts::default();
    reset.switch(0, 4, &mut reports::Filter::default());
    assert_eq!(reset.net_worth, Selection::All);
}
