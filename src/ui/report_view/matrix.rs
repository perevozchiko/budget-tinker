use super::*;

pub(super) fn matrix_open_id(key: &str) -> egui::Id {
    egui::Id::new(("income_matrix_open", key))
}

pub(super) fn matrix_line(
    ui: &mut egui::Ui,
    label: &str,
    values: &[String],
    width: f32,
    strong: bool,
    fill: Color32,
    indent: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 23.), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0., fill);
    let font = if strong {
        crate::palette::bold(12.)
    } else {
        FontId::proportional(12.)
    };
    let label_rect =
        egui::Rect::from_min_max(rect.min, egui::pos2(rect.left() + 220., rect.bottom()));
    let disclosure = label
        .strip_prefix("▸ ")
        .map(|label| (false, label))
        .or_else(|| label.strip_prefix("▾ ").map(|label| (true, label)));
    if let Some((open, _)) = disclosure {
        let center = label_rect.left_center() + Vec2::new(9., 0.);
        let points = if open {
            vec![
                center + Vec2::new(-4., -2.),
                center + Vec2::new(4., -2.),
                center + Vec2::new(0., 3.),
            ]
        } else {
            vec![
                center + Vec2::new(-2., -4.),
                center + Vec2::new(-2., 4.),
                center + Vec2::new(3., 0.),
            ]
        };
        ui.painter()
            .add(egui::Shape::convex_polygon(points, INK, Stroke::NONE));
    }
    ui.painter()
        .with_clip_rect(label_rect.intersect(ui.clip_rect()))
        .text(
            label_rect.left_center()
                + Vec2::new(
                    if indent {
                        24.
                    } else if disclosure.is_some() {
                        20.
                    } else {
                        6.
                    },
                    0.,
                ),
            egui::Align2::LEFT_CENTER,
            disclosure.map_or(label, |(_, label)| label),
            font.clone(),
            INK,
        );
    let cell_width = (width - 220.) / values.len().max(1) as f32;
    for (index, value) in values.iter().enumerate() {
        ui.painter().text(
            egui::pos2(
                rect.left() + 220. + cell_width * (index + 1) as f32 - 8.,
                rect.center().y,
            ),
            egui::Align2::RIGHT_CENTER,
            value,
            font.clone(),
            INK,
        );
    }
    response.on_hover_text(label);
    ui.interact(label_rect, ui.id().with(label), egui::Sense::click())
}

pub(super) fn income_table(ui: &mut egui::Ui, report: &Report, currency: &str) {
    ui.horizontal(|ui| {
        for (label, open) in [("Expand All", true), ("Collapse All", false)] {
            if ui.add(classic::Button::new(label).small()).clicked() {
                for row in &report.matrix {
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(matrix_open_id(&row.key), open));
                }
            }
        }
    });
    let width = ui
        .available_width()
        .max(220. + (report.months.len() + 2) as f32 * 118.);
    egui::ScrollArea::horizontal()
        .id_salt("income_matrix_horizontal")
        .show(ui, |ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing.y = 0.;
            let mut headers: Vec<_> = report
                .months
                .iter()
                .map(|m| report_month(&m.month))
                .collect();
            headers.extend(["Average".into(), "Total".into()]);
            matrix_line(
                ui,
                "Category / Source",
                &headers,
                width,
                true,
                Color32::from_gray(240),
                false,
            );
            egui::ScrollArea::vertical()
                .id_salt("income_matrix_vertical")
                .show(ui, |ui| {
                    for row in &report.matrix {
                        if row.key == "income"
                            || report.matrix.get(2).is_some_and(|r| r.key == row.key)
                        {
                            let label = if row.key == "income" {
                                "Income"
                            } else {
                                "Expenses"
                            };
                            matrix_line(
                                ui,
                                label,
                                &[],
                                width,
                                true,
                                Color32::from_gray(245),
                                false,
                            );
                        }
                        let open = ui.ctx().data_mut(|data| {
                            data.get_temp::<bool>(matrix_open_id(&row.key))
                                .unwrap_or(false)
                        });
                        let label = if row.children.is_empty() {
                            i18n::tr(ui.ctx(), &row.label)
                        } else {
                            format!(
                                "{} {}",
                                if open { "▾" } else { "▸" },
                                i18n::tr(ui.ctx(), &row.label)
                            )
                        };
                        let values = |r: &reports::MatrixRow| {
                            r.values
                                .iter()
                                .copied()
                                .chain([r.average(), r.total()])
                                .map(|v| i18n::money(v, currency))
                                .collect::<Vec<_>>()
                        };
                        let fill = match row.key.as_str() {
                            "total-income" => Color32::from_rgb(237, 244, 229),
                            "total-expenses" => Color32::from_rgb(255, 237, 237),
                            "net-income" => Color32::from_gray(232),
                            _ => Color32::WHITE,
                        };
                        ui.push_id(&row.key, |ui| {
                            if matrix_line(ui, &label, &values(row), width, true, fill, false)
                                .clicked()
                                && !row.children.is_empty()
                            {
                                ui.ctx().data_mut(|data| {
                                    data.insert_temp(matrix_open_id(&row.key), !open)
                                });
                            }
                            if open {
                                for child in &row.children {
                                    matrix_line(
                                        ui,
                                        &i18n::tr(ui.ctx(), &child.label),
                                        &values(child),
                                        width,
                                        false,
                                        Color32::from_gray(250),
                                        true,
                                    );
                                }
                            }
                        });
                    }
                });
        });
}
