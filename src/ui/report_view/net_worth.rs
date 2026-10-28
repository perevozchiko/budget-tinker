use super::*;

pub(super) fn net_worth_chart(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    report: &Report,
    currency: &str,
    selected_month: Option<&str>,
) -> Option<String> {
    let p = ui.painter().with_clip_rect(rect);
    let chart = egui::Rect::from_min_max(
        rect.min + Vec2::new(90., 20.),
        rect.max - Vec2::new(25., 40.),
    );
    if chart.width() <= 0. || chart.height() <= 0. {
        return None;
    }
    for (index, (name, color)) in [("Assets", GREEN), ("Debts", RED), ("Net Worth", TEAL)]
        .into_iter()
        .enumerate()
    {
        let x = chart.left() + index as f32 * 86.;
        p.rect_filled(
            egui::Rect::from_min_size(egui::pos2(x, chart.top() - 17.), Vec2::new(9., 9.)),
            1.,
            color,
        );
        p.text(
            egui::pos2(x + 13., chart.top() - 12.),
            egui::Align2::LEFT_CENTER,
            name,
            FontId::proportional(10.),
            INK,
        );
    }
    let low = report
        .months
        .iter()
        .map(|m| m.debts.min(m.net_worth))
        .min()
        .unwrap_or(0)
        .min(0) as f64;
    let high = report
        .months
        .iter()
        .map(|m| m.assets.max(m.net_worth))
        .max()
        .unwrap_or(0)
        .max(0) as f64;
    let span = (high - low).max(1.);
    let y = |value: f64| chart.bottom() - ((value - low) / span) as f32 * chart.height();
    for tick in 0..=4 {
        let value = low + span * tick as f64 / 4.;
        let top = y(value);
        p.hline(
            chart.x_range(),
            top,
            Stroke::new(1_f32, Color32::from_gray(226)),
        );
        p.text(
            egui::pos2(chart.left() - 8., top),
            egui::Align2::RIGHT_CENTER,
            i18n::money(value.round() as i64, currency),
            FontId::proportional(10.),
            INK,
        );
    }
    let count = report.months.len().max(1);
    let slot = chart.width() / count as f32;
    let mut previous = None;
    let mut clicked = None;
    for (index, month) in report.months.iter().enumerate() {
        let x = chart.left() + slot * (index as f32 + 0.5);
        for (value, left, right, color) in [
            (month.assets, -0.36, -0.03, GREEN),
            (month.debts, 0.03, 0.36, RED),
        ] {
            let bar = egui::Rect::from_min_max(
                egui::pos2(x + slot * left, y(value as f64).min(y(0.))),
                egui::pos2(x + slot * right, y(value as f64).max(y(0.))),
            );
            p.rect_filled(bar, 1., color);
        }
        let point = egui::pos2(x, y(month.net_worth as f64));
        if let Some(prev) = previous {
            p.line_segment([prev, point], Stroke::new(2.5_f32, TEAL));
        }
        p.circle_filled(point, 3.5, TEAL);
        previous = Some(point);
        if selected_month == Some(month.month.as_str()) {
            p.circle_stroke(point, 6., Stroke::new(1.5_f32, TEAL));
        }
        if index.is_multiple_of(count.div_ceil(8)) {
            p.text(
                egui::pos2(x, chart.bottom() + 15.),
                egui::Align2::CENTER_CENTER,
                report_month(&month.month),
                FontId::proportional(10.),
                INK,
            );
        }
        let hit = egui::Rect::from_min_max(
            egui::pos2(x - slot * 0.5, chart.top()),
            egui::pos2(x + slot * 0.5, chart.bottom()),
        );
        let response = ui
            .interact(
                hit,
                ui.id().with(("report_point", index)),
                egui::Sense::click(),
            )
            .on_hover_text(format!(
                "{}\nAssets: {}\nDebts: {}\nNet Worth: {}",
                report_month(&month.month),
                i18n::money(month.assets, currency),
                i18n::money(month.debts, currency),
                i18n::money(month.net_worth, currency),
            ));
        if response.clicked() {
            clicked = Some(month.month.clone());
        }
    }
    clicked
}
