use super::*;

#[test]
fn graphical_print_uses_current_filters_drill_and_preview_without_changing_budget() {
    let mut fixture = report_fixture();
    let before = serde_json::to_value(&fixture.app.b).unwrap();
    fixture.app.page = Page::Reports;
    fixture.app.report_filter.payee = Selection::Selected(vec!["Shop".into()]);
    let category = fixture.app.b.categories[0].id.clone();
    fixture.app.report_filter.category = Selection::Selected(vec![category.clone()]);
    fixture.app.report_navigation.push(Drill {
        scope: SpendingScope::Category(Some(category)),
        label: "Chosen category".into(),
        month: Some("2026-10".into()),
    });
    let document = fixture.app.report_print_document().unwrap();
    let json = serde_json::to_string(&document).unwrap();
    assert!(json.contains("Payees: Shop"));
    assert!(json.contains("Accounts: Budget Accounts"));
    assert!(json.contains("Detail: Chosen category / 2026-10"));
    assert!(json.contains("$10.00"));
    assert!(!json.contains("500.00"));
    fixture.app.chrome.print_preview = true;
    for zoom in [0.5, 1.0, 2.0] {
        fixture.app.chrome.print_zoom = zoom;
        let _ = fixture.ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1050., 712.),
                )),
                ..Default::default()
            },
            |ctx| fixture.app.print_preview(ctx),
        );
    }
    fixture.app.report_filter.from = "2026-".into();
    assert!(fixture.app.report_print_document().is_err());
    assert_eq!(serde_json::to_value(&fixture.app.b).unwrap(), before);
    assert_eq!(
        serde_json::to_value(store::read_budget(&fixture.app.store.path).unwrap()).unwrap(),
        before
    );
}
