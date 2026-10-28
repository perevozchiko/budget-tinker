use super::*;

pub(super) fn spending_trend(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    report: &Report,
    currency: &str,
) -> Option<(String, String)> {
    let mut selected = None;
    ui.horizontal_wrapped(|ui| {
        for series in &report.spending_series {
            let label = i18n::tr(ui.ctx(), &series.label);
            let galley = ui
                .painter()
                .layout_no_wrap(label.clone(), FontId::proportional(11.), INK);
            let width = (galley.size().x + 20.).min(ui.max_rect().width());
            let (item, response) =
                ui.allocate_exact_size(Vec2::new(width, 18.), egui::Sense::hover());
            let painter = ui.painter().with_clip_rect(item.intersect(ui.clip_rect()));
            painter.rect_filled(
                egui::Rect::from_center_size(
                    item.left_center() + Vec2::new(5., 0.),
                    Vec2::splat(10.),
                ),
                1.,
                group_color(&series.label),
            );
            painter.galley(
                item.left_center() + Vec2::new(15., -galley.size().y * 0.5),
                galley,
                INK,
            );
            response.on_hover_text(label);
        }
    });
    let top = ui.cursor().top().max(rect.top()) + 20.;
    let height = rect.bottom() - top - 42.;
    if height < 40. || rect.width() < 170. {
        return None;
    }
    // Give long periods a minimum column width instead of overlapping labels.
    let width = (rect.width() - 8.).max(110. + report.months.len() as f32 * 85.);
    egui::ScrollArea::horizontal()
        .id_salt("spending_trend_horizontal")
        .show(ui, |ui| {
            let (canvas, _) =
                ui.allocate_exact_size(Vec2::new(width, height + 62.), egui::Sense::hover());
            let chart = egui::Rect::from_min_max(
                canvas.min + Vec2::new(90., 20.),
                canvas.max - Vec2::new(15., 42.),
            );
            let mut low = 0_i64;
            let mut high = 0_i64;
            for index in 0..report.months.len() {
                low = low.min(
                    report
                        .spending_series
                        .iter()
                        .map(|s| s.values[index].min(0))
                        .sum(),
                );
                high = high.max(
                    report
                        .spending_series
                        .iter()
                        .map(|s| s.values[index].max(0))
                        .sum(),
                );
            }
            let span = (high as f64 - low as f64).max(1.);
            let y =
                |v: i64| chart.bottom() - ((v as f64 - low as f64) / span) as f32 * chart.height();
            let p = ui
                .painter()
                .with_clip_rect(canvas.intersect(ui.clip_rect()));
            for tick in 0..=4 {
                let value = (low as f64 + span * tick as f64 / 4.).round() as i64;
                p.hline(
                    chart.x_range(),
                    y(value),
                    Stroke::new(1_f32, Color32::from_gray(226)),
                );
                p.text(
                    egui::pos2(chart.left() - 8., y(value)),
                    egui::Align2::RIGHT_CENTER,
                    i18n::money(value, currency),
                    FontId::proportional(10.),
                    INK,
                );
            }
            p.hline(chart.x_range(), y(0), Stroke::new(1_f32, Color32::GRAY));
            let slot = chart.width() / report.months.len().max(1) as f32;
            for (index, month) in report.months.iter().enumerate() {
                let x = chart.left() + slot * (index as f32 + 0.5);
                let mut positive = 0_i64;
                let mut negative = 0_i64;
                for series in &report.spending_series {
                    let value = series.values[index];
                    if value == 0 {
                        continue;
                    }
                    let start = if value > 0 {
                        &mut positive
                    } else {
                        &mut negative
                    };
                    let end = *start + value;
                    let bar = egui::Rect::from_min_max(
                        egui::pos2(x - slot * 0.28, y(*start).min(y(end))),
                        egui::pos2(x + slot * 0.28, y(*start).max(y(end))),
                    );
                    p.rect_filled(bar, 0., group_color(&series.label));
                    let hit = ui.interact(
                        bar.intersect(ui.clip_rect()),
                        ui.id().with(("spending_segment", index, &series.key)),
                        egui::Sense::click(),
                    );
                    if hit.clicked() {
                        selected = Some((series.label.clone(), month.month.clone()));
                    }
                    hit.on_hover_text(format!(
                        "{}\n{}: {}\nTotal Spending: {}",
                        month_name(ui.ctx(), &month.month),
                        i18n::tr(ui.ctx(), &series.label),
                        i18n::money(value, currency),
                        i18n::money(month.expense, currency)
                    ));
                    *start = end;
                }
                p.text(
                    egui::pos2(x, chart.bottom() + 17.),
                    egui::Align2::CENTER_CENTER,
                    report_month(&month.month),
                    FontId::proportional(10.),
                    INK,
                );
                p.text(
                    egui::pos2(x, y(positive) - 9.),
                    egui::Align2::CENTER_BOTTOM,
                    i18n::money(month.expense, currency),
                    FontId::proportional(10.),
                    INK,
                );
            }
            if report.spending_series.is_empty() {
                p.text(
                    chart.center(),
                    egui::Align2::CENTER_CENTER,
                    "No spending for these filters.",
                    FontId::proportional(12.),
                    INK,
                );
            }
        });
    selected
}
