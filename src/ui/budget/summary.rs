use super::*;

impl App {
    pub(super) fn budget_summary(
        &mut self,
        ui: &mut egui::Ui,
        months: &[String],
        label_w: f32,
        col_w: f32,
    ) {
        let collapsed = self.preferences.budget_summary_collapsed;
        let height = if collapsed { 49. } else { 113. };
        let mut toggle = false;
        let (strip, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), height + 6.),
            egui::Sense::hover(),
        );
        let p = ui.painter().clone();
        for (i, month) in months.iter().enumerate() {
            let card = egui::Rect::from_min_size(
                egui::pos2(
                    strip.left() + label_w + 8. + i as f32 * (col_w + 8.),
                    strip.top(),
                ),
                Vec2::new(col_w, height),
            );
            classic::gradient(
                &p,
                card,
                5.,
                crate::palette::CARD_TOP,
                crate::palette::CARD_BOTTOM,
            );
            p.rect_stroke(
                card,
                5.,
                Stroke::new(1_f32, Color32::from_rgb(102, 109, 112)),
                egui::StrokeKind::Inside,
            );
            p.line_segment(
                [
                    card.min + Vec2::new(50., 1.),
                    egui::pos2(card.left() + 50., card.bottom() - 1.),
                ],
                Stroke::new(1_f32, Color32::from_rgb(47, 59, 67)),
            );
            let name = i18n::tr(ui.ctx(), MONTHS[month[5..].parse::<usize>().unwrap() - 1]);
            p.text(
                card.min + Vec2::new(8., 18.),
                egui::Align2::LEFT_CENTER,
                name.chars().take(3).collect::<String>(),
                crate::palette::bold(16.),
                Color32::WHITE,
            );
            p.text(
                card.min + Vec2::new(10., 34.),
                egui::Align2::LEFT_CENTER,
                &month[..4],
                crate::palette::bold(11.),
                Color32::LIGHT_GRAY,
            );
            if month == &current_month() {
                p.add(egui::Shape::convex_polygon(
                    vec![
                        card.min + Vec2::new(2., 2.),
                        card.min + Vec2::new(13., 2.),
                        card.min + Vec2::new(2., 13.),
                    ],
                    Color32::from_rgb(231, 207, 28),
                    Stroke::NONE,
                ));
            }
            let tools = egui::Rect::from_min_max(
                egui::pos2(card.right() - 22., card.top() + 1.),
                card.right_bottom() - Vec2::splat(1.),
            );
            p.rect_filled(tools, 0., Color32::from_white_alpha(9));
            if classic::icon_button(
                ui,
                egui::Rect::from_min_size(tools.min, Vec2::splat(21.)),
                6,
                Color32::from_gray(181),
            )
            .l_on_hover_text("Заметка к месяцу")
            .clicked()
            {
                self.open(DialogKind::Note(format!("month:{month}")));
            }
            let quick = classic::icon_button(
                ui,
                egui::Rect::from_min_size(tools.min + Vec2::new(0., 23.), Vec2::splat(21.)),
                7,
                Color32::from_gray(181),
            );
            #[cfg(test)]
            ui.ctx().data_mut(|d| {
                d.insert_temp(egui::Id::new(("month_quick_button", month)), quick.rect)
            });
            egui::Popup::menu(&quick).show(|ui| self.month_quick_menu(ui, month));
            let allocated: i64 = self
                .b
                .categories
                .iter()
                .map(|c| self.b.allocated(month, &c.id))
                .sum();
            let prev = month_shift(month, -1);
            let summary = self.b.month_summary(month).ok();
            let previous = summary.map(|s| s.not_budgeted);
            let available = self.b.available(month);
            let prev_name = i18n::tr(ui.ctx(), MONTHS[prev[5..].parse::<usize>().unwrap() - 1])
                .chars()
                .take(3)
                .collect::<String>();
            let short = name.chars().take(3).collect::<String>();
            let english = self.preferences.language == i18n::Language::En;
            let title_case = |s: String| {
                if english {
                    let mut c = s.chars();
                    format!(
                        "{}{}",
                        c.next().unwrap_or(' '),
                        c.as_str().to_ascii_lowercase()
                    )
                } else {
                    s
                }
            };
            let prev_name = title_case(prev_name);
            let short = title_case(short);
            let labels = if english {
                [
                    format!("Not Budgeted in {prev_name}"),
                    format!("Overspent in {prev_name}"),
                    format!("Income for {short}"),
                    format!("Budgeted in {short}"),
                ]
            } else {
                [
                    format!("Не назначено в {prev_name}"),
                    format!("Перерасход в {prev_name}"),
                    format!("Доход за {short}"),
                    format!("Назначено в {short}"),
                ]
            };
            if !collapsed {
                for (line, (value, label)) in [
                    previous,
                    Some(self.b.overspent(&prev)),
                    summary.map(|s| s.income),
                    Some(-allocated),
                ]
                .into_iter()
                .zip(labels)
                .enumerate()
                {
                    let y = card.top() + 12. + line as f32 * 13.;
                    let value = value
                        .map(|v| {
                            if line == 2 && v >= 0 {
                                format!("+{}", i18n::number(v, &self.b.base))
                            } else {
                                i18n::number(v, &self.b.base)
                            }
                        })
                        .unwrap_or_else(|| "—".into());
                    if line == 2 {
                        let response = ui
                            .interact(
                                egui::Rect::from_min_max(
                                    egui::pos2(card.left() + 51., y - 6.5),
                                    egui::pos2(tools.left(), y + 6.5),
                                ),
                                ui.id().with(("month_income", month)),
                                egui::Sense::click(),
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text("Show income transactions");
                        #[cfg(test)]
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new(("month_income", month)), response.rect)
                        });
                        egui::Popup::menu(&response).show(|ui| self.month_income_popup(ui, month));
                    }
                    p.text(
                        egui::pos2(card.left() + 128., y),
                        egui::Align2::RIGHT_CENTER,
                        value,
                        FontId::proportional(11.),
                        Color32::from_gray(198),
                    );
                    p.text(
                        egui::pos2(card.left() + 134., y),
                        egui::Align2::LEFT_CENTER,
                        label,
                        FontId::proportional(10.8),
                        Color32::from_gray(193),
                    );
                }
            }
            let amount_y = if collapsed { 18. } else { 82. };
            let label_y = if collapsed { 36. } else { 100. };
            let response = ui
                .interact(
                    egui::Rect::from_min_max(
                        egui::pos2(card.left() + 51., card.top() + amount_y - 14.),
                        egui::pos2(tools.left(), card.bottom()),
                    ),
                    ui.id().with(("summary_toggle", month)),
                    egui::Sense::click(),
                )
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text(if collapsed {
                    "Click to expand details"
                } else {
                    "Click to collapse details"
                });
            #[cfg(test)]
            ui.ctx().data_mut(|d| {
                d.insert_temp(egui::Id::new(("summary_toggle", month)), response.rect)
            });
            toggle |= response.clicked();
            match available {
                Ok(value) => {
                    p.text(
                        egui::pos2(card.center().x + 20., card.top() + amount_y),
                        egui::Align2::CENTER_CENTER,
                        format!("= {}", i18n::money(value, &self.b.base)),
                        crate::palette::bold(22.),
                        if value < 0 {
                            RED
                        } else if month == &current_month() {
                            crate::palette::LIME
                        } else {
                            Color32::WHITE
                        },
                    );
                    p.l_text(
                        egui::pos2(card.center().x + 20., card.top() + label_y),
                        egui::Align2::CENTER_CENTER,
                        "Доступно для распределения",
                        crate::palette::bold(11.),
                        Color32::WHITE,
                    );
                }
                Err(error) => {
                    p.l_text(
                        egui::pos2(card.center().x + 20., card.top() + amount_y),
                        egui::Align2::CENTER_CENTER,
                        "Бюджет недоступен",
                        crate::palette::bold(15.),
                        Color32::from_rgb(255, 213, 127),
                    );
                    ui.interact(
                        card,
                        ui.id().with(("missing_rate", month)),
                        egui::Sense::hover(),
                    )
                    .l_on_hover_text(error);
                }
            }
        }
        if toggle {
            self.preferences.budget_summary_collapsed = !collapsed;
            self.save_preferences();
            ui.ctx().request_repaint();
        }
    }
}
