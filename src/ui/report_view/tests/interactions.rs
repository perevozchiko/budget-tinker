use super::*;

fn render(f: &mut ReportFixture, events: Vec<egui::Event>) -> egui::FullOutput {
    f.ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(1440., 900.),
            )),
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| f.app.reports(ui));
        },
    )
}

fn click(f: &mut ReportFixture, label: &str) {
    render(f, vec![]);
    let output = render(f, vec![]);
    let pos = text_shapes(&output.shapes)
        .iter()
        .rev()
        .find(|(text, _)| text == label)
        .unwrap_or_else(|| {
            panic!(
                "missing {label}; visible: {:?}",
                text_shapes(&output.shapes)
                    .iter()
                    .map(|(s, _)| s)
                    .collect::<Vec<_>>()
            )
        })
        .1
        + Vec2::splat(3.);
    for pressed in [true, false] {
        render(
            f,
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
    render(f, vec![]);
}

fn escape(f: &mut ReportFixture) {
    render(
        f,
        vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
}

#[test]
fn every_report_uses_month_panels_with_immediate_presets_and_confirmed_custom_ranges() {
    let mut f = report_fixture();
    f.app.b.categories.truncate(3);
    f.app.store.save(&f.app.b).unwrap();
    let original = serde_json::to_value(&f.app.b).unwrap();
    for (index, title) in reports::TITLES.iter().enumerate() {
        if index != 0 {
            click(&mut f, title);
        }
        assert_eq!(f.app.report, index);
        // Fixed custom dates keep this interaction test independent of the wall clock.
        f.app.report_period = 6;
        f.app.report_filter.from = "2026-07-01".into();
        f.app.report_filter.through = "2026-10-31".into();
        click(&mut f, "Jul 2026 – Oct 2026");
        click(&mut f, "July");
        click(&mut f, "August");
        assert_eq!(f.app.report_period, 6);
        click(&mut f, "Cancel");
        assert_eq!(f.app.report_filter.from, "2026-07-01");
        click(&mut f, "Jul 2026 – Oct 2026");
        click(&mut f, "July");
        click(&mut f, "August");
        click(&mut f, "Done");
        assert_eq!(f.app.report_period, 6);
        assert_eq!(f.app.report_filter.from, "2026-08-01");
        assert_eq!(f.app.report_filter.through, "2026-10-31");
        let saved = f.app.preferences.reports.clone();
        let label = "Aug 2026 – Oct 2026";
        click(&mut f, label);
        click(&mut f, "October");
        click(&mut f, "February");
        // An inverted range cannot be applied or persisted.
        click(&mut f, "Done");
        assert_eq!(f.app.preferences.reports, saved);
        escape(&mut f);
        click(&mut f, label);
        click(&mut f, "This Month");
        assert_eq!(f.app.report_period, 0);
        assert_eq!(f.app.report_filter.from, format!("{}-01", &today()[..7]));
        assert!(
            !text_shapes(&render(&mut f, vec![]).shapes)
                .iter()
                .any(|(s, _)| s == "Done")
        );
    }
    assert_eq!(
        serde_json::to_value(store::read_budget(&f.app.store.path).unwrap()).unwrap(),
        original
    );
    assert!(f.app.undo.is_empty());
}

#[test]
fn every_report_applies_account_groups_and_spending_reports_confirm_payees_and_categories() {
    let mut f = report_fixture();
    f.app.b.categories.truncate(3);
    f.app.store.save(&f.app.b).unwrap();
    let original = serde_json::to_value(&f.app.b).unwrap();
    for (index, title) in reports::TITLES.iter().enumerate() {
        if index != 0 {
            click(&mut f, title);
        }
        click(
            &mut f,
            if index == 4 {
                "All Accounts"
            } else {
                "Budget Accounts"
            },
        );
        click(&mut f, "Select None");
        click(&mut f, "Off-Budget Accounts");
        click(&mut f, "Done");
        assert_eq!(
            f.app.report_filter.accounts,
            Selection::Selected(vec![f.app.b.accounts[1].id.clone()])
        );
        if index == 4 {
            assert_eq!(f.app.report_data().unwrap().total, 50000);
        }
        if index < 4 {
            click(&mut f, "All Payees");
            click(&mut f, "Select None");
            click(&mut f, "Shop");
            click(&mut f, "Done");
            assert_eq!(
                f.app.report_filter.payee,
                Selection::Selected(vec!["Shop".into()])
            );
            click(&mut f, "All Categories");
            click(&mut f, "Select None");
            click(&mut f, "Uncategorized Transactions");
            click(&mut f, "Done");
            assert_eq!(
                f.app.report_filter.category,
                Selection::Selected(vec![budgettinker::model::UNCATEGORIZED_CATEGORY.into()])
            );
            assert_eq!(f.app.report_data().unwrap().total, 0);
        } else {
            click(&mut f, "All Categories");
            assert!(
                !text_shapes(&render(&mut f, vec![]).shapes)
                    .iter()
                    .any(|(s, _)| s == "Done")
            );
            assert_eq!(f.app.report_filter.category, Selection::All);
            click(&mut f, "All Payees");
            assert_eq!(f.app.report_filter.payee, Selection::All);
        }
    }
    assert_eq!(
        serde_json::to_value(store::read_budget(&f.app.store.path).unwrap()).unwrap(),
        original
    );
    assert!(f.app.undo.is_empty());
}

#[test]
fn tab_filters_survive_switching_and_restart_without_changing_budget_data() {
    let mut f = report_fixture();
    click(&mut f, "All Dates");
    click(&mut f, "This Month");
    let first = f.app.report_filter.clone();
    click(&mut f, reports::TITLES[2]);
    assert_eq!(f.app.report_period, 5);
    click(&mut f, "All Dates");
    click(&mut f, "This Year");
    let trends = f.app.report_filter.clone();
    click(&mut f, reports::TITLES[0]);
    assert_eq!(f.app.report_period, 0);
    assert_eq!(f.app.report_filter, first);
    let path = f.app.store.path.clone();
    let mut restarted = App::new(
        &eframe::CreationContext::_new_kittest(f.ctx.clone()),
        Store::open(&path).unwrap(),
        store::read_budget(&path).unwrap(),
        false,
    );
    restarted.switch_report(2);
    assert_eq!(restarted.report_period, 4);
    assert_eq!(restarted.report_filter, trends);
}
