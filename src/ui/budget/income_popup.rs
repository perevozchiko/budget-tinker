use super::*;

impl App {
    pub(super) fn month_income_popup(&mut self, ui: &mut egui::Ui, month: &str) {
        ui.set_min_width(470.);
        let title = budgettinker::income::income_label(month);
        ui.label(RichText::new(&title).size(16.));
        ui.separator();
        let rows = match self.b.income_rows(month) {
            Ok(rows) => rows,
            Err(error) => {
                ui.colored_label(RED, error);
                return;
            }
        };
        let total: i64 = rows.iter().map(|(_, value)| value).sum();
        let previous = month_shift(month, -1);
        let received = |m: &str| {
            rows.iter()
                .filter(|(t, _)| t.date.starts_with(m))
                .map(|(_, value)| value)
                .sum::<i64>()
        };
        ui.label(format!(
            "{} total income was categorized as \"{title}\".",
            i18n::money(total, &self.b.base)
        ));
        for m in [previous.as_str(), month] {
            let name = chrono::NaiveDate::parse_from_str(&format!("{m}-01"), "%Y-%m-%d")
                .unwrap()
                .format("%B")
                .to_string();
            ui.label(format!(
                "{} was received in {name}",
                i18n::money(received(m), &self.b.base)
            ));
        }
        match self.b.month_summary(month) {
            Ok(summary) if summary.income != total => {
                ui.label(format!(
                    "{} exchange revaluation and other budget adjustments",
                    i18n::money(summary.income - total, &self.b.base)
                ));
            }
            Err(error) => {
                ui.colored_label(RED, error);
            }
            _ => {}
        }
        egui::ScrollArea::both().max_height(210.).show(ui, |ui| {
            egui::Grid::new(("income_table", month))
                .striped(true)
                .min_col_width(75.)
                .show(ui, |ui| {
                    for label in ["Date", "Account", "Payee", "Memo", "Amount"] {
                        ui.strong(label);
                    }
                    ui.end_row();
                    for (t, value) in &rows {
                        ui.label(&t.date);
                        ui.label(
                            self.b
                                .account(&t.account)
                                .map(|a| a.name.as_str())
                                .unwrap_or(""),
                        );
                        ui.label(&t.payee);
                        ui.label(&t.memo);
                        ui.label(i18n::number(*value, &self.b.base));
                        ui.end_row();
                    }
                });
        });
        ui.separator();
        ui.horizontal(|ui| {
            let response = classic::button(ui, "Show in Account View");
            #[cfg(test)]
            ui.ctx()
                .data_mut(|d| d.insert_temp(egui::Id::new("income_show_accounts"), response.rect));
            if response.clicked() {
                self.page = Page::Accounts;
                self.account = None;
                self.search = format!("income:{month}");
                self.selected.clear();
                self.register_timeframe = Default::default();
                ui.close();
            }
            if classic::button(ui, "Close").clicked() {
                ui.close();
            }
        });
    }
}
