use super::*;

pub(super) fn spending_navigation(ui: &mut egui::Ui, path: &mut Vec<Drill>) {
    if !path.is_empty() {
        ui.horizontal_wrapped(|ui| {
            if ui.button("Back").clicked() {
                path.pop();
            }
            if ui.button("All Spending").clicked() {
                path.clear();
            }
            for step in path.iter() {
                ui.label(format!(
                    "/ {}{}",
                    step.label,
                    step.month
                        .as_ref()
                        .map(|m| format!(" ({})", report_month(m)))
                        .unwrap_or_default()
                ));
            }
        });
    }
}

pub(super) fn spending_color(row: &SpendingRow) -> Color32 {
    match &row.scope {
        SpendingScope::Other(_) => Color32::from_rgb(135, 145, 150),
        SpendingScope::Group(name) => group_color(name),
        SpendingScope::Category(id) => group_color(&format!("category:{id:?}")),
        SpendingScope::Payee(name) => group_color(&format!("payee:{name}")),
    }
}

pub(super) fn spending_detail(
    ui: &mut egui::Ui,
    report: &Report,
    current: &Drill,
    currency: &str,
) -> Option<Drill> {
    let entries = report.spending_details(&current.scope, current.month.as_deref());
    let total: i64 = entries.iter().map(|e| e.value).sum();
    ui.label(format!(
        "{} — {}",
        current.label,
        i18n::money(total, currency)
    ));
    let mut next = None;
    if matches!(current.scope, SpendingScope::Other(_)) {
        let rows = report.spending_rows(Some(&current.scope), current.month.as_deref());
        ui.label(format!("{} payees", rows.len()));
        egui::ScrollArea::vertical()
            .id_salt("other_payees")
            .show(ui, |ui| {
                egui::Grid::new("other_payees_grid")
                    .min_col_width(80.)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("Payee");
                        ui.strong("Spending");
                        ui.end_row();
                        for row in rows {
                            ui.horizontal(|ui| {
                                let (marker, _) =
                                    ui.allocate_exact_size(Vec2::splat(8.), egui::Sense::hover());
                                ui.painter().circle_filled(
                                    marker.center(),
                                    3.,
                                    spending_color(&row),
                                );
                                if ui.selectable_label(false, &row.label).clicked() {
                                    next = Some(Drill {
                                        scope: row.scope.clone(),
                                        label: row.label.clone(),
                                        month: current.month.clone(),
                                    });
                                }
                            });
                            ui.label(i18n::money(row.value, currency));
                            ui.end_row();
                        }
                    });
            });
    } else {
        ui.label(format!(
            "{} spending entries · amounts in {}",
            entries.len(),
            currency
        ));
        egui::ScrollArea::both()
            .id_salt("spending_transactions")
            .show(ui, |ui| {
                egui::Grid::new("spending_details_grid")
                    .striped(true)
                    .min_col_width(80.)
                    .show(ui, |ui| {
                        for title in ["Date", "Account", "Payee", "Category", "Memo", "Spending"] {
                            ui.strong(title);
                        }
                        ui.end_row();
                        for entry in entries {
                            ui.label(&entry.date);
                            ui.label(&entry.account);
                            ui.label(if entry.payee.is_empty() {
                                "No Payee"
                            } else {
                                &entry.payee
                            });
                            ui.label(format!("{}: {}", entry.group, entry.category_label));
                            ui.label(&entry.memo);
                            ui.label(i18n::money(entry.value, currency));
                            ui.end_row();
                        }
                    });
            });
    }
    next
}

pub(super) fn donut(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    rows: &[SpendingRow],
    currency: &str,
) -> Option<usize> {
    let p = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    let radius = (rect.width() * 0.25)
        .min((rect.height() - 70.) * 0.5)
        .max(25.);
    let center = rect.center();
    let positive: i64 = rows.iter().map(|r| r.value.max(0)).sum();
    let total: i64 = rows.iter().map(|r| r.value).sum();
    let mut angle = -std::f32::consts::FRAC_PI_2;
    let response = ui.interact(rect, ui.id().with("report_donut"), egui::Sense::click());
    let mut selected = None;
    let mut callouts = vec![];
    if positive == 0 {
        p.circle_stroke(
            center,
            radius * 0.75,
            Stroke::new(radius * 0.5, Color32::from_gray(238)),
        );
    }
    for (index, row) in rows.iter().enumerate().filter(|(_, r)| r.value > 0) {
        let sweep = row.value as f32 / positive.max(1) as f32 * std::f32::consts::TAU;
        let steps = (sweep * 60.).ceil().max(2.) as usize;
        let mut mesh = egui::Mesh::default();
        for step in 0..=steps {
            let t = angle + sweep * step as f32 / steps as f32;
            for r in [radius * 0.50, radius] {
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: center + Vec2::angled(t) * r,
                    uv: egui::epaint::WHITE_UV,
                    color: spending_color(row),
                });
            }
            if step > 0 {
                let v = (step * 2) as u32;
                mesh.indices
                    .extend_from_slice(&[v - 2, v - 1, v, v, v - 1, v + 1]);
            }
        }
        p.add(egui::Shape::mesh(mesh));
        let direction = Vec2::angled(angle);
        p.line_segment(
            [
                center + direction * radius * 0.5,
                center + direction * radius,
            ],
            Stroke::new(1.5_f32, Color32::WHITE),
        );
        let direction = Vec2::angled(angle + sweep * 0.5);
        callouts.push((index, direction, center.y + direction.y * (radius + 12.)));
        if let Some(pointer) = response.hover_pos() {
            let delta = pointer - center;
            let t = (delta.angle() + std::f32::consts::FRAC_PI_2).rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::FRAC_PI_2;
            if delta.length() >= radius * 0.5
                && delta.length() <= radius
                && t >= angle
                && t <= angle + sweep
            {
                response.clone().on_hover_text(format!(
                    "{}: {}",
                    row.label,
                    i18n::money(row.value, currency)
                ));
                if response.clicked() {
                    selected = Some(index);
                }
            }
        }
        angle += sweep;
    }
    for right in [false, true] {
        let mut side: Vec<_> = callouts
            .iter()
            .filter(|(_, d, _)| (d.x >= 0.) == right)
            .collect();
        side.sort_by(|a, b| a.2.total_cmp(&b.2));
        let spacing = 38_f32.min((rect.height() - 36.) / side.len().max(1) as f32);
        let mut previous = rect.top() + 16. - spacing;
        for (position, (index, direction, desired_y)) in side.iter().enumerate() {
            let max_y = rect.bottom() - 28. - (side.len() - position - 1) as f32 * spacing;
            let y = desired_y.max(previous + spacing).min(max_y);
            previous = y;
            let row = &rows[*index];
            let x = if right {
                center.x + radius + 14.
            } else {
                center.x - radius - 14.
            };
            let point = egui::pos2(x, y);
            p.line_segment(
                [center + *direction * radius * 0.95, point],
                Stroke::new(1_f32, Color32::GRAY),
            );
            let width = (rect.width() * 0.25 - 18.).max(12.);
            let mut label = row.label.clone();
            while p
                .layout_no_wrap(label.clone(), FontId::proportional(11.), INK)
                .size()
                .x
                > width
                && label.chars().count() > 2
            {
                label.pop();
            }
            if label != row.label {
                label.pop();
                label.push('…');
            }
            let label_rect = egui::Rect::from_min_size(
                egui::pos2(if right { x } else { x - width }, y - 17.),
                Vec2::new(width, 34.),
            );
            let text_painter =
                p.with_clip_rect(label_rect.intersect(rect).intersect(ui.clip_rect()));
            text_painter.text(
                point,
                if right {
                    egui::Align2::LEFT_CENTER
                } else {
                    egui::Align2::RIGHT_CENTER
                },
                format!(
                    "{}\n{} ({:.0}%)",
                    label,
                    i18n::money(row.value, currency),
                    row.value as f64 / positive as f64 * 100.
                ),
                FontId::proportional(11.),
                INK,
            );
            let hit = ui.interact(
                label_rect,
                ui.id().with(("spending_callout", index)),
                egui::Sense::click(),
            );
            if hit.clicked() {
                selected = Some(*index);
            }
            hit.on_hover_text(format!(
                "{}: {}",
                row.label,
                i18n::money(row.value, currency)
            ));
        }
    }
    p.text(
        center - Vec2::new(0., 9.),
        egui::Align2::CENTER_CENTER,
        "Total Spending",
        FontId::proportional(14.),
        INK,
    );
    p.text(
        center + Vec2::new(0., 12.),
        egui::Align2::CENTER_CENTER,
        i18n::money(total, currency),
        crate::palette::bold(17.),
        INK,
    );
    if rows.iter().any(|r| r.value < 0) {
        p.text(
            rect.center_bottom(),
            egui::Align2::CENTER_BOTTOM,
            "Refunds reduce the total; the chart shows positive spending.",
            FontId::proportional(11.),
            Color32::GRAY,
        );
    }
    selected
}
