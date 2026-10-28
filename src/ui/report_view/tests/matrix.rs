use super::*;

#[test]
fn matrix_mouse_expansion_bulk_controls_and_small_window_rendering() {
    let ctx = egui::Context::default();
    let mut fonts = egui::FontDefinitions::default();
    fonts.families.insert(
        egui::FontFamily::Name("arial-bold".into()),
        fonts.families[&egui::FontFamily::Proportional].clone(),
    );
    ctx.set_fonts(fonts);
    let mut budget = Budget::new("USD");
    let account = budget
        .add_account("Checking", "USD", true, 1001, "2026-01-01")
        .unwrap();
    let category = budget.categories[0].id.clone();
    budget
        .put_transaction(Transaction {
            id: id(),
            account,
            date: "2026-01-15".into(),
            payee: "Shop".into(),
            amount: -301,
            base_amount: 0,
            category: Some(category),
            snapshot: None,
            income_month: None,
            memo: String::new(),
            splits: vec![],
            transfer: None,
            check_number: None,
            flag: None,
            cleared: false,
            reconciled: false,
        })
        .unwrap();
    let filter = reports::Filter {
        from: "2026-01-01".into(),
        through: "2026-02-28".into(),
        ..Default::default()
    };
    let report = reports::calculate(&budget, 3, &filter).unwrap();
    let render = |events, size| {
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| income_table(ui, &report, "USD"));
            },
        );
        text_shapes(&output.shapes)
    };
    let size = Vec2::new(1100., 600.);
    render(vec![], size);
    let initial = render(vec![], size);
    let child = &i18n::tr(&ctx, &report.matrix[2].children[0].label);
    assert!(!initial.iter().any(|(text, _)| text == child));
    assert!(initial.iter().any(|(text, _)| text == "Average"));
    assert!(initial.iter().any(|(text, _)| text == "Total"));
    let click = |position| {
        vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    };
    let group_position = initial
        .iter()
        .find(|(text, _)| text == &i18n::tr(&ctx, &report.matrix[2].label))
        .unwrap()
        .1
        + Vec2::splat(4.);
    render(click(group_position), size);
    let expanded = render(vec![], size);
    assert!(expanded.iter().any(|(text, _)| text == child));
    let collapse = expanded
        .iter()
        .find(|(text, _)| text == "Collapse All")
        .unwrap()
        .1
        + Vec2::splat(4.);
    render(click(collapse), size);
    assert!(!render(vec![], size).iter().any(|(text, _)| text == child));
    let expand = initial
        .iter()
        .find(|(text, _)| text == "Expand All")
        .unwrap()
        .1
        + Vec2::splat(4.);
    render(click(expand), size);
    let all = render(vec![], size);
    assert!(all.iter().any(|(text, _)| text == child));
    assert!(all.iter().any(|(text, _)| text == "Starting Balance"));
    render(vec![], Vec2::new(600., 350.));
    assert!(
        ctx.data_mut(|data| data.get_temp::<bool>(matrix_open_id(&report.matrix[2].key)))
            .unwrap()
    );
    assert_eq!(report.total, 700);
}
