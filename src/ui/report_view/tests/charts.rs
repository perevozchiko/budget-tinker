use super::*;

#[test]
fn net_worth_chart_shows_signed_components_and_selects_month() {
    let ctx = egui::Context::default();
    let mut fonts = egui::FontDefinitions::default();
    fonts.families.insert(
        egui::FontFamily::Name("arial-bold".into()),
        fonts.families[&egui::FontFamily::Proportional].clone(),
    );
    ctx.set_fonts(fonts);
    let mut budget = Budget::new("USD");
    budget
        .add_account("Checking", "USD", true, 10_000, "2026-01-01")
        .unwrap();
    budget
        .add_account("Card", "USD", true, -4_000, "2026-01-01")
        .unwrap();
    let report = reports::calculate(
        &budget,
        4,
        &reports::Filter {
            from: "2026-01-01".into(),
            through: "2026-02-28".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let render = |events| {
        let mut selected = None;
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(800., 500.),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let rect = ui.available_rect_before_wrap();
                    selected = net_worth_chart(ui, rect, &report, "USD", None);
                });
            },
        );
        let has_debt_bar = output.shapes.iter().any(|shape| {
            matches!(&shape.shape, egui::Shape::Rect(rect) if rect.fill == RED && rect.rect.height() > 20.)
        });
        (selected, text_shapes(&output.shapes), has_debt_bar)
    };
    render(vec![]);
    let (_, labels, has_debt_bar) = render(vec![]);
    assert!(has_debt_bar);
    assert!(labels.iter().any(|(label, _)| label == "Jan 2026"));
    assert!(labels.iter().any(|(label, _)| label == "Feb 2026"));
    let point = egui::pos2(600., 220.);
    let (selected, _, _) = render(vec![
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
    ]);
    assert_eq!(selected.as_deref(), Some("2026-02"));
    assert_eq!(
        (report.months[1].assets, report.months[1].debts),
        (10_000, -4_000)
    );
}

#[test]
fn donut_other_click_detail_and_back_preserve_period_and_colors() {
    let ctx = egui::Context::default();
    let mut fonts = egui::FontDefinitions::default();
    fonts.families.insert(
        egui::FontFamily::Name("arial-bold".into()),
        fonts.families[&egui::FontFamily::Proportional].clone(),
    );
    ctx.set_fonts(fonts);
    let mut b = Budget::new("USD");
    let account = b
        .add_account("Checking", "USD", true, 0, "2026-01-01")
        .unwrap();
    let category = b.categories[0].id.clone();
    for i in 0..12 {
        b.put_transaction(Transaction {
            id: id(),
            account: account.clone(),
            date: "2026-01-20".into(),
            payee: format!("Payee {i:02}"),
            amount: -(1200 - i * 100),
            base_amount: 0,
            category: Some(category.clone()),
            snapshot: None,
            income_month: None,
            memo: String::new(),
            splits: vec![],
            transfer: None,
            operation_fx: None,
            check_number: None,
            flag: None,
            cleared: false,
            reconciled: false,
        })
        .unwrap();
    }
    let filter = reports::Filter {
        from: "2026-01-01".into(),
        through: "2026-02-28".into(),
        ..Default::default()
    };
    let report = reports::calculate(&b, 1, &filter).unwrap();
    let rows = report.spending_rows(None, None);
    let stable = spending_color(&rows[0]);
    let mut path = vec![];
    let render = |path: &mut Vec<Drill>, events, size| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    spending_navigation(ui, path);
                    if let Some(current) = path.last().cloned() {
                        if let Some(next) = spending_detail(ui, &report, &current, "USD") {
                            path.push(next);
                        }
                    } else {
                        let rect = ui.available_rect_before_wrap();
                        if let Some(index) = donut(ui, rect, &rows, "USD") {
                            path.push(Drill {
                                scope: rows[index].scope.clone(),
                                label: rows[index].label.clone(),
                                month: None,
                            });
                        }
                    }
                });
            },
        )
    };
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
    let size = Vec2::new(680., 430.);
    render(&mut path, vec![], size);
    let output = render(&mut path, vec![], size);
    let texts = text_shapes(&output.shapes);
    assert!(texts.iter().any(|(t, _)| t.starts_with("All Other")));
    // Click the Other wedge, independent of callout truncation.
    let other = rows.last().unwrap();
    let positive: i64 = rows.iter().map(|r| r.value).sum();
    let angle = -std::f32::consts::FRAC_PI_2
        + std::f32::consts::TAU * (1. - other.value as f32 / positive as f32 * 0.5);
    let rect = egui::Rect::from_min_max(egui::pos2(8., 8.), egui::pos2(672., 422.));
    let radius = (rect.width() * 0.25).min((rect.height() - 70.) * 0.5);
    let point = rect.center() + Vec2::angled(angle) * radius * 0.75;
    render(&mut path, click(point), size);
    assert!(matches!(
        path.last().unwrap().scope,
        SpendingScope::Other(_)
    ));
    let output = render(&mut path, vec![], Vec2::new(420., 330.));
    let texts = text_shapes(&output.shapes);
    assert!(texts.iter().any(|(t, _)| t == "3 payees"));
    let payee = texts.iter().find(|(t, _)| t == "Payee 09").unwrap().1 + Vec2::splat(3.);
    render(&mut path, click(payee), Vec2::new(420., 330.));
    assert_eq!(path.len(), 2);
    let output = render(&mut path, vec![], size);
    assert!(
        text_shapes(&output.shapes)
            .iter()
            .any(|(t, _)| t == "2026-01-20")
    );
    for expected in [1, 0] {
        let output = render(&mut path, vec![], size);
        let pos = text_shapes(&output.shapes)
            .iter()
            .find(|(t, _)| t == "Back")
            .unwrap()
            .1
            + Vec2::splat(3.);
        render(&mut path, click(pos), size);
        assert_eq!(path.len(), expected);
    }
    assert_eq!(spending_color(&rows[0]), stable);
    assert_eq!(report.from, filter.from);
    assert_eq!(report.through, filter.through);
}

#[test]
fn spending_chart_draws_positive_and_refund_segments_with_stable_legend_and_tooltip() {
    let ctx = egui::Context::default();
    let mut budget = Budget::new("USD");
    let account = budget
        .add_account("Checking", "USD", true, 0, "2026-01-01")
        .unwrap();
    let category = budget.categories[0].id.clone();
    budget.categories[0].group = "Travel".into();
    for (date, amount) in [("2026-01-20", -750), ("2026-02-10", 200)] {
        budget
            .put_transaction(Transaction {
                id: id(),
                account: account.clone(),
                date: date.into(),
                payee: "Trip".into(),
                amount,
                base_amount: 0,
                category: Some(category.clone()),
                snapshot: None,
                income_month: None,
                memo: String::new(),
                splits: vec![],
                transfer: None,
                operation_fx: None,
                check_number: None,
                flag: None,
                cleared: false,
                reconciled: false,
            })
            .unwrap();
    }
    let report = reports::calculate(
        &budget,
        2,
        &reports::Filter {
            from: "2026-01-01".into(),
            through: "2026-02-28".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let render = |size, time, events| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let rect = ui.available_rect_before_wrap();
                    spending_trend(ui, rect, &report, "USD");
                });
            },
        )
    };
    let size = Vec2::new(800., 500.);
    render(size, 0., vec![]);
    let output = render(size, 0.1, vec![]);
    let texts = text_shapes(&output.shapes);
    for label in ["Travel", "7.50 USD", "-2.00 USD"] {
        assert!(texts.iter().any(|(text, _)| text == label), "{label}");
    }
    let segments = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect)
                if rect.fill == group_color("Travel") && rect.rect.width() > 20. =>
            {
                Some(rect.rect)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(segments.len(), 2);
    assert!((segments[0].bottom() - segments[1].top()).abs() < 0.1);
    let pointer = segments[1].center();
    render(size, 0.2, vec![egui::Event::PointerMoved(pointer)]);
    render(size, 1.5, vec![]);
    let hovered = render(size, 1.6, vec![]);
    assert!(
        text_shapes(&hovered.shapes)
            .iter()
            .any(|(text, _)| text.contains("Travel: -2.00 USD")
                && text.contains("Total Spending: -2.00 USD")),
        "{:?}",
        text_shapes(&hovered.shapes)
    );
    let mut selected = None;
    let click = segments[0].center();
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
            events: vec![
                egui::Event::PointerMoved(click),
                egui::Event::PointerButton {
                    pos: click,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::PointerButton {
                    pos: click,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let rect = ui.available_rect_before_wrap();
                selected = spending_trend(ui, rect, &report, "USD");
            });
        },
    );
    assert_eq!(selected, Some(("Travel".into(), "2026-01".into())));
    let small = render(Vec2::new(420., 330.), 2., vec![]);
    assert!(
        text_shapes(&small.shapes)
            .iter()
            .any(|(text, _)| text == "Travel")
    );
    assert_eq!(
        group_color("Travel"),
        group_color(&report.spending_series[0].label)
    );
    let mut crowded = report.clone();
    for label in [
        "Everyday Life",
        "Fun & Giving",
        "Health & Personal",
        "Hidden Categories",
        "Home & Bills",
        "True Expenses",
    ] {
        let mut series = report.spending_series[0].clone();
        series.key = label.into();
        series.label = label.into();
        series.values = vec![0, 0];
        crowded.spending_series.push(series);
    }
    let wrapped = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(420., 330.),
            )),
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let rect = ui.available_rect_before_wrap();
                spending_trend(ui, rect, &crowded, "USD");
            });
        },
    );
    for shape in &wrapped.shapes {
        if let egui::Shape::Text(text) = &shape.shape
            && crowded
                .spending_series
                .iter()
                .any(|s| s.label == text.galley.job.text)
        {
            assert!(
                text.pos.x + text.galley.size().x <= 412.,
                "Legend overflow: {}",
                text.galley.job.text
            );
        }
    }
}
