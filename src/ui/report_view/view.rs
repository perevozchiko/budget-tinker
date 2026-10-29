use super::*;

impl App {
    pub(in crate::ui) fn reports(&mut self, ui: &mut egui::Ui) {
        // Relative periods and All Dates follow the current day and transaction history.
        let previous = self.report_filter.clone();
        self.report_presets();
        if previous != self.report_filter {
            self.report_navigation.clear();
            self.report_selected_month = None;
        }
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.;
            for (index, title) in reports::TITLES.iter().enumerate() {
                if ui
                    .add(classic::Button::new(*title).selected(self.report == index))
                    .clicked()
                {
                    if self.report != index {
                        egui::Popup::close_all(ui.ctx());
                    }
                    self.switch_report(index);
                }
            }
        });
        ui.add_space(6.);
        let rect = ui.available_rect_before_wrap();
        let leaf = self
            .report_navigation
            .last()
            .is_some_and(|d| !matches!(d.scope, SpendingScope::Group(_)));
        let rail_width = if self.report == 3 || leaf {
            0.
        } else {
            (rect.width() * 0.27).clamp(205., 335.)
        };
        let main = egui::Rect::from_min_max(
            rect.min,
            egui::pos2(rect.right() - rail_width, rect.bottom()),
        );
        let rail = egui::Rect::from_min_max(main.right_top(), rect.max);
        ui.painter().rect_filled(main, 0., Color32::WHITE);
        ui.painter().rect_filled(rail, 0., RAIL);
        let mut export = false;
        let mut print = false;
        ui.scope_builder(egui::UiBuilder::new().max_rect(main.shrink(14.)), |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(reports::TITLES[self.report])
                        .size(22.)
                        .color(Color32::from_gray(88)),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    print = ui.add(classic::Button::new("Print").small()).clicked();
                    export = ui.add(classic::Button::new("Export").small()).clicked();
                });
            });
            ui.add_space(12.);
            let previous_filter = self.report_filter.clone();
            ui.push_id(
                ("report_tab_filters", self.report, self.store.path.clone()),
                |ui| self.report_filters(ui),
            );
            if previous_filter != self.report_filter {
                self.report_navigation.clear();
                self.report_selected_month = None;
            }
            ui.add_space(8.);
            ui.label(
                RichText::new(report_period_caption(&self.report_filter, &today()))
                    .size(11.)
                    .color(Color32::GRAY),
            );
            ui.add_space(6.);
            spending_navigation(ui, &mut self.report_navigation);
            match self.report_data() {
                Ok(report) => {
                    let chart = ui.available_rect_before_wrap();
                    if self
                        .report_navigation
                        .last()
                        .is_some_and(|d| !matches!(d.scope, SpendingScope::Group(_)))
                    {
                        let current = self.report_navigation.last().unwrap().clone();
                        if let Some(next) = spending_detail(ui, &report, &current, &self.b.base) {
                            self.report_navigation.push(next);
                        }
                    } else if self.report < 2 || !self.report_navigation.is_empty() {
                        let current = self.report_navigation.last();
                        let rows = report.spending_rows(
                            current.map(|d| &d.scope),
                            current.and_then(|d| d.month.as_deref()),
                        );
                        if let Some(index) = donut(ui, chart, &rows, &self.b.base) {
                            self.report_navigation.push(Drill {
                                scope: rows[index].scope.clone(),
                                label: rows[index].label.clone(),
                                month: current.and_then(|d| d.month.clone()),
                            });
                        }
                    } else if self.report == 3 {
                        income_table(ui, &report, &self.b.base);
                    } else if self.report == 2 {
                        if let Some((group, month)) =
                            spending_trend(ui, chart, &report, &self.b.base)
                        {
                            self.report_navigation.push(Drill {
                                scope: SpendingScope::Group(group.clone()),
                                label: group,
                                month: Some(month),
                            });
                        }
                    } else if let Some(month) = net_worth_chart(
                        ui,
                        chart,
                        &report,
                        &self.b.base,
                        self.report_selected_month.as_deref(),
                    ) {
                        self.report_selected_month = Some(month);
                    }
                }
                Err(error) => {
                    ui.colored_label(RED, crate::i18n::tr(ui.ctx(), error));
                }
            }
        });
        if let Ok(report) = self.report_data()
            && self.report != 3
            && !leaf
        {
            ui.scope_builder(egui::UiBuilder::new().max_rect(rail.shrink(12.)), |ui| {
                ui.set_clip_rect(rail.shrink(1.));
                ui.visuals_mut().override_text_color = Some(Color32::from_gray(238));
                ui.horizontal(|ui| {
                    ui.label(if self.report_navigation.is_empty() {
                        match self.report {
                            0 => "Category",
                            1 => "Payee",
                            _ => "Period / Summary",
                        }
                    } else {
                        "Category"
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(if self.report == 4 {
                            "Balance"
                        } else if self.report < 3 {
                            "Spending"
                        } else {
                            "Amount"
                        });
                    });
                });
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_salt("report_summary")
                    .max_height((rail.height() - 100.).max(40.))
                    .show(ui, |ui| {
                        let current = self.report_navigation.last().cloned();
                        let spending_rows = report.spending_rows(
                            current.as_ref().map(|d| &d.scope),
                            current.as_ref().and_then(|d| d.month.as_deref()),
                        );
                        let rows: Vec<_> = if self.report == 4 {
                            let selected = self
                                .report_selected_month
                                .as_ref()
                                .and_then(|month| report.months.iter().find(|m| &m.month == month))
                                .or_else(|| report.months.last());
                            selected.map_or_else(Vec::new, |month| {
                                vec![
                                    (
                                        format!("{} Assets", report_month(&month.month)),
                                        month.assets,
                                    ),
                                    ("Debts".into(), month.debts),
                                    ("Net Worth".into(), month.net_worth),
                                ]
                            })
                        } else if self.report < 2 || current.is_some() {
                            spending_rows
                                .iter()
                                .map(|r| (r.label.clone(), r.value))
                                .collect()
                        } else {
                            report.rows.clone()
                        };
                        for (index, (label, value)) in rows.iter().enumerate() {
                            let (row, response) = ui.allocate_exact_size(
                                Vec2::new(ui.available_width(), 25.),
                                egui::Sense::click(),
                            );
                            if response.clicked()
                                && let Some(row) = spending_rows
                                    .get(index)
                                    .filter(|_| self.report < 2 || current.is_some())
                            {
                                self.report_navigation.push(Drill {
                                    scope: row.scope.clone(),
                                    label: row.label.clone(),
                                    month: current.as_ref().and_then(|d| d.month.clone()),
                                });
                            }
                            let amount = i18n::money(*value, &self.b.base);
                            let label = i18n::tr(ui.ctx(), label);
                            let offset = if self.report < 2 || current.is_some() {
                                ui.painter().circle_filled(
                                    row.left_center() + Vec2::new(4., 0.),
                                    3.,
                                    spending_color(&spending_rows[index]),
                                );
                                16.
                            } else {
                                0.
                            };
                            let label_rect = egui::Rect::from_min_max(
                                row.min + Vec2::new(offset, 0.),
                                row.max - Vec2::new(95., 0.),
                            );
                            ui.painter()
                                .with_clip_rect(label_rect.intersect(ui.clip_rect()))
                                .text(
                                    label_rect.left_center(),
                                    egui::Align2::LEFT_CENTER,
                                    &label,
                                    FontId::proportional(12.),
                                    Color32::from_gray(238),
                                );
                            ui.painter().text(
                                row.right_center(),
                                egui::Align2::RIGHT_CENTER,
                                &amount,
                                FontId::proportional(12.),
                                Color32::from_gray(238),
                            );
                            response.on_hover_text(format!("{label}: {amount}"));
                        }
                    });
                ui.separator();
                let total_label = if self.report == 4 {
                    "Ending Balance"
                } else if self.report == 3 {
                    "Net Income"
                } else {
                    "Total"
                };
                let total_value = if self.report == 4 {
                    self.report_selected_month
                        .as_ref()
                        .and_then(|month| report.months.iter().find(|m| &m.month == month))
                        .or_else(|| report.months.last())
                        .map_or(0, |month| month.net_worth)
                } else {
                    self.report_navigation
                        .last()
                        .map(|d| {
                            report
                                .spending_details(&d.scope, d.month.as_deref())
                                .iter()
                                .map(|e| e.value)
                                .sum()
                        })
                        .unwrap_or(report.total)
                };
                let total = i18n::money(total_value, &self.b.base);
                if ui.available_width() < 260. {
                    ui.label(total_label);
                    ui.horizontal(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(&total).font(crate::palette::bold(20.)));
                        });
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.label(total_label);
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(&total).font(crate::palette::bold(20.)));
                        });
                    });
                }
            });
        }
        if export {
            self.export_current_report(ui.ctx());
        }
        if print {
            self.chrome.print_preview = true;
        }
        self.persist_report_preferences();
    }
}
